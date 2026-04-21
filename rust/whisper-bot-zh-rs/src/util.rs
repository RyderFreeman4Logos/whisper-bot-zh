use std::time::Duration;

/// Format a `Duration` as `HH:MM:SS.ss` — matches the footer format used in
/// Telegram replies (`\u{23f1} \u{8017}\u{65f6}: 00:00:02.50`).
#[must_use]
pub fn format_duration(d: Duration) -> String {
    let total_ms = d.as_millis();
    let hours = total_ms / 3_600_000;
    let minutes = (total_ms / 60_000) % 60;
    let seconds = (total_ms / 1000) % 60;
    let centis = (total_ms / 10) % 100;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{centis:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_short() {
        assert_eq!(format_duration(Duration::from_millis(2_500)), "00:00:02.50");
    }

    #[test]
    fn format_hours() {
        assert_eq!(
            format_duration(Duration::from_secs(3_723) + Duration::from_millis(450)),
            "01:02:03.45"
        );
    }
}
