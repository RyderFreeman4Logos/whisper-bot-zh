//! whisper-bot-zh-rs — Telegram bot core.
//!
//! Modules are kept small (<200 lines target) following the split-monolith-files
//! convention so each file fits in an LLM context window on its own.

pub mod asr;
pub mod audio;
pub mod auth;
pub mod bot;
pub mod config;
pub mod llm;
pub mod util;
