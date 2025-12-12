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

/// Execute a command using the given backend
fn execute_command<B: backend::PowerBackend>(
    backend: &mut B,
    command: cli::Command,
) -> Result<(), error::Error> {
    match command {
        cli::Command::On => {
            backend.set_power(output::PowerState::On)?;
            Ok(())
        }
        cli::Command::Off => {
            backend.set_power(output::PowerState::Off)?;
            Ok(())
        }
        cli::Command::Status { json } => {
            let state = backend.get_power()?;
            let status_output = output::StatusOutput::new(state);
            print!("{}", status_output.format(json));
            Ok(())
        }
    }
}

/// Main application logic - dispatches commands to appropriate backend
fn run(command: cli::Command) -> Result<(), error::Error> {
    // Detect which backend to use based on environment
    let backend_type = env::detect_backend()?;

    // Create appropriate backend and execute command
    match backend_type {
        env::Backend::Wayland => {
            let mut backend = wayland::WaylandBackend::new()?;
            execute_command(&mut backend, command)
        }
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
        let error = error::Error::UnsupportedEnvironment;
        let exit_code = error.exit_code();
        assert_eq!(exit_code, error::ExitCode::Error);
        assert_eq!(exit_code as i32, 1);
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
            error::Error::DaemonStartFailed,
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
