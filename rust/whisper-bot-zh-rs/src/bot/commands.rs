use teloxide::prelude::*;

use crate::auth::AuthService;

#[must_use]
pub fn is_supported_command(text: &str) -> bool {
    command_name(text).is_some_and(|command| matches!(command, "/start" | "/auth"))
}

pub async fn handle_command(
    bot: &Bot,
    message: &Message,
    text: &str,
    auth: &AuthService,
) -> ResponseResult<()> {
    match command_name(text) {
        Some("/start") => start_command(bot, message, command_args(text), auth).await,
        Some("/auth") => auth_command(bot, message, command_args(text), auth).await,
        _ => Ok(()),
    }
}

async fn start_command(
    bot: &Bot,
    message: &Message,
    password: Option<&str>,
    auth: &AuthService,
) -> ResponseResult<()> {
    if let Some(password) = password.filter(|value| !value.is_empty()) {
        return authenticate(bot, message, password, auth).await;
    }

    bot.send_message(
        message.chat.id,
        "👋 欢迎使用 Whisper 语音转文字机器人！\n\n请先使用 /start <password> 或 /auth <password> 完成认证，然后发送语音或音频消息。",
    )
    .await?;
    Ok(())
}

async fn auth_command(
    bot: &Bot,
    message: &Message,
    password: Option<&str>,
    auth: &AuthService,
) -> ResponseResult<()> {
    let Some(password) = password.filter(|value| !value.is_empty()) else {
        bot.send_message(message.chat.id, "⚠️ 请输入密码，格式：/auth <password>")
            .await?;
        return Ok(());
    };

    authenticate(bot, message, password, auth).await
}

async fn authenticate(
    bot: &Bot,
    message: &Message,
    password: &str,
    auth: &AuthService,
) -> ResponseResult<()> {
    let Some(user) = message.from.as_ref() else {
        return Ok(());
    };

    let ok = auth
        .authenticate_user(user.id.0, password)
        .await
        .unwrap_or(false);
    let reply = if ok {
        "✅ 认证成功！现在可以发送语音或音频消息了。"
    } else {
        "❌ 认证失败，密码错误。"
    };

    bot.send_message(message.chat.id, reply).await?;
    Ok(())
}

fn command_name(text: &str) -> Option<&str> {
    let token = text.split_whitespace().next()?;
    Some(token.split_once('@').map_or(token, |(name, _)| name))
}

fn command_args(text: &str) -> Option<&str> {
    text.split_once(char::is_whitespace)
        .map(|(_, args)| args.trim())
        .filter(|args| !args.is_empty())
}
