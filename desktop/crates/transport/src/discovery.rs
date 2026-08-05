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

/// Fully-qualified service type as mDNS expects it.
pub const SERVICE_FQDN: &str = "_tandem._tcp.local.";

/// Browses the LAN for the paired phone. Returns as soon as a candidate whose
/// advertised device id matches is found, or None when the timeout expires.
///
/// Discovery is a hint, never an authorization: the returned endpoint still has
/// to pass the pinned-key TLS handshake before it is trusted (docs/08).
pub async fn find_paired_phone(
    paired_device_id: &str,
    timeout: std::time::Duration,
) -> Result<Option<DiscoveredPhone>, TransportError> {
    if paired_device_id.is_empty() {
        return Ok(None);
    }
    browse(Some(paired_device_id.to_string()), timeout).await
}

/// Finds any Tandem phone on the LAN. Used during QR pairing, where the phone
/// has scanned this desktop but the desktop does not yet know which phone to
/// expect. The pairing token is what makes the choice safe: a phone that never
/// scanned the code cannot complete the exchange.
pub async fn find_any_phone(
    timeout: std::time::Duration,
) -> Result<Option<DiscoveredPhone>, TransportError> {
    browse(None, timeout).await
}

async fn browse(
    wanted_device_id: Option<String>,
    timeout: std::time::Duration,
) -> Result<Option<DiscoveredPhone>, TransportError> {
    let daemon = mdns_sd::ServiceDaemon::new().map_err(|e| TransportError::ConnectFailed {
        endpoint: SERVICE_FQDN.into(),
        reason: e.to_string(),
    })?;

    let receiver = daemon
        .browse(SERVICE_FQDN)
        .map_err(|e| TransportError::ConnectFailed {
            endpoint: SERVICE_FQDN.into(),
            reason: e.to_string(),
        })?;

    let found = tokio::task::spawn_blocking(move || {
        let deadline = std::time::Instant::now() + timeout;

        while std::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let Ok(event) = receiver.recv_timeout(remaining) else {
                break;
            };

            if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
                if let Some(candidate) = from_resolved(&info) {
                    let acceptable = match &wanted_device_id {
                        Some(id) => candidate.matches_paired(id),
                        None => true,
                    };
                    if acceptable {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    })
    .await
    .map_err(|e| TransportError::ConnectFailed {
        endpoint: SERVICE_FQDN.into(),
        reason: e.to_string(),
    })?;

    let _ = daemon.shutdown();
    Ok(found)
}

/// Converts one resolved advertisement into a candidate, or None when it lacks
/// the records Tandem needs.
fn from_resolved(info: &mdns_sd::ServiceInfo) -> Option<DiscoveredPhone> {
    let address = preferred_address(info.get_addresses())?;
    let records: Vec<(String, String)> = info
        .get_properties()
        .iter()
        .map(|property| (property.key().to_string(), property.val_str().to_string()))
        .collect();

    parse_txt_records(&address, info.get_port(), &records).ok()
}

/// Picks the address actually worth dialing. A phone advertises every address it
/// holds, including IPv6 link-local ones that need a scope id the desktop cannot
/// supply — connecting to those merely stalls until the OS gives up, so routable
/// IPv4 comes first.
fn preferred_address(addresses: &std::collections::HashSet<std::net::IpAddr>) -> Option<String> {
    let mut ordered: Vec<&std::net::IpAddr> = addresses.iter().collect();
    ordered.sort_by_key(|address| match address {
        std::net::IpAddr::V4(_) => 0,
        std::net::IpAddr::V6(v6) if !v6.is_unicast_link_local() => 1,
        std::net::IpAddr::V6(_) => 2,
    });
    ordered.first().map(|address| address.to_string())
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

    /// A link-local v6 address reached first would stall the whole attempt, so
    /// routable addresses have to win regardless of advertisement order.
    #[test]
    fn routable_addresses_are_preferred_over_link_local() {
        use std::net::IpAddr;

        let mut all = std::collections::HashSet::new();
        all.insert("fe80::f8a9:f1ff:fec4:917f".parse::<IpAddr>().unwrap());
        all.insert("192.168.1.86".parse::<IpAddr>().unwrap());
        assert_eq!(preferred_address(&all).unwrap(), "192.168.1.86");

        let only_link_local: std::collections::HashSet<IpAddr> =
            ["fe80::1".parse().unwrap()].into_iter().collect();
        assert_eq!(preferred_address(&only_link_local).unwrap(), "fe80::1");

        assert!(preferred_address(&std::collections::HashSet::new()).is_none());
    }

    #[test]
    fn only_the_paired_device_id_is_worth_dialing() {
        let p = parse_txt_records("192.168.1.20", 46521, &records()).unwrap();
        assert!(p.matches_paired("phone-1"));
        assert!(!p.matches_paired("phone-2"));
        assert!(!p.matches_paired(""));
    }
}
