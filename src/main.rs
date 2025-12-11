mod cli;
mod env;
mod error;

use std::process::ExitCode as StdExitCode;

fn main() -> StdExitCode {
    // Parse CLI arguments - clap handles usage errors and exits with code 2
    let command = cli::parse();
    
    // Run the main logic
    match run(command) {
        Ok(()) => StdExitCode::from(0),
        Err(e) => {
            // All errors go to stderr
            eprintln!("Error: {}", e);
            // Map our error to exit code
            let code = e.exit_code();
            StdExitCode::from(code as u8)
        }
    }
}

/// Main application logic - will be implemented in later features
fn run(command: cli::Command) -> Result<(), error::Error> {
    // Stub implementation - will be replaced by dispatch logic in F10
    // For now, just demonstrate error handling works
    match command {
        cli::Command::On => {
            // Placeholder: would call backend
            eprintln!("Debug: Command::On received");
            Ok(())
        }
        cli::Command::Off => {
            // Placeholder: would call backend
            eprintln!("Debug: Command::Off received");
            Ok(())
        }
        cli::Command::Status { json } => {
            // Placeholder: would query backend
            eprintln!("Debug: Command::Status {{ json: {} }} received", json);
            Ok(())
        }
    }
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
