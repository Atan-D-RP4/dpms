/// TTY Daemon Lifecycle Management
///
/// This module implements the daemon process for TTY display power control.
/// The daemon holds DRM master to keep the display off and responds to signals:
/// - SIGTERM/SIGINT: Restore display and exit cleanly
///
/// The daemon uses a PID file at `/run/user/$UID/powermon.pid` for single-instance
/// enforcement and IPC coordination.
use crate::drm_ops::{SeatHolder, open_drm};
use crate::error::Error;
use nix::libc;
use nix::sys::signal::{self, Signal};
use nix::unistd::{ForkResult, Pid, fork, setsid};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Global flag to signal daemon shutdown
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Get the PID file path for the daemon
///
/// # Returns
/// Path to `/run/user/$UID/powermon.pid`
///
/// # Errors
/// Returns `Error::PidFileError` if XDG_RUNTIME_DIR is not set
pub fn get_pid_file_path() -> Result<PathBuf, Error> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .or_else(|_| {
            // Fallback to /run/user/$UID if XDG_RUNTIME_DIR not set
            // Use Uid::effective() to get current user's UID
            let uid = nix::unistd::Uid::effective();
            Ok(format!("/run/user/{}", uid))
        })
        .map_err(|e: std::env::VarError| {
            Error::PidFileError(format!("Could not determine runtime directory: {}", e))
        })?;

    Ok(PathBuf::from(runtime_dir).join("powermon.pid"))
}

/// Check if a process with the given PID is running
///
/// # Parameters
/// - `pid`: Process ID to check
///
/// # Returns
/// `true` if the process exists and is running
fn is_process_running(pid: Pid) -> bool {
    // Sending signal 0 doesn't actually send a signal, but checks if we can send to the process
    signal::kill(pid, None).is_ok()
}

/// Read PID from PID file
///
/// # Parameters
/// - `path`: Path to PID file
///
/// # Returns
/// - `Ok(Some(Pid))` - PID was read successfully
/// - `Ok(None)` - PID file doesn't exist
/// - `Err(Error)` - Failed to read or parse PID file
fn read_pid_file<P: AsRef<Path>>(path: P) -> Result<Option<Pid>, Error> {
    let path = path.as_ref();

    if !path.exists() {
        return Ok(None);
    }

    let mut file = fs::File::open(path)
        .map_err(|e| Error::PidFileError(format!("Failed to open PID file: {}", e)))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| Error::PidFileError(format!("Failed to read PID file: {}", e)))?;

    let pid_num: i32 = contents
        .trim()
        .parse()
        .map_err(|e| Error::PidFileError(format!("Invalid PID in file: {}", e)))?;

    Ok(Some(Pid::from_raw(pid_num)))
}

/// Write PID to PID file
///
/// # Parameters
/// - `path`: Path to PID file
/// - `pid`: PID to write
///
/// # Returns
/// - `Ok(())` - PID was written successfully
/// - `Err(Error)` - Failed to write PID file
fn write_pid_file<P: AsRef<Path>>(path: P, pid: Pid) -> Result<(), Error> {
    let path = path.as_ref();

    let mut file = fs::File::create(path)
        .map_err(|e| Error::PidFileError(format!("Failed to create PID file: {}", e)))?;

    write!(file, "{}", pid)
        .map_err(|e| Error::PidFileError(format!("Failed to write PID: {}", e)))?;

    Ok(())
}

/// Remove PID file
///
/// # Parameters
/// - `path`: Path to PID file
///
/// # Returns
/// - `Ok(())` - PID file was removed or didn't exist
/// - `Err(Error)` - Failed to remove PID file
fn remove_pid_file<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    let path = path.as_ref();

    if path.exists() {
        fs::remove_file(path)
            .map_err(|e| Error::PidFileError(format!("Failed to remove PID file: {}", e)))?;
    }

    Ok(())
}

/// Check if the powermon daemon is currently running
///
/// Returns the PID of the running daemon, or None if no daemon is running.
/// Also cleans up stale PID files if the process is no longer alive.
///
/// # Returns
/// - `Some(Pid)` - Daemon is running with this PID
/// - `None` - No daemon is running (or stale PID was cleaned up)
pub fn is_daemon_running() -> Option<Pid> {
    let pid_path = match get_pid_file_path() {
        Ok(p) => p,
        Err(_) => return None,
    };

    let pid = match read_pid_file(&pid_path) {
        Ok(Some(pid)) => pid,
        Ok(None) => return None,
        Err(_) => return None,
    };

    // Check if process is still alive
    if is_process_running(pid) {
        Some(pid)
    } else {
        // Process is dead, clean up stale PID file
        let _ = remove_pid_file(&pid_path);
        None
    }
}

