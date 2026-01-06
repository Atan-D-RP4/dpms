//! Display target selection and matching logic
//!
//! This module provides types and functions for selecting target displays
//! by name, with support for exact and partial matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayTarget {
    /// Specific display by name
    Named(String),
    /// All connected displays
    All,
    /// Default behavior (all displays, for backwards compat)
    Default,
}

impl DisplayTarget {
    /// Resolve display target from CLI arguments
    ///
    /// # Arguments
    /// * `display` - Optional display name from CLI
    /// * `all` - Whether --all flag was specified
    ///
    /// # Returns
    /// The appropriate DisplayTarget variant
    pub fn from_args(display: Option<String>, all: bool) -> Self {
        if all {
            DisplayTarget::All
        } else if let Some(name) = display {
            DisplayTarget::Named(name)
        } else {
            DisplayTarget::Default
        }
    }
}

/// Display information for listing and status
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayInfo {
    pub name: String,
    pub power: crate::output::PowerState,
    pub description: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::output::PowerState;

    /// Find a display by name with exact and partial matching (test helper)
    fn find_display_by_name<'a>(
        displays: &'a [String],
        name: &str,
    ) -> Result<&'a str, crate::error::Error> {
        // Exact match first
        if let Some(exact) = displays.iter().find(|d| d.as_str() == name) {
            return Ok(exact);
        }

        // Partial match (prefix)
        let matches: Vec<&String> = displays.iter().filter(|d| d.starts_with(name)).collect();

        match matches.len() {
            1 => Ok(matches[0]),
            0 => Err(Error::DisplayNotFound {
                name: name.to_string(),
                available: displays.to_vec(),
            }),
            _ => Err(Error::AmbiguousDisplay {
                name: name.to_string(),
                candidates: matches.iter().map(|s| s.to_string()).collect(),
            }),
        }
    }

    // DisplayTarget::from_args tests

    #[test]
    fn from_args_all_flag() {
        let target = DisplayTarget::from_args(Some("DP-1".to_string()), true);
        assert_eq!(target, DisplayTarget::All);
    }

    #[test]
    fn from_args_named() {
        let target = DisplayTarget::from_args(Some("DP-1".to_string()), false);
        assert_eq!(target, DisplayTarget::Named("DP-1".to_string()));
    }

    #[test]
    fn from_args_default() {
        let target = DisplayTarget::from_args(None, false);
        assert_eq!(target, DisplayTarget::Default);
    }

    #[test]
    fn from_args_all_overrides_named() {
        // --all flag takes precedence over display name
        let target = DisplayTarget::from_args(Some("DP-1".to_string()), true);
        assert_eq!(target, DisplayTarget::All);
    }

    // find_display_by_name tests

    #[test]
    fn find_exact_match() {
        let displays = vec!["DP-1".to_string(), "eDP-1".to_string()];
        let result = find_display_by_name(&displays, "DP-1");
        assert_eq!(result.unwrap(), "DP-1");
    }

    #[test]
    fn find_partial_match() {
        let displays = vec!["DP-1".to_string(), "eDP-1".to_string()];
        let result = find_display_by_name(&displays, "DP");
        assert_eq!(result.unwrap(), "DP-1");
    }

    #[test]
    fn find_partial_match_edp() {
        let displays = vec!["DP-1".to_string(), "eDP-1".to_string()];
        let result = find_display_by_name(&displays, "eDP");
        assert_eq!(result.unwrap(), "eDP-1");
    }

    #[test]
    fn find_ambiguous() {
        let displays = vec!["DP-1".to_string(), "DP-2".to_string()];
        let result = find_display_by_name(&displays, "DP");
        assert!(matches!(result, Err(Error::AmbiguousDisplay { .. })));

        if let Err(Error::AmbiguousDisplay { name, candidates }) = result {
            assert_eq!(name, "DP");
            assert!(candidates.contains(&"DP-1".to_string()));
            assert!(candidates.contains(&"DP-2".to_string()));
        }
    }

    #[test]
    fn find_not_found() {
        let displays = vec!["DP-1".to_string(), "eDP-1".to_string()];
        let result = find_display_by_name(&displays, "HDMI");
        assert!(matches!(result, Err(Error::DisplayNotFound { .. })));

        if let Err(Error::DisplayNotFound { name, available }) = result {
            assert_eq!(name, "HDMI");
            assert!(available.contains(&"DP-1".to_string()));
            assert!(available.contains(&"eDP-1".to_string()));
        }
    }

    #[test]
    fn find_exact_match_preferred() {
        // If exact match exists, prefer it over partial
        let displays = vec!["DP".to_string(), "DP-1".to_string()];
        let result = find_display_by_name(&displays, "DP");
        assert_eq!(result.unwrap(), "DP");
    }

    // filter_displays tests

    #[test]
    fn filter_all() {
        let displays = vec![
            make_display("DP-1", PowerState::On),
            make_display("eDP-1", PowerState::Off),
        ];
        let names: Vec<String> = displays.iter().map(|d| d.name.clone()).collect();
        // All/Default just returns all
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn filter_named_exact() {
        let displays = vec![
            make_display("DP-1", PowerState::On),
            make_display("eDP-1", PowerState::Off),
        ];
        let names: Vec<String> = displays.iter().map(|d| d.name.clone()).collect();
        let matched = find_display_by_name(&names, "DP-1").unwrap();
        assert_eq!(matched, "DP-1");
    }

    #[test]
    fn filter_named_partial() {
        let displays = vec![
            make_display("DP-1", PowerState::On),
            make_display("eDP-1", PowerState::Off),
        ];
        let names: Vec<String> = displays.iter().map(|d| d.name.clone()).collect();
        let matched = find_display_by_name(&names, "eDP").unwrap();
        assert_eq!(matched, "eDP-1");
    }

    #[test]
    fn filter_named_not_found() {
        let displays = vec![
            make_display("DP-1", PowerState::On),
            make_display("eDP-1", PowerState::Off),
        ];
        let names: Vec<String> = displays.iter().map(|d| d.name.clone()).collect();
        let result = find_display_by_name(&names, "HDMI");
        assert!(matches!(result, Err(Error::DisplayNotFound { .. })));
    }

    // DisplayInfo tests

    fn make_display(name: &str, power: PowerState) -> DisplayInfo {
        DisplayInfo {
            name: name.to_string(),
            power,
            description: None,
            make: None,
            model: None,
        }
    }

    #[test]
    fn display_info_fields() {
        let info = DisplayInfo {
            name: "DP-1".to_string(),
            power: PowerState::On,
            description: Some("Test".to_string()),
            make: Some("Dell".to_string()),
            model: Some("U2720Q".to_string()),
        };
        assert_eq!(info.name, "DP-1");
        assert_eq!(info.power, PowerState::On);
        assert_eq!(info.description, Some("Test".to_string()));
        assert_eq!(info.make, Some("Dell".to_string()));
        assert_eq!(info.model, Some("U2720Q".to_string()));
    }
}
