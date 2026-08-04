//! Compile-time and runtime backend selection: picks linux_bluez, usb_dongle, or
//! null by platform, feature flags, and configuration; exposes a uniform
//! constructor to the daemon.

pub mod null_backend;

#[cfg(feature = "linux_bluez")]
pub mod linux_bluez;

#[cfg(feature = "usb_dongle")]
pub mod usb_dongle;

use crate::backend::BluetoothBackend;

/// Backend requested by configuration. `Auto` resolves by platform and enabled
/// features; the others are explicit overrides for development and support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendKind {
    #[default]
    Auto,
    LinuxBluez,
    UsbDongle,
    Null,
}

impl BackendKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "linux_bluez" | "bluez" => Some(Self::LinuxBluez),
            "usb_dongle" | "dongle" => Some(Self::UsbDongle),
            "null" | "none" => Some(Self::Null),
            _ => None,
        }
    }

    /// Resolves `Auto` against what this build actually contains, so a binary
    /// never claims a backend it cannot construct.
    pub fn resolve(self) -> Self {
        match self {
            Self::Auto => {
                if cfg!(all(target_os = "linux", feature = "linux_bluez")) {
                    Self::LinuxBluez
                } else if cfg!(feature = "usb_dongle") {
                    Self::UsbDongle
                } else {
                    Self::Null
                }
            }
            explicit => explicit,
        }
    }
}

/// Constructs the selected backend. Tier B-lite is always reachable, so audio
/// support can be absent without the daemon losing its shape.
pub fn create(kind: BackendKind) -> Box<dyn BluetoothBackend> {
    match kind.resolve() {
        #[cfg(all(target_os = "linux", feature = "linux_bluez"))]
        BackendKind::LinuxBluez => Box::new(linux_bluez::BluezBackend::default()),

        #[cfg(feature = "usb_dongle")]
        BackendKind::UsbDongle => Box::new(usb_dongle::UsbDongleBackend::default()),

        _ => Box::new(null_backend::NullBluetoothBackend),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_configuration_values_and_aliases() {
        assert_eq!(BackendKind::parse("auto"), Some(BackendKind::Auto));
        assert_eq!(BackendKind::parse("BlueZ"), Some(BackendKind::LinuxBluez));
        assert_eq!(BackendKind::parse("dongle"), Some(BackendKind::UsbDongle));
        assert_eq!(BackendKind::parse("none"), Some(BackendKind::Null));
        assert_eq!(BackendKind::parse("nonsense"), None);
    }

    #[test]
    fn auto_never_resolves_to_a_backend_this_build_lacks() {
        let expected = if cfg!(all(target_os = "linux", feature = "linux_bluez")) {
            BackendKind::LinuxBluez
        } else if cfg!(feature = "usb_dongle") {
            BackendKind::UsbDongle
        } else {
            BackendKind::Null
        };
        assert_eq!(BackendKind::Auto.resolve(), expected);
    }

    #[test]
    #[cfg(not(any(feature = "linux_bluez", feature = "usb_dongle")))]
    fn a_build_without_backend_features_falls_back_to_tier_b_lite() {
        let backend = create(BackendKind::Auto);
        assert!(!backend.supports_audio());
    }

    /// Tier B-lite must remain reachable in every build, so a user whose
    /// hardware or OS cannot carry audio still gets control and history.
    #[test]
    fn null_is_constructible_regardless_of_enabled_features() {
        assert!(!create(BackendKind::Null).supports_audio());
    }
}
