use std::sync::Arc;

use crate::config::Settings;

/// Placeholder auth service backed by `allowed_users.json`.
#[derive(Clone)]
pub struct AuthService {
    #[allow(dead_code)]
    settings: Arc<Settings>,
}

impl AuthService {
    #[must_use]
    pub fn new(settings: Arc<Settings>) -> Self {
        Self { settings }
    }
}
