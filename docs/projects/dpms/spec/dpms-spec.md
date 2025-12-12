# dpms Technical Specification

**Brief**: [dpms-brief.md](../brief/dpms-brief.md)
**Created**: 2025-12-12
**Status**: Draft
**Compliance Score**: 100%

## Executive Summary

dpms is a hybrid monitor power control tool for Linux that works in both Wayland compositor and TTY (no compositor) environments. It enables power savings on laptop servers by reliably disabling displays without requiring root privileges, leveraging libseat for session management and Wayland protocols for compositor communication.

## Data Contracts

### Inputs

| Source | Data | Type | Notes |
|--------|------|------|-------|
| CLI args | Command (`on`/`off`/`status`) | String | Required positional arg |
| CLI args | `--json` flag | Boolean | Optional, only for `status` |
| Environment | `WAYLAND_DISPLAY` | Env var | Presence indicates Wayland session |
| Environment | `XDG_RUNTIME_DIR` | Env var | Required for socket/PID file paths |
| Wayland | Compositor socket | Unix socket | At `$WAYLAND_DISPLAY` |
| Wayland | `zwlr_output_power_v1` events | Protocol | Power state from compositor |
| libseat | Session FD | File descriptor | From `libseat_open_seat()` |
| DRM | Device FD | File descriptor | From `libseat_open_device()` |
| DRM | Connector/CRTC info | ioctl response | Display topology |
| Filesystem | PID file | File | `/run/user/$UID/dpms.pid` |

### Outputs

| Consumer | Data | Type | Notes |
|----------|------|------|-------|
| User (stdout) | Status text | `"Display: On\|Off"` | Human-readable |
| User (stdout) | Status JSON | `{"power": "on"\|"off"}` | With `--json` flag |
| User (stderr) | Error messages | String | All errors to stderr |
| Shell | Exit code | Integer | 0=success, 1=error, 2=usage |
| Wayland | `set_mode` request | Protocol | Power state change |
| DRM | Atomic commit | ioctl | CRTC ACTIVE property |
| Filesystem | PID file | File | Daemon PID for single-instance |
| Daemon | Signal handling | SIGTERM | Graceful shutdown + restore |

### Interface Constraints

1. **CLI interface is stable v1 API**: `dpms {on|off|status} [--json]`
2. **Exit codes are stable**: 0=success, 1=error, 2=usage
3. **JSON output schema is stable**: `{"power": "on"|"off"}`
4. **PID file location is stable**: `/run/user/$UID/dpms.pid`

### Scope Classification

**GREENFIELD** - New CLI tool, all contracts defined by this spec.

## Technical Design

### Architecture

```
+-------------------------------------------------------------+
|                        dpms CLI                          |
|  +----------------------------------------------------------+
|  |                    Argument Parser                        |
|  |              (clap: on|off|status [--json])               |
|  +----------------------------------------------------------+
|                              |
|  +----------------------------------------------------------+
|  |                Environment Detector                       |
|  |         Check WAYLAND_DISPLAY -> try connect              |
|  +----------------------------------------------------------+
|                    |                    |
|           +-------v--------+  +--------v--------+
|           | Wayland Backend|  |   TTY Backend   |
|           |                |  |                 |
|           | wayland-client |  | libseat + drm   |
|           | zwlr_output_   |  | atomic commit   |
|           | power_v1       |  | + daemon        |
|           +----------------+  +-----------------+
+-------------------------------------------------------------+
```

### Module Structure

```
src/
├── main.rs           # Entry point, arg parsing, dispatch
├── cli.rs            # CLI argument definitions (clap)
├── detect.rs         # Environment detection logic
├── backend/
│   ├── mod.rs        # Backend trait definition
│   ├── wayland.rs    # Wayland protocol backend
│   └── tty/
│       ├── mod.rs    # TTY backend coordinator
│       ├── daemon.rs # Daemon fork/signal logic
│       └── drm.rs    # DRM atomic commit operations
├── error.rs          # Error types and exit codes
└── output.rs         # Status output formatting (text/JSON)
```

### Data Models

