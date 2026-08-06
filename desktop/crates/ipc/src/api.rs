//! IpcApi: every method (dial, answer, reject, end, mute, hold, unhold, merge,
//! dtmf, audio-route, history, pairing, settings, status) with its params,
//! results, and event payloads. Single source for both the Rust server and the
//! generated TS client types.

use serde::{Deserialize, Serialize};

/// Connection lifecycle the UI renders, mirroring the state table in docs/06.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    Idle,
    Discovering,
    Connecting,
    Authenticating,
    PairingProvisional,
    Resuming,
    Live,
    Backoff,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioRoute {
    Earpiece,
    Speaker,
    WiredHeadset,
    Bluetooth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CallState {
    Connecting,
    Dialing,
    Ringing,
    Active,
    Holding,
    Disconnecting,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallView {
    pub call_id: String,
    pub state: CallState,
    pub remote_number: String,
    pub remote_display_name: String,
    pub started_at_ms: i64,
    pub is_conference: bool,
    pub can_hold: bool,
    pub can_merge: bool,
    pub is_emergency: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub entry_id: String,
    pub number: String,
    pub display_name: String,
    pub started_at_ms: i64,
    pub duration_seconds: u32,
}

/// Every method the UI may invoke. Tagged by `method` so the wire form matches
/// JSON-RPC and the generated TypeScript union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum IpcRequest {
    Dial {
        number: String,
        sim_slot: i32,
    },
    Answer {
        call_id: String,
    },
    Reject {
        call_id: String,
    },
    End {
        call_id: String,
    },
    Mute {
        muted: bool,
    },
    Hold {
        call_id: String,
    },
    Unhold {
        call_id: String,
    },
    Merge {
        call_id: String,
        other_call_id: String,
    },
    Dtmf {
        call_id: String,
        digits: String,
    },
    AudioRoute {
        route: AudioRoute,
        bt_device_address: String,
    },
    History {
        since_ms: i64,
        limit: u32,
    },
    /// The phones' address books, merged and name-ordered.
    Contacts,
    Pairing {
        qr_payload: String,
    },
    /// Starts scan-to-pair: the daemon mints an offer, returns it for display as
    /// a QR code, and waits for a phone to scan it.
    PairingOffer,
    /// The user's verdict on the phone that scanned this desktop's code.
    PairingConfirm {
        accept: bool,
    },
    /// Forgets the paired phone and drops the session. Empty id means the
    /// selected phone.
    Unpair {
        #[serde(default)]
        phone_id: String,
    },
    /// Chooses which paired phone subsequent commands act on.
    SelectPhone {
        phone_id: String,
    },
    Settings,
    Status,
}

/// Results, keyed to the request that produced them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "result",
    content = "data",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum IpcResponse {
    Ok,
    CallId {
        call_id: String,
    },
    History {
        entries: Vec<HistoryEntry>,
        has_more: bool,
    },
    Contacts {
        entries: Vec<ContactView>,
    },
    Status(StatusResult),
    Settings(SettingsResult),
    Pairing(PairingResult),
    Offer(OfferResult),
}

/// The payload the UI renders as a QR code for the phone's camera.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferResult {
    pub payload: String,
    pub desktop_name: String,
}

/// One dialable number from a phone's address book.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactView {
    pub contact_id: String,
    pub display_name: String,
    pub number: String,
    pub label: String,
    pub starred: bool,
}

/// One place the call's audio can go. `btDeviceAddress` is empty for the phone's
/// own routes and names the device for Bluetooth targets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceView {
    pub route: AudioRoute,
    pub bt_device_address: String,
    pub name: String,
}

