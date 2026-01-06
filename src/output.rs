use crate::display::DisplayInfo;

/// Power state enum representing display power state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    On,
    Off,
}

/// Format status output for one or more displays
pub fn format_status(displays: &[DisplayInfo], json: bool) -> String {
    if displays.is_empty() {
        return if json {
            "[]".to_string()
        } else {
            String::new()
        };
    }

    if json {
        format_displays_json(displays)
    } else {
        format_displays_text(displays, false)
    }
}

/// Format list output for all displays
pub fn format_list(displays: &[DisplayInfo], json: bool, verbose: bool) -> String {
    if displays.is_empty() {
        return if json {
            "[]".to_string()
        } else {
            String::new()
        };
    }

    if json {
        format_displays_json(displays)
    } else {
        format_displays_text(displays, verbose)
    }
}

/// Format multiple displays as text
fn format_displays_text(displays: &[DisplayInfo], verbose: bool) -> String {
    displays
        .iter()
        .map(|d| format_display_line(d, verbose))
        .collect::<Vec<_>>()
        .join("")
}

/// Format a single display line for text output
fn format_display_line(display: &DisplayInfo, verbose: bool) -> String {
    let power_str = match display.power {
        PowerState::On => "On",
        PowerState::Off => "Off",
    };

    if verbose {
        // Include make/model if available
        let make = display.make.as_deref().unwrap_or("");
        let model = display.model.as_deref().unwrap_or("");
        if !make.is_empty() || !model.is_empty() {
            format!("{}: {} ({} {})\n", display.name, power_str, make, model)
                .replace("( ", "(")
                .replace(" )", ")")
                .replace("()", "")
        } else {
            format!("{}: {}\n", display.name, power_str)
        }
    } else {
        format!("{}: {}\n", display.name, power_str)
    }
}

/// Format multiple displays as JSON array
fn format_displays_json(displays: &[DisplayInfo]) -> String {
    let parts: Vec<String> = displays
        .iter()
        .map(|d| {
            let power_str = match d.power {
                PowerState::On => "on",
                PowerState::Off => "off",
            };
            format!(r#"{{"name":"{}","power":"{}"}}"#, d.name, power_str)
        })
        .collect();

    format!("[{}]", parts.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create DisplayInfo for tests
    fn make_display(name: &str, power: PowerState) -> DisplayInfo {
        DisplayInfo {
            name: name.to_string(),
            power,
            description: None,
            make: None,
            model: None,
        }
    }

    fn make_display_verbose(name: &str, power: PowerState, make: &str, model: &str) -> DisplayInfo {
        DisplayInfo {
            name: name.to_string(),
            power,
            description: None,
            make: Some(make.to_string()),
            model: Some(model.to_string()),
        }
    }

    // ===== Status tests =====

    #[test]
    fn format_status_single_text() {
        let displays = vec![make_display("DP-1", PowerState::On)];
        assert_eq!(format_status(&displays, false), "DP-1: On\n");
    }

    #[test]
    fn format_status_single_json() {
        let displays = vec![make_display("DP-1", PowerState::On)];
        assert_eq!(
            format_status(&displays, true),
            r#"[{"name":"DP-1","power":"on"}]"#
        );
    }

    #[test]
    fn format_status_multi_text() {
        let displays = vec![
            make_display("DP-1", PowerState::On),
            make_display("eDP-1", PowerState::Off),
        ];
        assert_eq!(format_status(&displays, false), "DP-1: On\neDP-1: Off\n");
    }

    #[test]
    fn format_status_multi_json() {
        let displays = vec![
            make_display("DP-1", PowerState::On),
            make_display("eDP-1", PowerState::Off),
        ];
        assert_eq!(
            format_status(&displays, true),
            r#"[{"name":"DP-1","power":"on"},{"name":"eDP-1","power":"off"}]"#
        );
    }

    // ===== List command tests =====

    #[test]
    fn format_list_text() {
        let displays = vec![
            make_display("DP-1", PowerState::On),
            make_display("eDP-1", PowerState::Off),
        ];
        assert_eq!(
            format_list(&displays, false, false),
            "DP-1: On\neDP-1: Off\n"
        );
    }

    #[test]
    fn format_list_json() {
        let displays = vec![make_display("DP-1", PowerState::On)];
        assert_eq!(
            format_list(&displays, true, false),
            r#"[{"name":"DP-1","power":"on"}]"#
        );
    }

    #[test]
    fn format_list_verbose() {
        let displays = vec![make_display_verbose(
            "DP-1",
            PowerState::On,
            "Dell",
            "U2720Q",
        )];
        assert_eq!(
            format_list(&displays, false, true),
            "DP-1: On (Dell U2720Q)\n"
        );
    }

    #[test]
    fn format_list_verbose_partial_info() {
        let displays = vec![DisplayInfo {
            name: "DP-1".to_string(),
            power: PowerState::On,
            description: None,
            make: Some("Dell".to_string()),
            model: None,
        }];
        let output = format_list(&displays, false, true);
        assert!(output.contains("DP-1: On"));
        assert!(output.contains("Dell"));
    }

    #[test]
    fn format_list_empty() {
        let displays: Vec<DisplayInfo> = vec![];
        assert_eq!(format_list(&displays, false, false), "");
        assert_eq!(format_list(&displays, true, false), "[]");
    }

    // ===== Empty input tests =====

    #[test]
    fn format_status_empty() {
        let displays: Vec<DisplayInfo> = vec![];
        assert_eq!(format_status(&displays, false), "");
        assert_eq!(format_status(&displays, true), "[]");
    }

    // ===== PowerState tests =====

    #[test]
    fn power_state_equality() {
        assert_eq!(PowerState::On, PowerState::On);
        assert_eq!(PowerState::Off, PowerState::Off);
        assert_ne!(PowerState::On, PowerState::Off);
    }

    #[test]
    fn power_state_copy_clone() {
        let state = PowerState::On;
        let copied = state;
        let cloned = state;

        assert_eq!(state, copied);
        assert_eq!(state, cloned);
    }

    #[test]
    fn json_output_is_valid_json() {
        let displays = vec![make_display("DP-1", PowerState::On)];
        let json = format_status(&displays, true);
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("\"power\""));
        assert!(json.contains("\"on\""));
    }

    #[test]
    fn json_array_output_is_valid() {
        let displays = vec![
            make_display("DP-1", PowerState::On),
            make_display("eDP-1", PowerState::Off),
        ];
        let json = format_status(&displays, true);
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"DP-1\""));
        assert!(json.contains("\"eDP-1\""));
    }
}
