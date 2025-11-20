# SenseVoice Telegram Bot

一个基于 [FunAudioLLM/SenseVoice](https://github.com/FunAudioLLM/SenseVoice) 的 Telegram 语音转文字机器人。
支持 **CUDA 硬件加速**，专为中文语音识别优化，具备私有化部署和权限控制功能。

## ✨ 特性 (Features)

-   🎤 **高精度中文识别**: 使用阿里 SenseVoiceSmall 模型，识别准确率高，速度快。
-   🚀 **GPU 加速**: 自动检测并使用 NVIDIA CUDA 进行推理。
-   🔒 **权限控制**: 内置密码认证机制，只有验证通过的用户才能使用 Bot。
-   📝 **流畅体验**: 语音消息自动排队处理，处理进度实时更新 (Reply 方式回复)。
-   📂 **多格式支持**: 支持 Telegram 语音消息及普通音频文件。

## 🛠️ 安装与部署 (Installation)

### 1. 环境要求
-   Linux / macOS / Windows
-   Python 3.10+
-   FFmpeg (必须安装并配置到 PATH)
-   (可选) NVIDIA GPU + CUDA Toolkit (推荐，CPU 亦可运行但较慢)

### 2. 安装依赖

推荐使用 `uv` 进行包管理（比 pip 快且稳）：

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

`.env` 文件说明：

```ini
BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrSTUvwxyz # 从 @BotFather 获取
ACCESS_PASSWORD=MyStrongPassword123         # 设置一个强密码，用户需用此认证
SENSEVOICE_MODEL_PATH=models/sensevoice     # 模型下载路径
MAX_CONCURRENT_TASKS=1                      # 并发处理任务数 (显存小建议设为 1)
LOG_LEVEL=INFO
```

### 4. 运行

```bash
# 首次运行会自动下载 SenseVoice 模型 (约 500MB+)
uv run python src/main.py
```

## 📖 使用指南 (User Guide)

1.  在 Telegram 中找到你的 Bot 并点击 Start。
2.  发送 `/auth <你的密码>` 进行认证（仅需一次）。
    -   例如: `/auth MyStrongPassword123`
3.  认证成功后，直接发送语音消息或音频文件，Bot 会自动回复转写结果。

## ⚙️ 部署为系统服务 (Systemd)

在 Linux 上，建议配置为 Systemd 服务以实现开机自启。

创建文件 `/etc/systemd/system/whisper-bot.service`:

```ini
[Unit]
Description=Whisper SenseVoice Bot
After=network.target

[Service]
# 修改为你的用户名和项目路径
User=your_user
WorkingDirectory=/path/to/whisper-bot-zh
# 确保使用的是 uv 创建的 venv 中的 python
ExecStart=/path/to/whisper-bot-zh/.venv/bin/python src/main.py
Restart=always
RestartSec=10
EnvironmentFile=/path/to/whisper-bot-zh/.env

[Install]
WantedBy=multi-user.target
```

启用并启动服务：

```bash
sudo systemctl daemon-reload
sudo systemctl enable whisper-bot
sudo systemctl start whisper-bot
sudo systemctl status whisper-bot
```

## 🧪 开发与测试

本项目采用测试驱动开发 (TDD)。

```bash
# 运行测试
uv run pytest
```

## License

MIT
