use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use tokio::sync::{Mutex, RwLock};

use crate::config::Settings;

#[derive(Clone)]
pub struct AuthService {
    password: Arc<String>,
    storage_path: Arc<PathBuf>,
    allowed_users: Arc<RwLock<BTreeSet<u64>>>,
    save_gate: Arc<Mutex<()>>,
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
            save_gate: Arc::new(Mutex::new(())),
        })
    }

    pub async fn authenticate_user(&self, user_id: u64, password: &str) -> Result<bool> {
        if password != self.password.as_str() {
            tracing::warn!(user_id, "authentication failed");
            return Ok(false);
        }

        let _save_guard = self.save_gate.lock().await;
        let snapshot = {
            let mut snapshot = self.allowed_users.read().await.clone();
            snapshot.insert(user_id);
            snapshot
        };
        persist_users(self.storage_path.as_path(), &snapshot).await?;
        *self.allowed_users.write().await = snapshot;
        Ok(true)
    }

    pub async fn is_user_allowed(&self, user_id: u64) -> bool {
        self.allowed_users.read().await.contains(&user_id)
    }
}

async fn load_users(path: &Path) -> Result<BTreeSet<u64>> {
    let raw = match tokio::fs::read_to_string(path).await {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read auth file {}", path.display()));
        }
    };

    match parse_users(&raw) {
        Ok(users) => Ok(users),
        Err(error) => preserve_corrupt_auth_file(path, error).await,
    }
}

fn parse_users(raw: &str) -> Result<BTreeSet<u64>> {
    let parsed = serde_json::from_str::<Vec<u64>>(raw).context("invalid auth json payload")?;
    Ok(parsed.into_iter().collect())
}

async fn persist_users(path: &Path, user_ids: &BTreeSet<u64>) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create auth directory {}", parent.display()))?;
    }

    let payload = serde_json::to_vec_pretty(&user_ids.iter().copied().collect::<Vec<_>>())
        .context("failed to serialize auth file")?;
    let temp_path = pending_auth_path(path);
    #[cfg(test)]
    maybe_delay_persist_for_tests(user_ids.len()).await;
    #[cfg(not(test))]
    maybe_delay_persist_for_tests(user_ids.len());
    tokio::fs::write(&temp_path, payload)
        .await
        .with_context(|| format!("failed to write auth file {}", temp_path.display()))?;
    tokio::fs::rename(&temp_path, path)
        .await
        .with_context(|| format!("failed to replace auth file {}", path.display()))?;
    Ok(())
}

fn pending_auth_path(path: &Path) -> PathBuf {
    path.with_extension("tmp")
}

async fn preserve_corrupt_auth_file(path: &Path, error: anyhow::Error) -> Result<BTreeSet<u64>> {
    let preserved_path = corrupted_auth_path(path)?;
    tokio::fs::rename(path, &preserved_path)
        .await
        .with_context(|| {
            format!(
                "failed to preserve corrupt auth file {} as {} after parse error: {error}",
                path.display(),
                preserved_path.display()
            )
        })?;

    tracing::warn!(
        path = %path.display(),
        preserved_path = %preserved_path.display(),
        error = %error,
        "failed to parse auth file; preserved corrupt file and starting with empty allowlist"
    );
    Ok(BTreeSet::new())
}

fn corrupted_auth_path(path: &Path) -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_secs();
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow!("auth path {} has no file name", path.display()))?;
    let mut preserved_name = OsString::from(file_name);
    preserved_name.push(format!(".corrupted.{timestamp}"));
    Ok(path.with_file_name(preserved_name))
}

#[cfg(test)]
async fn maybe_delay_persist_for_tests(user_count: usize) {
    if user_count == 1 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    } else {
        tokio::task::yield_now().await;
    }
}

#[cfg(not(test))]
fn maybe_delay_persist_for_tests(_user_count: usize) {}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use tempfile::tempdir;

    use super::{parse_users, AuthService};

    #[tokio::test]
    async fn concurrent_authentication_persists_all_users() -> Result<()> {
        let temp_dir = tempdir()?;
        let storage_path = temp_dir.path().join("allowed_users.json");
        let service = AuthService::from_parts(storage_path.clone(), "secret".to_owned()).await?;

        let first = {
            let service = service.clone();
            tokio::spawn(async move { service.authenticate_user(1, "secret").await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let second = {
            let service = service.clone();
            tokio::spawn(async move { service.authenticate_user(2, "secret").await })
        };

        assert!(first.await??);
        assert!(second.await??);
        assert!(service.is_user_allowed(1).await);
        assert!(service.is_user_allowed(2).await);

        let persisted = tokio::fs::read_to_string(&storage_path).await?;
        let stored_users = parse_users(&persisted)?;
        assert_eq!(stored_users.into_iter().collect::<Vec<_>>(), vec![1, 2]);
        Ok(())
    }
}
