use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::RwLock;

use crate::config::Settings;

#[derive(Clone)]
pub struct AuthService {
    password: Arc<String>,
    storage_path: Arc<PathBuf>,
    allowed_users: Arc<RwLock<BTreeSet<u64>>>,
}

impl AuthService {
    pub async fn new(settings: &Settings) -> Result<Self> {
        Self::from_parts(
            settings.allowed_users_file(),
            settings.access_password.clone(),
        )
        .await
    }

    pub async fn from_parts(storage_path: PathBuf, password: String) -> Result<Self> {
        let allowed_users = load_users(&storage_path).await?;
        Ok(Self {
            password: Arc::new(password),
            storage_path: Arc::new(storage_path),
            allowed_users: Arc::new(RwLock::new(allowed_users)),
        })
    }

    pub async fn authenticate_user(&self, user_id: u64, password: &str) -> Result<bool> {
        if password != self.password.as_str() {
            tracing::warn!(user_id, "authentication failed");
            return Ok(false);
        }

        let snapshot = {
            let mut guard = self.allowed_users.write().await;
            guard.insert(user_id);
            guard.iter().copied().collect::<Vec<_>>()
        };

        persist_users(self.storage_path.as_path(), &snapshot).await?;
        Ok(true)
    }

    pub async fn is_user_allowed(&self, user_id: u64) -> bool {
        self.allowed_users.read().await.contains(&user_id)
    }
}

async fn load_users(path: &Path) -> Result<BTreeSet<u64>> {
    match tokio::fs::read_to_string(path).await {
        Ok(raw) => match parse_users(&raw) {
            Ok(users) => Ok(users),
            Err(error) => {
                tracing::error!(path = %path.display(), %error, "failed to parse auth file");
                Ok(BTreeSet::new())
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeSet::new()),
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "failed to read auth file");
            Ok(BTreeSet::new())
        }
    }
}

fn parse_users(raw: &str) -> Result<BTreeSet<u64>> {
    let parsed = serde_json::from_str::<Vec<u64>>(raw).context("invalid auth json payload")?;
    Ok(parsed.into_iter().collect())
}

async fn persist_users(path: &Path, user_ids: &[u64]) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create auth directory {}", parent.display()))?;
    }

    let payload = serde_json::to_vec(user_ids).context("failed to serialize auth file")?;
    tokio::fs::write(path, payload)
        .await
        .with_context(|| format!("failed to write auth file {}", path.display()))?;
    Ok(())
}
