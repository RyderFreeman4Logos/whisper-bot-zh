//! System + user prompt templates shared by cloud and local refinement.
//! Keep text identical across paths so dual-model output is comparable.

pub const SYSTEM_PROMPT: &str = "\
You correct transcription text in place. \
Fix typos, add punctuation and paragraph breaks, preserve the speaker's \
meaning. Output ONLY the corrected text. Do not prepend any preface. \
Do not append any commentary, summary, or suggestions. Stop at the end \
of the source text.";

#[must_use]
pub fn user_message(raw_transcript: &str) -> String {
    format!(
        "--- BEGIN TRANSCRIPT ---\n{raw_transcript}\n--- END TRANSCRIPT ---\n\nReturn only the corrected text. No commentary."
    )
}
