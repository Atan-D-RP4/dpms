use clap::{Parser, Subcommand};

/// CLI command
#[derive(Debug, Clone)]
pub enum Command {
    On,
    Off,
    Status { json: bool },
    /// Internal: run as daemon process (not for user use)
    DaemonInternal,
}

/// Monitor power control tool
#[derive(Parser, Debug)]
#[command(name = "dpms")]
#[command(about = "Control monitor power state", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Turn display on
    On,
    /// Turn display off
    Off,
    /// Show display status
    Status {
        /// Output status as JSON
        #[arg(long)]
        json: bool,
    },
    /// Internal daemon mode (not for user use)
    #[command(hide = true)]
    DaemonInternal,
}

/// Parse command-line arguments and return the Command
pub fn parse() -> Command {
    let cli = Cli::parse();
    command_from_commands(cli.command)
}

/// Convert internal Commands enum to public Command enum
fn command_from_commands(cmd: Commands) -> Command {
    match cmd {
        Commands::On => Command::On,
        Commands::Off => Command::Off,
        Commands::Status { json } => Command::Status { json },
        Commands::DaemonInternal => Command::DaemonInternal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_on() {
        let cli = Cli::try_parse_from(["dpms", "on"]).unwrap();
        let command = command_from_commands(cli.command);
        assert!(matches!(command, Command::On));
    }

    #[test]
    fn parse_command_off() {
        let cli = Cli::try_parse_from(["dpms", "off"]).unwrap();
        let command = command_from_commands(cli.command);
        assert!(matches!(command, Command::Off));
    }

    #[test]
    fn parse_command_status() {
        let cli = Cli::try_parse_from(["dpms", "status"]).unwrap();
        let command = command_from_commands(cli.command);
        if let Command::Status { json } = command {
            assert!(!json, "Expected json to be false");
        } else {
            panic!("Expected Command::Status, got {:?}", command);
        }
    }

    #[test]
    fn parse_command_status_json() {
        let cli = Cli::try_parse_from(["dpms", "status", "--json"]).unwrap();
        let command = command_from_commands(cli.command);
        if let Command::Status { json } = command {
            assert!(json, "Expected json to be true");
        } else {
            panic!("Expected Command::Status, got {:?}", command);
        }
    }

    #[test]
    fn parse_invalid_command() {
        let result = Cli::try_parse_from(["dpms", "foo"]);
        assert!(
            result.is_err(),
            "Expected parsing to fail for invalid command"
        );
    }

    #[test]
    fn usage_error_exit_code() {
        // Verify clap errors for invalid commands return exit code 2
        // This tests AC2: usage errors exit with code 2
        let result = Cli::try_parse_from(["dpms", "invalid"]);
        assert!(result.is_err());

        let err = result.unwrap_err();
        // Clap errors have an exit method that returns the exit code
        // For usage/validation errors, clap returns exit code 2
        assert_eq!(err.exit_code(), 2, "Usage errors should exit with code 2");
    }
}
