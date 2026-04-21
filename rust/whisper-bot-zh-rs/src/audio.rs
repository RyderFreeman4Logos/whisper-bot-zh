//! `ffmpeg` subprocess wrapper — transcodes Telegram audio buffers into
//! 16kHz mono WAV in memory for Groq-compatible ASR backends.

use std::process::Stdio;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub async fn transcode_to_wav(input: Bytes, timeout: Duration) -> Result<Bytes> {
    let mut child = Command::new("ffmpeg");
    child
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-i")
        .arg("pipe:0")
        .arg("-f")
        .arg("wav")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        .arg("pipe:1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = child.spawn().context("failed to spawn ffmpeg")?;
    let mut stdin = child.stdin.take().context("failed to open ffmpeg stdin")?;
    stdin
        .write_all(&input)
        .await
        .context("failed to write audio to ffmpeg stdin")?;
    stdin
        .shutdown()
        .await
        .context("failed to close ffmpeg stdin")?;

    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .context("ffmpeg transcoding timed out")?
        .context("ffmpeg process failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffmpeg failed: {}", stderr.trim());
    }

    Ok(Bytes::from(output.stdout))
}
