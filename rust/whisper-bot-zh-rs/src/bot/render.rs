mod governor;
#[cfg(test)]
mod governor_test;
mod reply;

use teloxide::prelude::*;
use teloxide::types::InputFile;

pub use governor::TelegramGovernor;
pub use reply::{dual_refinement_reply, single_refinement_reply, transcript_reply, RenderedReply};

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

pub async fn update_status(
    governor: &TelegramGovernor,
    target: &Message,
    text: &str,
) -> ResponseResult<()> {
    governor.edit_plain_text(target, text).await
}
