//! SLC establishment state machine per HFP v1.8 §4.2: BRSF exchange, CIND read,
//! CMER enable, CHLD query, then connected-idle. Emits typed SLC events; drives
//! at.rs over the backend's RFCOMM channel.

use crate::error::BluetoothError;
use crate::hfp::at::{AgResponse, HfCommand};
use crate::hfp::codec_negotiation::CodecNegotiation;
use crate::hfp::indicators::Indicators;
use crate::hfp::HF_FEATURES;

/// Ordered phases of service-level connection bring-up. The order is fixed by
/// the specification; an AG that answers out of order is a protocol violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlcPhase {
    Idle,
    BrsfSent,
    CindTestSent,
    CindReadSent,
    CmerSent,
    ChldSent,
    Established,
    Failed,
}

/// AG feature bit indicating codec negotiation support, from HFP v1.8.
const AG_FEATURE_CODEC_NEGOTIATION: u32 = 1 << 9;
/// AG feature bit indicating three-way calling, which gates the CHLD query.
const AG_FEATURE_THREE_WAY: u32 = 1 << 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlcStateMachine {
    phase: SlcPhase,
    ag_features: u32,
    indicators: Indicators,
    codec: CodecNegotiation,
}

impl Default for SlcStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl SlcStateMachine {
    pub fn new() -> Self {
        Self {
            phase: SlcPhase::Idle,
            ag_features: 0,
            indicators: Indicators::default(),
            codec: CodecNegotiation::new(true),
        }
    }

    pub fn phase(&self) -> SlcPhase {
        self.phase
    }

    pub fn indicators(&self) -> &Indicators {
        &self.indicators
    }

    pub fn codec(&self) -> &CodecNegotiation {
        &self.codec
    }

    pub fn is_established(&self) -> bool {
        self.phase == SlcPhase::Established
    }

    pub fn ag_supports_codec_negotiation(&self) -> bool {
        self.ag_features & AG_FEATURE_CODEC_NEGOTIATION != 0
    }

    /// First command of the handshake.
    pub fn start(&mut self) -> HfCommand {
        self.phase = SlcPhase::BrsfSent;
        HfCommand::Brsf(HF_FEATURES)
    }

