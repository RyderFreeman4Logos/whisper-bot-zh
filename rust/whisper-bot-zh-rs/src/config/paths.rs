use std::path::PathBuf;

pub fn default_data_dir() -> PathBuf {
    home_dir().join(".config").join("whisper-bot-zh")
}

pub fn default_cache_dir() -> PathBuf {
    home_dir().join(".cache").join("whisper-bot-zh")
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map_or_else(|| PathBuf::from("/tmp"), PathBuf::from)
}
