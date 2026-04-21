pub const TELEGRAM_TEXT_LIMIT: usize = 3800;

#[must_use]
pub fn should_send_as_file(text: &str) -> bool {
    text.chars().count() > TELEGRAM_TEXT_LIMIT
}

#[cfg(test)]
mod tests {
    use super::{should_send_as_file, TELEGRAM_TEXT_LIMIT};

    #[test]
    fn respects_threshold() {
        assert!(!should_send_as_file(&"a".repeat(TELEGRAM_TEXT_LIMIT)));
        assert!(should_send_as_file(&"a".repeat(TELEGRAM_TEXT_LIMIT + 1)));
    }
}
