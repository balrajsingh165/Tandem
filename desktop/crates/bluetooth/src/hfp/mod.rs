//! OS-independent HFP v1.8 Hands-Free implementation: SLC bring-up, indicator
//! tracking, and codec negotiation as pure protocol logic over a byte channel
//! supplied by a backend. Call-control AT commands are deliberately not sent —
//! LAN is the intent path (docs/05).

pub mod at;
pub mod call_mirror;
pub mod codec_negotiation;
pub mod indicators;
pub mod slc;

pub use at::{AgResponse, HfCommand};
pub use codec_negotiation::Codec;
pub use indicators::{Indicators, CALL, CALLHELD, CALLSETUP};
pub use slc::{SlcPhase, SlcStateMachine};

/// Hands-free feature bitmap advertised in `AT+BRSF`. Tandem claims codec
/// negotiation and CLI presentation; it claims no call-control features because
/// it never issues call-control commands (docs/05).
pub const HF_FEATURES: u32 = 0b0000_1000_0000 | 0b0000_0000_0100;
