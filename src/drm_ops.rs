/// DRM operations module for TTY display power control
///
/// This module provides low-level DRM atomic modesetting operations to control
/// display power state via CRTC ACTIVE property. Uses libseat for device access
/// without requiring root privileges, with fallback to direct DRM access.
use crate::error::Error;
use drm::Device;
use drm::control::{AtomicCommitFlags, Device as ControlDevice, atomic, connector, crtc, property};
use std::fs::File;
use std::os::fd::{AsFd, BorrowedFd};

/// Common DRM device paths to try when opening a device
const DRM_DEVICE_PATHS: [&str; 3] = ["/dev/dri/card0", "/dev/dri/card1", "/dev/dri/card2"];

/// Wrapper around DRM device
///
/// Implements the `drm::Device` trait to enable DRM operations.
/// Can be opened via libseat (preferred) or directly (fallback).
#[derive(Debug)]
pub struct DrmDevice {
    inner: DrmDeviceInner,
}

/// Inner enum to hold either libseat device or direct file
#[derive(Debug)]
enum DrmDeviceInner {
    /// Opened via libseat - has DRM master privileges via seat
    Libseat(libseat::Device),
    /// Opened directly - may or may not have DRM master
    Direct(File),
}

impl AsFd for DrmDevice {
    fn as_fd(&self) -> BorrowedFd<'_> {
        match &self.inner {
            DrmDeviceInner::Libseat(dev) => dev.as_fd(),
            DrmDeviceInner::Direct(file) => file.as_fd(),
        }
    }
}

impl Device for DrmDevice {}
impl ControlDevice for DrmDevice {}

/// Holder for seat - may be None if using direct access
pub enum SeatHolder {
    Seat(libseat::Seat),
    None,
}

impl std::fmt::Debug for SeatHolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeatHolder::Seat(_) => write!(f, "SeatHolder::Seat(...)"),
            SeatHolder::None => write!(f, "SeatHolder::None"),
        }
    }
}

/// Open a DRM device using libseat for session management
///
/// This function initializes a libseat session and opens the first available
/// DRM device. This allows DRM operations without root privileges when running
/// in a logind session.
///
/// # Returns
/// - `Ok((SeatHolder, DrmDevice))` - The opened seat and DRM device
/// - `Err(Error::SeatError)` - Failed to open seat or device
///
/// # Example
/// ```no_run
/// # use powermon::drm_ops::open_drm_with_libseat;
/// let (seat, drm) = open_drm_with_libseat()?;
/// # Ok::<(), powermon::error::Error>(())
/// ```
pub fn open_drm_with_libseat() -> Result<(SeatHolder, DrmDevice), Error> {
    use std::sync::{Arc, Mutex};

    // Track seat events (we need to keep receiving events but don't need to act on them)
    let seat_event: Arc<Mutex<Option<libseat::SeatEvent>>> = Arc::new(Mutex::new(None));
    let seat_event_clone = Arc::clone(&seat_event);

    // Open seat with callback for events
    let mut seat = libseat::Seat::open(move |_seat, event| {
        *seat_event_clone.lock().unwrap() = Some(event);
    })
    .map_err(|e| Error::SeatError(format!("Failed to open seat: {:?}", e)))?;

    // Dispatch initial events
    seat.dispatch(0)
        .map_err(|e| Error::SeatError(format!("Failed to dispatch seat events: {:?}", e)))?;

    // Find first DRM device by trying common paths
    for path in &DRM_DEVICE_PATHS {
        // libseat opens the device and grants us DRM master privileges
        // We MUST use the fd returned by libseat, not open a new one
        match seat.open_device(path) {
            Ok(libseat_device) => {
                // Create DRM device from the libseat device
                let drm_device = DrmDevice {
                    inner: DrmDeviceInner::Libseat(libseat_device),
                };

                // Set DRM client capabilities for atomic modesetting
                if let Err(e) =
                    drm_device.set_client_capability(drm::ClientCapability::Atomic, true)
                {
                    return Err(Error::DrmError(format!(
                        "Failed to set atomic capability: {:?}",
                        e
                    )));
                }

                return Ok((SeatHolder::Seat(seat), drm_device));
            }
            Err(_) => continue,
        }
    }

    Err(Error::SeatError(
        "No DRM device found in standard paths".to_string(),
    ))
}

/// Open a DRM device directly without libseat
///
/// This is a fallback for when libseat is unavailable (e.g., SSH session).
/// Requires user to be in the video group. May not have DRM master if another
/// process holds it.
///
/// # Returns
/// - `Ok((SeatHolder::None, DrmDevice))` - The opened DRM device
/// - `Err(Error::DrmError)` - Failed to open device
pub fn open_drm_direct() -> Result<(SeatHolder, DrmDevice), Error> {
    let mut last_error: Option<String> = None;

    for path in &DRM_DEVICE_PATHS {
        match File::open(path) {
            Ok(file) => {
                let drm_device = DrmDevice {
                    inner: DrmDeviceInner::Direct(file),
                };

                // Set DRM client capabilities for atomic modesetting
                if let Err(e) =
                    drm_device.set_client_capability(drm::ClientCapability::Atomic, true)
                {
                    // This device doesn't support atomic, try next
                    last_error = Some(format!("{}: atomic not supported ({:?})", path, e));
                    continue;
                }

                return Ok((SeatHolder::None, drm_device));
            }
            Err(e) => {
                last_error = Some(format!("{}: {}", path, e));
                continue;
            }
        }
    }

    Err(Error::DrmError(
        last_error.unwrap_or_else(|| "No DRM device found".to_string()),
    ))
}

