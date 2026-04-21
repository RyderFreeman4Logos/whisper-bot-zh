use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bytes::Bytes;
use futures::TryStreamExt;
use teloxide::net::Download;
use teloxide::prelude::*;

use crate::asr::AsrService;
use crate::audio;
use crate::auth::AuthService;
use crate::llm::LlmService;

use super::{flow, render};

const FFMPEG_TIMEOUT: Duration = Duration::from_secs(60);

pub async fn handle_audio(
    bot: &Bot,
    message: &Message,
    asr: &AsrService,
    llm: &LlmService,
    auth: &AuthService,
) -> ResponseResult<()> {
    if let Err(error) = Box::pin(handle_audio_inner(bot, message, asr, llm, auth)).await {
        tracing::error!(%error, "voice handler failed");
        if let Err(send_error) = bot
            .send_message(message.chat.id, format!("❌ 处理出错：{error:#}"))
            .await
        {
            tracing::warn!(%send_error, "failed to send error message");
        }
    }
    Ok(())
}

async fn handle_audio_inner(
    bot: &Bot,
    message: &Message,
    asr: &AsrService,
    llm: &LlmService,
    auth: &AuthService,
) -> Result<()> {
    let user = message.from.as_ref().context("message sender is missing")?;
    if !auth.is_user_allowed(user.id.0).await {
        bot.send_message(
            message.chat.id,
            "🔒 请先使用 /start <password> 或 /auth <password> 完成认证。",
        )
        .await
        .context("failed to send auth warning")?;
        return Ok(());
    }

    let file_id = if let Some(voice) = message.voice() {
        voice.file.id.clone()
    } else if let Some(audio) = message.audio() {
        audio.file.id.clone()
    } else {
        anyhow::bail!("message has no voice or audio payload");
    };
    let progress = bot
        .send_message(message.chat.id, "⏳ 正在接收并处理音频...")
        .await
        .context("failed to send progress message")?;

    let telegram_file = bot
        .get_file(file_id)
        .await
        .context("failed to fetch Telegram file metadata")?;
    let downloaded = download_audio(bot, &telegram_file.path).await?;
    let wav_bytes = audio::transcode_to_wav(downloaded, FFMPEG_TIMEOUT).await?;

    bot.edit_message_text(progress.chat.id, progress.id, "🔄 正在进行语音识别...")
        .await
        .context("failed to update ASR status message")?;

    let started = Instant::now();
    let transcript = asr.transcribe(wav_bytes).await?;
    if transcript.is_empty() {
        bot.edit_message_text(progress.chat.id, progress.id, "⚠️ 未能识别出文字。")
            .await
            .context("failed to report empty transcript")?;
        return Ok(());
    }

    render::deliver(
        bot,
        &progress,
        render::transcript_reply(&transcript, asr.model(), started.elapsed()),
    )
    .await
    .context("failed to deliver transcript")?;

    if !llm.is_enabled() {
        return Ok(());
    }

    let status = if llm.has_cloud() && llm.has_local() {
        "✨ 双模型并行润色中..."
    } else {
        "✨ 正在进行智能润色..."
    };
    let refinement_message = bot
        .send_message(message.chat.id, status)
        .await
        .context("failed to send refinement status")?;

    if llm.has_cloud() && llm.has_local() {
        let mut updates = flow::dual::collect(llm.clone(), transcript.clone());
        while let Some(snapshot) = updates.recv().await {
            let snapshot = snapshot?;
            render::deliver(
                bot,
                &refinement_message,
                render::dual_refinement_reply(snapshot.cloud.as_ref(), snapshot.local.as_ref()),
            )
            .await
            .context("failed to deliver dual refinement progress")?;
        }
    } else {
        let result = flow::single::collect(llm, &transcript).await?;
        render::deliver(
            bot,
            &refinement_message,
            render::single_refinement_reply(&result),
        )
        .await
        .context("failed to deliver single refinement result")?;
    }

    Ok(())
}
async fn download_audio(bot: &Bot, path: &str) -> Result<Bytes> {
    let chunks = bot
        .download_file_stream(path)
        .try_collect::<Vec<_>>()
        .await
        .context("failed to download Telegram audio")?;
    Ok(Bytes::from(chunks.concat()))
}