    /// Advances the handshake. Returns the next command to send, or None when the
    /// SLC is established and the link goes idle awaiting indicator events.
    pub fn on_response(
        &mut self,
        response: &AgResponse,
    ) -> Result<Option<HfCommand>, BluetoothError> {
        match (self.phase, response) {
            (SlcPhase::BrsfSent, AgResponse::Brsf(features)) => {
                self.ag_features = *features;
                Ok(None)
            }
            (SlcPhase::BrsfSent, AgResponse::Ok) => {
                self.phase = SlcPhase::CindTestSent;
                Ok(Some(HfCommand::CindTest))
            }
            (SlcPhase::CindTestSent, AgResponse::CindSupported(names)) => {
                self.indicators.set_order(names.clone());
                Ok(None)
            }
            (SlcPhase::CindTestSent, AgResponse::Ok) => {
                self.phase = SlcPhase::CindReadSent;
                Ok(Some(HfCommand::CindRead))
            }
            (SlcPhase::CindReadSent, AgResponse::CindValues(values)) => {
                self.indicators.set_values(values);
                Ok(None)
            }
            (SlcPhase::CindReadSent, AgResponse::Ok) => {
                self.phase = SlcPhase::CmerSent;
                Ok(Some(HfCommand::CmerEnable))
            }
            (SlcPhase::CmerSent, AgResponse::Ok) => {
                if self.ag_features & AG_FEATURE_THREE_WAY != 0 {
                    self.phase = SlcPhase::ChldSent;
                    Ok(Some(HfCommand::ChldTest))
                } else {
                    self.phase = SlcPhase::Established;
                    Ok(None)
                }
            }
            (SlcPhase::ChldSent, AgResponse::Ok) => {
                self.phase = SlcPhase::Established;
                Ok(None)
            }
            (SlcPhase::ChldSent, AgResponse::Unhandled(_)) => Ok(None),

            (SlcPhase::Established, AgResponse::Ciev { index, value }) => {
                self.indicators.apply_ciev(*index, *value);
                Ok(None)
            }
            (SlcPhase::Established, AgResponse::Bcs(codec_id)) => {
                self.codec.accept(*codec_id).map(Some)
            }
            (SlcPhase::Established, _) => Ok(None),

            (_, AgResponse::Error) => {
                self.phase = SlcPhase::Failed;
                Err(BluetoothError::SlcFailed(format!(
                    "audio gateway rejected {:?}",
                    self.phase
                )))
            }
            (phase, unexpected) => {
                self.phase = SlcPhase::Failed;
                Err(BluetoothError::SlcFailed(format!(
                    "unexpected {unexpected:?} during {phase:?}"
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hfp::indicators::{CALL, CALLSETUP};

    /// Drives a full bring-up against an AG advertising codec negotiation and
    /// three-way calling.
    fn establish() -> SlcStateMachine {
        let mut slc = SlcStateMachine::new();
        assert_eq!(slc.start(), HfCommand::Brsf(HF_FEATURES));

        slc.on_response(&AgResponse::Brsf(
            AG_FEATURE_CODEC_NEGOTIATION | AG_FEATURE_THREE_WAY,
        ))
        .unwrap();
        assert_eq!(
            slc.on_response(&AgResponse::Ok).unwrap(),
            Some(HfCommand::CindTest)
        );

        slc.on_response(&AgResponse::CindSupported(vec![
            "service".into(),
            CALL.into(),
            CALLSETUP.into(),
        ]))
        .unwrap();
        assert_eq!(
            slc.on_response(&AgResponse::Ok).unwrap(),
            Some(HfCommand::CindRead)
        );

        slc.on_response(&AgResponse::CindValues(vec![1, 0, 0]))
            .unwrap();
        assert_eq!(
            slc.on_response(&AgResponse::Ok).unwrap(),
            Some(HfCommand::CmerEnable)
        );
        assert_eq!(
            slc.on_response(&AgResponse::Ok).unwrap(),
            Some(HfCommand::ChldTest)
        );
        assert_eq!(slc.on_response(&AgResponse::Ok).unwrap(), None);
        slc
    }

    #[test]
    fn completes_the_specified_bring_up_order() {
        let slc = establish();
        assert!(slc.is_established());
        assert!(slc.ag_supports_codec_negotiation());
        assert_eq!(slc.indicators().get("service"), Some(1));
    }

    #[test]
    fn skips_chld_when_the_ag_lacks_three_way_calling() {
        let mut slc = SlcStateMachine::new();
        slc.start();
        slc.on_response(&AgResponse::Brsf(0)).unwrap();
        slc.on_response(&AgResponse::Ok).unwrap();
        slc.on_response(&AgResponse::CindSupported(vec!["service".into()]))
            .unwrap();
        slc.on_response(&AgResponse::Ok).unwrap();
        slc.on_response(&AgResponse::CindValues(vec![1])).unwrap();
        slc.on_response(&AgResponse::Ok).unwrap();
        assert_eq!(slc.on_response(&AgResponse::Ok).unwrap(), None);
        assert!(slc.is_established());
    }

    #[test]
    fn indicator_events_flow_once_established() {
        let mut slc = establish();
        slc.on_response(&AgResponse::Ciev { index: 2, value: 1 })
            .unwrap();
        assert_eq!(slc.indicators().get(CALL), Some(1));
    }

    #[test]
    fn codec_selection_is_echoed_back_after_establishment() {
        let mut slc = establish();
        assert_eq!(
            slc.on_response(&AgResponse::Bcs(2)).unwrap(),
            Some(HfCommand::Bcs(2))
        );
        assert_eq!(slc.codec().effective_sample_rate_hz(), 16_000);
    }

    #[test]
    fn an_error_during_bring_up_fails_the_slc() {
        let mut slc = SlcStateMachine::new();
        slc.start();
        assert!(slc.on_response(&AgResponse::Error).is_err());
        assert_eq!(slc.phase(), SlcPhase::Failed);
    }

    #[test]
    fn out_of_order_responses_are_protocol_violations() {
        let mut slc = SlcStateMachine::new();
        slc.start();
        assert!(slc
            .on_response(&AgResponse::CindValues(vec![1, 0]))
            .is_err());
        assert_eq!(slc.phase(), SlcPhase::Failed);
    }
}
