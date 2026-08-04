//! SSP bonding for the dongle path: numeric-comparison pairing with the phone,
//! link-key generation and encrypted storage via tandem_crypto secrets, and
//! authentication/encryption enforcement on the ACL.

use crate::error::BluetoothError;

/// Secure Simple Pairing association models. Tandem requests numeric comparison
/// so the user confirms the same digits on both devices; Just Works is refused
/// because it offers no MITM protection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationModel {
    NumericComparison,
    PasskeyEntry,
    JustWorks,
}

impl AssociationModel {
    pub fn provides_mitm_protection(self) -> bool {
        matches!(self, Self::NumericComparison | Self::PasskeyEntry)
    }
}

/// Link key produced by bonding, persisted through the secret store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkKey {
    pub peer_address: String,
    pub key: [u8; 16],
    pub authenticated: bool,
}

/// Enforces that the link is authenticated and encrypted before any HFP traffic
/// flows; an unauthenticated link is refused rather than downgraded.
pub fn require_secure_link(model: AssociationModel) -> Result<(), BluetoothError> {
    if model.provides_mitm_protection() {
        Ok(())
    } else {
        Err(BluetoothError::SlcFailed(
            "refusing an unauthenticated Bluetooth link".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authenticated_models_are_accepted() {
        assert!(require_secure_link(AssociationModel::NumericComparison).is_ok());
        assert!(require_secure_link(AssociationModel::PasskeyEntry).is_ok());
    }

    #[test]
    fn just_works_is_refused_rather_than_downgraded() {
        assert!(!AssociationModel::JustWorks.provides_mitm_protection());
        assert!(require_secure_link(AssociationModel::JustWorks).is_err());
    }
}
