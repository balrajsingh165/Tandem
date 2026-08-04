//! Desktop-side emergency-number pre-check against the list synced from the
//! phone: blocks the dial locally with clear UX before any request is sent.
//! Defense in depth — the phone enforces the same policy authoritatively
//! (ADR-0008).

use crate::error::CoreError;

/// Emergency numbers as reported by the phone in `SessionWelcome`. Refreshed on
/// every session; the phone-side guard remains authoritative, so mid-session
/// staleness is acceptable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmergencyNumbers {
    numbers: Vec<String>,
}

/// Conservative floor used before any session has reported a list, so a dial
/// attempted while disconnected still refuses the most common short codes.
const FALLBACK: &[&str] = &["112", "911", "999", "000", "110", "118", "119"];

impl EmergencyNumbers {
    pub fn from_session(numbers: Vec<String>) -> Self {
        Self {
            numbers: numbers.iter().map(|n| normalize(n)).collect(),
        }
    }

    pub fn is_emergency(&self, dial_string: &str) -> bool {
        let candidate = normalize(dial_string);
        if candidate.is_empty() {
            return false;
        }
        if self.numbers.contains(&candidate) {
            return true;
        }
        self.numbers.is_empty() && FALLBACK.contains(&candidate.as_str())
    }

    /// Refuses locally so no `DialRequest` is ever put on the wire for an
    /// emergency number; the UI must direct the user to the handset.
    pub fn guard(&self, dial_string: &str) -> Result<(), CoreError> {
        if self.is_emergency(dial_string) {
            return Err(CoreError::EmergencyBlocked {
                number: dial_string.to_string(),
            });
        }
        Ok(())
    }
}

fn normalize(dial_string: &str) -> String {
    dial_string
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '*' || *c == '#')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synced() -> EmergencyNumbers {
        EmergencyNumbers::from_session(vec!["112".into(), "911".into()])
    }

    #[test]
    fn refuses_synced_emergency_numbers() {
        assert!(synced().guard("911").is_err());
        assert!(synced().guard("112").is_err());
    }

    #[test]
    fn refuses_despite_dial_string_formatting() {
        assert!(synced().guard("9-1-1").is_err());
        assert!(synced().guard(" 911 ").is_err());
        assert!(synced().guard("(911)").is_err());
    }

    #[test]
    fn allows_ordinary_numbers() {
        assert!(synced().guard("+14155550123").is_ok());
        assert!(synced().guard("9115550123").is_ok());
    }

    #[test]
    fn falls_back_before_any_session_has_reported() {
        let none = EmergencyNumbers::default();
        assert!(none.guard("112").is_err());
        assert!(none.guard("+14155550123").is_ok());
    }

    #[test]
    fn synced_list_replaces_the_fallback() {
        let jp = EmergencyNumbers::from_session(vec!["110".into()]);
        assert!(jp.guard("110").is_err());
        assert!(jp.guard("911").is_ok());
    }
}
