use clap::{Parser, Subcommand};

/// CLI command
#[derive(Debug, Clone)]
pub enum Command {
    On,
    Off,
    Status { json: bool },
}

/// Monitor power control tool
#[derive(Parser, Debug)]
#[command(name = "powermon")]
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
}

/// Parse command-line arguments and return the Command
pub fn parse() -> Command {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::On => Command::On,
        Commands::Off => Command::Off,
        Commands::Status { json } => Command::Status { json },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_on() {
        let cli = Cli::try_parse_from(["powermon", "on"]).unwrap();
        let command = match cli.command {
            Commands::On => Command::On,
            Commands::Off => Command::Off,
            Commands::Status { json } => Command::Status { json },
        };
        assert!(matches!(command, Command::On));
    }

    #[test]
    fn test_parse_command_off() {
        let cli = Cli::try_parse_from(["powermon", "off"]).unwrap();
        let command = match cli.command {
            Commands::On => Command::On,
            Commands::Off => Command::Off,
            Commands::Status { json } => Command::Status { json },
        };
        assert!(matches!(command, Command::Off));
    }

    #[test]
    fn test_parse_command_status() {
        let cli = Cli::try_parse_from(["powermon", "status"]).unwrap();
        let command = match cli.command {
            Commands::On => Command::On,
            Commands::Off => Command::Off,
            Commands::Status { json } => Command::Status { json },
        };
        if let Command::Status { json } = command {
            assert!(!json, "Expected json to be false");
        } else {
            panic!("Expected Command::Status, got {:?}", command);
        }
    }

    #[test]
    fn test_parse_command_status_json() {
        let cli = Cli::try_parse_from(["powermon", "status", "--json"]).unwrap();
        let command = match cli.command {
            Commands::On => Command::On,
            Commands::Off => Command::Off,
            Commands::Status { json } => Command::Status { json },
        };
        if let Command::Status { json } = command {
            assert!(json, "Expected json to be true");
        } else {
            panic!("Expected Command::Status, got {:?}", command);
        }
    }

    #[test]
    fn test_parse_invalid_command() {
        let result = Cli::try_parse_from(["powermon", "foo"]);
        assert!(result.is_err(), "Expected parsing to fail for invalid command");
    }
}
