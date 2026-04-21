//! Password-based authentication + persistent `allowed_users.json` store.
//!
//! TODO: implement. See `src/whisper_bot/services/auth.py`.

pub mod storage;

pub use storage::AuthService;