/// Open a DRM device, trying libseat first then falling back to direct access
///
/// # Returns
/// - `Ok((SeatHolder, DrmDevice))` - The opened DRM device
/// - `Err(Error)` - Both libseat and direct access failed
pub fn open_drm() -> Result<(SeatHolder, DrmDevice), Error> {
    // Try libseat first (preferred - handles session activation properly)
    match open_drm_with_libseat() {
        Ok(result) => Ok(result),
        Err(_libseat_err) => {
            // Libseat failed, try direct access
            open_drm_direct()
        }
    }
}

impl DrmDevice {
    /// Find the CRTC handle for the first connected connector
    ///
    /// Scans all connectors to find the first one in Connected state,
    /// then returns its associated CRTC handle.
    ///
    /// # Returns
    /// - `Ok(CrtcHandle)` - The CRTC handle for the connected display
    /// - `Err(Error::NoConnectedDisplay)` - No connected display found
    /// - `Err(Error::DrmError)` - DRM operation failed
    ///
    /// # Example
    /// ```no_run
    /// # use powermon::drm_ops::open_drm_with_libseat;
    /// # let (_seat, drm) = open_drm_with_libseat()?;
    /// let crtc = drm.find_active_crtc()?;
    /// # Ok::<(), powermon::error::Error>(())
    /// ```
    pub fn find_active_crtc(&self) -> Result<crtc::Handle, Error> {
        // Get resource handles
        let res = self
            .resource_handles()
            .map_err(|e| Error::DrmError(format!("Failed to get resource handles: {:?}", e)))?;

        // Iterate through connectors to find first connected one
        for conn_handle in res.connectors() {
            let conn_info = self
                .get_connector(*conn_handle, false)
                .map_err(|e| Error::DrmError(format!("Failed to get connector info: {:?}", e)))?;

            if conn_info.state() == connector::State::Connected {
                // Get the encoder for this connector
                if let Some(encoder_handle) = conn_info.current_encoder() {
                    let encoder_info = self.get_encoder(encoder_handle).map_err(|e| {
                        Error::DrmError(format!("Failed to get encoder info: {:?}", e))
                    })?;

                    if let Some(crtc_handle) = encoder_info.crtc() {
                        return Ok(crtc_handle);
                    }
                }

                // If no current encoder, try the first possible encoder
                for &enc_handle in conn_info.encoders() {
                    let encoder_info = self.get_encoder(enc_handle).map_err(|e| {
                        Error::DrmError(format!("Failed to get encoder info: {:?}", e))
                    })?;

                    if let Some(crtc_handle) = encoder_info.crtc() {
                        return Ok(crtc_handle);
                    }
                }
            }
        }

        Err(Error::NoDisplayFound)
    }

    /// Set CRTC ACTIVE property via atomic commit
    ///
    /// Uses DRM atomic modesetting to set the ACTIVE property of the specified CRTC.
    /// This turns the display on (active=true) or off (active=false).
    ///
    /// # Parameters
    /// - `crtc`: The CRTC handle to modify
    /// - `active`: true to turn display on, false to turn it off
    ///
    /// # Returns
    /// - `Ok(())` - Atomic commit succeeded
    /// - `Err(Error::DrmError)` - Atomic commit or property lookup failed
    ///
    /// # Example
    /// ```no_run
    /// # use powermon::drm_ops::open_drm_with_libseat;
    /// # let (_seat, drm) = open_drm_with_libseat()?;
    /// # let crtc = drm.find_active_crtc()?;
    /// // Turn display off
    /// drm.set_crtc_active(crtc, false)?;
    /// // Turn display on
    /// drm.set_crtc_active(crtc, true)?;
    /// # Ok::<(), powermon::error::Error>(())
    /// ```
    pub fn set_crtc_active(&self, crtc_handle: crtc::Handle, active: bool) -> Result<(), Error> {
        // Find the ACTIVE property for this CRTC
        let props = self
            .get_properties(crtc_handle)
            .map_err(|e| Error::DrmError(format!("Failed to get CRTC properties: {:?}", e)))?;

        let mut active_prop_id = None;
        for (&prop_handle, _) in props.iter() {
            let prop_info = self
                .get_property(prop_handle)
                .map_err(|e| Error::DrmError(format!("Failed to get property info: {:?}", e)))?;

            if prop_info.name().to_str() == Ok("ACTIVE") {
                active_prop_id = Some(prop_handle);
                break;
            }
        }

        let active_prop = active_prop_id
            .ok_or_else(|| Error::DrmError("ACTIVE property not found for CRTC".to_string()))?;

        // Create atomic request
        let mut req = atomic::AtomicModeReq::new();
        req.add_property(crtc_handle, active_prop, property::Value::Boolean(active));

        // Commit with ALLOW_MODESET flag (required for ACTIVE property changes)
        let flags = AtomicCommitFlags::ALLOW_MODESET;
        self.atomic_commit(flags, req)
            .map_err(|e| Error::DrmError(format!("Atomic commit failed: {:?}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drm_device_implements_required_traits() {
        // This is a compile-time test - if it compiles, the traits are implemented
        fn assert_device<T: Device>() {}
        fn assert_control_device<T: ControlDevice>() {}

        assert_device::<DrmDevice>();
        assert_control_device::<DrmDevice>();
    }

    // Note: Integration tests that require actual DRM hardware cannot be run in CI
    // These would be part of manual testing on real hardware
}
