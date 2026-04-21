use anyhow::Result;
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
    tokio::fs::write(&storage_path, b"{not-json").await?;

    let service = AuthService::from_parts(storage_path, "secret".to_owned()).await?;

    assert!(!service.is_user_allowed(42).await);
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
