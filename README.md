# SenseVoice Telegram Bot

一个基于 [faster-whisper](https://github.com/SYSTRAN/faster-whisper) 的 Telegram 语音转文字机器人。
支持 **CUDA 硬件加速**，专为中文语音识别优化，具备私有化部署、权限控制及 **LLM 智能润色**功能。

## ✨ 特性 (Features)

-   🎤 **高精度中文识别**: 使用 OpenAI Whisper `large-v2` 模型 (Int8 量化)，在 7GB 显存下提供顶级的识别效果。
-   ✨ **LLM 智能润色**: 集成多种大模型 (Gemini, Claude, Groq 等) 自动纠正错别字、添加标点、优化排版。
-   🚀 **GPU 加速**: 基于 CTranslate2 推理引擎，速度比原版 Whisper 快 4 倍。
-   🔒 **权限控制**: 内置密码认证机制，只有验证通过的用户才能使用 Bot。
-   📝 **流畅体验**: 语音消息自动排队处理，实时反馈处理进度。

## 🛠️ 安装与部署 (Installation)

### 1. 环境要求
-   Linux / macOS / Windows
-   Python 3.10+
-   FFmpeg (必须安装并配置到 PATH)
-   (可选) NVIDIA GPU + CUDA Toolkit (推荐，CPU 亦可运行但较慢)

### 2. 安装依赖

推荐使用 `uv` 进行包管理：

```bash
# 安装 uv (如果未安装)
pip install uv

# 克隆项目
git clone https://github.com/your-repo/whisper-bot-zh.git
cd whisper-bot-zh

# 初始化环境并安装依赖
uv sync
```

### 3. 配置

复制环境变量模板并编辑：

```bash
cp .env.example .env
nano .env
```

#### 基础配置

```ini
BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrSTUvwxyz
ACCESS_PASSWORD=MyStrongPassword123
WHISPER_MODEL_SIZE=large-v2    # 推荐 large-v2 或 large-v3
WHISPER_COMPUTE_TYPE=int8      # 7GB 显存以下推荐 int8，大显存可用 float16
MAX_CONCURRENT_TASKS=1
LOG_LEVEL=INFO
```

#### 🧠 智能润色配置 (LLM)

Bot 支持调用 LLM 对识别结果进行二次修正（纠错、标点、分段）。支持所有兼容 OpenAI 格式或 LiteLLM 支持的模型。

**多模型故障转移 (Model Fallback):**
为了应对免费 API (如 Groq) 的 Rate Limit (429) 问题，您可以配置多个模型（用逗号分隔）。Bot 会按顺序尝试，如果前一个模型失败，自动切换到下一个。

**配置示例 (.env):**

```ini
# 推荐配置：Groq 70B 主力 -> Groq 8B 备选 -> Gemini Flash 保底
LLM_MODEL=groq/llama-3.3-70b-versatile,groq/llama-3.1-8b-instant,gemini/gemini-2.0-flash-exp

# 配置对应的 API Key
GROQ_API=your_groq_api_key
GEMINI_API=your_google_api_key

# 可选：自定义润色提示词
LLM_SYSTEM_PROMPT="你是一个中文编辑，请修正错别字并添加标点，保持原意。"
```

**推荐模型配置:**

| 厂商 | 推荐模型 ID (`LLM_MODEL`) | 优势 | 环境变量 Key |
| :--- | :--- | :--- | :--- |
| **Google** | `gemini/gemini-2.0-flash-exp` | **最佳保底**。免费额度极高，中文强。 | `GEMINI_API` |
| **Groq** | `groq/llama-3.3-70b-versatile` | **最佳主力**。速度极快，质量高，但有 TPM 限制。 | `GROQ_API` |
| **Anthropic** | `anthropic/claude-3-5-haiku-20241022` | 指令遵循能力极强，润色风格自然。 | `ANTHROPIC_API` |
| **DeepSeek** | `deepseek/deepseek-chat` | 中文语境理解最深，且价格极低。 | `OPENAI_API_KEY` (设BaseURL) |

### 4. 运行

```bash
# 首次运行会自动下载 Whisper 模型 (约 3GB)
uv run python -m src.main
```

## 📖 使用指南 (User Guide)

1.  在 Telegram 中找到你的 Bot 并点击 Start。
2.  发送 `/auth <你的密码>` 进行认证（仅需一次）。
3.  直接发送语音消息或音频文件。
4.  Bot 会首先回复 **原始识别结果**。
5.  几秒后，Bot 会回复 **AI 润色后的文本** (如果配置了 LLM)。

## ⚙️ 部署为系统服务 (Systemd)

创建文件 `/etc/systemd/system/whisper-bot.service`:

```ini
[Unit]
Description=Whisper Bot Service
After=network.target

[Service]
User=your_user
WorkingDirectory=/path/to/whisper-bot-zh
ExecStart=/path/to/whisper-bot-zh/.venv/bin/python -m src.main
Restart=always
RestartSec=10
EnvironmentFile=/path/to/whisper-bot-zh/.env

[Install]
WantedBy=multi-user.target
```

## 🧪 开发与测试

```bash
uv run pytest
```

## License

MIT