```rust
/// Power state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    On,
    Off,
}

/// CLI command
#[derive(Debug, Clone)]
pub enum Command {
    On,
    Off,
    Status { json: bool },
}

/// Detected environment
#[derive(Debug)]
pub enum Environment {
    Wayland { display: String },
    Tty,
}

/// Backend trait - implemented by Wayland and TTY backends
pub trait PowerBackend {
    fn set_power(&mut self, state: PowerState) -> Result<(), Error>;
    fn get_power(&self) -> Result<PowerState, Error>;
}

/// Exit codes
pub enum ExitCode {
    Success = 0,
    Error = 1,
    Usage = 2,
}

/// Status output for JSON serialization
#[derive(Serialize)]
pub struct StatusOutput {
    power: String,  // "on" | "off"
}
```

### State Machine: TTY Backend Daemon

```
+-------------------------------------------------------------+
|                    TTY Daemon Lifecycle                      |
+-------------------------------------------------------------+

  dpms off                    dpms on
       |                               |
       v                               v
+--------------+              +--------------+
| Check PID    |              | Check PID    |
| file exists? |              | file exists? |
+------+-------+              +------+-------+
       |                             |
   No  |  Yes                    No  |  Yes
       |   |                         |   |
       v   v                         v   v
+--------------+              +--------------+
| Fork daemon  |  Already off | Already on   | Send SIGTERM
|              |  (success)   | (success)    | to daemon
+------+-------+              +--------------+     |
       |                                           |
       v                                           v
+--------------+                          +--------------+
| Parent: exit |                          | Daemon:      |
| success      |                          | - Restore    |
+--------------+                          |   CRTC       |
       |                                  | - Remove PID |
       |                                  | - Exit       |
       v                                  +--------------+
+--------------+
| Daemon:      |
| - Open seat  |
| - Open DRM   |
| - Disable    |
|   CRTC       |
| - Write PID  |
| - Wait for   |
|   SIGTERM    |
+--------------+
```

### Key Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
nix = { version = "0.27", features = ["signal", "process", "fs"] }

# Wayland backend
wayland-client = "0.31"
wayland-protocols-wlr = { version = "0.2", features = ["client"] }

