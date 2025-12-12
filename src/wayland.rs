/// Wayland backend for monitor power control
///
/// This module implements the PowerBackend trait using the Wayland compositor's
/// `zwlr_output_power_management_v1` protocol to control display power state.
///
/// The backend connects to the Wayland display socket, binds to the necessary
/// global objects, and uses the power management protocol to send power state
/// commands to the compositor.
use crate::backend::PowerBackend;
use crate::error::Error;
use crate::output::PowerState;

use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_output, wl_registry},
};
use wayland_protocols_wlr::output_power_management::v1::client::{
    zwlr_output_power_manager_v1, zwlr_output_power_v1,
};

/// Wayland backend implementing PowerBackend trait
pub struct WaylandBackend {
    connection: Connection,
    state: WaylandState,
}

/// Internal state for Wayland event handling
struct WaylandState {
    power_manager: Option<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1>,
    output: Option<wl_output::WlOutput>,
    current_mode: Option<zwlr_output_power_v1::Mode>,
    failed: bool,
}

impl WaylandBackend {
    /// Create a new Wayland backend by connecting to the compositor
    ///
    /// This connects to the Wayland display using the WAYLAND_DISPLAY environment
    /// variable and binds to the necessary global objects.
    ///
    /// # Returns
    /// - `Ok(WaylandBackend)` if connection succeeds
    /// - `Err(Error::Io)` if connection fails
    /// - `Err(Error::ProtocolNotSupported)` if compositor doesn't support power management
    pub fn new() -> Result<Self, Error> {
        // Connect to Wayland display
        let connection = Connection::connect_to_env()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotConnected, e))?;

        // Initialize registry and get globals
        let (globals, mut event_queue) =
            registry_queue_init(&connection).map_err(std::io::Error::other)?;

        let qh = event_queue.handle();

        // Create initial state
        let mut state = WaylandState {
            power_manager: None,
            output: None,
            current_mode: None,
            failed: false,
        };

        // Bind to output (get first output)
        state.output = globals
            .bind::<wl_output::WlOutput, _, _>(&qh, 1..=4, ())
            .ok();

        // Bind to power manager
        state.power_manager = globals
            .bind::<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1, _, _>(&qh, 1..=1, ())
            .ok();

        // Check if power manager is available
        if state.power_manager.is_none() {
            return Err(Error::ProtocolNotSupported);
        }

        // Check if output is available
        if state.output.is_none() {
            return Err(Error::NoDisplayFound);
        }

        // Flush initial requests
        event_queue
            .roundtrip(&mut state)
            .map_err(std::io::Error::other)?;

        Ok(Self { connection, state })
    }
}

impl PowerBackend for WaylandBackend {
    fn set_power(&mut self, state: PowerState) -> Result<(), Error> {
        let mut event_queue = self.connection.new_event_queue();
        let qh = event_queue.handle();

        // Get power manager and output
        let power_manager = self
            .state
            .power_manager
            .as_ref()
            .ok_or(Error::ProtocolNotSupported)?;
        let output = self.state.output.as_ref().ok_or(Error::NoDisplayFound)?;

        // Create power control object for this output
        let power_control = power_manager.get_output_power(output, &qh, ());

        // Convert PowerState to Mode
        let mode = match state {
            PowerState::On => zwlr_output_power_v1::Mode::On,
            PowerState::Off => zwlr_output_power_v1::Mode::Off,
        };

        // Send set_mode request
        power_control.set_mode(mode);

        // Destroy the power control object (single-use per protocol spec)
        power_control.destroy();

        // Flush and wait for compositor to process
        event_queue
            .roundtrip(&mut self.state)
            .map_err(std::io::Error::other)?;

        // Check if operation failed
        if self.state.failed {
            self.state.failed = false; // Reset flag
            return Err(Error::ProtocolNotSupported);
        }

        Ok(())
    }

    fn get_power(&self) -> Result<PowerState, Error> {
        let mut event_queue = self.connection.new_event_queue();
        let qh = event_queue.handle();

        // Get power manager and output
        let power_manager = self
            .state
            .power_manager
            .as_ref()
            .ok_or(Error::ProtocolNotSupported)?;
        let output = self.state.output.as_ref().ok_or(Error::NoDisplayFound)?;

        // Create power control object for this output
        let power_control = power_manager.get_output_power(output, &qh, ());

        // Create temporary state for this query
        let mut query_state = WaylandState {
            power_manager: self.state.power_manager.clone(),
            output: self.state.output.clone(),
            current_mode: None,
            failed: false,
        };

        // Roundtrip to receive mode event
        event_queue
            .roundtrip(&mut query_state)
            .map_err(std::io::Error::other)?;

        // Destroy the power control object
        power_control.destroy();

        // Check if we received the mode
        match query_state.current_mode {
            Some(zwlr_output_power_v1::Mode::On) => Ok(PowerState::On),
            Some(zwlr_output_power_v1::Mode::Off) => Ok(PowerState::Off),
            _ => {
                // If no mode received, assume On (compositor default)
                Ok(PowerState::On)
            }
        }
    }
}

// Implement Dispatch for registry events (needed for bind operations)
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Registry events are handled by registry_queue_init
    }
}

// Implement Dispatch for output events (we don't need to handle these)
impl Dispatch<wl_output::WlOutput, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_output::WlOutput,
        _event: wl_output::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // We don't need to handle output events for power control
    }
}

// Implement Dispatch for power manager events (none defined in protocol)
impl Dispatch<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1,
        _event: zwlr_output_power_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events defined for power manager
    }
}

// Implement Dispatch for power control events (mode and failed)
impl Dispatch<zwlr_output_power_v1::ZwlrOutputPowerV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &zwlr_output_power_v1::ZwlrOutputPowerV1,
        event: zwlr_output_power_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_power_v1::Event::Mode { mode: WEnum::Value(m) } => {
                // Store the current mode, extracting from WEnum wrapper
                state.current_mode = Some(m);
            }
            zwlr_output_power_v1::Event::Failed => {
                // Operation failed
                state.failed = true;
            }
            _ => {}
        }
    }
}
