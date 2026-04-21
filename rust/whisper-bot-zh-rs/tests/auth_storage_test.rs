use anyhow::Result;
use std::fs;

use tempfile::tempdir;
use whisper_bot_zh::auth::AuthService;

#[tokio::test]
async fn missing_auth_file_starts_with_empty_user_set() -> Result<()> {
    let temp_dir = tempdir()?;
    let storage_path = temp_dir.path().join("allowed_users.json");

    let service = AuthService::from_parts(storage_path, "secret".to_owned()).await?;

    assert!(!service.is_user_allowed(42).await);
    Ok(())
}

#[tokio::test]
async fn corrupt_auth_file_starts_with_empty_user_set() -> Result<()> {
    let temp_dir = tempdir()?;
    let storage_path = temp_dir.path().join("allowed_users.json");
    let corrupt_payload = "{not-json";
    tokio::fs::write(&storage_path, corrupt_payload).await?;

    let service = AuthService::from_parts(storage_path.clone(), "secret".to_owned()).await?;

    assert!(!service.is_user_allowed(42).await);
    let preserved_paths = fs::read_dir(temp_dir.path())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::result::Result<Vec<_>, std::io::Error>>()?
        .into_iter()
        .filter(|path| {
            path.file_name().is_some_and(|name| {
                name.to_string_lossy()
                    .starts_with("allowed_users.json.corrupted.")
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(preserved_paths.len(), 1);
    assert_eq!(
        tokio::fs::read_to_string(&preserved_paths[0]).await?,
        corrupt_payload
    );
    if storage_path.exists() {
        assert!(tokio::fs::read_to_string(&storage_path)
            .await?
            .trim()
            .is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn authenticate_user_keeps_memory_unchanged_when_persist_fails() -> Result<()> {
    let temp_dir = tempdir()?;
    let parent_path = temp_dir.path().join("blocked");
    let storage_path = parent_path.join("allowed_users.json");
    let service = AuthService::from_parts(storage_path, "secret".to_owned()).await?;

    tokio::fs::write(&parent_path, b"not a directory").await?;

    let error = service
        .authenticate_user(7, "secret")
        .await
        .expect_err("persist failure should surface");

    assert!(error
        .to_string()
        .contains("failed to create auth directory"));
    assert!(!service.is_user_allowed(7).await);
    Ok(())
}