/// Signal handler for SIGTERM and SIGINT
///
/// Sets the global shutdown flag to request daemon exit
extern "C" fn handle_shutdown_signal(_: libc::c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

/// Install signal handlers for SIGTERM and SIGINT
///
/// # Returns
/// - `Ok(())` - Signal handlers installed successfully
/// - `Err(Error)` - Failed to install signal handlers
fn install_signal_handlers() -> Result<(), Error> {
    // Use sigaction for reliable signal handling
    let sig_action = signal::SigAction::new(
        signal::SigHandler::Handler(handle_shutdown_signal),
        signal::SaFlags::empty(),
        signal::SigSet::empty(),
    );

    // Install handler for SIGTERM
    unsafe {
        signal::sigaction(Signal::SIGTERM, &sig_action)
            .map_err(|e| Error::SignalError(format!("Failed to install SIGTERM handler: {}", e)))?;
    }

    // Install handler for SIGINT
    unsafe {
        signal::sigaction(Signal::SIGINT, &sig_action)
            .map_err(|e| Error::SignalError(format!("Failed to install SIGINT handler: {}", e)))?;
    }

    Ok(())
}

/// Daemon main loop
///
/// This function runs in the child process after fork. It:
/// 1. Opens libseat session and DRM device
/// 2. Disables CRTC (turns off display)
/// 3. Writes PID file
/// 4. Installs signal handlers for SIGTERM and SIGINT
/// 5. Waits for shutdown signal
/// 6. Restores CRTC (turns on display)
/// 7. Cleans up and exits
///
/// # Returns
/// This function does not return - it exits the process
fn daemon_main() -> ! {
    // Install signal handlers first
    if let Err(e) = install_signal_handlers() {
        eprintln!("Failed to install signal handlers: {}", e);
        std::process::exit(1);
    }

    // Open seat and DRM device
    let (mut seat_holder, drm) = match open_drm() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to open DRM device: {}", e);
            std::process::exit(1);
        }
    };

    // Find active CRTC
    let crtc_handle = match drm.find_active_crtc() {
        Ok(handle) => handle,
        Err(e) => {
            eprintln!("Failed to find active CRTC: {}", e);
            std::process::exit(1);
        }
    };

    // Disable CRTC (turn off display)
    if let Err(e) = drm.set_crtc_active(crtc_handle, false) {
        eprintln!("Failed to disable CRTC: {}", e);
        std::process::exit(1);
    }

    // Write PID file
    let pid_path = match get_pid_file_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get PID file path: {}", e);
            // Try to restore display before exiting
            let _ = drm.set_crtc_active(crtc_handle, true);
            std::process::exit(1);
        }
    };

    if let Err(e) = write_pid_file(&pid_path, Pid::this()) {
        eprintln!("Failed to write PID file: {}", e);
        // Try to restore display before exiting
        let _ = drm.set_crtc_active(crtc_handle, true);
        std::process::exit(1);
    }

    // Main daemon loop - wait for shutdown signal
    while !SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
        // Dispatch seat events if using libseat (required to keep session alive)
        if let SeatHolder::Seat(ref mut seat) = seat_holder
            && let Err(e) = seat.dispatch(100)
        {
            eprintln!("Failed to dispatch seat events: {:?}", e);
            break;
        }

        // Sleep briefly to avoid busy-waiting
        thread::sleep(Duration::from_millis(100));
    }

    // Shutdown sequence: restore display
    if let Err(e) = drm.set_crtc_active(crtc_handle, true) {
        eprintln!("Failed to restore CRTC: {}", e);
    }

    // Remove PID file
    if let Err(e) = remove_pid_file(&pid_path) {
        eprintln!("Failed to remove PID file: {}", e);
    }

    // Exit cleanly
    std::process::exit(0);
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
/// - `Err(Error::ForkError)` - Fork operation failed
pub fn start_daemon() -> Result<(), Error> {
    // Check if daemon is already running (defense in depth)
    if let Some(_pid) = is_daemon_running() {
        return Ok(()); // Already running, idempotent
    }

    // Fork into parent and child
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            // Parent process: wait for daemon to start and write PID file
            // Retry up to 20 times (2 seconds total) to handle slow DRM init
            let pid_path = get_pid_file_path()?;
            for _ in 0..20 {
                thread::sleep(Duration::from_millis(100));

                if pid_path.exists() {
                    // Verify the PID in the file is actually the child we forked
                    if let Ok(Some(pid)) = read_pid_file(&pid_path)
                        && pid == child
                    {
                        return Ok(());
                    }
                }
            }

            Err(Error::DaemonStartFailed)
        }
        Ok(ForkResult::Child) => {
            // Child process: become session leader and run daemon
            setsid().map_err(|e| Error::ForkError(format!("Failed to setsid: {}", e)))?;

            // Run daemon main loop (this never returns)
            daemon_main();
        }
        Err(e) => Err(Error::ForkError(format!("Fork failed: {}", e))),
    }
}

