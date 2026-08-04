//! Fake HFP AG speaking the AT protocol over an in-memory byte channel: drives
//! SLC bring-up, indicator sequences, codec negotiation, and SCO open/close for
//! hfp core tests (docs/15 integration tier).

use tandem_bluetooth::hfp::at::{AgResponse, HfCommand};
use tandem_bluetooth::hfp::indicators::{CALL, CALLHELD, CALLSETUP, SERVICE};

/// AG feature bits the fake advertises, matching a typical Android gateway.
pub const AG_FEATURES_TYPICAL: u32 = (1 << 9) | (1 << 0);

/// Scriptable Audio Gateway. It answers hands-free commands exactly as the
/// specification requires, so the real SLC state machine can be driven against
/// it without hardware.
#[derive(Debug, Clone)]
pub struct FakeAudioGateway {
    features: u32,
    indicator_names: Vec<String>,
    indicator_values: Vec<u8>,
    wide_band: bool,
    pub received: Vec<HfCommand>,
}

impl Default for FakeAudioGateway {
    fn default() -> Self {
        Self {
            features: AG_FEATURES_TYPICAL,
            indicator_names: vec![
                SERVICE.into(),
                CALL.into(),
                CALLSETUP.into(),
                CALLHELD.into(),
            ],
            indicator_values: vec![1, 0, 0, 0],
            wide_band: true,
            received: Vec::new(),
        }
    }
}

impl FakeAudioGateway {
    /// An AG without codec negotiation or three-way calling, to exercise the
    /// narrow-band and CHLD-skipping paths.
    pub fn minimal() -> Self {
        Self {
            features: 0,
            wide_band: false,
            ..Self::default()
        }
    }

    pub fn features(&self) -> u32 {
        self.features
    }

    pub fn supports_wide_band(&self) -> bool {
        self.wide_band
    }

    /// Responds to one hands-free command. Every command yields its unsolicited
    /// results first and terminates with OK, as the specification requires.
    pub fn respond(&mut self, command: &HfCommand) -> Vec<AgResponse> {
        self.received.push(command.clone());
        match command {
            HfCommand::Brsf(_) => vec![AgResponse::Brsf(self.features), AgResponse::Ok],
            HfCommand::CindTest => vec![
                AgResponse::CindSupported(self.indicator_names.clone()),
                AgResponse::Ok,
            ],
            HfCommand::CindRead => vec![
                AgResponse::CindValues(self.indicator_values.clone()),
                AgResponse::Ok,
            ],
            HfCommand::CmerEnable => vec![AgResponse::Ok],
            HfCommand::ChldTest => vec![
                AgResponse::Unhandled("+CHLD: (0,1,2,3)".into()),
                AgResponse::Ok,
            ],
            HfCommand::Bac(_) => {
                let mut out = vec![AgResponse::Ok];
                if self.wide_band {
                    out.push(AgResponse::Bcs(2));
                }
                out
            }
            HfCommand::Bcs(_) | HfCommand::ClipEnable | HfCommand::Clcc => vec![AgResponse::Ok],
            HfCommand::SpeakerGain(g) => vec![AgResponse::SpeakerGain(*g), AgResponse::Ok],
            HfCommand::MicrophoneGain(g) => vec![AgResponse::MicrophoneGain(*g), AgResponse::Ok],
        }
    }

    /// Emits an indicator change, as the AG would when the call plane moves.
    pub fn change_indicator(&mut self, name: &str, value: u8) -> Option<AgResponse> {
        let position = self.indicator_names.iter().position(|n| n == name)?;
        self.indicator_values[position] = value;
        Some(AgResponse::Ciev {
            index: position + 1,
            value,
        })
    }

    /// Simulates an incoming call: RING plus caller identity.
    pub fn ring(&mut self, number: &str) -> Vec<AgResponse> {
        let mut out = vec![AgResponse::Ring];
        out.push(AgResponse::Clip {
            number: number.to_string(),
        });
        if let Some(ciev) = self.change_indicator(CALLSETUP, 1) {
            out.push(ciev);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tandem_bluetooth::hfp::slc::{SlcPhase, SlcStateMachine};

    /// Drives the real SLC state machine against the fake AG until the
    /// service-level connection is established.
    fn bring_up(ag: &mut FakeAudioGateway) -> SlcStateMachine {
        let mut slc = SlcStateMachine::new();
        let mut pending = Some(slc.start());

        while let Some(command) = pending.take() {
            for response in ag.respond(&command) {
                if let Some(next) = slc
                    .on_response(&response)
                    .expect("fake AG speaks valid HFP")
                {
                    pending = Some(next);
                }
            }
        }
        slc
    }

    #[test]
    fn a_typical_gateway_reaches_an_established_slc() {
        let mut ag = FakeAudioGateway::default();
        let slc = bring_up(&mut ag);
        assert_eq!(slc.phase(), SlcPhase::Established);
        assert!(slc.ag_supports_codec_negotiation());
        assert_eq!(slc.indicators().get(SERVICE), Some(1));
    }

    #[test]
    fn bring_up_issues_the_specified_command_sequence() {
        let mut ag = FakeAudioGateway::default();
        bring_up(&mut ag);
        assert!(matches!(ag.received[0], HfCommand::Brsf(_)));
        assert_eq!(ag.received[1], HfCommand::CindTest);
        assert_eq!(ag.received[2], HfCommand::CindRead);
        assert_eq!(ag.received[3], HfCommand::CmerEnable);
        assert_eq!(ag.received[4], HfCommand::ChldTest);
    }

    #[test]
    fn a_minimal_gateway_skips_chld_and_still_establishes() {
        let mut ag = FakeAudioGateway::minimal();
        let slc = bring_up(&mut ag);
        assert_eq!(slc.phase(), SlcPhase::Established);
        assert!(!slc.ag_supports_codec_negotiation());
        assert!(!ag.received.contains(&HfCommand::ChldTest));
    }

    #[test]
    fn indicator_changes_reach_the_state_machine() {
        let mut ag = FakeAudioGateway::default();
        let mut slc = bring_up(&mut ag);

        let ciev = ag.change_indicator(CALL, 1).unwrap();
        slc.on_response(&ciev).unwrap();
        assert_eq!(slc.indicators().get(CALL), Some(1));
    }

    #[test]
    fn an_incoming_call_is_visible_through_indicators() {
        let mut ag = FakeAudioGateway::default();
        let mut slc = bring_up(&mut ag);

        for response in ag.ring("+14155550123") {
            slc.on_response(&response).unwrap();
        }
        assert_eq!(slc.indicators().get(CALLSETUP), Some(1));
    }

    #[test]
    fn wide_band_gateways_negotiate_msbc_end_to_end() {
        let mut ag = FakeAudioGateway::default();
        let mut slc = bring_up(&mut ag);

        let advertise = slc.codec().advertise();
        for response in ag.respond(&advertise) {
            if let Some(reply) = slc.on_response(&response).unwrap() {
                ag.respond(&reply);
            }
        }
        assert_eq!(slc.codec().effective_sample_rate_hz(), 16_000);
    }

    #[test]
    fn narrow_band_gateways_stay_at_cvsd() {
        let mut ag = FakeAudioGateway::minimal();
        let mut slc = bring_up(&mut ag);

        let advertise = slc.codec().advertise();
        for response in ag.respond(&advertise) {
            slc.on_response(&response).unwrap();
        }
        assert_eq!(slc.codec().effective_sample_rate_hz(), 8_000);
    }
}