# TTY backend
libseat = "0.2"
drm = "0.11"
```

## Features

### Feature: CLI Argument Parser (F1)

**Brief Reference**: CLI Interface
**Phase**: 1
**Complexity**: S
**Dependencies**: None

**Acceptance Criteria**:

- GIVEN user runs `dpms on`
  WHEN command is parsed
  THEN `Command::On` is returned

- GIVEN user runs `dpms off`
  WHEN command is parsed
  THEN `Command::Off` is returned

- GIVEN user runs `dpms status`
  WHEN command is parsed
  THEN `Command::Status { json: false }` is returned

- GIVEN user runs `dpms status --json`
  WHEN command is parsed
  THEN `Command::Status { json: true }` is returned

- GIVEN user runs `dpms foo`
  WHEN command is parsed
  THEN error is returned with exit code 2

______________________________________________________________________

### Feature: Error Types & Exit Codes (F2)

**Brief Reference**: Error Handling, Non-functional requirements
**Phase**: 1
**Complexity**: S
**Dependencies**: None

**Acceptance Criteria**:

- GIVEN any error occurs
  WHEN error is displayed
  THEN message goes to stderr
  AND exit code is 1

- GIVEN usage error occurs
  WHEN error is displayed
  THEN usage help is shown
  AND exit code is 2

______________________________________________________________________

### Feature: Output Formatting (F3)

**Brief Reference**: Status, --json flag
**Phase**: 1
**Complexity**: S
**Dependencies**: F1

**Acceptance Criteria**:

- GIVEN `PowerState::On` and `json=false`
  WHEN formatted
  THEN output is `"Display: On\n"`

- GIVEN `PowerState::Off` and `json=false`
  WHEN formatted
  THEN output is `"Display: Off\n"`

- GIVEN `PowerState::On` and `json=true`
  WHEN formatted
  THEN output is valid JSON `{"power":"on"}`

- GIVEN `PowerState::Off` and `json=true`
  WHEN formatted
  THEN output is valid JSON `{"power":"off"}`

______________________________________________________________________

### Feature: Environment Detection (F4)

**Brief Reference**: Environment Detection
**Phase**: 1
**Complexity**: M
**Dependencies**: None

**Acceptance Criteria**:

- GIVEN `WAYLAND_DISPLAY` is set
  AND compositor is running and connectable
  WHEN environment is detected
  THEN `Environment::Wayland { display }` is returned

- GIVEN `WAYLAND_DISPLAY` is not set
  AND libseat can open a seat
  WHEN environment is detected
  THEN `Environment::Tty` is returned

- GIVEN `WAYLAND_DISPLAY` is not set
  AND libseat cannot open a seat
  WHEN environment is detected
  THEN `Error::UnsupportedEnvironment` is returned

______________________________________________________________________

### Feature: Backend Trait (F5)

**Brief Reference**: Architecture
**Phase**: 1
**Complexity**: S
**Dependencies**: F2

**Acceptance Criteria**:

- GIVEN `PowerBackend` trait
  WHEN implemented by Wayland backend
  THEN `set_power()` and `get_power()` work correctly

- GIVEN `PowerBackend` trait
  WHEN implemented by TTY backend
  THEN `set_power()` and `get_power()` work correctly

______________________________________________________________________

### Feature: Wayland Backend (F6)

**Brief Reference**: Wayland Backend requirements
**Phase**: 2
**Complexity**: L
**Dependencies**: F4, F5

**Acceptance Criteria**:

- GIVEN user is in Wayland session
  AND compositor supports `zwlr_output_power_management_v1`
  AND display is currently on
  WHEN user runs `dpms off`
  THEN `set_mode(off)` is sent to compositor
  AND display turns off
  AND command exits with code 0

- GIVEN user is in Wayland session
  AND compositor supports `zwlr_output_power_management_v1`
  AND display is currently off
  WHEN user runs `dpms on`
  THEN `set_mode(on)` is sent to compositor
  AND display turns on
  AND command exits with code 0

- GIVEN user is in Wayland session
  AND compositor supports `zwlr_output_power_management_v1`
  WHEN user runs `dpms status`
  THEN current power state is queried from compositor
  AND stdout shows "Display: On" or "Display: Off"
  AND command exits with code 0

- GIVEN user is in Wayland session
  AND compositor does NOT support `zwlr_output_power_management_v1`
  WHEN user runs `dpms off`
  THEN stderr shows "Compositor does not support power management protocol"
  AND command exits with code 1

______________________________________________________________________

### Feature: TTY DRM Operations (F7)

**Brief Reference**: TTY Backend - DRM atomic commits
**Phase**: 3
**Complexity**: M
**Dependencies**: F5

**Acceptance Criteria**:

- GIVEN DRM device is accessible via libseat
  AND first connected connector is found
  WHEN CRTC ACTIVE is set to 0
  THEN atomic commit succeeds
  AND display turns off

- GIVEN DRM device is accessible via libseat
  AND CRTC is currently disabled
  WHEN CRTC ACTIVE is set to 1
  THEN atomic commit succeeds
  AND display turns on

______________________________________________________________________

### Feature: TTY Daemon Lifecycle (F8)

**Brief Reference**: TTY Backend - Daemon mode
**Phase**: 3
**Complexity**: L
**Dependencies**: F7

**Acceptance Criteria**:

- GIVEN user is on TTY
  AND no daemon is running
  WHEN user runs `dpms off`
  THEN daemon process is forked
  AND daemon opens seat and DRM device
  AND daemon disables CRTC
  AND daemon writes PID to `/run/user/$UID/dpms.pid`
  AND parent exits with code 0

- GIVEN daemon is running
  AND PID file exists
  WHEN user runs `dpms on`
  THEN SIGTERM is sent to daemon
  AND daemon restores CRTC ACTIVE=1
  AND daemon removes PID file
  AND daemon exits
  AND command exits with code 0

- GIVEN daemon is running
  WHEN user runs `dpms off`
  THEN stderr shows "Display already off"
  AND command exits with code 0 (idempotent)

- GIVEN no daemon is running
  WHEN user runs `dpms on`
  THEN stderr shows "Display already on"
  AND command exits with code 0 (idempotent)

- GIVEN stale PID file exists (process dead)
  WHEN user runs `dpms off`
  THEN stale PID file is removed
  AND new daemon is spawned normally

- GIVEN daemon is killed unexpectedly (SIGKILL)
  WHEN DRM master is released
  THEN display automatically returns to on state

______________________________________________________________________

### Feature: TTY Backend Coordinator (F9)

**Brief Reference**: TTY Backend integration
**Phase**: 3
**Complexity**: M
**Dependencies**: F7, F8

**Acceptance Criteria**:

- GIVEN TTY environment detected
  WHEN `set_power(Off)` is called
  THEN daemon is spawned via F8 logic

- GIVEN TTY environment detected
  WHEN `set_power(On)` is called
  THEN daemon is signaled via F8 logic

- GIVEN TTY environment detected
  WHEN `get_power()` is called
  THEN daemon running state is checked
  AND correct PowerState is returned

______________________________________________________________________

### Feature: Main Dispatch Logic (F10)

**Brief Reference**: Entry point
**Phase**: 4
**Complexity**: S
**Dependencies**: F1, F4, F5, F6, F9

**Acceptance Criteria**:

- GIVEN CLI args are parsed
  AND environment is detected
  WHEN command is On/Off/Status
  THEN correct backend is invoked
  AND result is handled properly

______________________________________________________________________

### Feature: Systemd Service Unit (F11)

**Brief Reference**: Systemd Integration
**Phase**: 4
**Complexity**: S
**Dependencies**: F10

**Acceptance Criteria**:

- GIVEN `dpms.service` is enabled
  AND system boots to multi-user.target
  WHEN boot sequence completes
  THEN dpms daemon is running
  AND display is off

- GIVEN `dpms.service` is active
  WHEN user runs `systemctl stop dpms`
  THEN display turns on
  AND daemon exits cleanly

## Implementation Phases

### Phase 1: Foundation

**Goal**: Establish project structure, CLI, error handling, environment detection

**Features**: F1, F2, F3, F4, F5

**Done Criteria**:

- `dpms --help` shows usage
- `dpms status` detects environment and prints detection result (stub)
- Invalid commands exit with code 2
- Error messages go to stderr

______________________________________________________________________

### Phase 2: Wayland Backend

**Goal**: Complete Wayland power control functionality

**Features**: F6

**Done Criteria**:

- `dpms off` turns off display in Wayland session
- `dpms on` turns on display in Wayland session
- `dpms status` reports correct state
- `dpms status --json` returns valid JSON
- Works without root privileges

______________________________________________________________________

### Phase 3: TTY Backend

**Goal**: Complete TTY power control with daemon

**Features**: F7, F8, F9

**Done Criteria**:

- `dpms off` turns off display on TTY
- `dpms on` restores display on TTY
- `dpms status` reports correct state based on daemon
- Double off/on are idempotent
- Stale PID files are handled
- Works without root privileges (with logind session)

______________________________________________________________________

### Phase 4: Integration & Systemd

**Goal**: End-to-end integration, systemd service

**Features**: F10, F11

**Done Criteria**:

- All 17 acceptance criteria pass
- Systemd service works at boot
- `systemctl stop dpms` restores display
- Manual end-to-end testing complete

## Test Strategy

### Unit Tests

| Test | Feature | Input | Expected Output |
|------|---------|-------|-----------------|
| `test_parse_command_on` | F1 | `["dpms", "on"]` | `Command::On` |
| `test_parse_command_off` | F1 | `["dpms", "off"]` | `Command::Off` |
| `test_parse_command_status` | F1 | `["dpms", "status"]` | `Command::Status { json: false }` |
| `test_parse_command_status_json` | F1 | `["dpms", "status", "--json"]` | `Command::Status { json: true }` |
| `test_parse_invalid_command` | F1 | `["dpms", "foo"]` | Error, exit code 2 |
| `test_format_status_text_on` | F3 | `PowerState::On, json=false` | `"Display: On\n"` |
| `test_format_status_text_off` | F3 | `PowerState::Off, json=false` | `"Display: Off\n"` |
| `test_format_status_json_on` | F3 | `PowerState::On, json=true` | `{"power":"on"}` |
| `test_format_status_json_off` | F3 | `PowerState::Off, json=true` | `{"power":"off"}` |
| `test_detect_wayland_env_set` | F4 | `WAYLAND_DISPLAY=wayland-0` | `Environment::Wayland` |
| `test_detect_tty_no_wayland` | F4 | `WAYLAND_DISPLAY` unset | `Environment::Tty` |
| `test_pid_file_path` | F8 | `XDG_RUNTIME_DIR=/run/user/1000` | `/run/user/1000/dpms.pid` |
| `test_is_daemon_running_no_file` | F8 | PID file doesn't exist | `false` |
| `test_is_daemon_running_stale` | F8 | PID file exists, process dead | `false`, file removed |

### Integration Tests (Environment-Dependent)

| Test | Acceptance Criteria | Environment |
|------|---------------------|-------------|
| `test_wayland_off_on_cycle` | AC-4, AC-5 | Wayland session |
| `test_wayland_status` | AC-6 | Wayland session |
| `test_tty_off_on_cycle` | AC-1, AC-2 | TTY |
| `test_tty_status` | AC-3 | TTY |
| `test_double_off` | AC-13 | Both |
| `test_double_on` | AC-14 | Both |

### Manual Test Checklist

- [ ] **TTY-001**: Boot to TTY, run `dpms off`, verify display off
- [ ] **TTY-002**: Run `dpms on`, verify display restored
- [ ] **TTY-003**: Run `dpms status`, verify "Display: Off" when off
- [ ] **TTY-004**: Kill daemon with `kill -9`, verify display comes back
- [ ] **WAY-001**: In niri/sway, run `dpms off`, verify display off
- [ ] **WAY-002**: Move mouse, verify display stays off
- [ ] **WAY-003**: Run `dpms on`, verify display on
- [ ] **SYS-001**: Enable service, reboot, verify display off at boot
- [ ] **SYS-002**: Run `systemctl stop dpms`, verify display on
- [ ] **PERM-001**: Run as non-root user on TTY, verify success
- [ ] **PERM-002**: Run as non-root user in Wayland, verify success

### Coverage Targets

| Module | Target | Rationale |
|--------|--------|-----------|
| `cli.rs` | 100% | Simple arg parsing, fully testable |
| `output.rs` | 100% | Pure formatting functions |
| `error.rs` | 100% | Error type definitions |
| `detect.rs` | 80% | Some paths need real environment |
| `backend/wayland.rs` | 50% | Protocol logic needs real compositor |
| `backend/tty/daemon.rs` | 60% | Fork/signal logic partially testable |
| `backend/tty/drm.rs` | 40% | DRM operations need real hardware |

## Pseudocode

### Environment Detection

```
FUNCTION detect_environment() -> Result<Environment, Error>
    // Try Wayland first
    IF env_var("WAYLAND_DISPLAY") is set THEN
        display_name = get_env("WAYLAND_DISPLAY")
        TRY
            connection = wayland_connect(display_name)
            connection.close()
            RETURN Environment::Wayland { display: display_name }
        CATCH connection_error
            log_debug("Wayland display set but connection failed")
        END TRY
    END IF

    // Try TTY/libseat
    TRY
        seat = libseat_open_seat()
        seat.close()
        RETURN Environment::Tty
    CATCH seat_error
        RETURN Error::UnsupportedEnvironment(
            "Neither Wayland compositor nor logind session available"
        )
    END TRY
