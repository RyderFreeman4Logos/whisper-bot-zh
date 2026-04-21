use teloxide::prelude::*;
use teloxide::types::{InputFile, ParseMode};

use crate::llm::RefinementResult;
use crate::util::format_duration;

use super::telegram_limit::should_send_as_file;

pub struct RenderedReply {
    html: String,
    plain: String,
    caption: String,
    file_name: &'static str,
}

#[must_use]
pub fn transcript_reply(
    content: &str,
    model: &str,
    duration: std::time::Duration,
) -> RenderedReply {
    let footer = format!(
        "🎙️ 由模型 {model} 转录，耗时：{}",
        format_duration(duration)
    );
    single_block_reply(content, footer, "transcript.txt")
}

#[must_use]
pub fn single_refinement_reply(result: &RefinementResult) -> RenderedReply {
    let footer = format!(
        "✨ 由模型 {} 润色，耗时：{}",
        result.model,
        format_duration(result.duration)
    );
    single_block_reply(&result.text, footer, "refined.txt")
}

#[must_use]
pub fn dual_refinement_reply(
    cloud: Option<&RefinementResult>,
    local: Option<&RefinementResult>,
) -> RenderedReply {
    let sections = [
        cloud.map(|result| section("☁️ 云端", result)),
        local.map(|result| section("💻 本地", result)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    let plain = sections
        .iter()
        .map(|section| format!("{}\n{}", section.header, section.body))
        .collect::<Vec<_>>()
        .join("\n\n");
    let html = sections
        .iter()
        .map(|section| {
            format!(
                "{}\n<pre>{}</pre>",
                escape_html(&section.header),
                escape_html(&section.body)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    RenderedReply {
        html,
        plain,
        caption: "双模型润色结果已生成。".to_owned(),
        file_name: "refined.txt",
    }
}

pub async fn deliver(bot: &Bot, target: &Message, reply: RenderedReply) -> ResponseResult<()> {
    if should_send_as_file(&reply.plain) {
        bot.edit_message_text(target.chat.id, target.id, "文本较长，已作为文件发送。")
            .await?;
        bot.send_document(
            target.chat.id,
            InputFile::memory(reply.plain.into_bytes()).file_name(reply.file_name),
        )
        .caption(reply.caption)
        .await?;
        return Ok(());
    }

    bot.edit_message_text(target.chat.id, target.id, reply.html)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

fn single_block_reply(content: &str, footer: String, file_name: &'static str) -> RenderedReply {
    let plain = format!("{content}\n\n{footer}");
    let html = format!(
        "<pre>{}</pre>\n\n{}",
        escape_html(content),
        escape_html(&footer)
    );
    RenderedReply {
        html,
        plain,
        caption: footer,
        file_name,
    }
}

fn section(label: &str, result: &RefinementResult) -> RenderSection {
    RenderSection {
        header: format!(
            "{label} · {} · {}",
            result.model,
            format_duration(result.duration)
        ),
        body: result.text.clone(),
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

struct RenderSection {
    header: String,
    body: String,
}
