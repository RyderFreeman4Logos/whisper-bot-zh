# SenseVoice Telegram Bot

> ⚠️ **免责声明 / Disclaimer**
> 
> 本项目代码完全由 **Gemini 3 Pro Preview** 模型在用户的 Prompt 指导下自动生成。作者仅提供了设计思路与提示词。
> 
> 虽然作者认为本项目风险极低（即使出问题也不太可能造成实质损失），但**使用者需自行承担所有运行风险**。我们非常欢迎社区开发者提交 PR 或 Issue 进行代码审查 (Review) 与改进，共同完善这个项目。

一个基于 [faster-whisper](https://github.com/SYSTRAN/faster-whisper) 的 Telegram 语音转文字机器人。
支持 **CUDA 硬件加速**，专为中文语音识别优化，具备私有化部署、权限控制及 **LLM 智能润色**功能。

## ✨ 特性 (Features)

-   🎤 **高精度中文识别**: 使用 OpenAI Whisper `large-v2` 模型 (Int8 量化)，在 7GB 显存下提供顶级的识别效果。
-   ✨ **LLM 智能润色**: 集成多种大模型 (Gemini, Claude, Groq 等) 自动纠正错别字、添加标点、优化排版。
-   🚀 **GPU 加速**: 基于 CTranslate2 推理引擎，速度比原版 Whisper 快 4 倍。
-   🔒 **权限控制**: 内置密码认证机制，只有验证通过的用户才能使用 Bot。
-   📝 **流畅体验**: 语音消息自动排队处理，实时反馈处理进度。
-   🛠️ **开箱即用**: 支持作为 CLI 工具直接安装，无需复杂的环境配置。

## 🛠️ 安装与部署 (Installation)

### 方法一：快速安装 (CLI 工具) 🚀

最简单的使用方式，适合普通用户。

1.  **安装 uv**:
    ```bash
    pip install uv
    ```

2.  **安装 Bot**:
    ```bash
    # 直接从 GitHub 安装到系统隔离环境
    uv tool install git+https://github.com/your-repo/whisper-bot-zh.git
    ```

3.  **准备配置**:
    默认配置文件路径为 `~/.config/whisper-bot-zh/`。
    
    ```bash
    mkdir -p ~/.config/whisper-bot-zh
    # 下载示例配置 (请替换为实际的 .env.example 链接或手动创建)
    curl -o ~/.config/whisper-bot-zh/.env https://raw.githubusercontent.com/your-repo/whisper-bot-zh/main/.env.example
    nano ~/.config/whisper-bot-zh/.env
    ```

4.  **运行**:
    ```bash
    whisper-bot-zh
    ```
    
    *默认数据目录:*
    *   配置: `~/.config/whisper-bot-zh`
    *   模型/缓存: `~/.cache/whisper-bot-zh`

    *自定义路径运行:*
    ```bash
    whisper-bot-zh --env-file /opt/bot/.env --model-dir /mnt/data/models
    ```

---

### 方法二：源码部署 (开发者) 🛠️

适合需要二次开发或调试的用户。

1.  **环境要求**:
    -   Linux / macOS / Windows
    -   Python 3.10+
    -   FFmpeg (必须安装并配置到 PATH)
    -   (可选) NVIDIA GPU + CUDA Toolkit

2.  **安装依赖**:
    ```bash
    git clone https://github.com/your-repo/whisper-bot-zh.git
    cd whisper-bot-zh
    uv sync
    ```

3.  **配置**:
    ```bash
    cp .env.example .env
    nano .env
    ```

4.  **运行**:
    ```bash
    # 首次运行会自动下载 Whisper 模型 (约 3GB)
    uv run python -m src.main
    ```

## ⚙️ 配置详解 (Configuration)

### 基础配置 (.env)

```ini
BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrSTUvwxyz
ACCESS_PASSWORD=MyStrongPassword123
WHISPER_MODEL_SIZE=large-v2    # 推荐 large-v2 或 large-v3
WHISPER_COMPUTE_TYPE=int8      # 7GB 显存以下推荐 int8，大显存可用 float16
```

### 🧠 智能润色配置 (LLM)

Bot 支持调用 LLM 对识别结果进行二次修正（纠错、标点、分段）。

**多模型故障转移 (Model Fallback):**
为了应对免费 API (如 Groq) 的 Rate Limit (429) 问题，您可以配置多个模型（用逗号分隔）。Bot 会按顺序尝试，如果前一个模型失败，自动切换到下一个。

```ini
# 推荐配置：Groq 70B 主力 -> Groq 8B 备选 -> Gemini Flash 保底
LLM_MODEL=groq/llama-3.3-70b-versatile,groq/llama-3.1-8b-instant,gemini/gemini-2.0-flash-exp

# 配置对应的 API Key
GROQ_API=your_groq_api_key
GEMINI_API=your_google_api_key
```

| 厂商 | 推荐模型 ID | 优势 |
| :--- | :--- | :--- |
| **Google** | `gemini/gemini-2.0-flash-exp` | **最佳保底**。免费额度极高，中文强。 |
| **Groq** | `groq/llama-3.3-70b-versatile` | **最佳主力**。速度极快，TPM 限制较严。 |

## ⚙️ 系统服务部署 (Systemd)

如果您使用 `uv tool install` 安装，Systemd 配置如下：

```ini
[Unit]
Description=Whisper Bot Service
After=network.target

[Service]
User=your_user
# 指向 uv 安装的二进制文件位置 (通常在 ~/.local/bin 或 /root/.local/bin)
ExecStart=/home/your_user/.local/bin/whisper-bot-zh
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

## License

Apache 2.0
