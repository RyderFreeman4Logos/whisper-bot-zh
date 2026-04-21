# Groq (OpenAI-Compatible) ASR Backend + LLM Correction Fix

**Date:** 2026-04-21
**Status:** Design — pending user approval
**Scope:** Replace local `faster-whisper` ASR with an OpenAI-compatible remote
transcription client (Groq by default). Fix the "trailing suggestion/commentary"
failure mode in LLM refinement by rewriting the system prompt and exposing
sampling knobs.

## 1. Goals

1. **Remove GPU/local-model dependency.** The bot currently needs CUDA + ~3GB
   VRAM + ~2GB of `nvidia-cudnn-cu12` / `nvidia-cublas-cu12` wheels on disk.
   After this change, it runs on any box that can reach Groq's API. If the
   user wants to serve a local Whisper (e.g., `whisper.cpp` server) later,
   they change three env vars — no code change.
2. **Stop LLM hallucinated tails.** Current system prompt only forbids
   prefaces; models still append "建议 / 补充 / 评论" blocks after the
   refined text. Rewrite the prompt to forbid trailing content as strongly as
   prefacing, and expose `temperature` / `top_p` / `max_tokens` via config so
   the operator can tune.

## 2. Non-Goals

- Keeping `faster-whisper` as a fallback. **The local path is deleted.** If
  the remote API is unreachable, transcription fails loudly. (User runs
  a local OpenAI-compatible server separately if they want offline.)
