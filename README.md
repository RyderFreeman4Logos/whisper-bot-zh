# Whisper Telegram Bot (zh)

> ⚠️ **免责声明 / Disclaimer**
>
> 本项目代码由 LLM（Gemini 3 Pro Preview / Claude Opus 4.7）在作者指导下生成。
> 作者提供设计思路与提示词。虽然风险极低，**使用者需自行承担所有运行风险**。
> 欢迎社区提交 PR / Issue。

通过任意 **OpenAI 兼容的 Whisper 服务**（默认 [Groq Whisper large-v3](https://console.groq.com/docs/speech-text)）把 Telegram 语音消息转成中文文字的机器人，支持 LLM 自动纠错 / 标点 / 分段。

## ✨ 特性

-   🎤 **零 GPU 部署**：默认调用 Groq 托管的 Whisper large-v3，**不需要本地显卡 / CUDA / 模型下载**。
-   🔌 **后端可切换**：改 3 个 env (`ASR_BASE_URL` / `ASR_API_KEY` / `ASR_MODEL`) 即可指向任何 OpenAI 兼容端点（自建 whisper.cpp server、vLLM-whisper、其他云厂商等）。
-   ✨ **LLM 智能润色**：集成 `litellm`，支持 Groq / Gemini / Anthropic 等多厂商 fallback 链，自动纠错、加标点、分段。system prompt 严格禁止"尾巴幻觉"（LLM 擅自追加建议/补充）。
-   🔒 **密码认证**：`/auth <password>` 认证后用户 ID 落盘，重启依然有效。
-   📝 **流畅体验**：语音消息排队处理，实时反馈进度；超长文本自动转成 `.txt` 文件发送。

## 🛠️ 安装与部署

### 方法一：CLI 工具（推荐）

```bash
pip install uv
uv tool install git+https://github.com/RyderFreeman4Logos/whisper-bot-zh.git
```

准备配置：
```bash
mkdir -p ~/.config/whisper-bot-zh
curl -o ~/.config/whisper-bot-zh/.env \
  https://raw.githubusercontent.com/RyderFreeman4Logos/whisper-bot-zh/main/.env.example
nano ~/.config/whisper-bot-zh/.env  # 填入 BOT_TOKEN / ACCESS_PASSWORD / GROQ_API_KEY
```

运行：
```bash
whisper-bot-zh
```

默认目录：
- 配置：`~/.config/whisper-bot-zh/`
- 缓存：`~/.cache/whisper-bot-zh/`

自定义路径：
```bash
whisper-bot-zh --env-file /opt/bot/.env --data-dir /mnt/data/bot
```

### 方法二：源码开发

```bash
git clone https://github.com/RyderFreeman4Logos/whisper-bot-zh.git
cd whisper-bot-zh
uv sync
cp .env.example .env
nano .env
uv run python -m whisper_bot.main
```

要求：Python 3.10+、FFmpeg 已装在 PATH。

上面这组 `cp .env.example .env` / `uv run python -m whisper_bot.main` 是 Python 源码开发流程。
Rust CLI / `cargo run` 不会默认读取项目根目录 `.env`；Rust 运行时只会读取
`~/.config/whisper-bot-zh/.env`，或显式传入 `--env-file /path/to/.env`。

## ⚙️ 配置

### 基础

```ini
BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrSTUvwxyz
ACCESS_PASSWORD=MyStrongPassword123
```

### ASR 端点（默认 Groq）

```ini
ASR_BASE_URL=https://api.groq.com/openai/v1
ASR_API_KEY=gsk_...          # 可留空，回落到 GROQ_API_KEY / OPENAI_API_KEY
ASR_MODEL=whisper-large-v3
ASR_LANGUAGE=zh
ASR_PROMPT=以下是一段简体中文内容:
ASR_TEMPERATURE=0.0
```

**切到本地 / 其他端点**只需改前三个值，例如指向自建 `whisper.cpp` HTTP server：
```ini
ASR_BASE_URL=http://localhost:9000/v1
ASR_API_KEY=anything
ASR_MODEL=whisper-large-v3
```

### LLM 润色

```ini
# 按优先级逗号分隔，前一个限流/失败自动切下一个
LLM_MODEL=groq/llama-3.3-70b-versatile,groq/llama-3.1-8b-instant,gemini/gemini-2.0-flash-exp
GROQ_API=gsk_...
GEMINI_API=AIza...
ANTHROPIC_API_KEY=sk-ant-...
DEEPSEEK_API=sk-...
XAI_API=xai-...
# ZENMUX_URL=https://your-zenmux.example/v1
# ZENMUX_API=...

# 调优旋钮
LLM_TEMPERATURE=0.2   # 越低越保真
# LLM_TOP_P=0.9
# LLM_MAX_TOKENS=4096
```

| 厂商   | 推荐模型 ID                          | 特点                          |
| :----- | :----------------------------------- | :---------------------------- |
| Groq   | `groq/llama-3.3-70b-versatile`       | 速度极快；TPM 限额严格        |
| Google | `gemini/gemini-2.0-flash-exp`        | 免费额度高，中文处理稳定      |

## ⚙️ Systemd 部署

```ini
[Unit]
Description=Whisper Bot Service
After=network.target

[Service]
User=your_user
ExecStart=/home/your_user/.local/bin/whisper-bot-zh
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

## License

Apache 2.0
