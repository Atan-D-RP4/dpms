/// Wayland backend for monitor power control
///
/// This module implements the PowerBackend trait using the Wayland compositor's
/// `zwlr_output_power_management_v1` protocol to control display power state.
///
/// The backend connects to the Wayland display socket, binds to the necessary
/// global objects, and uses the power management protocol to send power state
/// commands to the compositor.
use std::collections::HashMap;

use crate::backend::PowerBackend;
use crate::display::{DisplayInfo, DisplayTarget};
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

/// Information about a single output
struct OutputInfo {
    proxy: wl_output::WlOutput,
    name: Option<String>,
    description: Option<String>,
    make: Option<String>,
    model: Option<String>,
}

/// Wayland backend implementing PowerBackend trait
pub struct WaylandBackend {
    connection: Connection,
    state: WaylandState,
}

/// Internal state for Wayland event handling
struct WaylandState {
    power_manager: Option<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1>,
    /// All discovered outputs, keyed by wl_output id
    outputs: HashMap<u32, OutputInfo>,
    /// Current mode received from power query
    current_mode: Option<zwlr_output_power_v1::Mode>,
    failed: bool,
}

/// Minimal state for querying power mode (avoids cloning full WaylandState)
#[derive(Default)]
struct QueryState {
    current_mode: Option<zwlr_output_power_v1::Mode>,
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
            outputs: HashMap::new(),
            current_mode: None,
            failed: false,
        };

        // Bind to power manager (required)
        state.power_manager = globals
            .bind::<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1, _, _>(&qh, 1..=1, ())
            .ok();
        if state.power_manager.is_none() {
            return Err(Error::ProtocolNotSupported);
        }

        // Bind to all outputs - iterate through globals to find all wl_output
        // We need to do a roundtrip first to ensure we have all globals
        event_queue
            .roundtrip(&mut state)
            .map_err(std::io::Error::other)?;

        // Now bind to each wl_output global
        for global in globals.contents().clone_list() {
            if global.interface == "wl_output" {
                // Use version 4 for wl_output (supports name and description events)
                // Clamp to what the compositor advertises
                let version = global.version.min(4);
                if let Ok(output) =
                    globals.bind::<wl_output::WlOutput, _, _>(&qh, version..=version, global.name)
                {
                    state.outputs.insert(
                        global.name,
                        OutputInfo {
                            proxy: output,
                            name: None,
                            description: None,
                            make: None,
                            model: None,
                        },
                    );
                }
            }
        }

        if state.outputs.is_empty() {
            return Err(Error::NoDisplayFound);
        }

        // Roundtrip to receive output info events (name, description, etc.)
        event_queue
            .roundtrip(&mut state)
            .map_err(std::io::Error::other)?;

        Ok(Self { connection, state })
    }

    /// Resolve display target to list of output IDs
    fn resolve_targets(&self, target: &DisplayTarget) -> Result<Vec<u32>, Error> {
        match target {
            DisplayTarget::All | DisplayTarget::Default => {
                // Return all output IDs
                Ok(self.state.outputs.keys().copied().collect())
            }
            DisplayTarget::Named(name) => {
                // Exact match first
                for (id, info) in &self.state.outputs {
                    if info.name.as_deref() == Some(name.as_str()) {
                        return Ok(vec![*id]);
                    }
                }

                // Partial match (prefix)
                let matches: Vec<u32> = self
                    .state
                    .outputs
                    .iter()
                    .filter(|(_, info)| {
                        info.name
                            .as_ref()
                            .map(|n| n.starts_with(name))
                            .unwrap_or(false)
                    })
                    .map(|(id, _)| *id)
                    .collect();

                if matches.len() == 1 {
                    Ok(matches)
                } else if matches.len() > 1 {
                    let candidates: Vec<String> = matches
                        .iter()
                        .filter_map(|id| self.state.outputs.get(id).and_then(|o| o.name.clone()))
                        .collect();
                    Err(Error::AmbiguousDisplay {
                        name: name.clone(),
                        candidates,
                    })
                } else {
                    let available: Vec<String> = self
                        .state
                        .outputs
                        .values()
                        .filter_map(|o| o.name.clone())
                        .collect();
                    Err(Error::DisplayNotFound {
                        name: name.clone(),
                        available,
                    })
                }
            }
        }
    }
}

