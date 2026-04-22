use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bytes::Bytes;
use futures::TryStreamExt;
use teloxide::net::Download;
use teloxide::prelude::*;
use tokio::sync::OwnedSemaphorePermit;

use crate::asr::AsrService;
use crate::audio;
use crate::llm::LlmService;

use super::super::render::TelegramGovernor;
use super::super::{flow, render, ThrottledBot};

const FFMPEG_TIMEOUT: Duration = Duration::from_secs(60);

/// Handle a Telegram voice or audio message end to end.
///
/// # Errors
/// Returns an error only if Telegram rejects the final user-visible recovery
/// reply after an internal processing failure.
pub async fn handle_audio(
    bot: &ThrottledBot,
    governor: &TelegramGovernor,
    message: &Message,
    asr: &AsrService,
    llm: &LlmService,
    voice_permit: OwnedSemaphorePermit,
) -> ResponseResult<()> {
    if let Err(error) = Box::pin(handle_audio_inner(
        bot,
        governor,
        message,
        asr,
        llm,
        voice_permit,
    ))
    .await
    {
        tracing::error!(%error, "voice handler failed");
        if let Err(send_error) = governor
            .send_reply(
                message.chat.id,
                message.id,
                format!("❌ 处理出错：{error:#}"),
            )
            .await
        {
            tracing::warn!(%send_error, "failed to send error message");
        }
    }
    Ok(())
}

async fn handle_audio_inner(
    bot: &ThrottledBot,
    governor: &TelegramGovernor,
    message: &Message,
    asr: &AsrService,
    llm: &LlmService,
    _voice_permit: OwnedSemaphorePermit,
) -> Result<()> {
    let file_id = if let Some(voice) = message.voice() {
        voice.file.id.clone()
    } else if let Some(audio) = message.audio() {
        audio.file.id.clone()
    } else {
        anyhow::bail!("message has no voice or audio payload");
    };
    let progress = governor
        .send_reply(message.chat.id, message.id, "⏳ 正在接收并处理音频...")
        .await
        .context("failed to send progress message")?;

    let telegram_file = bot
        .get_file(file_id)
        .await
        .context("failed to fetch Telegram file metadata")?;
    let downloaded = download_audio(bot, &telegram_file.path).await?;
    let wav_bytes = audio::transcode_to_wav(downloaded, FFMPEG_TIMEOUT).await?;

    governor
        .edit_plain_text(&progress, "🔄 正在进行语音识别...")
        .await
        .context("failed to update ASR status message")?;

    let started = Instant::now();
    let transcript = asr.transcribe(wav_bytes).await?;
    if transcript.is_empty() {
        governor
            .edit_plain_text(&progress, "⚠️ 未能识别出文字。")
            .await
            .context("failed to report empty transcript")?;
        return Ok(());
    }

    render::deliver(
        governor,
        &progress,
        message.id,
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
    let refinement_message = governor
        .send_reply(message.chat.id, message.id, status)
        .await
        .context("failed to send refinement status")?;

    if llm.has_cloud() && llm.has_local() {
        let mut updates = flow::dual::collect(llm.clone(), transcript.clone());
        let mut deferred_long_file_notice_sent = false;
        while let Some(snapshot) = updates.recv().await {
            let snapshot = snapshot?;
            let reply =
                render::dual_refinement_reply(snapshot.cloud.as_ref(), snapshot.local.as_ref());

            if should_defer_dual_file_delivery(&snapshot, &reply) {
                if !deferred_long_file_notice_sent {
                    render::update_status(
                        governor,
                        &refinement_message,
                        "📝 文本较长，等待双模型完成后发送最终文件...",
                    )
                    .await
                    .context("failed to update deferred dual-file status")?;
                    deferred_long_file_notice_sent = true;
                }
                continue;
            }

            render::deliver(governor, &refinement_message, message.id, reply)
                .await
                .context("failed to deliver dual refinement progress")?;
        }
    } else {
        let result = flow::single::collect(llm, &transcript).await?;
        render::deliver(
            governor,
            &refinement_message,
            message.id,
            render::single_refinement_reply(&result),
        )
        .await
        .context("failed to deliver single refinement result")?;
    }

    Ok(())
}

async fn download_audio(bot: &ThrottledBot, path: &str) -> Result<Bytes> {
    let chunks = bot
        .download_file_stream(path)
        .try_collect::<Vec<_>>()
        .await
        .context("failed to download Telegram audio")?;
    Ok(Bytes::from(chunks.concat()))
}

pub(super) fn should_defer_dual_file_delivery(
    snapshot: &flow::DualProgress,
    reply: &render::RenderedReply,
) -> bool {
    !snapshot.is_complete() && reply.wants_file()
}