END FUNCTION
```

### TTY Daemon: Power Off

```
FUNCTION handle_tty_off() -> Result<(), Error>
    pid_path = get_pid_file_path()

    IF file_exists(pid_path) THEN
        pid = read_pid_file(pid_path)
        IF process_is_running(pid) AND process_is_dpms(pid) THEN
            eprintln("Display already off")
            RETURN Ok(())
        ELSE
            remove_file(pid_path)  // Stale
        END IF
    END IF

    match fork() {
        Parent(child_pid) =>
            sleep(100ms)
            IF file_exists(pid_path) THEN
                RETURN Ok(())
            ELSE
                RETURN Error::DaemonStartFailed
            END IF

        Child =>
            setsid()
            run_daemon_main()
    }
END FUNCTION

FUNCTION run_daemon_main()
    seat = libseat_open_seat(callbacks)
    drm_path = find_drm_device()
    drm_fd = seat.open_device(drm_path)

    connector = find_first_connected_connector(drm_fd)
    crtc = get_crtc_for_connector(drm_fd, connector)

    // Disable display
    atomic_request = create_atomic_request(drm_fd)
    atomic_request.add_property(crtc, "ACTIVE", 0)
    atomic_request.commit()

    write_pid_file(get_pid_file_path(), getpid())
    install_signal_handler(SIGTERM, handle_sigterm)

    // Wait loop
    WHILE NOT received_sigterm DO
        seat.dispatch()
        sleep(100ms)
    END WHILE

    // Restore display
    atomic_request = create_atomic_request(drm_fd)
    atomic_request.add_property(crtc, "ACTIVE", 1)
    atomic_request.commit()

    remove_file(get_pid_file_path())
    seat.close_device(drm_fd)
    seat.close()
    exit(0)
