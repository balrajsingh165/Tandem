//! Browses _tandem._tcp via mdns-sd, parses TXT records (version, device id,
//! name), and emits candidate endpoints — filtered against the paired phone's
//! identity before any connection attempt.

use crate::error::TransportError;

/// DNS-SD service type the phone advertises.
pub const SERVICE_TYPE: &str = "_tandem._tcp";

/// TXT record keys carried in the advertisement. The advertisement is public, so
/// it never contains secrets — only enough to recognize a known phone.
pub const TXT_KEY_VERSION: &str = "v";
pub const TXT_KEY_DEVICE_ID: &str = "id";
pub const TXT_KEY_NAME: &str = "name";

/// A phone seen on the LAN, before trust is established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPhone {
    pub device_id: String,
    pub display_name: String,
    pub protocol_version: u32,
    pub host: String,
    pub port: u16,
}

impl DiscoveredPhone {
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Discovery is untrusted input: a candidate is only worth dialing if it
    /// claims the device id we already paired with. The TLS pin is what actually
    /// proves identity.
    pub fn matches_paired(&self, paired_device_id: &str) -> bool {
        !paired_device_id.is_empty() && self.device_id == paired_device_id
    }
}

/// Parses the TXT key/value pairs of an advertisement into a candidate.
pub fn parse_txt_records(
    host: &str,
    port: u16,
    records: &[(String, String)],
) -> Result<DiscoveredPhone, TransportError> {
    let lookup = |key: &str| {
        records
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };

    let device_id = lookup(TXT_KEY_DEVICE_ID)
        .ok_or_else(|| TransportError::ProtocolViolation("advertisement lacks id".into()))?;
    let version: u32 = lookup(TXT_KEY_VERSION)
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| TransportError::ProtocolViolation("advertisement lacks v".into()))?;

    Ok(DiscoveredPhone {
        device_id: device_id.to_string(),
        display_name: lookup(TXT_KEY_NAME).unwrap_or_default().to_string(),
        protocol_version: version,
        host: host.to_string(),
        port,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn records() -> Vec<(String, String)> {
        vec![
            ("v".into(), "1".into()),
            ("id".into(), "phone-1".into()),
            ("name".into(), "Pixel".into()),
        ]
    }

    #[test]
    fn parses_a_well_formed_advertisement() {
        let p = parse_txt_records("192.168.1.20", 46521, &records()).unwrap();
        assert_eq!(p.device_id, "phone-1");
        assert_eq!(p.protocol_version, 1);
        assert_eq!(p.endpoint(), "192.168.1.20:46521");
    }

    #[test]
    fn advertisements_missing_required_keys_are_rejected() {
        let no_id: Vec<_> = records().into_iter().filter(|(k, _)| k != "id").collect();
        assert!(parse_txt_records("h", 1, &no_id).is_err());
        let no_v: Vec<_> = records().into_iter().filter(|(k, _)| k != "v").collect();
        assert!(parse_txt_records("h", 1, &no_v).is_err());
    }

    #[test]
    fn only_the_paired_device_id_is_worth_dialing() {
        let p = parse_txt_records("192.168.1.20", 46521, &records()).unwrap();
        assert!(p.matches_paired("phone-1"));
        assert!(!p.matches_paired("phone-2"));
        assert!(!p.matches_paired(""));
    }
}
