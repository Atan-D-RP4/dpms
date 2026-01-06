/// PowerBackend trait for monitor power control
///
/// This trait provides a common interface for controlling monitor power state
/// across different environments (Wayland compositor and TTY).
///
/// Implementations:
/// - Wayland backend: Uses `zwlr_output_power_management_v1` protocol
/// - X11 backend: Would use XRandR (not yet implemented)
/// - TTY backend: Uses libseat + DRM atomic commits with daemon mode
use crate::display::{DisplayInfo, DisplayTarget};
use crate::error::Error;
use crate::output::PowerState;

/// PowerBackend interface for monitor power control
///
/// Provides methods to set and query the power state of connected displays.
/// All backends must implement this trait to provide a consistent interface
/// regardless of the underlying environment.
pub trait PowerBackend {
    /// Set the power state of the specified display(s)
    ///
    /// # Parameters
    /// - `target`: Which display(s) to target (Named, All, or Default)
    /// - `state`: Target power state (On or Off)
    ///
    /// # Returns
    /// - `Ok(())` if the power state was successfully changed
    /// - `Err(Error)` if the operation failed
    fn set_power(&mut self, target: &DisplayTarget, state: PowerState) -> Result<(), Error>;

    /// Get the current power state of the specified display(s)
    ///
    /// # Parameters
    /// - `target`: Which display(s) to query (Named, All, or Default)
    ///
    /// # Returns
    /// - `Ok(Vec<DisplayInfo>)` with power state for each targeted display
    /// - `Err(Error)` if the status could not be determined
    fn get_power(&self, target: &DisplayTarget) -> Result<Vec<DisplayInfo>, Error>;

    /// List all connected displays with their power states
    ///
    /// # Returns
    /// - `Ok(Vec<DisplayInfo>)` with all connected displays
    /// - `Err(Error)` if displays could not be enumerated
    fn list_displays(&self) -> Result<Vec<DisplayInfo>, Error>;
}