END FUNCTION
```

### TTY Daemon: Power On

```
FUNCTION handle_tty_on() -> Result<(), Error>
    pid_path = get_pid_file_path()

    IF NOT file_exists(pid_path) THEN
        eprintln("Display already on")
        RETURN Ok(())
    END IF

    pid = read_pid_file(pid_path)

    IF NOT process_is_running(pid) THEN
        remove_file(pid_path)
        eprintln("Display already on")
        RETURN Ok(())
    END IF

    kill(pid, SIGTERM)

    FOR i IN 0..50 DO  // 5 second timeout
        IF NOT process_is_running(pid) THEN
            RETURN Ok(())
        END IF
        sleep(100ms)
    END FOR

    RETURN Error::DaemonStopTimeout
END FUNCTION
```

### TTY Status

```
FUNCTION handle_tty_status() -> Result<PowerState, Error>
    pid_path = get_pid_file_path()

    IF file_exists(pid_path) THEN
        pid = read_pid_file(pid_path)
        IF process_is_running(pid) AND process_is_dpms(pid) THEN
            RETURN PowerState::Off
        ELSE
            remove_file(pid_path)
            RETURN PowerState::On
        END IF
    ELSE
        RETURN PowerState::On
    END IF
END FUNCTION
```

### Wayland Backend

```
FUNCTION handle_wayland_power(state: PowerState, display: String) -> Result<(), Error>
    connection = wayland_connect(display)

    power_manager = bind_global(zwlr_output_power_manager_v1)
    output = bind_global(wl_output)

    IF power_manager is None THEN
        RETURN Error::ProtocolNotSupported
    END IF

    power_control = power_manager.get_output_power(output)

    mode = IF state == PowerState::On THEN Mode::On ELSE Mode::Off
    power_control.set_mode(mode)

    connection.roundtrip()
    RETURN Ok(())
