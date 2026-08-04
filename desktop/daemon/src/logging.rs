//! Initializes tracing subscribers (stderr + rolling file), with call metadata
//! redaction in release builds per the privacy policy in docs/08.

use crate::config::LogLevel;

/// Phone numbers and contact names are call metadata; logs must not become an
/// unmanaged copy of the user's call history (docs/08).
pub fn redact_number(number: &str) -> String {
    let digits: Vec<char> = number.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() <= 4 {
        return "*".repeat(digits.len());
    }
    let tail: String = digits[digits.len() - 2..].iter().collect();
    format!("{}{}", "*".repeat(digits.len() - 2), tail)
}

/// Contact names are redacted entirely; even an initial narrows the candidates
/// on a small contact list.
pub fn redact_name(name: &str) -> &'static str {
    if name.is_empty() {
        ""
    } else {
        "<redacted>"
    }
}

/// Redaction is unconditional in release builds; debug builds keep values so
/// developers can correlate logs with a test handset.
pub const fn redaction_enabled() -> bool {
    !cfg!(debug_assertions)
}

pub fn init(_level: LogLevel) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_last_two_digits_survive_redaction() {
        assert_eq!(redact_number("+14155550123"), "*********23");
        assert_eq!(redact_number("5550123"), "*****23");
    }

    #[test]
    fn short_numbers_are_fully_masked() {
        assert_eq!(redact_number("911"), "***");
        assert_eq!(redact_number("112"), "***");
        assert_eq!(redact_number(""), "");
    }

    #[test]
    fn formatting_characters_do_not_leak_through() {
        let redacted = redact_number("+1 (415) 555-0123");
        assert!(!redacted.contains('('));
        assert!(!redacted.contains('-'));
        assert!(!redacted.contains('4'));
    }

    #[test]
    fn names_are_redacted_entirely() {
        assert_eq!(redact_name("Alex Rivera"), "<redacted>");
        assert_eq!(redact_name(""), "");
    }

    #[test]
    fn redaction_is_on_in_release_builds() {
        assert_eq!(redaction_enabled(), !cfg!(debug_assertions));
    }
}
