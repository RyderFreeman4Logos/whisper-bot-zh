# Changelog

## [0.1.0] - 2025-11-20

### Added
- Initial project structure with `uv` and `python-telegram-bot`.
- **ASR Service**: Integrated `FunASR` SenseVoiceSmall model with CUDA support and concurrency limiting (`asyncio.Semaphore`).
- **Auth Service**: Implemented password-based authentication with persistent storage (`data/allowed_users.json`).
- **Bot Handlers**: Added `/start`, `/auth`, and Voice/Audio message handling with "Processing" queue status updates.
- **Configuration**: Environment variable management via `pydantic-settings`.
- **Tests**: Comprehensive unit/integration tests for Config, Auth, ASR (mocked), and Handlers.
- **Documentation**: Added `README.md` with deployment instructions (Systemd).
- e7a7f6f **ASR Configuration**: Added support for fine-tuning Whisper inference:
    - `WHISPER_INITIAL_PROMPT` (default: "以下是一段简体中文内容:") to guide the model for better context and Traditional/Simplified Chinese handling.
    - `WHISPER_VAD_FILTER` (default: `True`) to reduce hallucinations during silence.
    - Changed default `WHISPER_COMPUTE_TYPE` to `int8` for optimized performance on 7GB VRAM target.
- 5703d4f **LLM Post-processing**: Integrated `litellm` to provide optional AI-powered text refinement (correction, punctuation, paragraphing).
    - Supported multi-provider configuration (Gemini, Anthropic, Groq, xAI, Zenmux).
    - Implemented two-stage reply flow: first raw transcription, then refined text.
- be5153f **UX Metadata**: Added execution metadata footer to bot replies:
    - Raw transcription now includes elapsed time (e.g., `⏱️ 耗时: 00:00:02.50`).
    - LLM refined output now includes the model name used (e.g., `🤖 模型: gemini/gemini-2.0-flash-exp`).

### Fixed
- **ASR**: Added automatic model downloading from ModelScope if the local model directory is missing or empty.
- 22de607 **Runtime**: Implemented dynamic `LD_LIBRARY_PATH` patching and auto-restart mechanism in `src/main.py`. This allows `faster-whisper` (CTranslate2) to locate `nvidia-cudnn` and `nvidia-cublas` libraries installed in the virtual environment, resolving "Cannot load symbol" errors without requiring system-level CUDA installation.
- 7d8466c **Syntax**: Corrected `IndentationError` in `src/services/asr.py` and `src/bot/handlers.py` that occurred during the regex cleaning implementation. Applied consistent formatting to all files.
- 70948c8 **Reliability**: Enabled automatic retries (`num_retries=3`) in LLM service to gracefully handle Rate Limit (429) and transient network errors with exponential backoff.

### Changed
- 73334ef **Core Upgrade**: Switched ASR backend from `FunASR/SenseVoice` to `faster-whisper` to support `large-v2` model within 7GB VRAM constraints.
    - Removed `funasr`, `modelscope`, `torch` dependencies.
    - Added `faster-whisper`.
    - Updated `src/services/asr.py` to use `WhisperModel` with `float16` quantization.
- 6ccb9f4 **Refactor**: Migrated entire bot framework from `python-telegram-bot` to `aiogram` v3 to resolve persistent connection timeouts (IPv6/HTTPX issues).
    - Replaced `Handlers` with `Router` architecture.
    - Implemented `AuthMiddleware` for cleaner request interception.
- **UX**: Improved ASR output formatting:
    - Cleaned special tokens (e.g., `<|zh|>`) from transcription result.
    - Wrapped output in Markdown code blocks for monospaced font and easy copying.
    - a23693e Applied Markdown code block formatting to LLM refined output for consistency and ease of copying.