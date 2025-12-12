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
    /// # use powermon::tty::TtyBackend;
    /// let backend = TtyBackend::new()?;
    /// # Ok::<(), powermon::error::Error>(())
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
                if is_daemon_running().is_some() {
                    // Already off, idempotent operation
                    eprintln!("Display already off");
                    return Ok(());
                }

                // Start daemon - it will turn off the display
                start_daemon()?;
                Ok(())
            }
            PowerState::On => {
                // Check if daemon is running
                if is_daemon_running().is_none() {
                    // Already on, idempotent operation
                    eprintln!("Display already on");
                    return Ok(());
                }

                // Signal daemon to restore display and exit
                signal_daemon(true)?;
                Ok(())
            }
        }
    }

    fn get_power(&self) -> Result<PowerState, Error> {
        // Query daemon running state
        // If daemon is running, display is off
        // If daemon is not running, display is on
        match is_daemon_running() {
            Some(_pid) => Ok(PowerState::Off),
            None => Ok(PowerState::On),
        }
    }
}

// ============================================================================
// Daemon coordination functions
// ============================================================================
// These functions delegate to the daemon module (F8).

/// Check if the powermon daemon is currently running
///
/// Returns the PID of the running daemon, or None if no daemon is running.
/// Also cleans up stale PID files if the process is no longer alive.
fn is_daemon_running() -> Option<nix::unistd::Pid> {
    crate::daemon::is_daemon_running()
}

/// Start the powermon daemon
///
/// Forks a new daemon process that:
/// 1. Opens a libseat session
/// 2. Opens DRM device
/// 3. Disables CRTC (turns off display)
/// 4. Writes PID file
/// 5. Waits for SIGTERM/SIGINT to restore and exit
///
/// The parent process returns immediately after verifying the daemon started.
///
/// # Returns
/// - `Ok(())` - Daemon started successfully
/// - `Err(Error::DaemonStartFailed)` - Daemon failed to start
fn start_daemon() -> Result<(), Error> {
    crate::daemon::start_daemon()
}

/// Signal the daemon to restore display and exit
///
/// Sends SIGTERM to the daemon process, which triggers it to:
/// 1. Restore CRTC ACTIVE property to 1 (turn display back on)
/// 2. Remove PID file
/// 3. Exit cleanly
///
/// # Parameters
/// - `_on`: true to turn display on (send SIGTERM), false is unused
///
/// # Returns
/// - `Ok(())` - Signal sent and daemon stopped successfully
/// - `Err(Error::DaemonStopTimeout)` - Daemon didn't stop within timeout
fn signal_daemon(_on: bool) -> Result<(), Error> {
    crate::daemon::stop_daemon()
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
