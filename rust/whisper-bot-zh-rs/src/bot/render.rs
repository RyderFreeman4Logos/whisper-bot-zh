mod governor;
#[cfg(test)]
mod governor_test;
mod reply;

use teloxide::prelude::*;
use teloxide::types::InputFile;

pub use governor::TelegramGovernor;
pub use reply::{dual_refinement_reply, single_refinement_reply, transcript_reply, RenderedReply};

/// Deliver a rendered reply, falling back to a file when the text is too long.
///
/// # Errors
/// Returns an error if Telegram rejects any intermediate edit, message, or
/// document upload required to deliver the reply.
pub async fn deliver(
    governor: &TelegramGovernor,
    target: &Message,
    reply: RenderedReply,
) -> ResponseResult<()> {
    if reply.wants_file() {
        governor
            .edit_plain_text(target, "文本较长，已作为文件发送。")
            .await?;
        governor
            .send_document(
                target.chat.id,
                InputFile::memory(reply.plain().as_bytes().to_vec()).file_name(reply.file_name()),
                reply.caption().to_owned(),
            )
            .await?;
        return Ok(());
    }

    governor
        .edit_html_text(target, reply.html().to_owned())
        .await
}

/// Update the current status message for a chat.
///
/// # Errors
/// Returns an error if Telegram rejects the status edit.
pub async fn update_status(
    governor: &TelegramGovernor,
    target: &Message,
    text: &str,
) -> ResponseResult<()> {
    governor.edit_plain_text(target, text).await
}
