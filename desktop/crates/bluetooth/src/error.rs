//! BluetoothError: adapter, bonding, RFCOMM, SCO, and HFP-protocol failures with
//! degradation guidance (audio loss never ends the call — docs/05).

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BluetoothError {
    #[error("no Bluetooth adapter is available to Tandem")]
    NoAdapter,

    #[error("adapter is present but powered off")]
    AdapterOff,

    #[error("device {0} is not bonded with this desktop")]
    NotBonded(String),

    #[error("RFCOMM channel to the audio gateway failed: {0}")]
    Rfcomm(String),

    #[error("service-level connection could not be established: {0}")]
    SlcFailed(String),

    #[error("SCO audio link failed: {0}")]
    Sco(String),

    #[error("audio gateway sent a malformed AT response: {0}")]
    MalformedAt(String),

    #[error("no common codec with the audio gateway")]
    CodecNegotiationFailed,

    #[error("this backend is not available in this build or on this platform")]
    BackendUnavailable,
}

impl BluetoothError {
    /// Losing the media path is never fatal to the call: the phone falls back to
    /// its earpiece and the cellular leg continues untouched (docs/05). Only
    /// configuration-level failures stop Tandem from retrying attachment.
    pub fn degrades_to_handset(&self) -> bool {
        matches!(
            self,
            Self::Rfcomm(_) | Self::SlcFailed(_) | Self::Sco(_) | Self::MalformedAt(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_failures_degrade_rather_than_drop_the_call() {
        assert!(BluetoothError::Sco("timeout".into()).degrades_to_handset());
        assert!(BluetoothError::SlcFailed("no reply".into()).degrades_to_handset());
        assert!(BluetoothError::Rfcomm("reset".into()).degrades_to_handset());
    }

    #[test]
    fn configuration_failures_are_not_a_degradation_path() {
        assert!(!BluetoothError::NoAdapter.degrades_to_handset());
        assert!(!BluetoothError::NotBonded("AA:BB".into()).degrades_to_handset());
        assert!(!BluetoothError::BackendUnavailable.degrades_to_handset());
    }
}
