/// PowerBackend trait for monitor power control
///
/// This trait provides a common interface for controlling monitor power state
/// across different environments (Wayland compositor and TTY).
///
/// Implementations:
/// - Wayland backend: Uses `zwlr_output_power_management_v1` protocol
/// - TTY backend: Uses libseat + DRM atomic commits with daemon mode
use crate::error::Error;
use crate::output::PowerState;

/// PowerBackend interface for monitor power control
///
/// Provides methods to set and query the power state of connected displays.
/// All backends must implement this trait to provide a consistent interface
/// regardless of the underlying environment.
pub trait PowerBackend {
    /// Set the power state of the display
    ///
    /// # Parameters
    /// - `state`: Target power state (On or Off)
    ///
    /// # Returns
    /// - `Ok(())` if the power state was successfully changed
    /// - `Err(Error)` if the operation failed
    ///
    /// # Examples
    /// ```no_run
    /// # use powermon::backend::PowerBackend;
    /// # use powermon::output::PowerState;
    /// # use powermon::error::Error;
    /// # fn example(mut backend: impl PowerBackend) -> Result<(), Error> {
    /// // Turn display off
    /// backend.set_power(PowerState::Off)?;
    ///
    /// // Turn display on
    /// backend.set_power(PowerState::On)?;
    /// # Ok(())
    /// # }
    /// ```
    fn set_power(&mut self, state: PowerState) -> Result<(), Error>;

    /// Get the current power state of the display
    ///
    /// # Returns
    /// - `Ok(PowerState::On)` if the display is currently on
    /// - `Ok(PowerState::Off)` if the display is currently off
    /// - `Err(Error)` if the status could not be determined
    ///
    /// # Examples
    /// ```no_run
    /// # use powermon::backend::PowerBackend;
    /// # use powermon::output::PowerState;
    /// # use powermon::error::Error;
    /// # fn example(backend: impl PowerBackend) -> Result<(), Error> {
    /// let status = backend.get_power()?;
    /// match status {
    ///     PowerState::On => println!("Display is on"),
    ///     PowerState::Off => println!("Display is off"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn get_power(&self) -> Result<PowerState, Error>;
}
