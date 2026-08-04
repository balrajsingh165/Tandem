//! Wide-band speech negotiation: advertises mSBC via AT+BAC, answers +BCS codec
//! selection, and configures the SCO path for the agreed codec (CVSD fallback
//! always supported).

use crate::error::BluetoothError;
use crate::hfp::at::HfCommand;

/// Codec IDs from the HFP specification's assigned numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    /// Mandatory narrow-band codec; every AG supports it.
    Cvsd,
    /// Wide-band speech; preferred when the AG offers it.
    Msbc,
}

impl Codec {
    pub fn id(self) -> u8 {
        match self {
            Self::Cvsd => 1,
            Self::Msbc => 2,
        }
    }

    pub fn from_id(id: u8) -> Result<Self, BluetoothError> {
        match id {
            1 => Ok(Self::Cvsd),
            2 => Ok(Self::Msbc),
            _ => Err(BluetoothError::CodecNegotiationFailed),
        }
    }

    /// Sample rate the audio pipeline must run at for this codec.
    pub fn sample_rate_hz(self) -> u32 {
        match self {
            Self::Cvsd => tandem_audio::NARROW_BAND_HZ,
            Self::Msbc => tandem_audio::WIDE_BAND_HZ,
        }
    }
}

/// Tracks which codec the link ended up using.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecNegotiation {
    wide_band_supported: bool,
    selected: Option<Codec>,
}

impl Default for CodecNegotiation {
    fn default() -> Self {
        Self::new(true)
    }
}

impl CodecNegotiation {
    pub fn new(wide_band_supported: bool) -> Self {
        Self {
            wide_band_supported,
            selected: None,
        }
    }

    /// Codecs advertised in `AT+BAC`. CVSD is always included because it is
    /// mandatory, so negotiation can never end with no common codec.
    pub fn advertise(&self) -> HfCommand {
        let codecs = if self.wide_band_supported {
            vec![Codec::Cvsd.id(), Codec::Msbc.id()]
        } else {
            vec![Codec::Cvsd.id()]
        };
        HfCommand::Bac(codecs)
    }

    /// Accepts the AG's `+BCS` selection, echoing it back with `AT+BCS`.
    pub fn accept(&mut self, codec_id: u8) -> Result<HfCommand, BluetoothError> {
        let codec = Codec::from_id(codec_id)?;
        if matches!(codec, Codec::Msbc) && !self.wide_band_supported {
            return Err(BluetoothError::CodecNegotiationFailed);
        }
        self.selected = Some(codec);
        Ok(HfCommand::Bcs(codec.id()))
    }

    pub fn selected(&self) -> Option<Codec> {
        self.selected
    }

    /// Rate the pipeline runs at; CVSD until the AG says otherwise.
    pub fn effective_sample_rate_hz(&self) -> u32 {
        self.selected.unwrap_or(Codec::Cvsd).sample_rate_hz()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_both_codecs_when_wide_band_is_available() {
        let n = CodecNegotiation::new(true);
        assert_eq!(n.advertise(), HfCommand::Bac(vec![1, 2]));
    }

    #[test]
    fn always_advertises_the_mandatory_codec() {
        let n = CodecNegotiation::new(false);
        assert_eq!(n.advertise(), HfCommand::Bac(vec![1]));
    }

    #[test]
    fn accepting_msbc_selects_wide_band_rate() {
        let mut n = CodecNegotiation::new(true);
        assert_eq!(n.accept(2).unwrap(), HfCommand::Bcs(2));
        assert_eq!(n.selected(), Some(Codec::Msbc));
        assert_eq!(n.effective_sample_rate_hz(), 16_000);
    }

    #[test]
    fn falls_back_to_narrow_band_rate() {
        let mut n = CodecNegotiation::new(true);
        n.accept(1).unwrap();
        assert_eq!(n.selected(), Some(Codec::Cvsd));
        assert_eq!(n.effective_sample_rate_hz(), 8_000);
    }

    #[test]
    fn defaults_to_narrow_band_before_negotiation() {
        assert_eq!(
            CodecNegotiation::new(true).effective_sample_rate_hz(),
            8_000
        );
    }

    #[test]
    fn unknown_or_unsupported_codecs_are_refused() {
        assert!(CodecNegotiation::new(true).accept(7).is_err());
        assert!(CodecNegotiation::new(false).accept(2).is_err());
    }
}
