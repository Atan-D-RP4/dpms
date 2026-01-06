/// TTY backend for monitor power control
///
/// This module implements the PowerBackend trait for TTY environments using
/// a daemon process and DRM atomic commits. The daemon manages display power
/// state via libseat and DRM operations.
///
/// The backend coordinates with the daemon lifecycle:
/// - When turning display off: spawns daemon if not running
/// - When turning display on: signals daemon to restore and exit
/// - When querying status: checks if daemon is running
use crate::backend::PowerBackend;
use crate::daemon;
use crate::display::{DisplayInfo, DisplayTarget};
use crate::error::Error;
use crate::output::PowerState;

/// TTY backend implementing PowerBackend trait
///
/// This backend uses a daemon process to manage display power state in TTY
/// environments. It delegates actual power control to daemon functions (F8).
///
/// Note: TTY backend currently operates on all displays as a single unit.
/// Multi-display selection (F20) is a future enhancement.
pub struct TtyBackend;

impl TtyBackend {
    /// Create a new TTY backend
    ///
    /// # Returns
    /// - `Ok(TtyBackend)` - Backend ready to use
    ///
    /// # Note
    /// This does not validate DRM access upfront. DRM access is only needed
    /// when starting the daemon (dpms off). Signaling an existing daemon
    /// (dpms on) only requires sending SIGTERM, not DRM access.
    ///
    /// # Example
    /// ```no_run
    /// # use dpms::tty::TtyBackend;
    /// let backend = TtyBackend::new()?;
    /// # Ok::<(), dpms::error::Error>(())
    /// ```
    pub fn new() -> Result<Self, Error> {
        Ok(TtyBackend)
    }

    /// Get the current power state (internal helper)
    fn current_power_state(&self) -> PowerState {
        match daemon::is_daemon_running() {
            Some(_pid) => PowerState::Off,
            None => PowerState::On,
        }
    }
}

impl PowerBackend for TtyBackend {
    fn set_power(&mut self, target: &DisplayTarget, state: PowerState) -> Result<(), Error> {
        // TTY backend currently doesn't support per-display control
        // Warn if a specific display is targeted
        if let DisplayTarget::Named(name) = target {
            eprintln!(
                "Warning: TTY backend does not support per-display control. \
                 Ignoring display name '{}', operating on all displays.",
                name
            );
        }

        match state {
            PowerState::Off => {
                // Check if daemon is already running
                if daemon::is_daemon_running().is_some() {
                    // Already off, idempotent operation
                    eprintln!("Display already off");
                    return Ok(());
                }

                // Start daemon - it will turn off the display
                daemon::start_daemon()
            }
            PowerState::On => {
                // Check if daemon is running
                if daemon::is_daemon_running().is_none() {
                    // Already on, idempotent operation
                    eprintln!("Display already on");
                    return Ok(());
                }

                // Signal daemon to restore display and exit
                daemon::stop_daemon()
            }
        }
    }

    fn get_power(&self, target: &DisplayTarget) -> Result<Vec<DisplayInfo>, Error> {
        // TTY backend currently doesn't support per-display queries
        if let DisplayTarget::Named(name) = target {
            eprintln!(
                "Warning: TTY backend does not support per-display queries. \
                 Ignoring display name '{}', showing all displays.",
                name
            );
        }

        // Return a single "display" representing the TTY state
        let power = self.current_power_state();
        
        Ok(vec![DisplayInfo {
            name: "tty".to_string(),
            power,
            description: Some("TTY/Console display".to_string()),
            make: None,
            model: None,
        }])
    }

    fn list_displays(&self) -> Result<Vec<DisplayInfo>, Error> {
        self.get_power(&DisplayTarget::All)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tty_backend_implements_power_backend() {
        // Compile-time check that TtyBackend implements PowerBackend
        fn assert_power_backend<T: PowerBackend>() {}
        assert_power_backend::<TtyBackend>();
    }

    #[test]
    fn get_power_returns_display_info() {
        let backend = TtyBackend;
        let result = backend.get_power(&DisplayTarget::Default);

        assert!(result.is_ok());
        let displays = result.unwrap();
        assert_eq!(displays.len(), 1);
        assert_eq!(displays[0].name, "tty");
    }

    #[test]
    fn list_displays_returns_tty_display() {
        let backend = TtyBackend;
        let result = backend.list_displays();

        assert!(result.is_ok());
        let displays = result.unwrap();
        assert_eq!(displays.len(), 1);
        assert_eq!(displays[0].name, "tty");
    }

    // Note: More comprehensive tests require F8 implementation or mocking
    // Integration tests will verify the full daemon coordination logic
}
