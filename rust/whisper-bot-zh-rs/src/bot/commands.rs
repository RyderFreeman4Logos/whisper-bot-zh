use teloxide::prelude::*;

use crate::auth::AuthService;

use super::render::TelegramGovernor;

const AUTH_SUCCESS_REPLY: &str = "✅ 认证成功！现在可以发送语音或音频消息了。";
const AUTH_INVALID_PASSWORD_REPLY: &str = "❌ 认证失败，密码错误。";
const AUTH_INTERNAL_ERROR_REPLY: &str = "❌ 认证失败：内部错误，请稍后重试。";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuthOutcome {
    Success,
    InvalidPassword,
    InternalError,
}

#[must_use]
pub fn is_supported_command(text: &str, bot_username: &str) -> bool {
    command_name(text, bot_username).is_some_and(|command| matches!(command, "/start" | "/auth"))
}

/// Handle a supported command message.
///
/// # Errors
/// Returns an error if Telegram rejects any reply sent while processing the
/// command.
pub async fn handle_command(
    governor: &TelegramGovernor,
    message: &Message,
    text: &str,
    bot_username: &str,
    auth: &AuthService,
) -> ResponseResult<()> {
    match command_name(text, bot_username) {
        Some("/start") => start_command(governor, message, command_args(text), auth).await,
        Some("/auth") => auth_command(governor, message, command_args(text), auth).await,
        _ => Ok(()),
    }
}

async fn start_command(
    governor: &TelegramGovernor,
    message: &Message,
    password: Option<&str>,
    auth: &AuthService,
) -> ResponseResult<()> {
    if let Some(password) = password.filter(|value| !value.is_empty()) {
        return authenticate(governor, message, password, auth).await;
    }

    governor
        .send_message(
            message.chat.id,
            "👋 欢迎使用 Whisper 语音转文字机器人！\n\n请先使用 /start <password> 或 /auth <password> 完成认证，然后发送语音或音频消息。",
        )
        .await?;
    Ok(())
}

async fn auth_command(
    governor: &TelegramGovernor,
    message: &Message,
    password: Option<&str>,
    auth: &AuthService,
) -> ResponseResult<()> {
    let Some(password) = password.filter(|value| !value.is_empty()) else {
        governor
            .send_message(message.chat.id, "⚠️ 请输入密码，格式：/auth <password>")
            .await?;
        return Ok(());
    };

    authenticate(governor, message, password, auth).await
}

async fn authenticate(
    governor: &TelegramGovernor,
    message: &Message,
    password: &str,
    auth: &AuthService,
) -> ResponseResult<()> {
    let Some(user) = message.from.as_ref() else {
        return Ok(());
    };

    let outcome = match auth.authenticate_user(user.id.0, password).await {
        Ok(true) => AuthOutcome::Success,
        Ok(false) => AuthOutcome::InvalidPassword,
        Err(error) => {
            tracing::error!(
                user_id = user.id.0,
                error = %error,
                "authentication state persistence failed"
            );
            AuthOutcome::InternalError
        }
    };

    governor
        .send_message(message.chat.id, auth_reply(outcome))
        .await?;
    Ok(())
}

fn command_name<'a>(text: &'a str, bot_username: &str) -> Option<&'a str> {
    let token = text.split_whitespace().next()?;
    match token.split_once('@') {
        Some((_name, target)) if !target.eq_ignore_ascii_case(bot_username) => None,
        Some((name, _)) => Some(name),
        None => Some(token),
    }
}

fn command_args(text: &str) -> Option<&str> {
    text.split_once(char::is_whitespace)
        .map(|(_, args)| args.trim())
        .filter(|args| !args.is_empty())
}

fn auth_reply(outcome: AuthOutcome) -> &'static str {
    match outcome {
        AuthOutcome::Success => AUTH_SUCCESS_REPLY,
        AuthOutcome::InvalidPassword => AUTH_INVALID_PASSWORD_REPLY,
        AuthOutcome::InternalError => AUTH_INTERNAL_ERROR_REPLY,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        auth_reply, command_name, AuthOutcome, AUTH_INTERNAL_ERROR_REPLY,
        AUTH_INVALID_PASSWORD_REPLY,
    };

    #[test]
    fn internal_error_reply_is_distinct_from_wrong_password() {
        assert_eq!(
            auth_reply(AuthOutcome::InternalError),
            AUTH_INTERNAL_ERROR_REPLY
        );
        assert_ne!(
            auth_reply(AuthOutcome::InternalError),
            AUTH_INVALID_PASSWORD_REPLY
        );
    }

    #[test]
    fn ignores_commands_addressed_to_other_bots() {
        assert_eq!(command_name("/auth@other_bot secret", "this_bot"), None);
    }

    #[test]
    fn accepts_commands_addressed_to_this_bot() {
        assert_eq!(
            command_name("/auth@this_bot secret", "this_bot"),
            Some("/auth")
        );
    }
}
