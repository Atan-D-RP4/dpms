#![allow(clippy::enum_variant_names)]
/// Exit codes for dpms CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Operation completed successfully
    Success = 0,
    /// Runtime error occurred
    Error = 1,
    /// Invalid command-line usage (reserved for clap, currently unused by dpms)
    Usage = 2,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> i32 {
        code as i32
    }
}

impl From<ExitCode> for std::process::ExitCode {
    fn from(code: ExitCode) -> std::process::ExitCode {
        std::process::ExitCode::from(code as u8)
    }
}

/// Error types for dpms
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Neither Wayland nor TTY environment available")]
    UnsupportedEnvironment,

    #[error("Compositor does not support power management protocol")]
    ProtocolNotSupported,

    #[error("No connected display found")]
    NoDisplayFound,

    #[error("Display '{name}' not found. Available: {}", available.join(", "))]
    DisplayNotFound { name: String, available: Vec<String> },

    #[error("Display '{name}' is ambiguous. Did you mean: {}?", candidates.join(", "))]
    AmbiguousDisplay { name: String, candidates: Vec<String> },

    #[error("Daemon failed to start: {0}")]
    DaemonStartFailed(String),

    #[error("Daemon did not stop within timeout period")]
    DaemonStopTimeout,

    #[error("Signal operation failed: {0}")]
    SignalError(String),

    #[error("PID file operation failed: {0}")]
    PidFileError(String),

    #[error("DRM operation failed: {0}")]
    DrmError(String),

    #[error("libseat operation failed: {0}")]
    SeatError(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Get the appropriate exit code for this error
    pub fn exit_code(&self) -> ExitCode {
        // All runtime errors use ExitCode::Error (1)
        // Usage errors would be handled separately by clap
        ExitCode::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_values() {
        assert_eq!(ExitCode::Success as i32, 0);
        assert_eq!(ExitCode::Error as i32, 1);
        assert_eq!(ExitCode::Usage as i32, 2);
    }

    #[test]
    fn exit_code_from_conversion() {
        assert_eq!(i32::from(ExitCode::Success), 0);
        assert_eq!(i32::from(ExitCode::Error), 1);
        assert_eq!(i32::from(ExitCode::Usage), 2);
    }

    #[test]
    fn all_errors_return_error_exit_code() {
        let errors = [
            Error::UnsupportedEnvironment,
            Error::ProtocolNotSupported,
            Error::NoDisplayFound,
            Error::DisplayNotFound {
                name: "HDMI-1".to_string(),
                available: vec!["DP-1".to_string(), "eDP-1".to_string()],
            },
            Error::AmbiguousDisplay {
                name: "DP".to_string(),
                candidates: vec!["DP-1".to_string(), "DP-2".to_string()],
            },
            Error::DaemonStartFailed("test".to_string()),
            Error::DaemonStopTimeout,
            Error::SignalError("test".to_string()),
            Error::PidFileError("test".to_string()),
            Error::DrmError("test".to_string()),
            Error::SeatError("test".to_string()),
            Error::Io(std::io::Error::other("test")),
        ];

        for error in errors {
            assert_eq!(
                error.exit_code(),
                ExitCode::Error,
                "Error variant {:?} should return ExitCode::Error",
                error
            );
        }
    }

    #[test]
    fn error_messages_are_non_empty() {
        let errors = [
            Error::UnsupportedEnvironment,
            Error::ProtocolNotSupported,
            Error::NoDisplayFound,
            Error::DisplayNotFound {
                name: "HDMI-1".to_string(),
                available: vec!["DP-1".to_string(), "eDP-1".to_string()],
            },
            Error::AmbiguousDisplay {
                name: "DP".to_string(),
                candidates: vec!["DP-1".to_string(), "DP-2".to_string()],
            },
            Error::DaemonStartFailed("test".to_string()),
            Error::DaemonStopTimeout,
            Error::SignalError("test".to_string()),
            Error::PidFileError("test".to_string()),
            Error::DrmError("test".to_string()),
            Error::SeatError("test".to_string()),
            Error::Io(std::io::Error::other("test")),
        ];

        for error in errors {
            let message = error.to_string();
            assert!(
                !message.is_empty(),
                "Error variant {:?} should have a non-empty message",
                error
            );
        }
    }

    #[test]
    fn display_not_found_error_message() {
        let error = Error::DisplayNotFound {
            name: "HDMI-1".to_string(),
            available: vec!["DP-1".to_string(), "eDP-1".to_string()],
        };
        let message = error.to_string();
        assert!(message.contains("HDMI-1"));
        assert!(message.contains("not found"));
        assert!(message.contains("DP-1"));
        assert!(message.contains("eDP-1"));
    }

    #[test]
    fn ambiguous_display_error_message() {
        let error = Error::AmbiguousDisplay {
            name: "DP".to_string(),
            candidates: vec!["DP-1".to_string(), "DP-2".to_string()],
        };
        let message = error.to_string();
        assert!(message.contains("DP"));
        assert!(message.contains("ambiguous"));
        assert!(message.contains("DP-1"));
        assert!(message.contains("DP-2"));
    }
}
