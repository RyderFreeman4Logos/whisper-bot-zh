//! Password-based authentication + persistent `allowed_users.json` storage.

pub mod storage;

pub use storage::AuthService;