/// Stop the daemon by sending SIGTERM
///
/// Sends SIGTERM to the daemon process, which triggers it to:
/// 1. Restore CRTC ACTIVE property to 1 (turn display back on)
/// 2. Remove PID file
/// 3. Exit cleanly
///
/// # Returns
/// - `Ok(())` - Daemon stopped successfully
/// - `Err(Error::DaemonStopTimeout)` - Daemon didn't stop within timeout
/// - `Err(Error)` - Failed to send signal or read PID file
pub fn stop_daemon() -> Result<(), Error> {
    let pid_path = get_pid_file_path()?;

    let pid = match read_pid_file(&pid_path)? {
        Some(pid) => pid,
        None => {
            // No PID file, daemon not running
            return Ok(());
        }
    };

    // Check if process is actually running
    if !is_process_running(pid) {
        // Process already dead, clean up stale PID file
        remove_pid_file(&pid_path)?;
        return Ok(());
    }

    // Send SIGTERM to daemon
    signal::kill(pid, Signal::SIGTERM)
        .map_err(|e| Error::SignalError(format!("Failed to send SIGTERM: {}", e)))?;

    // Wait for daemon to exit (up to 5 seconds)
    for _ in 0..50 {
        thread::sleep(Duration::from_millis(100));

        if !is_process_running(pid) {
            // Daemon stopped, clean up PID file if still present
            let _ = remove_pid_file(&pid_path);
            return Ok(());
        }
    }

    // Timeout - daemon didn't stop
    Err(Error::DaemonStopTimeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_pid_file_path() {
        // Test with XDG_RUNTIME_DIR set
        unsafe {
            std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
        }
        let path = get_pid_file_path().unwrap();
        assert_eq!(path, PathBuf::from("/run/user/1000/powermon.pid"));
    }

    #[test]
    fn is_process_running_self() {
        // Test with our own PID (which is definitely running)
        let pid = Pid::this();
        assert!(is_process_running(pid));
    }

    #[test]
    fn is_process_running_nonexistent() {
        // Test with a PID that definitely doesn't exist (PID_MAX is typically 32768)
        let pid = Pid::from_raw(99999);
        assert!(!is_process_running(pid));
    }

    #[test]
    fn read_pid_file_nonexistent() {
        let result = read_pid_file("/tmp/powermon-test-nonexistent.pid").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn write_and_read_pid_file() {
        let test_path = "/tmp/powermon-test-write-read.pid";
        let test_pid = Pid::from_raw(12345);

        // Clean up any existing file
        let _ = fs::remove_file(test_path);

        // Write PID
        write_pid_file(test_path, test_pid).unwrap();

        // Read it back
        let read_pid = read_pid_file(test_path).unwrap();
        assert_eq!(read_pid, Some(test_pid));

        // Clean up
        let _ = fs::remove_file(test_path);
    }

    #[test]
    fn test_remove_pid_file() {
        let test_path = "/tmp/powermon-test-remove.pid";

        // Create a file
        let _ = fs::File::create(test_path);

        // Remove it
        remove_pid_file(test_path).unwrap();

        // Verify it's gone
        assert!(!Path::new(test_path).exists());

        // Removing non-existent file should succeed (idempotent)
        remove_pid_file(test_path).unwrap();
    }

    #[test]
    fn is_daemon_running_no_pid_file() {
        // When no PID file exists, should return None
        // This assumes no actual daemon is running
        unsafe {
            std::env::set_var("XDG_RUNTIME_DIR", "/tmp/powermon-test-nofile");
        }
        let result = is_daemon_running();
        assert!(result.is_none());
    }

    // Note: Full integration tests for fork/daemon require real DRM hardware
    // and are part of manual testing
}
