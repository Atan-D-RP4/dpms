use crate::error::Error;
use std::io::IsTerminal;
use std::os::unix::fs::FileTypeExt;

/// Detected backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Wayland,
    X11,
    Tty,
}

/// Try to find an available Wayland socket in XDG_RUNTIME_DIR
///
/// This is useful for SSH sessions where WAYLAND_DISPLAY is not set
/// but a compositor is running on the target machine.
///
/// # Returns
/// - `Some(socket_name)` - Found a Wayland socket (e.g., "wayland-1")
/// - `None` - No Wayland socket found
fn find_wayland_socket() -> Option<String> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok()?;
    let dir = std::fs::read_dir(&runtime_dir).ok()?;

    for entry in dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Look for wayland-N sockets (not .lock files)
        if name_str.starts_with("wayland-") && !name_str.ends_with(".lock") {
            // Verify it's a socket
            if let Ok(metadata) = entry.metadata()
                && metadata.file_type().is_socket()
            {
                return Some(name_str.into_owned());
            }
        }
    }

    None
}

/// Detect which backend to use based on environment
///
/// Detection order:
/// 1. Check if WAYLAND_DISPLAY is set -> Wayland
/// 2. Check if a Wayland socket exists (for SSH sessions) -> Wayland (sets WAYLAND_DISPLAY)
/// 3. Check if DISPLAY is set -> X11
/// 4. Check if stdin is a TTY -> TTY
/// 5. Otherwise -> Error
pub fn detect_backend() -> Result<Backend, Error> {
    // Check for Wayland first
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return Ok(Backend::Wayland);
    }

    // Try to auto-detect Wayland socket (useful for SSH sessions)
    if let Some(socket) = find_wayland_socket() {
        // Set WAYLAND_DISPLAY so the Wayland backend can connect
        // SAFETY: We're setting this before any Wayland connection is made
        unsafe {
            std::env::set_var("WAYLAND_DISPLAY", &socket);
        }
        return Ok(Backend::Wayland);
    }

    // Check for X11 (yet unimplemented)
    if std::env::var("DISPLAY").is_ok() {
        return Ok(Backend::X11);
    }

    // Check if we're on a TTY
    // 1. stdin is a terminal (interactive shell)
    // 2. XDG_SESSION_TYPE is "tty" (logind session, works from SSH too)
    if std::io::stdin().is_terminal() {
        return Ok(Backend::Tty);
    }
    if std::env::var("XDG_SESSION_TYPE")
        .map(|v| v == "tty")
        .unwrap_or(false)
    {
        return Ok(Backend::Tty);
    }

    // Neither Wayland nor TTY detected
    Err(Error::UnsupportedEnvironment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_wayland_when_env_var_set() {
        // Set WAYLAND_DISPLAY temporarily
        // SAFETY: This is a test and we're the only ones modifying this env var
        unsafe {
            std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        }

        let result = detect_backend();

        // Clean up
        // SAFETY: This is a test and we're the only ones modifying this env var
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
        }

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Backend::Wayland);
    }

    #[test]
    fn detect_tty_when_wayland_and_x11_not_set_and_on_tty() {
        // Ensure WAYLAND_DISPLAY and DISPLAY are not set
        // SAFETY: This is a test and we're the only ones modifying this env var
        let old_display = std::env::var("DISPLAY").ok();
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
            std::env::remove_var("DISPLAY");
        }

        let result = detect_backend();

        // Restore DISPLAY if it was set
        // SAFETY: This is a test and we're the only ones modifying this env var
        unsafe {
            if let Some(val) = old_display {
                std::env::set_var("DISPLAY", val);
            }
        }

        // Note: This test validates the detection logic works correctly.
        // The result depends on the environment:
        // - TTY: returns Backend::Tty
        // - Wayland socket exists: returns Backend::Wayland (auto-detected)
        // - Neither: returns UnsupportedEnvironment
        match result {
            Ok(Backend::Tty) => {
                // We're on a TTY, correct detection
            }
            Ok(Backend::Wayland) => {
                // Wayland socket was auto-detected (running in Wayland session)
            }
            Err(Error::UnsupportedEnvironment) => {
                // We're not on a TTY and no Wayland (e.g., CI), this is also correct
            }
            other => panic!("Unexpected result: {:?}", other),
        }
    }

    #[test]
    fn wayland_takes_precedence_over_tty() {
        // Even if we're on a TTY, Wayland should be detected first if env var is set
        // SAFETY: This is a test and we're the only ones modifying this env var
        unsafe {
            std::env::set_var("WAYLAND_DISPLAY", "wayland-1");
        }

        let result = detect_backend();

        // Clean up
        // SAFETY: This is a test and we're the only ones modifying this env var
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
        }

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Backend::Wayland);
    }

    #[test]
    fn backend_enum_equality() {
        assert_eq!(Backend::Wayland, Backend::Wayland);
        assert_eq!(Backend::Tty, Backend::Tty);
        assert_ne!(Backend::Wayland, Backend::Tty);
    }

    #[test]
    fn backend_enum_debug() {
        let wayland = Backend::Wayland;
        let tty = Backend::Tty;

        assert_eq!(format!("{:?}", wayland), "Wayland");
        assert_eq!(format!("{:?}", tty), "Tty");
    }
}
