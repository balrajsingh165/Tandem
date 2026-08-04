//! Developer CLI probing a USB Bluetooth controller for Tandem compatibility:
//! HCI version, SCO-over-USB support, mSBC capability, and exclusive-claim
//! viability; prints a supported/unsupported verdict used in docs/13 bring-up.
//! [Tier B — Win/macOS USB dongle]

use std::process::ExitCode;

/// What the probe learned about one controller.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeReport {
    vendor_id: u16,
    product_id: u16,
    product_name: String,
    hci_version: Option<u8>,
    isochronous_endpoints: bool,
    msbc_capable: bool,
    exclusive_claim: bool,
}

/// Why a controller cannot serve as Tandem's Hands-Free radio.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Blocker {
    NoHciResponse,
    HciTooOld { found: u8, required: u8 },
    NoIsochronousEndpoints,
    ExclusiveClaimRefused,
}

impl Blocker {
    fn explain(&self) -> String {
        match self {
            Self::NoHciResponse => {
                "controller did not answer HCI Reset; it may be claimed by the OS stack".into()
            }
            Self::HciTooOld { found, required } => format!(
                "HCI version {found} is below the required {required}; eSCO and mSBC are unreliable"
            ),
            Self::NoIsochronousEndpoints => {
                "no isochronous endpoints; SCO call audio cannot be carried over USB".into()
            }
            Self::ExclusiveClaimRefused => {
                "exclusive claim refused; rebind the device to WinUSB, or grant IOKit access".into()
            }
        }
    }
}

/// HCI 4.0 is the floor: earlier controllers lack dependable eSCO for wide-band
/// speech.
const MIN_HCI_VERSION: u8 = 6;

impl ProbeReport {
    /// Every blocking reason, so one probe run tells the developer everything to
    /// fix rather than one problem at a time.
    fn blockers(&self) -> Vec<Blocker> {
        let mut blockers = Vec::new();
        match self.hci_version {
            None => blockers.push(Blocker::NoHciResponse),
            Some(version) if version < MIN_HCI_VERSION => blockers.push(Blocker::HciTooOld {
                found: version,
                required: MIN_HCI_VERSION,
            }),
            Some(_) => {}
        }
        if !self.isochronous_endpoints {
            blockers.push(Blocker::NoIsochronousEndpoints);
        }
        if !self.exclusive_claim {
            blockers.push(Blocker::ExclusiveClaimRefused);
        }
        blockers
    }

    fn is_supported(&self) -> bool {
        self.blockers().is_empty()
    }

    /// mSBC absence is a quality limitation, not a blocker: CVSD is mandatory and
    /// always available, so the controller still carries narrow-band call audio.
    fn wide_band_warning(&self) -> Option<&'static str> {
        if self.is_supported() && !self.msbc_capable {
            Some("mSBC unavailable: calls will use narrow-band CVSD only")
        } else {
            None
        }
    }

    fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{:04x}:{:04x}  {}\n",
            self.vendor_id, self.product_id, self.product_name
        ));
        out.push_str(&format!(
            "  HCI version        {}\n",
            self.hci_version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "no response".into())
        ));
        out.push_str(&format!(
            "  SCO over USB       {}\n",
            yes_no(self.isochronous_endpoints)
        ));
        out.push_str(&format!("  mSBC wide-band     {}\n", yes_no(self.msbc_capable)));
        out.push_str(&format!(
            "  Exclusive claim    {}\n",
            yes_no(self.exclusive_claim)
        ));

        if self.is_supported() {
            out.push_str("\nVERDICT: supported\n");
            if let Some(warning) = self.wide_band_warning() {
                out.push_str(&format!("  note: {warning}\n"));
            }
        } else {
            out.push_str("\nVERDICT: unsupported\n");
            for blocker in self.blockers() {
                out.push_str(&format!("  - {}\n", blocker.explain()));
            }
        }
        out
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

/// Enumerates candidate controllers. Returns empty until the nusb-backed
/// implementation lands with the dongle backend.
fn probe_controllers() -> Vec<ProbeReport> {
    Vec::new()
}

fn main() -> ExitCode {
    let reports = probe_controllers();

    if reports.is_empty() {
        eprintln!("usb-dongle-probe: no USB Bluetooth controllers found");
        eprintln!("  connect a controller, and on Windows rebind it to WinUSB first (docs/13)");
        return ExitCode::FAILURE;
    }

    let mut any_supported = false;
    for report in &reports {
        print!("{}", report.render());
        any_supported |= report.is_supported();
    }

    if any_supported {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supported() -> ProbeReport {
        ProbeReport {
            vendor_id: 0x0a12,
            product_id: 0x0001,
            product_name: "Generic BT 5.0".into(),
            hci_version: Some(10),
            isochronous_endpoints: true,
            msbc_capable: true,
            exclusive_claim: true,
        }
    }

    #[test]
    fn a_complete_controller_is_supported() {
        let report = supported();
        assert!(report.is_supported());
        assert!(report.blockers().is_empty());
        assert!(report.render().contains("VERDICT: supported"));
    }

    #[test]
    fn missing_isochronous_endpoints_blocks_call_audio() {
        let report = ProbeReport {
            isochronous_endpoints: false,
            ..supported()
        };
        assert!(!report.is_supported());
        assert_eq!(report.blockers(), vec![Blocker::NoIsochronousEndpoints]);
    }

    #[test]
    fn an_old_controller_is_rejected_with_both_versions_named() {
        let report = ProbeReport {
            hci_version: Some(4),
            ..supported()
        };
        assert_eq!(
            report.blockers(),
            vec![Blocker::HciTooOld {
                found: 4,
                required: MIN_HCI_VERSION
            }]
        );
        assert!(report.render().contains("below the required"));
    }

    #[test]
    fn a_silent_controller_suggests_the_os_stack_still_owns_it() {
        let report = ProbeReport {
            hci_version: None,
            ..supported()
        };
        assert!(report.blockers().contains(&Blocker::NoHciResponse));
        assert!(report.render().contains("claimed by the OS stack"));
    }

    /// One run should list every problem, not just the first.
    #[test]
    fn all_blockers_are_reported_together() {
        let report = ProbeReport {
            hci_version: None,
            isochronous_endpoints: false,
            exclusive_claim: false,
            ..supported()
        };
        assert_eq!(report.blockers().len(), 3);
    }

    /// Lacking wide-band speech is a quality note, not a rejection — CVSD is
    /// mandatory and always available.
    #[test]
    fn missing_msbc_warns_but_still_passes() {
        let report = ProbeReport {
            msbc_capable: false,
            ..supported()
        };
        assert!(report.is_supported());
        assert!(report.wide_band_warning().is_some());
        assert!(report.render().contains("narrow-band CVSD only"));
    }
}