- Adding `top_k` to LLM config. `litellm`'s unified params don't include it,
  OpenAI-compat endpoints (Groq's Llama) don't accept it, and per-provider
  `extra_body` plumbing isn't worth the complexity for marginal quality gain
  over `temperature`+`top_p`. Can revisit if a concrete quality problem
  points at it.
- Changing the LLM fallback chain logic (`groq/70b → groq/8b → gemini/flash`).

## 3. Architecture

### 3.1 ASR — single OpenAI-compatible client

```
┌──────────────┐         ┌──────────────┐         ┌─────────────────┐
│   handlers   │────▶────│  AsrClient   │────▶────│ OpenAI-compat   │
│  (aiogram)   │  bytes  │ (services/   │  HTTP   │ audio API       │
│              │         │   asr.py)    │         │ (Groq default)  │
└──────────────┘         └──────────────┘         └─────────────────┘
```

`AsrClient` wraps `openai.AsyncOpenAI` — that SDK works against **any** server
speaking the OpenAI audio API (Groq, local `whisper.cpp` with OpenAI shim,
self-hosted vLLM-whisper, etc.). Switching backends = change `ASR_BASE_URL`
+ `ASR_API_KEY` + `ASR_MODEL`.

**Interface (unchanged from caller's POV):**
```python
class AsrClient:
    model: str  # e.g. "whisper-large-v3"
    async def transcribe(self, audio: BinaryIO | bytes) -> str: ...
```

**Concurrency:** keep the existing `asyncio.Semaphore(max_concurrent)` guard —
still useful for rate-limit smoothing against remote APIs, even though GPU
contention is gone.

### 3.2 LLM — prompt rewrite + tunable sampling

`services/llm.py`:
- System prompt replaced (see §5).
- User message wraps raw transcript in explicit boundary text that repeats the
  "no commentary" constraint once more (belt-and-suspenders against long
  system-prompt attenuation).
- Reads `temperature` / `top_p` / `max_tokens` from `Settings` and passes them
  to `litellm.acompletion`. Omit `top_p` / `max_tokens` from the call when
  config is empty (let provider default kick in).

## 4. Config schema

### New keys (`.env` / `config.py` / `.env.example`)

| Key                 | Default                              | Purpose                                     |
| ---                 | ---                                  | ---                                         |
| `ASR_BASE_URL`      | `https://api.groq.com/openai/v1`     | OpenAI-compat endpoint                      |
| `ASR_API_KEY`       | *(unset; falls back to `GROQ_API_KEY` env)* | Credential                           |
| `ASR_MODEL`         | `whisper-large-v3`                   | Model name on the endpoint                  |
| `ASR_LANGUAGE`      | `zh`                                 | `language` param                            |
| `ASR_PROMPT`        | `以下是一段简体中文内容:`            | `prompt` param (simplified-Chinese bias)    |
| `ASR_TEMPERATURE`   | `0`                                  | `temperature` param for transcription       |
| `LLM_TEMPERATURE`   | `0.2`                                | was hardcoded `0.3`; nudge down for fidelity |
| `LLM_TOP_P`         | *(unset → omitted from call)*        | Pass-through to `litellm`                   |
| `LLM_MAX_TOKENS`    | *(unset → omitted from call)*        | Pass-through to `litellm`                   |

### Removed keys

- `WHISPER_MODEL_SIZE`
- `WHISPER_COMPUTE_TYPE`
- `WHISPER_INITIAL_PROMPT`
- `WHISPER_VAD_FILTER`

These are local-inference-only and no longer meaningful. Users with these in
their existing `.env` see no error (pydantic `extra="ignore"` already tolerates
them); CHANGELOG calls out the removal.

### API-key fallback logic

```python
api_key = (
    settings.ASR_API_KEY
    or os.environ.get("GROQ_API_KEY")   # user's current .env already has this
    or os.environ.get("OPENAI_API_KEY")
)
if not api_key:
    raise ConfigError("No ASR API key. Set ASR_API_KEY or GROQ_API_KEY.")
```

So the user's existing `GROQ_API_KEY=gsk_...` in `~/.config/whisper-bot-zh/.env`
keeps working without edits.

## 5. LLM system prompt rewrite

```
你是一个严格的中文语音转写润色器。

输入：一段由语音识别得到的原始文本。
输出：且仅输出对输入文本的润色版本。

润色 = 只做以下 3 件事：
1. 改正错别字和语音识别错误（同音字、音近字）。
2. 补上合理的标点符号。
3. 按语义分段。

严禁（出现即视为失败）：
- 添加任何解释、建议、补充、点评、总结、备注、注释、推测、延伸、参考。
- 改写原意、删减关键信息、补全原文没说完的话。
- 输出前言（"好的"、"以下是..." 之类）。
- 输出结束标记或结语。

原文讲到哪，你就润色到哪；原文结束，你立即停止输出，不再多写一个字。
```

User-message wrapper:
```python
user_content = (
    "润色下面这段转写。严禁添加评论/建议/总结/补充，"
    "严禁改写原意，严禁替说话人补全没说完的话。直接输出润色后的文本：\n\n"
    f"{text}"
)
```

## 6. File-level changes

| File                                  | Change                                                                                          |
| ---                                   | ---                                                                                             |
| `src/whisper_bot/services/asr.py`     | Rewrite: `WhisperEngine` → `AsrClient` using `openai.AsyncOpenAI`. Drop `faster-whisper` import, drop CUDA device/compute-type fields. Keep `Semaphore` + `async transcribe`. |
| `src/whisper_bot/services/llm.py`     | New system prompt. Pass `temperature` / `top_p` / `max_tokens` from `Settings` to `acompletion`. Omit top_p / max_tokens when unset.                                         |
| `src/whisper_bot/config.py`           | Add new `ASR_*` + `LLM_TOP_P` / `LLM_MAX_TOKENS` fields. Drop `WHISPER_*` fields. Adjust `LLM_TEMPERATURE` default to `0.2`.                                                 |
| `src/whisper_bot/main.py`             | Delete `_ensure_cuda_libs_in_ld_path()` + `import nvidia.*` block (~40 LOC). Keep socket IPv4 patch. Construct `AsrClient` instead of `WhisperEngine`.                       |
| `src/whisper_bot/bot/handlers.py`     | Footer text uses `asr_client.model` (string) instead of `model_size` / `compute_type`. Pass BytesIO to `AsrClient.transcribe`.                                               |
| `pyproject.toml`                      | **Remove** `faster-whisper`, `nvidia-cudnn-cu12`, `nvidia-cublas-cu12`. **Add** `openai>=1.0`. Keep `litellm`.                                                                |
| `uv.lock`                             | Regenerated by `uv sync` after dep changes.                                                     |
| `.env.example`                        | Replace `WHISPER_*` section with `ASR_*`; add new `LLM_*` knobs with comments.                  |
| `README.md`                           | Rewrite "特性" bullet for ASR (no more GPU/CUDA/large-v2 language). Update install section.    |
| `CHANGELOG.zh.md`, `CHANGELOG.md`     | `### Breaking` section for `WHISPER_*` removal + `### 变更` for Groq default + `### 修复` for tail-hallucination fix.                                                        |
| `tests/test_asr.py`                   | Replace `WhisperEngine` mocks with `AsyncOpenAI.audio.transcriptions.create` mocks; assert the request payload.                                                              |
| `tests/test_llm.py`                   | Assert new system prompt used; assert temperature/top_p/max_tokens plumbing.                    |

## 7. Error handling

- **No API key at startup** → raise `ConfigError` in `AsrClient.__init__`,
  `main.py` catches and exits with clear message (mirroring existing missing-`BOT_TOKEN` path).
- **API call fails at runtime** → let exception propagate; existing
  `handlers.py` try/except already formats `❌ 处理出错: {e}` to Telegram user.
- **Rate-limit (429) from Groq** → `openai.AsyncOpenAI` raises
  `openai.RateLimitError`; same path as above, user sees the error. (LLM
  fallback chain stays as-is; ASR has no fallback chain by design — out of
  scope.)

## 8. Testing

- Unit tests use `unittest.mock.patch` on `openai.AsyncOpenAI` and
  `litellm.acompletion`; no network.
- Verify:
  - `AsrClient` constructs the client with correct `base_url` + `api_key`.
  - `.transcribe()` passes `model`, `language`, `prompt`, `temperature` from config.
  - `.transcribe()` passes audio as `("audio.wav", bytes, "audio/wav")`.
  - `LLMService.refine_text` uses the new system prompt verbatim.
  - `LLMService.refine_text` passes `temperature` / `top_p` / `max_tokens` iff configured.
  - Fallback chain still works when first model raises.

## 9. Migration notes (for user's existing `~/.config/whisper-bot-zh/.env`)

Current file already has `GROQ_API_KEY=gsk_...` — no changes required; will be
picked up via the fallback logic in §4. The stale `SENSEVOICE_MODEL_PATH` line
stays harmless (pydantic ignores). No action needed on user's side unless they
want to point at a non-Groq endpoint.

## 10. Open questions

None — awaiting sign-off to proceed to implementation plan.
