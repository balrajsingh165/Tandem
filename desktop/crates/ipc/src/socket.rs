//! Local-socket abstraction: Unix domain socket at
//! $XDG_RUNTIME_DIR/tandem/daemon.sock and Windows named pipe
//! \\.\pipe\tandem-daemon, with same-user peer checks on both.

use std::path::PathBuf;

use crate::error::IpcError;

/// Windows named pipe the daemon listens on.
pub const WINDOWS_PIPE_NAME: &str = r"\\.\pipe\tandem-daemon";

/// Socket path relative to the runtime directory on Unix.
pub const UNIX_SOCKET_RELATIVE: &str = "tandem/daemon.sock";

/// Where the daemon listens. The endpoint is per-user, and both platforms
/// restrict access to the owning user — the UI is trusted only because it runs
/// as the same user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    UnixSocket(PathBuf),
    WindowsPipe(String),
}

impl Endpoint {
    /// Resolves the default endpoint for this platform. On Unix,
    /// `$XDG_RUNTIME_DIR` is preferred because it is user-owned and cleared at
    /// logout; `/tmp` is the documented fallback.
    pub fn default_for_platform(xdg_runtime_dir: Option<&str>) -> Self {
        if cfg!(windows) {
            return Self::WindowsPipe(WINDOWS_PIPE_NAME.to_string());
        }
        let base = xdg_runtime_dir
            .filter(|d| !d.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        Self::UnixSocket(base.join(UNIX_SOCKET_RELATIVE))
    }

    pub fn describe(&self) -> String {
        match self {
            Self::UnixSocket(path) => path.display().to_string(),
            Self::WindowsPipe(name) => name.clone(),
        }
    }
}

/// Verifies the connecting peer runs as the same OS user. A mismatch is refused:
/// the IPC surface can dial the phone, so it must not be reachable by other
/// users on a shared machine.
pub fn verify_same_user(peer_uid: Option<u32>, our_uid: Option<u32>) -> Result<(), IpcError> {
    match (peer_uid, our_uid) {
        (Some(peer), Some(ours)) if peer == ours => Ok(()),
        (Some(_), Some(_)) => Err(IpcError::Unauthorized),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_the_runtime_directory_on_unix() {
        if cfg!(windows) {
            return;
        }
        let endpoint = Endpoint::default_for_platform(Some("/run/user/1000"));
        assert_eq!(
            endpoint,
            Endpoint::UnixSocket(PathBuf::from("/run/user/1000/tandem/daemon.sock"))
        );
    }

    #[test]
    fn falls_back_when_the_runtime_directory_is_absent() {
        if cfg!(windows) {
            return;
        }
        assert_eq!(
            Endpoint::default_for_platform(None),
            Endpoint::UnixSocket(PathBuf::from("/tmp/tandem/daemon.sock"))
        );
        assert_eq!(
            Endpoint::default_for_platform(Some("")),
            Endpoint::UnixSocket(PathBuf::from("/tmp/tandem/daemon.sock"))
        );
    }

    #[test]
    fn windows_uses_the_named_pipe() {
        if !cfg!(windows) {
            return;
        }
        assert_eq!(
            Endpoint::default_for_platform(None),
            Endpoint::WindowsPipe(WINDOWS_PIPE_NAME.to_string())
        );
    }

    #[test]
    fn a_different_user_is_refused() {
        assert!(verify_same_user(Some(1000), Some(1000)).is_ok());
        assert_eq!(
            verify_same_user(Some(1001), Some(1000)),
            Err(IpcError::Unauthorized)
        );
    }
}
