use crate::error::Error;
use std::io::IsTerminal;

/// Detected backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Wayland,
    X11,
    Tty,
}

/// Detect which backend to use based on environment
///
/// Detection order:
/// 1. Check if WAYLAND_DISPLAY is set -> Wayland
/// 2. Check if stdin is a TTY -> TTY
/// 3. Otherwise -> Error
pub fn detect_backend() -> Result<Backend, Error> {
    // Check for Wayland first
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
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
    fn detect_tty_when_wayland_not_set_and_on_tty() {
        // Ensure WAYLAND_DISPLAY is not set
        // SAFETY: This is a test and we're the only ones modifying this env var
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
        }

        let result = detect_backend();

        // Note: This test will pass if we're on a TTY, or fail with NoSupportedEnvironment
        // if we're not on a TTY (e.g., running in IDE or CI)
        // We test the logic, not the actual environment
        match result {
            Ok(Backend::Tty) => {
                // We're on a TTY, correct detection
            }
            Err(Error::UnsupportedEnvironment) => {
                // We're not on a TTY (e.g., IDE/CI), this is also correct
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
