//! BluetoothBackend fake: scripted adapter/bond/RFCOMM/SCO behavior including
//! mid-call SCO drops, backing controller and degradation tests.

use std::collections::VecDeque;

use tandem_bluetooth::backend::{AdapterInfo, BackendEvent, BluetoothBackend, ScoState};
use tandem_bluetooth::error::BluetoothError;
use tandem_bluetooth::hfp::codec_negotiation::Codec;

/// In-memory backend whose failures are scripted, so degradation paths can be
/// exercised deterministically.
#[derive(Debug)]
pub struct FakeBluetoothBackend {
    adapter: AdapterInfo,
    bonded: Vec<String>,
    sco: ScoState,
    at_inbox: VecDeque<String>,
    pub at_written: Vec<String>,
    pub sco_written: Vec<i16>,
    events: Vec<BackendEvent>,
    fail_sco_open: bool,
}

impl Default for FakeBluetoothBackend {
    fn default() -> Self {
        Self {
            adapter: AdapterInfo {
                address: "AA:BB:CC:DD:EE:FF".into(),
                name: "Fake Adapter".into(),
                powered: true,
            },
            bonded: vec!["11:22:33:44:55:66".into()],
            sco: ScoState::Closed,
            at_inbox: VecDeque::new(),
            at_written: Vec::new(),
            sco_written: Vec::new(),
            events: Vec::new(),
            fail_sco_open: false,
        }
    }
}

impl FakeBluetoothBackend {
    /// Makes the next `open_sco` fail, so callers can assert the call survives.
    pub fn fail_next_sco_open(&mut self) {
        self.fail_sco_open = true;
    }

    /// Queues a line the AG would send.
    pub fn push_at_line(&mut self, line: &str) {
        self.at_inbox.push_back(line.to_string());
    }

    /// Simulates the link dropping mid-call. The cellular call must continue on
    /// the handset (docs/05).
    pub fn drop_link(&mut self, reason: &str) {
        self.sco = ScoState::Closed;
        self.events.push(BackendEvent::LinkLost {
            reason: reason.to_string(),
        });
    }
}

impl BluetoothBackend for FakeBluetoothBackend {
    fn adapter(&self) -> Result<AdapterInfo, BluetoothError> {
        Ok(self.adapter.clone())
    }

    fn is_bonded(&self, address: &str) -> Result<bool, BluetoothError> {
        Ok(self.bonded.iter().any(|a| a == address))
    }

    fn connect_rfcomm(&mut self, address: &str) -> Result<(), BluetoothError> {
        if !self.is_bonded(address)? {
            return Err(BluetoothError::NotBonded(address.to_string()));
        }
        self.events.push(BackendEvent::RfcommConnected {
            address: address.to_string(),
        });
        Ok(())
    }

    fn write_at(&mut self, line: &str) -> Result<(), BluetoothError> {
        self.at_written.push(line.to_string());
        Ok(())
    }

    fn read_at_line(&mut self) -> Result<Option<String>, BluetoothError> {
        Ok(self.at_inbox.pop_front())
    }

    fn open_sco(&mut self, codec: Codec) -> Result<(), BluetoothError> {
        if self.fail_sco_open {
            self.fail_sco_open = false;
            return Err(BluetoothError::Sco("scripted failure".into()));
        }
        self.sco = ScoState::Open { codec };
        self.events.push(BackendEvent::ScoStateChanged(self.sco));
        Ok(())
    }

    fn close_sco(&mut self) {
        self.sco = ScoState::Closed;
        self.events.push(BackendEvent::ScoStateChanged(self.sco));
    }

    fn sco_state(&self) -> ScoState {
        self.sco
    }

    fn read_sco(&mut self, out: &mut [i16]) -> Result<usize, BluetoothError> {
        if !matches!(self.sco, ScoState::Open { .. }) {
            return Err(BluetoothError::Sco("SCO is not open".into()));
        }
        out.fill(0);
        Ok(out.len())
    }

    fn write_sco(&mut self, samples: &[i16]) -> Result<(), BluetoothError> {
        if !matches!(self.sco, ScoState::Open { .. }) {
            return Err(BluetoothError::Sco("SCO is not open".into()));
        }
        self.sco_written.extend_from_slice(samples);
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.events)
    }

    fn disconnect(&mut self) {
        self.close_sco();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unbonded_devices_cannot_open_rfcomm() {
        let mut b = FakeBluetoothBackend::default();
        assert!(matches!(
            b.connect_rfcomm("99:99:99:99:99:99"),
            Err(BluetoothError::NotBonded(_))
        ));
        assert!(b.connect_rfcomm("11:22:33:44:55:66").is_ok());
    }

    #[test]
    fn audio_only_flows_while_sco_is_open() {
        let mut b = FakeBluetoothBackend::default();
        assert!(b.write_sco(&[1, 2, 3]).is_err());

        b.open_sco(Codec::Msbc).unwrap();
        assert_eq!(b.sco_state(), ScoState::Open { codec: Codec::Msbc });
        b.write_sco(&[1, 2, 3]).unwrap();
        assert_eq!(b.sco_written, vec![1, 2, 3]);

        b.close_sco();
        assert!(b.write_sco(&[4]).is_err());
    }

    /// A scripted SCO failure is a media-path failure only: it degrades to the
    /// handset and never ends the cellular call.
    #[test]
    fn a_failed_sco_open_is_a_degradation_not_a_call_failure() {
        let mut b = FakeBluetoothBackend::default();
        b.fail_next_sco_open();
        let err = b.open_sco(Codec::Cvsd).unwrap_err();
        assert!(err.degrades_to_handset());
        assert_eq!(b.sco_state(), ScoState::Closed);

        b.open_sco(Codec::Cvsd).unwrap();
        assert!(matches!(b.sco_state(), ScoState::Open { .. }));
    }

    #[test]
    fn a_mid_call_link_drop_is_reported_as_an_event() {
        let mut b = FakeBluetoothBackend::default();
        b.open_sco(Codec::Cvsd).unwrap();
        b.drain_events();

        b.drop_link("out of range");
        let events = b.drain_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, BackendEvent::LinkLost { .. })));
        assert_eq!(b.sco_state(), ScoState::Closed);
    }
}
