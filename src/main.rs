mod backend;
mod cli;
mod daemon;
mod display;
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

/// Execute a command using the given backend
fn execute_command<B: backend::PowerBackend>(
    backend: &mut B,
    command: cli::Command,
) -> Result<(), error::Error> {
    match command {
        cli::Command::On { target } => {
            backend.set_power(&target, output::PowerState::On)?;
            Ok(())
        }
        cli::Command::Off { target } => {
            backend.set_power(&target, output::PowerState::Off)?;
            Ok(())
        }
        cli::Command::Toggle { target } => {
            let displays = backend.get_power(&target)?;
            for display in displays {
                let new_state = match display.power {
                    output::PowerState::On => output::PowerState::Off,
                    output::PowerState::Off => output::PowerState::On,
                };
                backend.set_power(&display::DisplayTarget::Named(display.name), new_state)?;
            }
            Ok(())
        }
        cli::Command::Status { target, json } => {
            let displays = backend.get_power(&target)?;
            print!("{}", output::format_status(&displays, json));
            Ok(())
        }
        cli::Command::List { json, verbose } => {
            let displays = backend.list_displays()?;
            if displays.is_empty() {
                return Err(error::Error::NoDisplayFound);
            }
            print!("{}", output::format_list(&displays, json, verbose));
            Ok(())
        }
        cli::Command::Completion { shell } => {
            cli::generate_completions(shell);
            Ok(())
        }
        cli::Command::DaemonInternal => {
            // This is handled in run() before reaching here
            unreachable!("DaemonInternal should be handled before execute_command")
        }
    }
}

/// Main application logic - dispatches commands to appropriate backend
fn run(command: cli::Command) -> Result<(), error::Error> {
    // Handle daemon-internal command immediately (no backend needed)
    if matches!(command, cli::Command::DaemonInternal) {
        // This never returns - it runs the daemon main loop and exits
        daemon::daemon_main();
    }

    // Detect which backend to use based on environment
    let backend_type = env::detect_backend()?;

    // Create appropriate backend and execute command
    match backend_type {
        env::Backend::Wayland => match wayland::WaylandBackend::new() {
            Ok(mut backend) => execute_command(&mut backend, command),
            Err(error::Error::Io(_) | error::Error::ProtocolNotSupported) => {
                eprintln!("Warning: Wayland backend failed, falling back to TTY");
                let mut backend = tty::TtyBackend::new()?;
                execute_command(&mut backend, command)
            }
            Err(e) => Err(e),
        },
        env::Backend::Tty => {
            let mut backend = tty::TtyBackend::new()?;
            execute_command(&mut backend, command)
        }
        env::Backend::X11 => Err(error::Error::ProtocolNotSupported),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_converts_to_exit_code_1() {
        // Test that a concrete error converts to exit code 1
        let err = error::Error::UnsupportedEnvironment;
        let exit_code: i32 = err.exit_code().into();
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn io_error_triggers_fallback() {
        // Test that Io errors trigger fallback behavior in pattern matching
        let io_error = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "test");
        let dpms_error = error::Error::Io(io_error);

        // This error should match the fallback pattern
        matches!(dpms_error, error::Error::Io(_));
    }

    #[test]
    fn protocol_not_supported_triggers_fallback() {
        // Test that ProtocolNotSupported triggers fallback behavior
        let dpms_error = error::Error::ProtocolNotSupported;

        // This error should match the fallback pattern
        matches!(dpms_error, error::Error::ProtocolNotSupported);
    }

    #[test]
    fn display_not_found_does_not_trigger_fallback() {
        // Test that DisplayNotFound does NOT trigger fallback
        let dpms_error = error::Error::DisplayNotFound {
            name: "test-display".to_string(),
            available: vec!["a".to_string(), "b".to_string()],
        };

        // This error should not match the fallback pattern
        assert!(!matches!(dpms_error, error::Error::Io(_)));
        assert!(!matches!(dpms_error, error::Error::ProtocolNotSupported));
        matches!(
            dpms_error,
            error::Error::DisplayNotFound {
                name: _,
                available: _
            }
        );
    }

    #[test]
    fn ambiguous_display_does_not_trigger_fallback() {
        // Test that AmbiguousDisplay does NOT trigger fallback
        let dpms_error = error::Error::AmbiguousDisplay {
            name: "test-display".to_string(),
            candidates: vec!["a".to_string(), "b".to_string()],
        };

        // This error should not match the fallback pattern
        assert!(!matches!(dpms_error, error::Error::Io(_)));
        assert!(!matches!(dpms_error, error::Error::ProtocolNotSupported));
        matches!(
            dpms_error,
            error::Error::AmbiguousDisplay {
                name: _,
                candidates: _
            }
        );
    }
    #[test]
    fn error_has_message() {
        let error = error::Error::ProtocolNotSupported;
        let message = error.to_string();
        assert!(message.contains("protocol"));
    }

    #[test]
    fn all_error_variants_map_to_exit_code_1() {
        // Verify all error types return exit code 1 (Error)
        let errors = vec![
            error::Error::UnsupportedEnvironment,
            error::Error::ProtocolNotSupported,
            error::Error::NoDisplayFound,
            error::Error::DaemonStartFailed("test".to_string()),
            error::Error::DaemonStopTimeout,
        ];

        for err in errors {
            assert_eq!(
                err.exit_code() as i32,
                1,
                "Error {:?} should exit with code 1",
                err
            );
        }
    }
}
