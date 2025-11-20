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

### Fixed
- **ASR**: Added automatic model downloading from ModelScope if the local model directory is missing or empty.
