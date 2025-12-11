/// Power state enum representing display power state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    On,
    Off,
}

/// Status output for formatting
#[derive(Debug)]
pub struct StatusOutput {
    pub state: PowerState,
}

impl StatusOutput {
    /// Create a new StatusOutput
    pub fn new(state: PowerState) -> Self {
        Self { state }
    }

    /// Format the status output based on json flag
    /// 
    /// Returns:
    /// - If json=false: "Display: On\n" or "Display: Off\n"
    /// - If json=true: `{"power":"on"}` or `{"power":"off"}`
    pub fn format(&self, json: bool) -> String {
        if json {
            // Manual JSON formatting (no serde dependency)
            match self.state {
                PowerState::On => r#"{"power":"on"}"#.to_string(),
                PowerState::Off => r#"{"power":"off"}"#.to_string(),
            }
        } else {
            // Human-readable text format
            match self.state {
                PowerState::On => "Display: On\n".to_string(),
                PowerState::Off => "Display: Off\n".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_status_text_on() {
        let output = StatusOutput::new(PowerState::On);
        assert_eq!(output.format(false), "Display: On\n");
    }

    #[test]
    fn test_format_status_text_off() {
        let output = StatusOutput::new(PowerState::Off);
        assert_eq!(output.format(false), "Display: Off\n");
    }

    #[test]
    fn test_format_status_json_on() {
        let output = StatusOutput::new(PowerState::On);
        assert_eq!(output.format(true), r#"{"power":"on"}"#);
    }

    #[test]
    fn test_format_status_json_off() {
        let output = StatusOutput::new(PowerState::Off);
        assert_eq!(output.format(true), r#"{"power":"off"}"#);
    }

    #[test]
    fn test_power_state_equality() {
        assert_eq!(PowerState::On, PowerState::On);
        assert_eq!(PowerState::Off, PowerState::Off);
        assert_ne!(PowerState::On, PowerState::Off);
    }

    #[test]
    fn test_power_state_copy_clone() {
        let state = PowerState::On;
        let copied = state;
        let cloned = state.clone();
        
        assert_eq!(state, copied);
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_status_output_new() {
        let output = StatusOutput::new(PowerState::On);
        assert_eq!(output.state, PowerState::On);
        
        let output = StatusOutput::new(PowerState::Off);
        assert_eq!(output.state, PowerState::Off);
    }

    #[test]
    fn test_json_output_is_valid_json() {
        // Verify JSON output can be parsed (manual validation)
        let output_on = StatusOutput::new(PowerState::On);
        let json_on = output_on.format(true);
        assert!(json_on.starts_with('{'));
        assert!(json_on.ends_with('}'));
        assert!(json_on.contains("\"power\""));
        assert!(json_on.contains("\"on\""));
        
        let output_off = StatusOutput::new(PowerState::Off);
        let json_off = output_off.format(true);
        assert!(json_off.starts_with('{'));
        assert!(json_off.ends_with('}'));
        assert!(json_off.contains("\"power\""));
        assert!(json_off.contains("\"off\""));
    }

    #[test]
    fn test_text_output_format() {
        // Verify text output has correct format with newline
        let output_on = StatusOutput::new(PowerState::On);
        let text_on = output_on.format(false);
        assert!(text_on.starts_with("Display: "));
        assert!(text_on.ends_with('\n'));
        
        let output_off = StatusOutput::new(PowerState::Off);
        let text_off = output_off.format(false);
        assert!(text_off.starts_with("Display: "));
        assert!(text_off.ends_with('\n'));
    }

    #[test]
    fn test_format_is_deterministic() {
        // Verify multiple calls return same result
        let output = StatusOutput::new(PowerState::On);
        let result1 = output.format(true);
        let result2 = output.format(true);
        assert_eq!(result1, result2);
        
        let result3 = output.format(false);
        let result4 = output.format(false);
        assert_eq!(result3, result4);
    }
}
