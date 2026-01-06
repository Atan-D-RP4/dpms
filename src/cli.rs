use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell as ClapShell, generate};
use std::io;

use crate::display::DisplayTarget;

/// Shell type for completions
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Elvish,
    #[value(name = "powershell")]
    Powershell,
}

impl From<Shell> for ClapShell {
    fn from(shell: Shell) -> Self {
        match shell {
            Shell::Bash => ClapShell::Bash,
            Shell::Zsh => ClapShell::Zsh,
            Shell::Fish => ClapShell::Fish,
            Shell::Elvish => ClapShell::Elvish,
            Shell::Powershell => ClapShell::PowerShell,
        }
    }
}

/// CLI command
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    On {
        target: DisplayTarget,
    },
    Off {
        target: DisplayTarget,
    },
    Toggle {
        target: DisplayTarget,
    },
    Status {
        target: DisplayTarget,
        json: bool,
    },
    List {
        json: bool,
        verbose: bool,
    },
    Completion {
        shell: Shell,
    },
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
    On {
        /// Target display name (e.g., DP-1, eDP-1)
        display: Option<String>,

        /// Target all displays
        #[arg(long)]
        all: bool,
    },
    /// Turn display off
    Off {
        /// Target display name (e.g., DP-1, eDP-1)
        display: Option<String>,

        /// Target all displays
        #[arg(long)]
        all: bool,
    },
    /// Toggle display power state
    Toggle {
        /// Target display name (e.g., DP-1, eDP-1)
        display: Option<String>,

        /// Target all displays
        #[arg(long)]
        all: bool,
    },
    /// Show display power status
    Status {
        /// Target display name (e.g., DP-1, eDP-1)
        display: Option<String>,

        /// Output status as JSON
        #[arg(long)]
        json: bool,
    },
    /// List all connected displays
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Show detailed information (make, model)
        #[arg(short, long)]
        verbose: bool,
    },
    /// Generate shell completion script
    Completion {
        /// Shell type
        #[arg(value_enum)]
        shell: Shell,
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

/// Generate shell completion script to stdout
pub fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let clap_shell: ClapShell = shell.into();
    generate(clap_shell, &mut cmd, "dpms", &mut io::stdout());
}

