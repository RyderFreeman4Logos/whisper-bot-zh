use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
#[path = "config/http.rs"]
mod http;
#[path = "config/paths.rs"]
mod paths;
#[path = "config/timeouts.rs"]
mod timeouts;
use paths::{default_cache_dir, default_data_dir};

#[derive(Debug, Clone)]
pub struct Settings {
    pub bot_token: String,
    pub access_password: String,
    pub max_concurrent_tasks: usize,
    pub log_level: String,
    pub asr_base_url: String,
    pub asr_api_key: Option<String>,
    pub asr_model: String,
    pub asr_language: String,
    pub asr_prompt: String,
    pub asr_temperature: f32,

    pub llm_model: Option<String>,
    pub llm_temperature: f32,
    pub llm_top_p: Option<f32>,
    pub llm_max_tokens: Option<u32>,
    pub llm_local_base_url: Option<String>,
    pub llm_local_api_key: Option<String>,
    pub llm_local_model: Option<String>,

    pub llm_cloud_timeout_sec: f64,
    pub llm_local_timeout_sec: f64,
    pub llm_heartbeat_interval_sec: f64,
    pub groq_api: Option<String>,
    pub gemini_api: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub deepseek_api: Option<String>,
    pub xai_api: Option<String>,
    pub zenmux_api: Option<String>,
    pub zenmux_url: Option<String>,
    pub openai_api_key: Option<String>,
    pub proxy_url: Option<String>,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl Settings {
    pub fn load(cli_env_file: Option<&Path>, cli_data_dir: Option<&Path>) -> Result<Self> {
        let env_file_path = cli_env_file.map(Path::to_path_buf).or_else(|| {
            let xdg = default_data_dir().join(".env");
            xdg.exists().then_some(xdg)
        });

        if let Some(path) = env_file_path.as_deref() {
            // Override: .env wins over inherited shell env — the Python version
            // documented this after stale shell GROQ_API_KEY shadowed the fresh
            // value in ~/.config/whisper-bot-zh/.env.
            dotenvy::from_path_override(path)
                .with_context(|| format!("loading env file {}", path.display()))?;
        }

        let settings = Self {
            bot_token: required("BOT_TOKEN")?,
            access_password: required("ACCESS_PASSWORD")?,
            max_concurrent_tasks: optional_parsed("MAX_CONCURRENT_TASKS")?.unwrap_or(1),
            log_level: optional("LOG_LEVEL").unwrap_or_else(|| "info".into()),
            asr_base_url: optional("ASR_BASE_URL")
                .unwrap_or_else(|| "https://api.groq.com/openai/v1".into()),
            asr_api_key: optional("ASR_API_KEY"),
            asr_model: optional("ASR_MODEL").unwrap_or_else(|| "whisper-large-v3".into()),
            asr_language: optional("ASR_LANGUAGE").unwrap_or_else(|| "zh".into()),
            asr_prompt: optional("ASR_PROMPT")
                .unwrap_or_else(|| "\u{4ee5}\u{4e0b}\u{662f}\u{4e00}\u{6bb5}\u{7b80}\u{4f53}\u{4e2d}\u{6587}\u{5185}\u{5bb9}:".into()),
            asr_temperature: optional_parsed("ASR_TEMPERATURE")?.unwrap_or(0.0),

            llm_model: optional("LLM_MODEL"),
            llm_temperature: optional_parsed("LLM_TEMPERATURE")?.unwrap_or(0.2),
            llm_top_p: optional_parsed("LLM_TOP_P")?,
            llm_max_tokens: optional_parsed("LLM_MAX_TOKENS")?,
            llm_local_base_url: optional("LLM_LOCAL_BASE_URL"),
            llm_local_api_key: optional("LLM_LOCAL_API_KEY"),
            llm_local_model: optional("LLM_LOCAL_MODEL"),
            llm_cloud_timeout_sec: optional_parsed("LLM_CLOUD_TIMEOUT_SEC")?.unwrap_or(120.0),
            llm_local_timeout_sec: optional_parsed("LLM_LOCAL_TIMEOUT_SEC")?.unwrap_or(1800.0),
            llm_heartbeat_interval_sec: optional_parsed("LLM_HEARTBEAT_INTERVAL_SEC")?
                .unwrap_or(20.0),
            groq_api: optional("GROQ_API").or_else(|| optional("GROQ_API_KEY")),
            gemini_api: optional("GEMINI_API"),
            anthropic_api_key: optional("ANTHROPIC_API_KEY").or_else(|| optional("ANTHROPIC_API")),
            deepseek_api: optional("DEEPSEEK_API").or_else(|| optional("DEEPSEEK_API_KEY")),
            xai_api: optional("XAI_API").or_else(|| optional("XAI_API_KEY")),
            zenmux_api: optional("ZENMUX_API"),
            zenmux_url: optional("ZENMUX_URL"),
            openai_api_key: optional("OPENAI_API_KEY"),
            proxy_url: optional("PROXY_URL"),
            data_dir: cli_data_dir
                .map(Path::to_path_buf)
                .or_else(|| optional("DATA_DIR").map(PathBuf::from))
                .unwrap_or_else(default_data_dir),
            cache_dir: optional("CACHE_DIR").map_or_else(default_cache_dir, PathBuf::from),
        };

        settings.validate()?;
        Ok(settings)
    }

    fn validate(&self) -> Result<()> {
        if self.bot_token.trim().is_empty() {
            bail!("BOT_TOKEN must not be empty");
        }
        if self.access_password.trim().is_empty() {
            bail!("ACCESS_PASSWORD must not be empty");
        }
        timeouts::validate(self)?;
        Ok(())
    }

    #[must_use]
    pub fn has_cloud(&self) -> bool {
        !self.cloud_models().is_empty()
    }

    #[must_use]
    pub fn has_local(&self) -> bool {
        self.llm_local_base_url.is_some()
            && self.llm_local_api_key.is_some()
            && self.llm_local_model.is_some()
    }

    #[must_use]
    pub fn cloud_models(&self) -> Vec<String> {
        self.llm_model
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(str::trim)
                    .filter(|m| !m.is_empty())
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn asr_effective_api_key(&self) -> Option<&str> {
        self.asr_api_key
            .as_deref()
            .or(self.groq_api.as_deref())
            .or(self.openai_api_key.as_deref())
    }

    #[must_use]
    pub fn allowed_users_file(&self) -> PathBuf {
        self.data_dir.join("allowed_users.json")
    }

    #[must_use]
    pub fn bot_token_suffix(&self) -> String {
        let token = &self.bot_token;
        if token.len() > 6 {
            format!("...{}", &token[token.len() - 6..])
        } else {
            "******".into()
        }
    }
}

fn required(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("{key} must be set in environment"))
}

fn optional(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

fn optional_parsed<T>(key: &str) -> Result<Option<T>>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match optional(key) {
        Some(s) => s
            .parse::<T>()
            .map(Some)
            .map_err(|e| anyhow!("{key}={s:?} parse error: {e}")),
        None => Ok(None),
    }
}