END FUNCTION

FUNCTION handle_wayland_status(display: String) -> Result<PowerState, Error>
    connection = wayland_connect(display)

    power_manager = bind_global(zwlr_output_power_manager_v1)
    output = bind_global(wl_output)
    power_control = power_manager.get_output_power(output)

    // Mode event received during roundtrip
    connection.roundtrip()

    RETURN IF current_mode == Mode::On THEN PowerState::On ELSE PowerState::Off
END FUNCTION
```

### Main Entry Point

```
FUNCTION main() -> ExitCode
    args = parse_args()

    match args {
        Error(e) =>
            eprintln(e)
            RETURN ExitCode::Usage

        command =>
            match detect_environment() {
                Error(e) =>
                    eprintln("Error: {}", e)
                    RETURN ExitCode::Error

                Environment::Wayland { display } =>
                    result = handle_wayland(command, display)

                Environment::Tty =>
                    result = handle_tty(command)
            }

            match result {
                Ok(()) => RETURN ExitCode::Success
                Ok(status) =>
                    print_status(status, args.json)
                    RETURN ExitCode::Success
                Err(e) =>
                    eprintln("Error: {}", e)
                    RETURN ExitCode::Error
            }
    }
END FUNCTION
```

## Systemd Service Unit

```ini
# /etc/systemd/system/dpms.service
[Unit]
Description=Monitor Power Control
Documentation=man:dpms(1)
After=systemd-logind.service
Requires=systemd-logind.service

[Service]
Type=forking
ExecStart=/usr/local/bin/dpms off
ExecStop=/usr/local/bin/dpms on
PIDFile=/run/user/%U/dpms.pid
RemainAfterExit=yes
Restart=no

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
```
