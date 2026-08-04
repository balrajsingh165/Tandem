//! USB transport for HCI (interrupt/bulk/isochronous endpoints per the Bluetooth
//! USB transport spec) via WinUSB/IOKit through nusb; owns exclusive device claim
//! and hotplug detection.

use crate::error::BluetoothError;

/// Endpoint roles defined by the Bluetooth USB transport layer. Commands go out
/// on control, events arrive on interrupt, ACL uses bulk, and SCO uses
/// isochronous — which is why a controller without usable isochronous endpoints
/// cannot carry call audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointRole {
    Command,
    Event,
    AclIn,
    AclOut,
    ScoIn,
    ScoOut,
}

/// A controller Tandem may claim exclusively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerDescriptor {
    pub vendor_id: u16,
    pub product_id: u16,
    pub product_name: String,
    pub has_isochronous_endpoints: bool,
}

impl ControllerDescriptor {
    /// A controller is only usable for Tier B if it can carry SCO over
    /// isochronous endpoints; otherwise audio attach can never succeed.
    pub fn supports_call_audio(&self) -> bool {
        self.has_isochronous_endpoints
    }
}

/// Claims the controller exclusively so the OS Bluetooth stack cannot drive it
/// concurrently.
pub fn claim(_descriptor: &ControllerDescriptor) -> Result<(), BluetoothError> {
    Err(BluetoothError::BackendUnavailable)
}

pub fn release() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controllers_without_isochronous_endpoints_cannot_carry_audio() {
        let usable = ControllerDescriptor {
            vendor_id: 0x0a12,
            product_id: 0x0001,
            product_name: "Generic BT 5.0".into(),
            has_isochronous_endpoints: true,
        };
        let unusable = ControllerDescriptor {
            has_isochronous_endpoints: false,
            ..usable.clone()
        };
        assert!(usable.supports_call_audio());
        assert!(!unusable.supports_call_audio());
    }
}
