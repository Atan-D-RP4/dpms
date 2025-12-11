mod backend;
mod cli;
mod daemon;
mod drm_ops;
mod env;
mod error;
mod output;
mod tty;
mod wayland;

use std::process::ExitCode as StdExitCode;

fn main() -> StdExitCode {
    // Parse CLI arguments - clap handles usage errors and exits with code 2 (default clap behavior)
    let command = cli::parse();
    
    // Run the main logic
    match run(command) {
        Ok(()) => error::ExitCode::Success.into(),
        Err(e) => {
            // All errors go to stderr
            eprintln!("Error: {}", e);
            // Map our error to exit code using proper From impl
            e.exit_code().into()
        }
    }
}

/// Main application logic - will be implemented in later features
fn run(_command: cli::Command) -> Result<(), error::Error> {
    // Stub implementation - will be replaced by dispatch logic in F10
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_success_returns_exit_code_0() {
        let command = cli::Command::On;
        let result = run(command);
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_converts_to_exit_code_1() {
        let error = error::Error::UnsupportedEnvironment;
        let exit_code = error.exit_code();
        assert_eq!(exit_code, error::ExitCode::Error);
        assert_eq!(exit_code as i32, 1);
    }

    #[test]
    fn test_error_has_message() {
        let error = error::Error::ProtocolNotSupported;
        let message = error.to_string();
        assert!(message.contains("protocol"));
    }

    #[test]
    fn test_all_error_variants_map_to_exit_code_1() {
        // Verify all error types return exit code 1 (Error)
        let errors = vec![
            error::Error::UnsupportedEnvironment,
            error::Error::ProtocolNotSupported,
            error::Error::NoDisplayFound,
            error::Error::DaemonStartFailed,
            error::Error::DaemonStopTimeout,
        ];

        for err in errors {
            assert_eq!(err.exit_code() as i32, 1, "Error {:?} should exit with code 1", err);
        }
    }
}
