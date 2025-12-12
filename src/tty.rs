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
use crate::error::Error;
use crate::output::PowerState;

/// TTY backend implementing PowerBackend trait
///
/// This backend uses a daemon process to manage display power state in TTY
/// environments. It delegates actual power control to daemon functions (F8).
pub struct TtyBackend;

impl TtyBackend {
    /// Create a new TTY backend
    ///
    /// # Returns
    /// - `Ok(TtyBackend)` - Backend ready to use
    /// - `Err(Error)` - If TTY environment validation fails
    ///
    /// # Example
    /// ```no_run
    /// # use dpms::tty::TtyBackend;
    /// let backend = TtyBackend::new()?;
    /// # Ok::<(), dpms::error::Error>(())
    /// ```
    pub fn new() -> Result<Self, Error> {
        // Validate we can access DRM/seat by attempting to open
        // This ensures we fail fast if permissions are wrong
        // But we don't keep the connection open (daemon will open its own)
        crate::drm_ops::open_drm()?;

        Ok(TtyBackend)
    }
}

impl PowerBackend for TtyBackend {
    fn set_power(&mut self, state: PowerState) -> Result<(), Error> {
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

    fn get_power(&self) -> Result<PowerState, Error> {
        // Query daemon running state
        // If daemon is running, display is off
        // If daemon is not running, display is on
        match daemon::is_daemon_running() {
            Some(_pid) => Ok(PowerState::Off),
            None => Ok(PowerState::On),
        }
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
    fn get_power_when_daemon_not_running() {
        let backend = TtyBackend;
        let result = backend.get_power();

        // When daemon is not running (stub returns None), display should be On
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PowerState::On);
    }

    // Note: More comprehensive tests require F8 implementation or mocking
    // Integration tests will verify the full daemon coordination logic
}
