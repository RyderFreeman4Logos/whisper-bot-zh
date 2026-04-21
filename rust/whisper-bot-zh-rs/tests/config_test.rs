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
