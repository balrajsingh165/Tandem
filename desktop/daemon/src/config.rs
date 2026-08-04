//! Loads and validates config.toml (paired-phone endpoint hints, backend
//! selection, audio devices, log level) with CLI overrides; documents every key
//! in docs/09.

use tandem_bluetooth::backends::BackendKind;

/// Effective daemon configuration after file and CLI layers are merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub phone_host: Option<String>,
    pub phone_port: u16,
    pub bluetooth_backend: BackendKind,
    pub audio_capture_device: String,
    pub audio_playback_device: String,
    pub log_level: LogLevel,
    pub ipc_socket_override: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "error" => Some(Self::Error),
            "warn" | "warning" => Some(Self::Warn),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            "trace" => Some(Self::Trace),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            phone_host: None,
            phone_port: tandem_transport::DEFAULT_PORT,
            bluetooth_backend: BackendKind::Auto,
            audio_capture_device: String::new(),
            audio_playback_device: String::new(),
            log_level: LogLevel::Info,
            ipc_socket_override: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("unknown flag {0}")]
    UnknownFlag(String),

    #[error("flag {0} requires a value")]
    MissingValue(String),

    #[error("invalid value for {flag}: {value}")]
    InvalidValue { flag: String, value: String },
}

impl Config {
    /// Applies CLI overrides on top of the file layer. Flags mirror the keys in
    /// config.toml so a setting can be tried without editing the file.
    pub fn apply_args<I, S>(mut self, args: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();
        let mut index = 0;

        while index < args.len() {
            let flag = args[index].clone();

            match flag.as_str() {
                "--phone-host" => self.phone_host = Some(take_value(&args, &mut index, &flag)?),
                "--phone-port" => {
                    let raw = take_value(&args, &mut index, &flag)?;
                    self.phone_port = raw.parse().map_err(|_| ConfigError::InvalidValue {
                        flag: "--phone-port".into(),
                        value: raw,
                    })?;
                }
                "--bluetooth-backend" => {
                    let raw = take_value(&args, &mut index, &flag)?;
                    self.bluetooth_backend =
                        BackendKind::parse(&raw).ok_or_else(|| ConfigError::InvalidValue {
                            flag: "--bluetooth-backend".into(),
                            value: raw,
                        })?;
                }
                "--audio-capture" => {
                    self.audio_capture_device = take_value(&args, &mut index, &flag)?
                }
                "--audio-playback" => {
                    self.audio_playback_device = take_value(&args, &mut index, &flag)?
                }
                "--log-level" => {
                    let raw = take_value(&args, &mut index, &flag)?;
                    self.log_level =
                        LogLevel::parse(&raw).ok_or_else(|| ConfigError::InvalidValue {
                            flag: "--log-level".into(),
                            value: raw,
                        })?;
                }
                "--ipc-socket" => {
                    self.ipc_socket_override = Some(take_value(&args, &mut index, &flag)?)
                }
                other => return Err(ConfigError::UnknownFlag(other.to_string())),
            }
            index += 1;
        }

        Ok(self)
    }
}

/// Consumes the value following a flag, advancing the cursor past it.
fn take_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, ConfigError> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| ConfigError::MissingValue(flag.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_use_the_documented_port_and_auto_backend() {
        let config = Config::default();
        assert_eq!(config.phone_port, 46521);
        assert_eq!(config.bluetooth_backend, BackendKind::Auto);
        assert_eq!(config.log_level, LogLevel::Info);
    }

    #[test]
    fn cli_flags_override_the_defaults() {
        let config = Config::default()
            .apply_args([
                "--phone-host",
                "192.168.1.20",
                "--phone-port",
                "47000",
                "--bluetooth-backend",
                "null",
                "--log-level",
                "debug",
            ])
            .unwrap();
        assert_eq!(config.phone_host.as_deref(), Some("192.168.1.20"));
        assert_eq!(config.phone_port, 47000);
        assert_eq!(config.bluetooth_backend, BackendKind::Null);
        assert_eq!(config.log_level, LogLevel::Debug);
    }

    #[test]
    fn unknown_flags_are_rejected_rather_than_ignored() {
        assert_eq!(
            Config::default().apply_args(["--nonsense"]),
            Err(ConfigError::UnknownFlag("--nonsense".into()))
        );
    }

    #[test]
    fn flags_missing_values_are_rejected() {
        assert_eq!(
            Config::default().apply_args(["--phone-port"]),
            Err(ConfigError::MissingValue("--phone-port".into()))
        );
    }

    #[test]
    fn invalid_values_name_the_offending_flag() {
        assert_eq!(
            Config::default().apply_args(["--phone-port", "not-a-port"]),
            Err(ConfigError::InvalidValue {
                flag: "--phone-port".into(),
                value: "not-a-port".into()
            })
        );
        assert!(Config::default()
            .apply_args(["--bluetooth-backend", "quantum"])
            .is_err());
    }

    #[test]
    fn log_levels_round_trip_through_their_names() {
        for level in [
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
            LogLevel::Trace,
        ] {
            assert_eq!(LogLevel::parse(level.as_str()), Some(level));
        }
        assert_eq!(LogLevel::parse("WARNING"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::parse("loud"), None);
    }
}
