//! Parser and serializer for the HFP AT command subset (BRSF, CIND, CMER, CIEV,
//! BAC, BCS, CLCC, CLIP, VGS, VGM and friends), line-discipline aware, tolerant
//! of AG quirks.

use crate::error::BluetoothError;

/// Unsolicited results and replies the audio gateway can send to the hands-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgResponse {
    Ok,
    Error,
    Ring,
    /// AG feature bitmap from `+BRSF:`.
    Brsf(u32),
    /// Indicator ordering from `+CIND: (...)`, name by 1-based position.
    CindSupported(Vec<String>),
    /// Current indicator values from `+CIND: 1,0,...`.
    CindValues(Vec<u8>),
    /// `+CIEV: <index>,<value>` — an indicator changed.
    Ciev {
        index: usize,
        value: u8,
    },
    /// `+BCS: <codec>` — the AG selected a codec.
    Bcs(u8),
    /// `+CLIP: "<number>",<type>` — calling line identity.
    Clip {
        number: String,
    },
    /// `+VGS:`/`+VGM:` — speaker and microphone gain sync.
    SpeakerGain(u8),
    MicrophoneGain(u8),
    /// Anything well-formed but not in the subset Tandem acts on.
    Unhandled(String),
}

/// Commands the hands-free is permitted to send. Call-control commands are
/// deliberately absent: user intent travels over the LAN control plane, never
/// over HFP, so the two paths cannot race (docs/05 single-command-path rule).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HfCommand {
    /// `AT+BRSF=<features>` — announce hands-free features.
    Brsf(u32),
    /// `AT+CIND=?` — query indicator ordering.
    CindTest,
    /// `AT+CIND?` — read current indicator values.
    CindRead,
    /// `AT+CMER=3,0,0,1` — enable indicator reporting.
    CmerEnable,
    /// `AT+CHLD=?` — query supported call-hold features (capability probe only).
    ChldTest,
    /// `AT+BAC=<codecs>` — advertise available codecs.
    Bac(Vec<u8>),
    /// `AT+BCS=<codec>` — confirm the AG's codec selection.
    Bcs(u8),
    /// `AT+CLCC` — poll the current call list for consistency checking.
    Clcc,
    /// `AT+CLIP=1` — enable calling line identification.
    ClipEnable,
    /// `AT+VGS=<gain>` / `AT+VGM=<gain>` — volume sync.
    SpeakerGain(u8),
    MicrophoneGain(u8),
}

impl HfCommand {
    pub fn serialize(&self) -> String {
        match self {
            Self::Brsf(features) => format!("AT+BRSF={features}\r"),
            Self::CindTest => "AT+CIND=?\r".into(),
            Self::CindRead => "AT+CIND?\r".into(),
            Self::CmerEnable => "AT+CMER=3,0,0,1\r".into(),
            Self::ChldTest => "AT+CHLD=?\r".into(),
            Self::Bac(codecs) => {
                let list = codecs
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("AT+BAC={list}\r")
            }
            Self::Bcs(codec) => format!("AT+BCS={codec}\r"),
            Self::Clcc => "AT+CLCC\r".into(),
            Self::ClipEnable => "AT+CLIP=1\r".into(),
            Self::SpeakerGain(gain) => format!("AT+VGS={gain}\r"),
            Self::MicrophoneGain(gain) => format!("AT+VGM={gain}\r"),
        }
    }
}

/// Parses one response line. AGs vary in whitespace and line endings, so the
/// parser trims liberally rather than rejecting on formatting.
pub fn parse_response(line: &str) -> Result<AgResponse, BluetoothError> {
    let trimmed = line.trim_matches(|c| c == '\r' || c == '\n' || c == ' ');
    if trimmed.is_empty() {
        return Err(BluetoothError::MalformedAt("empty line".into()));
    }

    match trimmed {
        "OK" => return Ok(AgResponse::Ok),
        "ERROR" => return Ok(AgResponse::Error),
        "RING" => return Ok(AgResponse::Ring),
        _ => {}
    }

    let Some((tag, value)) = trimmed.split_once(':') else {
        return Ok(AgResponse::Unhandled(trimmed.to_string()));
    };
    let value = value.trim();

    match tag.trim() {
        "+BRSF" => value
            .parse::<u32>()
            .map(AgResponse::Brsf)
            .map_err(|_| BluetoothError::MalformedAt(trimmed.into())),

        "+BCS" => value
            .parse::<u8>()
            .map(AgResponse::Bcs)
            .map_err(|_| BluetoothError::MalformedAt(trimmed.into())),

        "+VGS" => value
            .parse::<u8>()
            .map(AgResponse::SpeakerGain)
            .map_err(|_| BluetoothError::MalformedAt(trimmed.into())),

        "+VGM" => value
            .parse::<u8>()
            .map(AgResponse::MicrophoneGain)
            .map_err(|_| BluetoothError::MalformedAt(trimmed.into())),

        "+CIEV" => {
            let (index, val) = value
                .split_once(',')
                .ok_or_else(|| BluetoothError::MalformedAt(trimmed.into()))?;
            let index = index
                .trim()
                .parse::<usize>()
                .map_err(|_| BluetoothError::MalformedAt(trimmed.into()))?;
            let val = val
                .trim()
                .parse::<u8>()
                .map_err(|_| BluetoothError::MalformedAt(trimmed.into()))?;
            Ok(AgResponse::Ciev { index, value: val })
        }

        "+CLIP" => {
            let number = value
                .split(',')
                .next()
                .unwrap_or_default()
                .trim()
                .trim_matches('"')
                .to_string();
            Ok(AgResponse::Clip { number })
        }

        "+CIND" => {
            if value.contains('(') {
                Ok(AgResponse::CindSupported(parse_cind_names(value)))
            } else {
                let values = value
                    .split(',')
                    .map(|v| v.trim().parse::<u8>().unwrap_or(0))
                    .collect();
                Ok(AgResponse::CindValues(values))
            }
        }

        _ => Ok(AgResponse::Unhandled(trimmed.to_string())),
    }
}

