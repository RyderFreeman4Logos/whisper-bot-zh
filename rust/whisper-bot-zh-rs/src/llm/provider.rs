use anyhow::{bail, Context, Result};

use crate::config::Settings;

pub struct ResolvedCloudModel {
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
}

pub fn resolve_model(settings: &Settings, configured_model: &str) -> Result<ResolvedCloudModel> {
    resolve_model_with_base_url(settings, configured_model, None)
}

fn resolve_model_with_base_url(
    settings: &Settings,
    configured_model: &str,
    base_url_override: Option<&str>,
) -> Result<ResolvedCloudModel> {
    let (provider, model_name) = parse_model(configured_model)?;
    let (base_url, api_key) = match provider {
        "groq" => (
            base_url_override
                .unwrap_or("https://api.groq.com/openai/v1")
                .to_owned(),
            required_key(
                settings.groq_api.as_deref(),
                "GROQ_API or GROQ_API_KEY",
                configured_model,
            )?,
        ),
        "gemini" => (
            base_url_override
                .unwrap_or("https://generativelanguage.googleapis.com/v1beta/openai")
                .to_owned(),
            required_key(
                settings.gemini_api.as_deref(),
                "GEMINI_API",
                configured_model,
            )?,
        ),
        "anthropic" => (
            base_url_override
                .unwrap_or("https://api.anthropic.com/v1")
                .to_owned(),
            required_key(
                settings.anthropic_api_key.as_deref(),
                "ANTHROPIC_API_KEY or ANTHROPIC_API",
                configured_model,
            )?,
        ),
        "deepseek" => (
            base_url_override
                .unwrap_or("https://api.deepseek.com/v1")
                .to_owned(),
            required_key(
                settings.deepseek_api.as_deref(),
                "DEEPSEEK_API",
                configured_model,
            )?,
        ),
        "xai" => (
            base_url_override
                .unwrap_or("https://api.x.ai/v1")
                .to_owned(),
            required_key(settings.xai_api.as_deref(), "XAI_API", configured_model)?,
        ),
        "zenmux" => (
            base_url_override
                .map(ToOwned::to_owned)
                .or_else(|| settings.zenmux_url.clone())
                .context("zenmux/ cloud models require ZENMUX_URL")?,
            required_key(
                settings.zenmux_api.as_deref(),
                "ZENMUX_API",
                configured_model,
            )?,
        ),
        unknown => bail!("unsupported cloud provider prefix `{unknown}` in `{configured_model}`"),
    };

    Ok(ResolvedCloudModel {
        base_url,
        api_key,
        model_name: model_name.to_owned(),
    })
}

fn parse_model(configured_model: &str) -> Result<(&str, &str)> {
    match configured_model.split_once('/') {
        Some((provider, model_name)) if !model_name.trim().is_empty() => Ok((provider, model_name)),
        Some((provider, _)) => {
            bail!("cloud model `{configured_model}` is missing a model name after `{provider}/`")
        }
        None => Ok(("groq", configured_model)),
    }
}

fn required_key(value: Option<&str>, env_name: &str, configured_model: &str) -> Result<String> {
    value
        .map(ToOwned::to_owned)
        .with_context(|| format!("cloud model `{configured_model}` requires {env_name}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::Result;
    use serde_json::json;
    use tempfile::tempdir;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer};

    use super::resolve_model_with_base_url;
    use crate::config::Settings;
    use crate::llm::ChatRefiner;

    #[tokio::test]
    async fn anthropic_models_resolve_to_expected_key_and_endpoint() -> Result<()> {
        let settings = settings_from_env("ANTHROPIC_API_KEY=anthropic-secret\n")?;
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer anthropic-secret"))
            .and(body_partial_json(json!({ "model": "claude-3-7-sonnet" })))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{ "message": { "content": "anthropic output" } }]
            })))
            .mount(&server)
            .await;

        let resolved = resolve_model_with_base_url(
            &settings,
            "anthropic/claude-3-7-sonnet",
            Some(server.uri().as_str()),
        )?;
        let refiner = ChatRefiner::new(
            reqwest::Client::new(),
            &resolved.base_url,
            resolved.api_key,
            &resolved.model_name,
            settings
                .llm_system_prompt
                .clone()
                .unwrap_or_else(|| crate::llm::prompt::SYSTEM_PROMPT.to_owned()),
            settings.cloud_timeout(),
            settings.heartbeat_interval(),
            settings.llm_temperature,
            settings.llm_top_p,
            settings.llm_max_tokens,
            None,
        );

        let result = refiner.refine("原始文本").await?;
        assert_eq!(result.model, "claude-3-7-sonnet");
        assert_eq!(result.text, "anthropic output");
        Ok(())
    }

    fn settings_from_env(extra: &str) -> Result<Settings> {
        let temp_dir = tempdir()?;
        let env_file = temp_dir.path().join(".env");
        fs::write(
            &env_file,
            format!(
                "BOT_TOKEN=test-token\nACCESS_PASSWORD=test-password\nLLM_MODEL=groq/test-model\n{extra}"
            ),
        )?;
        Settings::load(Some(&env_file), None)
    }
}
