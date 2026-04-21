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
fn data_dir_from_env_file_is_used() {
    let _env = EnvGuard::set(&[
        ("HOME", None),
        ("BOT_TOKEN", None),
        ("ACCESS_PASSWORD", None),
        ("DATA_DIR", None),
        ("CACHE_DIR", None),
    ]);
    let temp_dir = tempdir().expect("temp dir");
    let data_dir = temp_dir.path().join("env-data");
    let cache_dir = temp_dir.path().join("env-cache");
    let env_file = temp_dir.path().join("bot.env");
    fs::write(
        &env_file,
        format!(
            "BOT_TOKEN=x\nACCESS_PASSWORD=y\nDATA_DIR={}\nCACHE_DIR={}\n",
            data_dir.display(),
            cache_dir.display()
        ),
    )
    .expect("write env file");

    let settings = Settings::load(Some(&env_file), None).expect("load settings");

    assert_eq!(settings.data_dir, data_dir);
    assert_eq!(settings.cache_dir, cache_dir);
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

#[test]
fn env_only_runs_without_home_when_explicit_data_dir_set() {
    let temp_dir = tempdir().expect("temp dir");
    let data_dir = temp_dir.path().join("data");
    let cache_dir = temp_dir.path().join("cache");
    let _env = EnvGuard::set(&[
        ("HOME", None),
        ("BOT_TOKEN", Some("test-token")),
        ("ACCESS_PASSWORD", Some("test-password")),
        ("DATA_DIR", Some(data_dir.to_str().expect("utf-8 path"))),
        ("CACHE_DIR", Some(cache_dir.to_str().expect("utf-8 path"))),
    ]);

    let settings = Settings::load(None, None).expect("explicit env paths should bypass HOME");

    assert_eq!(settings.data_dir, data_dir);
    assert_eq!(settings.cache_dir, cache_dir);
}

#[test]
fn cli_data_dir_bypasses_home() {
    let temp_dir = tempdir().expect("temp dir");
    let cli_data_dir = temp_dir.path().join("data");
    let cache_dir = temp_dir.path().join("cache");
    let _env = EnvGuard::set(&[
        ("HOME", None),
        ("BOT_TOKEN", Some("test-token")),
        ("ACCESS_PASSWORD", Some("test-password")),
        ("DATA_DIR", None),
        ("CACHE_DIR", Some(cache_dir.to_str().expect("utf-8 path"))),
    ]);

    let settings = Settings::load(None, Some(cli_data_dir.as_path()))
        .expect("cli data dir should bypass HOME");

    assert_eq!(settings.data_dir, cli_data_dir);
    assert_eq!(settings.cache_dir, cache_dir);
}

#[test]
fn cli_data_dir_wins_when_finding_env_file() {
    let temp_dir = tempdir().expect("temp dir");
    let env_data_dir = temp_dir.path().join("env-data");
    let cli_data_dir = temp_dir.path().join("cli-data");
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&env_data_dir).expect("create env data dir");
    std::fs::create_dir_all(&cli_data_dir).expect("create cli data dir");
    std::fs::write(
        env_data_dir.join(".env"),
        "BOT_TOKEN=env-token\nACCESS_PASSWORD=env-password\n",
    )
    .expect("write env data dir .env");
    std::fs::write(
        cli_data_dir.join(".env"),
        "BOT_TOKEN=cli-token\nACCESS_PASSWORD=cli-password\n",
    )
    .expect("write cli data dir .env");

    let _env = EnvGuard::set(&[
        ("HOME", None),
        (
            "DATA_DIR",
            Some(env_data_dir.to_str().expect("utf-8 env data dir")),
        ),
        (
            "CACHE_DIR",
            Some(cache_dir.to_str().expect("utf-8 cache dir")),
        ),
        ("BOT_TOKEN", None),
        ("ACCESS_PASSWORD", None),
    ]);

    let settings = Settings::load(None, Some(cli_data_dir.as_path()))
        .expect("cli data dir should control env-file discovery");

    assert_eq!(settings.bot_token, "cli-token");
    assert_eq!(settings.access_password, "cli-password");
    assert_eq!(settings.data_dir, cli_data_dir);
}