/// Convert internal Commands enum to public Command enum
fn command_from_commands(cmd: Commands) -> Command {
    match cmd {
        Commands::On { display, all } => Command::On {
            target: DisplayTarget::from_args(display, all),
        },
        Commands::Off { display, all } => Command::Off {
            target: DisplayTarget::from_args(display, all),
        },
        Commands::Toggle { display, all } => Command::Toggle {
            target: DisplayTarget::from_args(display, all),
        },
        Commands::Status { display, json } => Command::Status {
            target: if let Some(name) = display {
                DisplayTarget::Named(name)
            } else {
                DisplayTarget::Default
            },
            json,
        },
        Commands::List { json, verbose } => Command::List { json, verbose },
        Commands::Completion { shell } => Command::Completion { shell },
        Commands::DaemonInternal => Command::DaemonInternal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Basic command parsing tests (backwards compatibility)

    #[test]
    fn parse_command_on() {
        let cli = Cli::try_parse_from(["dpms", "on"]).unwrap();
        let command = command_from_commands(cli.command);
        assert!(matches!(
            command,
            Command::On {
                target: DisplayTarget::Default
            }
        ));
    }

    #[test]
    fn parse_command_off() {
        let cli = Cli::try_parse_from(["dpms", "off"]).unwrap();
        let command = command_from_commands(cli.command);
        assert!(matches!(
            command,
            Command::Off {
                target: DisplayTarget::Default
            }
        ));
    }

    #[test]
    fn parse_command_status() {
        let cli = Cli::try_parse_from(["dpms", "status"]).unwrap();
        let command = command_from_commands(cli.command);
        if let Command::Status { target, json } = command {
            assert_eq!(target, DisplayTarget::Default);
            assert!(!json);
        } else {
            panic!("Expected Command::Status, got {:?}", command);
        }
    }

    #[test]
    fn parse_command_status_json() {
        let cli = Cli::try_parse_from(["dpms", "status", "--json"]).unwrap();
        let command = command_from_commands(cli.command);
        if let Command::Status { target, json } = command {
            assert_eq!(target, DisplayTarget::Default);
            assert!(json);
        } else {
            panic!("Expected Command::Status, got {:?}", command);
        }
    }

    // New v2 command parsing tests

    #[test]
    fn parse_on_with_display() {
        let cli = Cli::try_parse_from(["dpms", "on", "DP-1"]).unwrap();
        let command = command_from_commands(cli.command);
        assert_eq!(
            command,
            Command::On {
                target: DisplayTarget::Named("DP-1".to_string())
            }
        );
    }

    #[test]
    fn parse_off_with_display() {
        let cli = Cli::try_parse_from(["dpms", "off", "eDP-1"]).unwrap();
        let command = command_from_commands(cli.command);
        assert_eq!(
            command,
            Command::Off {
                target: DisplayTarget::Named("eDP-1".to_string())
            }
        );
    }

    #[test]
    fn parse_off_all() {
        let cli = Cli::try_parse_from(["dpms", "off", "--all"]).unwrap();
        let command = command_from_commands(cli.command);
        assert_eq!(
            command,
            Command::Off {
                target: DisplayTarget::All
            }
        );
    }

    #[test]
    fn parse_toggle() {
        let cli = Cli::try_parse_from(["dpms", "toggle"]).unwrap();
        let command = command_from_commands(cli.command);
        assert_eq!(
            command,
            Command::Toggle {
                target: DisplayTarget::Default
            }
        );
    }

    #[test]
    fn parse_toggle_with_display() {
        let cli = Cli::try_parse_from(["dpms", "toggle", "DP-1"]).unwrap();
        let command = command_from_commands(cli.command);
        assert_eq!(
            command,
            Command::Toggle {
                target: DisplayTarget::Named("DP-1".to_string())
            }
        );
    }

    #[test]
    fn parse_list() {
        let cli = Cli::try_parse_from(["dpms", "list"]).unwrap();
        let command = command_from_commands(cli.command);
        assert_eq!(
            command,
            Command::List {
                json: false,
                verbose: false
            }
        );
    }

    #[test]
    fn parse_list_json() {
        let cli = Cli::try_parse_from(["dpms", "list", "--json"]).unwrap();
        let command = command_from_commands(cli.command);
        assert_eq!(
            command,
            Command::List {
                json: true,
                verbose: false
            }
        );
    }

    #[test]
    fn parse_list_verbose() {
        let cli = Cli::try_parse_from(["dpms", "list", "-v"]).unwrap();
        let command = command_from_commands(cli.command);
        assert_eq!(
            command,
            Command::List {
                json: false,
                verbose: true
            }
        );
    }

    #[test]
    fn parse_list_verbose_long() {
        let cli = Cli::try_parse_from(["dpms", "list", "--verbose"]).unwrap();
        let command = command_from_commands(cli.command);
        assert_eq!(
            command,
            Command::List {
                json: false,
                verbose: true
            }
        );
    }

    #[test]
    fn parse_status_with_display() {
        let cli = Cli::try_parse_from(["dpms", "status", "DP-1"]).unwrap();
        let command = command_from_commands(cli.command);
        assert_eq!(
            command,
            Command::Status {
                target: DisplayTarget::Named("DP-1".to_string()),
                json: false
            }
        );
    }

    #[test]
    fn parse_status_with_display_json() {
        let cli = Cli::try_parse_from(["dpms", "status", "DP-1", "--json"]).unwrap();
        let command = command_from_commands(cli.command);
        assert_eq!(
            command,
            Command::Status {
                target: DisplayTarget::Named("DP-1".to_string()),
                json: true
            }
        );
    }

    #[test]
    fn parse_completion_bash() {
        let cli = Cli::try_parse_from(["dpms", "completion", "bash"]).unwrap();
        let command = command_from_commands(cli.command);
        assert!(matches!(
            command,
            Command::Completion { shell: Shell::Bash }
        ));
    }

    #[test]
    fn parse_completion_zsh() {
        let cli = Cli::try_parse_from(["dpms", "completion", "zsh"]).unwrap();
        let command = command_from_commands(cli.command);
        assert!(matches!(command, Command::Completion { shell: Shell::Zsh }));
    }

    #[test]
    fn parse_completion_fish() {
        let cli = Cli::try_parse_from(["dpms", "completion", "fish"]).unwrap();
        let command = command_from_commands(cli.command);
        assert!(matches!(
            command,
            Command::Completion { shell: Shell::Fish }
        ));
    }

    #[test]
    fn parse_completion_powershell() {
        let cli = Cli::try_parse_from(["dpms", "completion", "powershell"]).unwrap();
        let command = command_from_commands(cli.command);
        assert!(matches!(
            command,
            Command::Completion {
                shell: Shell::Powershell
            }
        ));
    }

    #[test]
    fn parse_completion_invalid() {
        let result = Cli::try_parse_from(["dpms", "completion", "invalid"]);
        assert!(result.is_err());
    }

    // Error handling tests

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
        let result = Cli::try_parse_from(["dpms", "invalid"]);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 2, "Usage errors should exit with code 2");
    }
}
