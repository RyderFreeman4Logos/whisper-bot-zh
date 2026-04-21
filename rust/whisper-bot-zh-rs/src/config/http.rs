use anyhow::{Context, Result};

use super::Settings;

impl Settings {
    /// Build the shared outbound HTTP client.
    ///
    /// # Errors
    /// Returns an error if the configured proxy URL is invalid or the client
    /// cannot be constructed.
    pub fn outbound_http_client(&self) -> Result<reqwest::Client> {
        build_http_client(self.proxy_url.as_deref())
    }

    /// Build the Telegram-specific HTTP client.
    ///
    /// # Errors
    /// Returns an error if the configured proxy URL is invalid or the Telegram
    /// client cannot be constructed.
    pub fn telegram_client(&self) -> Result<telegram_reqwest::Client> {
        build_telegram_client(self.proxy_url.as_deref())
    }
}

fn build_http_client(proxy_url: Option<&str>) -> Result<reqwest::Client> {
    apply_proxy(reqwest::Client::builder(), proxy_url)?
        .build()
        .context("failed to build outbound HTTP client")
}

fn build_telegram_client(proxy_url: Option<&str>) -> Result<telegram_reqwest::Client> {
    apply_telegram_proxy(teloxide::net::default_reqwest_settings(), proxy_url)?
        .build()
        .context("failed to build Telegram HTTP client")
}

fn apply_proxy(
    builder: reqwest::ClientBuilder,
    proxy_url: Option<&str>,
) -> Result<reqwest::ClientBuilder> {
    match proxy_url {
        Some(url) => {
            let proxy =
                reqwest::Proxy::all(url).with_context(|| format!("invalid PROXY_URL `{url}`"))?;
            Ok(builder.proxy(proxy))
        }
        None => Ok(builder),
    }
}

fn apply_telegram_proxy(
    builder: telegram_reqwest::ClientBuilder,
    proxy_url: Option<&str>,
) -> Result<telegram_reqwest::ClientBuilder> {
    match proxy_url {
        Some(url) => {
            let proxy = telegram_reqwest::Proxy::all(url)
                .with_context(|| format!("invalid PROXY_URL `{url}`"))?;
            Ok(builder.proxy(proxy))
        }
        None => Ok(builder),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::build_http_client;

    #[test]
    fn builds_http_client_with_proxy_url() -> Result<()> {
        let _client = build_http_client(Some("http://127.0.0.1:8080"))?;
        Ok(())
    }
}
