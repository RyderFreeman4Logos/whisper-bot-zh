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

### Changed
- 6ccb9f4 **Refactor**: Migrated entire bot framework from `python-telegram-bot` to `aiogram` v3 to resolve persistent connection timeouts (IPv6/HTTPX issues).
    - Replaced `Handlers` with `Router` architecture.
    - Implemented `AuthMiddleware` for cleaner request interception.
- **UX**: Improved ASR output formatting:
    - Cleaned special tokens (e.g., `<|zh|>`) from transcription result.
    - Wrapped output in Markdown code blocks for monospaced font and easy copying.