impl PowerBackend for WaylandBackend {
    fn set_power(&mut self, target: &DisplayTarget, state: PowerState) -> Result<(), Error> {
        let target_ids = self.resolve_targets(target)?;

        let mut event_queue = self.connection.new_event_queue();
        let qh = event_queue.handle();

        // Get power manager
        let power_manager = self
            .state
            .power_manager
            .as_ref()
            .ok_or(Error::ProtocolNotSupported)?;

        // Convert PowerState to Mode
        let mode = match state {
            PowerState::On => zwlr_output_power_v1::Mode::On,
            PowerState::Off => zwlr_output_power_v1::Mode::Off,
        };

        // Set power for each target output
        for id in target_ids {
            if let Some(output_info) = self.state.outputs.get(&id) {
                // Create power control object for this output
                let power_control = power_manager.get_output_power(&output_info.proxy, &qh, ());

                // Send set_mode request
                power_control.set_mode(mode);

                // Destroy the power control object (single-use per protocol spec)
                power_control.destroy();
            }
        }

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

    fn get_power(&self, target: &DisplayTarget) -> Result<Vec<DisplayInfo>, Error> {
        let target_ids = self.resolve_targets(target)?;

        let mut event_queue = self.connection.new_event_queue();
        let qh = event_queue.handle();

        // Get power manager
        let power_manager = self
            .state
            .power_manager
            .as_ref()
            .ok_or(Error::ProtocolNotSupported)?;

        let mut results = Vec::new();

        for id in target_ids {
            if let Some(output_info) = self.state.outputs.get(&id) {
                // Create power control object for this output
                let power_control = power_manager.get_output_power(&output_info.proxy, &qh, ());

                // Create minimal query state
                let mut query_state = QueryState::default();

                // Roundtrip to receive mode event
                event_queue
                    .roundtrip(&mut query_state)
                    .map_err(std::io::Error::other)?;

                // Destroy the power control object
                power_control.destroy();

                // Convert mode to PowerState
                let power = match query_state.current_mode {
                    Some(zwlr_output_power_v1::Mode::Off) => PowerState::Off,
                    _ => PowerState::On, // Default to On if unknown
                };

                results.push(DisplayInfo {
                    name: output_info
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("output-{}", id)),
                    power,
                    description: output_info.description.clone(),
                    make: output_info.make.clone(),
                    model: output_info.model.clone(),
                });
            }
        }

        Ok(results)
    }

    fn list_displays(&self) -> Result<Vec<DisplayInfo>, Error> {
        self.get_power(&DisplayTarget::All)
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

// Implement Dispatch for output events to capture name, description, make, model
impl Dispatch<wl_output::WlOutput, u32> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &wl_output::WlOutput,
        event: wl_output::Event,
        data: &u32,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let output_id = *data;
        if let Some(output_info) = state.outputs.get_mut(&output_id) {
            match event {
                wl_output::Event::Name { name } => {
                    output_info.name = Some(name);
                }
                wl_output::Event::Description { description } => {
                    output_info.description = Some(description);
                }
                wl_output::Event::Geometry { make, model, .. } => {
                    output_info.make = Some(make);
                    output_info.model = Some(model);
                }
                _ => {}
            }
        }
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
            zwlr_output_power_v1::Event::Mode {
                mode: WEnum::Value(m),
            } => {
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

// Implement Dispatch for QueryState (minimal state for get_power queries)
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for QueryState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Not used for queries
    }
}

impl Dispatch<wl_output::WlOutput, u32> for QueryState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_output::WlOutput,
        _event: wl_output::Event,
        _data: &u32,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Not used for queries
    }
}

impl Dispatch<zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1, ()> for QueryState {
    fn event(
        _state: &mut Self,
        _proxy: &zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1,
        _event: zwlr_output_power_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Not used for queries
    }
}

impl Dispatch<zwlr_output_power_v1::ZwlrOutputPowerV1, ()> for QueryState {
    fn event(
        state: &mut Self,
        _proxy: &zwlr_output_power_v1::ZwlrOutputPowerV1,
        event: zwlr_output_power_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwlr_output_power_v1::Event::Mode {
            mode: WEnum::Value(m),
        } = event
        {
            state.current_mode = Some(m);
        }
    }
}