/// One paired phone as the switcher shows it. Every phone keeps its own session
/// and its own call state, so this is per-phone rather than a global.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneSummary {
    pub device_id: String,
    pub name: String,
    pub connection: ConnectionStatus,
    pub calls: Vec<CallView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResult {
    /// Every paired phone. Commands act on `selected_phone_id`.
    pub phones: Vec<PhoneSummary>,
    pub selected_phone_id: String,
    pub connection: ConnectionStatus,
    pub phone_name: String,
    pub calls: Vec<CallView>,
    pub audio_route: AudioRoute,
    pub microphone_muted: bool,
    /// False on a Tier B-lite build, so the UI can explain that audio stays on
    /// the phone rather than offering a control that cannot work.
    pub desktop_audio_available: bool,
    /// Everywhere this call's audio can be sent, as the phone reports it.
    pub audio_devices: Vec<AudioDeviceView>,
    /// Which of `audio_devices` is live; empty for a phone-local route.
    pub active_bt_device_address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsResult {
    pub desktop_display_name: String,
    pub autostart_enabled: bool,
    pub notify_incoming_calls: bool,
    pub bluetooth_backend: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingResult {
    pub state: String,
    pub short_code: Option<String>,
    pub phone_name: String,
}

/// Pushed to every connected UI as state changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum IpcEvent {
    ConnectionChanged {
        connection: ConnectionStatus,
    },
    CallsChanged {
        calls: Vec<CallView>,
    },
    AudioRouteChanged {
        route: AudioRoute,
        bt_device_address: String,
    },
    /// The set of places audio can go changed, or a different one became live.
    AudioDevicesChanged {
        devices: Vec<AudioDeviceView>,
        active_route: AudioRoute,
        active_bt_device_address: String,
    },
    HistoryChanged {
        log_version: u64,
    },
    /// The synced address book changed size, so the UI should re-read it.
    ContactsChanged {
        count: u32,
    },
    /// A phone was paired, removed, or changed connection state.
    PhonesChanged {
        phones: Vec<PhoneSummary>,
        selected_phone_id: String,
    },
    /// Fired for the local pre-check and for a phone-side refusal alike, so the
    /// guidance is identical in both cases (ADR-0008).
    EmergencyBlocked {
        number: String,
        guidance: String,
    },
    AudioPipelineChanged {
        sco_active: bool,
        latency_ms: Option<u32>,
    },
    PairingProgress {
        state: String,
        short_code: Option<String>,
    },
    /// A phone scanned this desktop's code and is waiting to be let in. The UI
    /// must ask before the daemon sends anything to it.
    PairingApprovalRequested {
        phone_name: String,
        phone_fingerprint: String,
    },
    Revoked {
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<T>(value: &T) -> T
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        serde_json::from_str(&serde_json::to_string(value).unwrap()).unwrap()
    }

    #[test]
    fn requests_round_trip_through_json() {
        let dial = IpcRequest::Dial {
            number: "+14155550123".into(),
            sim_slot: -1,
        };
        assert_eq!(round_trip(&dial), dial);
        assert_eq!(round_trip(&IpcRequest::Status), IpcRequest::Status);
    }

    #[test]
    fn requests_are_tagged_by_method_for_json_rpc() {
        let json = serde_json::to_value(IpcRequest::Answer {
            call_id: "c1".into(),
        })
        .unwrap();
        assert_eq!(json["method"], "answer");
        assert_eq!(json["params"]["callId"], "c1");
    }

    #[test]
    fn events_are_tagged_by_type() {
        let json = serde_json::to_value(IpcEvent::EmergencyBlocked {
            number: "911".into(),
            guidance: "Dial on the handset".into(),
        })
        .unwrap();
        assert_eq!(json["type"], "emergencyBlocked");
        assert_eq!(json["number"], "911");
    }

    /// docs/06 defines nine connection states; the UI must be able to render
    /// every one, including the provisional pairing session.
    #[test]
    fn all_nine_connection_states_are_representable() {
        let states = [
            ConnectionStatus::Idle,
            ConnectionStatus::Discovering,
            ConnectionStatus::Connecting,
            ConnectionStatus::Authenticating,
            ConnectionStatus::PairingProvisional,
            ConnectionStatus::Resuming,
            ConnectionStatus::Live,
            ConnectionStatus::Backoff,
            ConnectionStatus::Terminated,
        ];
        assert_eq!(states.len(), 9);
        for state in states {
            assert_eq!(round_trip(&state), state);
        }
        assert_eq!(
            serde_json::to_value(ConnectionStatus::PairingProvisional).unwrap(),
            "pairingProvisional"
        );
    }

    #[test]
    fn status_result_round_trips_with_camel_case_fields() {
        let status = StatusResult {
            phones: Vec::new(),
            selected_phone_id: String::new(),
            connection: ConnectionStatus::Live,
            phone_name: "Pixel".into(),
            calls: vec![CallView {
                call_id: "c1".into(),
                state: CallState::Active,
                remote_number: "+14155550123".into(),
                remote_display_name: "Alex".into(),
                started_at_ms: 1_700_000_000_000,
                is_conference: false,
                can_hold: true,
                can_merge: false,
                is_emergency: false,
            }],
            audio_route: AudioRoute::Bluetooth,
            microphone_muted: false,
            desktop_audio_available: true,
            audio_devices: Vec::new(),
            active_bt_device_address: String::new(),
        };
        assert_eq!(round_trip(&status), status);
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["desktopAudioAvailable"], true);
        assert_eq!(json["calls"][0]["canHold"], true);
    }
}