/// Extracts indicator names from `("service",(0,1)),("call",(0,1))` form. Order
/// is the contract: `+CIEV` refers to indicators by 1-based position.
fn parse_cind_names(value: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = value;
    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else { break };
        names.push(after[..close].to_string());
        rest = &after[close + 1..];
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_terminal_responses() {
        assert_eq!(parse_response("OK\r\n").unwrap(), AgResponse::Ok);
        assert_eq!(parse_response("\r\nERROR\r\n").unwrap(), AgResponse::Error);
        assert_eq!(parse_response("RING").unwrap(), AgResponse::Ring);
    }

    #[test]
    fn parses_indicator_ordering_and_values() {
        let supported =
            parse_response(r#"+CIND: ("service",(0,1)),("call",(0,1)),("callsetup",(0-3))"#)
                .unwrap();
        assert_eq!(
            supported,
            AgResponse::CindSupported(vec!["service".into(), "call".into(), "callsetup".into()])
        );

        assert_eq!(
            parse_response("+CIND: 1,0,0").unwrap(),
            AgResponse::CindValues(vec![1, 0, 0])
        );
    }

    #[test]
    fn parses_indicator_events() {
        assert_eq!(
            parse_response("+CIEV: 2,1").unwrap(),
            AgResponse::Ciev { index: 2, value: 1 }
        );
        assert_eq!(
            parse_response("+CIEV:3,0").unwrap(),
            AgResponse::Ciev { index: 3, value: 0 }
        );
    }

    #[test]
    fn parses_codec_gain_and_caller_id() {
        assert_eq!(parse_response("+BCS: 2").unwrap(), AgResponse::Bcs(2));
        assert_eq!(
            parse_response("+VGS: 9").unwrap(),
            AgResponse::SpeakerGain(9)
        );
        assert_eq!(
            parse_response("+VGM: 12").unwrap(),
            AgResponse::MicrophoneGain(12)
        );
        assert_eq!(
            parse_response(r#"+CLIP: "+14155550123",145"#).unwrap(),
            AgResponse::Clip {
                number: "+14155550123".into()
            }
        );
    }

    #[test]
    fn unknown_but_well_formed_lines_are_carried_not_rejected() {
        assert_eq!(
            parse_response("+CCWA: 1").unwrap(),
            AgResponse::Unhandled("+CCWA: 1".into())
        );
        assert_eq!(
            parse_response("SOMETHING").unwrap(),
            AgResponse::Unhandled("SOMETHING".into())
        );
    }

    #[test]
    fn malformed_numeric_fields_are_errors() {
        assert!(parse_response("+BRSF: abc").is_err());
        assert!(parse_response("+CIEV: 2").is_err());
        assert!(parse_response("   ").is_err());
    }

    #[test]
    fn serializes_the_permitted_command_set() {
        assert_eq!(HfCommand::Brsf(191).serialize(), "AT+BRSF=191\r");
        assert_eq!(HfCommand::CindTest.serialize(), "AT+CIND=?\r");
        assert_eq!(HfCommand::CindRead.serialize(), "AT+CIND?\r");
        assert_eq!(HfCommand::CmerEnable.serialize(), "AT+CMER=3,0,0,1\r");
        assert_eq!(HfCommand::Bac(vec![1, 2]).serialize(), "AT+BAC=1,2\r");
        assert_eq!(HfCommand::Bcs(2).serialize(), "AT+BCS=2\r");
        assert_eq!(HfCommand::Clcc.serialize(), "AT+CLCC\r");
        assert_eq!(HfCommand::SpeakerGain(7).serialize(), "AT+VGS=7\r");
    }

    /// The single-command-path rule is structural: there is no way to express a
    /// call-control command, so no code path can accidentally send one.
    #[test]
    fn no_command_variant_can_control_a_call() {
        let every_command = [
            HfCommand::Brsf(0),
            HfCommand::CindTest,
            HfCommand::CindRead,
            HfCommand::CmerEnable,
            HfCommand::ChldTest,
            HfCommand::Bac(vec![1]),
            HfCommand::Bcs(1),
            HfCommand::Clcc,
            HfCommand::ClipEnable,
            HfCommand::SpeakerGain(0),
            HfCommand::MicrophoneGain(0),
        ];
        for command in every_command {
            let wire = command.serialize();
            assert!(!wire.starts_with("ATA"), "{wire} answers a call over HFP");
            assert!(!wire.starts_with("ATD"), "{wire} dials over HFP");
            assert!(!wire.starts_with("AT+CHUP"), "{wire} hangs up over HFP");
            let is_chld_probe = wire == "AT+CHLD=?\r";
            assert!(
                !wire.starts_with("AT+CHLD=") || is_chld_probe,
                "{wire} manipulates calls over HFP"
            );
        }
    }
}
