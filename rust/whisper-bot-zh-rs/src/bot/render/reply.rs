use crate::llm::RefinementResult;
use crate::util::format_duration;

use super::super::telegram_limit::should_send_as_file;

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
        section_with_state("☁️ 云端", cloud),
        section_with_state("💻 本地", local),
    ];
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

impl RenderedReply {
    #[must_use]
    pub fn caption(&self) -> &str {
        &self.caption
    }

    #[must_use]
    pub fn file_name(&self) -> &'static str {
        self.file_name
    }

    #[must_use]
    pub fn html(&self) -> &str {
        &self.html
    }

    #[must_use]
    pub fn plain(&self) -> &str {
        &self.plain
    }

    #[must_use]
    pub fn wants_file(&self) -> bool {
        should_send_as_file(&self.plain)
    }
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

fn section_with_state(label: &str, result: Option<&RefinementResult>) -> RenderSection {
    match result {
        None => RenderSection {
            header: format!("⏳ {label}"),
            body: "(正在校对...)".to_owned(),
        },
        Some(r) if r.ok => RenderSection {
            header: format!("✅ {label} · {} · {}", r.model, format_duration(r.duration)),
            body: r.text.clone(),
        },
        Some(r) => RenderSection {
            header: format!("⚠️ {label} · {}", r.model),
            body: r.text.clone(),
        },
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
