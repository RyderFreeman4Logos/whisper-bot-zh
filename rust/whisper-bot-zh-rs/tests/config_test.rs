use std::fs;

#[path = "support/env.rs"]
mod env_support;

use tempfile::tempdir;
use whisper_bot_zh::config::Settings;

use env_support::EnvGuard;

#[test]
fn cli_data_dir_overrides_env_file_data_dir() {
    let _env = EnvGuard::set(&[
        ("BOT_TOKEN", None),
        ("ACCESS_PASSWORD", None),
        ("DATA_DIR", None),
    ]);
    let temp_dir = tempdir().expect("temp dir");
    let env_file = temp_dir.path().join("bot.env");
    let cli_data_dir = temp_dir.path().join("cli-data");
    fs::write(
        &env_file,
        "BOT_TOKEN=test-token\nACCESS_PASSWORD=test-password\nDATA_DIR=/env/path\n",
    )
    .expect("write env file");

    let settings =
        Settings::load(Some(&env_file), Some(cli_data_dir.as_path())).expect("load settings");

    assert_eq!(settings.data_dir, cli_data_dir);
}

#[test]
fn rejects_non_positive_timeout_values() {
    let _env = EnvGuard::set(&[
        ("BOT_TOKEN", None),
        ("ACCESS_PASSWORD", None),
        ("LLM_CLOUD_TIMEOUT_SEC", None),
    ]);
    let temp_dir = tempdir().expect("temp dir");
    let env_file = temp_dir.path().join("bot.env");
    fs::write(
        &env_file,
        "BOT_TOKEN=test-token\nACCESS_PASSWORD=test-password\nLLM_CLOUD_TIMEOUT_SEC=-1\n",
    )
    .expect("write env file");

    let error = Settings::load(Some(&env_file), None).expect_err("invalid timeout should fail");

    assert!(error.to_string().contains("LLM_CLOUD_TIMEOUT_SEC"));
}

#[test]
fn rejects_missing_home_when_default_paths_are_needed() {
    let _env = EnvGuard::set(&[
        ("HOME", None),
        ("BOT_TOKEN", Some("test-token")),
        ("ACCESS_PASSWORD", Some("test-password")),
        ("DATA_DIR", None),
        ("CACHE_DIR", None),
    ]);

    let error = Settings::load(None, None).expect_err("missing HOME should fail");

    assert_eq!(
        error.to_string(),
        "HOME is required to resolve default data dir; pass --env-file and --data-dir explicitly if running without HOME"
    );
}
