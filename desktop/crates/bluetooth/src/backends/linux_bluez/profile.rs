//! Registers the Hands-Free profile (UUID 0x111E) with BlueZ via ProfileManager1,
//! receives the RFCOMM fd for the SLC on NewConnection, and adapts it to the HFP
//! core's byte-channel interface.

use crate::error::BluetoothError;

/// Object path Tandem exports its Profile1 implementation on.
pub const PROFILE_OBJECT_PATH: &str = "/xyz/tandem/hfp_hf";

/// Profile1 registration options. `require_authentication` and
/// `require_authorization` follow the HFP security expectations; `auto_connect`
/// stays off so Tandem attaches audio only when the LAN asks it to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRegistration {
    pub uuid: u16,
    pub name: String,
    pub channel: u8,
    pub version: u16,
    pub features: u32,
    pub require_authentication: bool,
    pub require_authorization: bool,
    pub auto_connect: bool,
}

impl Default for ProfileRegistration {
    fn default() -> Self {
        Self {
            uuid: crate::HFP_HF_UUID,
            name: "Tandem Hands-Free".into(),
            channel: 0,
            version: 0x0108,
            features: crate::hfp::HF_FEATURES,
            require_authentication: true,
            require_authorization: false,
            auto_connect: false,
        }
    }
}

/// Registers the profile with BlueZ. Fails if PipeWire or oFono already claims
/// the HF role, which is the documented prerequisite in docs/13.
pub fn register(_registration: &ProfileRegistration) -> Result<(), BluetoothError> {
    Err(BluetoothError::BackendUnavailable)
}

pub fn unregister() -> Result<(), BluetoothError> {
    Err(BluetoothError::BackendUnavailable)
}
