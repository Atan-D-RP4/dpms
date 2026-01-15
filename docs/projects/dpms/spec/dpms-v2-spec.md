# dpms v2 Technical Specification: Multi-Display & UX Enhancements

**Brief**: Discussion analysis of dpms vs wlout (2025-12-21)
**Base Spec**: [dpms-spec.md](./dpms-spec.md)
**Created**: 2025-12-21
**Status**: Draft
**Compliance Score**: 100%

## Executive Summary

This specification extends dpms with multi-display selection, toggle functionality, display listing, and shell completions. These enhancements are inspired by patterns from wlout while maintaining dpms's focused purpose as a power management tool. The goal is to make dpms production-ready for multi-monitor setups without scope creep into display configuration territory.

## Design Principles

1. **Stay focused**: Power management only (no resolution, position, mirroring)
2. **Backwards compatible**: Existing CLI continues to work unchanged
3. **Minimal dependencies**: Avoid adding serde; keep binary small
4. **Consistent patterns**: Follow existing code style and architecture

## Data Contracts

### New Inputs

| Source | Data | Type | Notes |
|--------|------|------|-------|
| CLI args | `<DISPLAY>` | Optional String | Display name (e.g., `DP-1`, `eDP-1`) |
| CLI args | `--all` flag | Boolean | Target all displays explicitly |
| CLI args | `completion <SHELL>` | Subcommand | Generate shell completions |
| Wayland | Multiple `wl_output` | Protocol objects | All connected outputs |
| Wayland | Output name/description | Protocol events | From `wl_output` events |

### New Outputs

| Consumer | Data | Type | Notes |
|----------|------|------|-------|
| User (stdout) | Display list | Multi-line text | `<name>: On\|Off` per line |
| User (stdout) | Display list JSON | JSON array | `[{"name":"DP-1","power":"on"},...]` |
| User (stdout) | Shell completion | Script | bash/zsh/fish/elvish/powershell |

### Interface Constraints (Additions)

1. **New CLI is additive**: All v1 commands work unchanged
2. **Default behavior preserved**: No args = operate on all displays (backwards compat)
3. **JSON schema extended**: Array format for multi-display, object for single
4. **Exit codes unchanged**: 0=success, 1=error, 2=usage

### Scope Classification

**BROWNFIELD** - Extending existing tool; must preserve v1 CLI contract.

## Technical Design

### Extended Architecture

```
+------------------------------------------------------------------+
|                          dpms CLI v2                              |
|  +--------------------------------------------------------------+
|  |                    Argument Parser                            |
|  |  on|off|toggle|status|list [DISPLAY] [--all] [--json]        |
|  |  completion <SHELL>                                           |
|  +--------------------------------------------------------------+
|                              |
|  +--------------------------------------------------------------+
|  |              Display Selection Layer (NEW)                    |
|  |         Resolve target: specific | all | default              |
|  +--------------------------------------------------------------+
|                              |
|  +--------------------------------------------------------------+
|  |                Environment Detector                           |
|  |         Check WAYLAND_DISPLAY -> try connect                  |
|  +--------------------------------------------------------------+
|                    |                    |
|           +--------v-------+  +---------v--------+
|           | Wayland Backend|  |   TTY Backend    |
|           | (multi-output) |  | (multi-output*)  |
|           +----------------+  +------------------+
+------------------------------------------------------------------+

* TTY multi-output support is Phase 2; initially targets primary display
```

### Extended Module Structure

```
src/
├── main.rs           # Entry point, dispatch
├── cli.rs            # CLI definitions (extended)
├── env.rs            # Environment detection (unchanged)
├── backend.rs        # Backend trait (extended)
├── wayland.rs        # Wayland backend (extended for multi-output)
├── tty.rs            # TTY backend coordinator
├── daemon.rs         # Daemon lifecycle (unchanged)
├── drm_ops.rs        # DRM operations (unchanged initially)
├── output.rs         # Output formatting (extended)
├── display.rs        # NEW: Display selection & matching
└── error.rs          # Error types (extended)
```

### Extended Data Models

```rust
/// Display target selection
#[derive(Debug, Clone)]
pub enum DisplayTarget {
    /// Specific display by name
    Named(String),
    /// All connected displays
    All,
    /// Default behavior (all displays, for backwards compat)
    Default,
}

/// Extended CLI command
#[derive(Debug, Clone)]
pub enum Command {
    On { target: DisplayTarget },
    Off { target: DisplayTarget },
    Toggle { target: DisplayTarget },
    Status { target: DisplayTarget, json: bool },
    List { json: bool, verbose: bool },
    Completion { shell: Shell },
    /// Internal: daemon mode
    DaemonInternal,
}

/// Shell type for completions
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Elvish,
    PowerShell,
}

/// Display info for listing/status
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    pub name: String,
    pub power: PowerState,
    pub description: Option<String>,  // For verbose mode
    pub make: Option<String>,         // For verbose mode
    pub model: Option<String>,        // For verbose mode
}

/// Extended PowerBackend trait
pub trait PowerBackend {
    /// Set power state for specific display(s)
    fn set_power(&mut self, target: &DisplayTarget, state: PowerState) -> Result<(), Error>;
    
    /// Get power state for specific display(s)
    fn get_power(&self, target: &DisplayTarget) -> Result<Vec<DisplayInfo>, Error>;
    
    /// List all connected displays
    fn list_displays(&self) -> Result<Vec<DisplayInfo>, Error>;
}
```

### Display Selection Logic

```
FUNCTION resolve_display_target(arg: Option<String>, all_flag: bool) -> DisplayTarget
    IF all_flag THEN
        RETURN DisplayTarget::All
    ELSE IF arg is Some(name) THEN
        RETURN DisplayTarget::Named(name)
    ELSE
        RETURN DisplayTarget::Default  // Backwards compat: all displays
    END IF
END FUNCTION

FUNCTION find_display_by_name(displays: Vec<DisplayInfo>, name: &str) -> Result<DisplayInfo>
    // Exact match first
    IF displays.any(d => d.name == name) THEN
        RETURN Ok(exact_match)
    END IF
    
    // Partial match (prefix)
    matches = displays.filter(d => d.name.starts_with(name))
    IF matches.len() == 1 THEN
        RETURN Ok(matches[0])
    ELSE IF matches.len() > 1 THEN
        RETURN Err(AmbiguousDisplay { name, candidates: matches })
    ELSE
        RETURN Err(DisplayNotFound { name, available: displays })
    END IF
END FUNCTION
```

## Features

### Feature: Extended CLI Parser (F12)

**Brief Reference**: CLI enhancement discussion
**Phase**: 1
**Complexity**: M
**Dependencies**: F1 (existing CLI)

**Acceptance Criteria**:

- GIVEN user runs `dpms on`
  WHEN command is parsed
  THEN `Command::On { target: DisplayTarget::Default }` is returned
  AND backwards compatibility is maintained

- GIVEN user runs `dpms off DP-1`
  WHEN command is parsed
  THEN `Command::Off { target: DisplayTarget::Named("DP-1") }` is returned

- GIVEN user runs `dpms off --all`
  WHEN command is parsed
  THEN `Command::Off { target: DisplayTarget::All }` is returned

- GIVEN user runs `dpms toggle`
  WHEN command is parsed
  THEN `Command::Toggle { target: DisplayTarget::Default }` is returned

- GIVEN user runs `dpms toggle eDP-1`
  WHEN command is parsed
  THEN `Command::Toggle { target: DisplayTarget::Named("eDP-1") }` is returned

- GIVEN user runs `dpms list`
  WHEN command is parsed
  THEN `Command::List { json: false, verbose: false }` is returned

- GIVEN user runs `dpms list --json`
  WHEN command is parsed
  THEN `Command::List { json: true, verbose: false }` is returned

- GIVEN user runs `dpms list --verbose`
  WHEN command is parsed
  THEN `Command::List { json: false, verbose: true }` is returned

- GIVEN user runs `dpms status DP-1`
  WHEN command is parsed
  THEN `Command::Status { target: DisplayTarget::Named("DP-1"), json: false }` is returned

- GIVEN user runs `dpms completion bash`
  WHEN command is parsed
  THEN `Command::Completion { shell: Shell::Bash }` is returned

______________________________________________________________________

### Feature: Display Selection Module (F13)

**Brief Reference**: Multi-display selection discussion
**Phase**: 1
**Complexity**: S
**Dependencies**: None

**Acceptance Criteria**:

- GIVEN display name "DP-1" and available displays ["DP-1", "eDP-1"]
  WHEN display is resolved
  THEN exact match "DP-1" is returned

- GIVEN display name "DP" and available displays ["DP-1", "eDP-1"]
  WHEN display is resolved
  THEN partial match "DP-1" is returned

- GIVEN display name "DP" and available displays ["DP-1", "DP-2"]
  WHEN display is resolved
  THEN `AmbiguousDisplay` error is returned
  AND error message lists candidates

- GIVEN display name "HDMI-1" and available displays ["DP-1", "eDP-1"]
  WHEN display is resolved
  THEN `DisplayNotFound` error is returned
  AND error message lists available displays

______________________________________________________________________

### Feature: Toggle Command (F14)

**Brief Reference**: Toggle command discussion
**Phase**: 1
**Complexity**: S
**Dependencies**: F12, F5 (backend trait)

**Acceptance Criteria**:

- GIVEN display is currently on
  WHEN user runs `dpms toggle`
  THEN display is turned off
  AND command exits with code 0

- GIVEN display is currently off
  WHEN user runs `dpms toggle`
  THEN display is turned on
  AND command exits with code 0

- GIVEN display "DP-1" is on and "eDP-1" is off
  WHEN user runs `dpms toggle DP-1`
  THEN only "DP-1" is turned off
  AND "eDP-1" remains off
  AND command exits with code 0

______________________________________________________________________

### Feature: List Command (F15)

**Brief Reference**: List displays discussion
**Phase**: 1
**Complexity**: M
**Dependencies**: F12, extended backend

**Acceptance Criteria**:

- GIVEN two displays connected: DP-1 (on), eDP-1 (off)
  WHEN user runs `dpms list`
  THEN stdout shows:
  ```
  DP-1: On
  eDP-1: Off
  ```
  AND command exits with code 0

- GIVEN two displays connected: DP-1 (on), eDP-1 (off)
  WHEN user runs `dpms list --json`
  THEN stdout shows valid JSON:
  ```json
  [{"name":"DP-1","power":"on"},{"name":"eDP-1","power":"off"}]
  ```
  AND command exits with code 0

- GIVEN display DP-1 with make "Dell" model "U2720Q"
  WHEN user runs `dpms list --verbose`
  THEN stdout shows:
  ```
  DP-1: On (Dell U2720Q)
  ```

- GIVEN no displays connected
  WHEN user runs `dpms list`
  THEN stderr shows "No displays found"
  AND command exits with code 1

______________________________________________________________________

### Feature: Shell Completions (F16)

**Brief Reference**: Shell completions discussion
**Phase**: 1
**Complexity**: S
**Dependencies**: F12

**Acceptance Criteria**:

- GIVEN user runs `dpms completion bash`
  WHEN completion script is generated
  THEN valid bash completion script is written to stdout
  AND command exits with code 0

- GIVEN user runs `dpms completion zsh`
  WHEN completion script is generated
  THEN valid zsh completion script is written to stdout
  AND command exits with code 0

- GIVEN user runs `dpms completion fish`
  WHEN completion script is generated
  THEN valid fish completion script is written to stdout
  AND command exits with code 0

- GIVEN user runs `dpms completion invalid`
  WHEN command is parsed
  THEN clap error is shown
  AND command exits with code 2

______________________________________________________________________

### Feature: Extended Wayland Backend (F17)

**Brief Reference**: Multi-display Wayland support
**Phase**: 2
**Complexity**: L
**Dependencies**: F6, F13, F15

**Acceptance Criteria**:

- GIVEN Wayland session with multiple outputs
  WHEN backend discovers outputs
  THEN all outputs are collected with names and power states

- GIVEN DisplayTarget::Named("DP-1")
  WHEN set_power(Off) is called
  THEN only DP-1 receives set_mode(Off)
  AND other displays are unaffected

- GIVEN DisplayTarget::All
  WHEN set_power(Off) is called
  THEN all displays receive set_mode(Off)

- GIVEN DisplayTarget::Default (no arg provided)
  WHEN set_power(Off) is called
  THEN all displays receive set_mode(Off)
  AND behavior matches v1 (backwards compat)

- GIVEN display "DP-1" does not exist
  WHEN set_power is called with Named("DP-1")
  THEN DisplayNotFound error is returned
  AND available displays are listed in error message

______________________________________________________________________

### Feature: Extended Output Formatting (F18)

**Brief Reference**: Multi-display output formatting
**Phase**: 1
**Complexity**: M
**Dependencies**: F3, F15

**Acceptance Criteria**:

- GIVEN single DisplayInfo with power On
  WHEN formatted as text
  THEN output is `"DP-1: On\n"`

- GIVEN single DisplayInfo with power Off
  WHEN formatted as text
  THEN output is `"DP-1: Off\n"`

- GIVEN multiple DisplayInfo items
  WHEN formatted as text
  THEN each display is on its own line

- GIVEN single DisplayInfo with power On
  WHEN formatted as JSON (single display query)
  THEN output is `{"name":"DP-1","power":"on"}`

- GIVEN multiple DisplayInfo items
  WHEN formatted as JSON (list or multi-display)
  THEN output is JSON array `[{"name":"DP-1","power":"on"},...]`

- GIVEN DisplayInfo with make/model and verbose=true
  WHEN formatted as text
  THEN output includes make/model: `"DP-1: On (Dell U2720Q)\n"`

______________________________________________________________________

### Feature: Extended Error Types (F19)

**Brief Reference**: Error handling for new features
**Phase**: 1
**Complexity**: S
**Dependencies**: F2

**Acceptance Criteria**:

- GIVEN DisplayNotFound error with name "HDMI-1" and available ["DP-1", "eDP-1"]
  WHEN error is displayed
  THEN message is: `Display 'HDMI-1' not found. Available: DP-1, eDP-1`
  AND exit code is 1

- GIVEN AmbiguousDisplay error with name "DP" and candidates ["DP-1", "DP-2"]
  WHEN error is displayed
  THEN message is: `Display 'DP' is ambiguous. Did you mean: DP-1, DP-2?`
  AND exit code is 1

______________________________________________________________________

### Feature: Extended TTY Backend (F20)

**Brief Reference**: Multi-display TTY support (limited)
**Phase**: 3
**Complexity**: M
**Dependencies**: F9, F17

**Acceptance Criteria**:

- GIVEN TTY environment with multiple displays
  WHEN list_displays() is called
  THEN all connected displays are returned via DRM enumeration

- GIVEN DisplayTarget::Named("DP-1") on TTY
  WHEN set_power is called
  THEN operation targets specific CRTC for that connector
  
- GIVEN DisplayTarget::All on TTY
  WHEN set_power(Off) is called
  THEN all CRTCs are disabled

**Note**: TTY multi-display requires DRM enumeration of all connectors/CRTCs. Initial implementation may limit to primary display with clear error message for unsupported operations.

______________________________________________________________________

## Implementation Phases

### Phase 1: CLI & Foundation

**Goal**: Extended CLI, display selection, list command, shell completions

**Features**: F12, F13, F14, F15, F16, F18, F19

**Done Criteria**:

- `dpms on/off/toggle [DISPLAY]` parses correctly
- `dpms list [--json] [--verbose]` works (stub data in non-Wayland)
- `dpms completion <SHELL>` generates valid scripts
- `dpms toggle` flips state correctly
- All new CLI tests pass
- Existing tests still pass (backwards compat)

______________________________________________________________________

### Phase 2: Wayland Multi-Display

**Goal**: Full multi-display support in Wayland backend

**Features**: F17

**Done Criteria**:

- `dpms off DP-1` turns off only DP-1
- `dpms off --all` turns off all displays
- `dpms off` (no args) turns off all displays (backwards compat)
- `dpms list` shows all displays with correct power states
- `dpms status DP-1` shows single display status
- Partial name matching works (`DP` matches `DP-1`)
- Ambiguous names produce helpful error

______________________________________________________________________

### Phase 3: TTY Multi-Display (Optional)

**Goal**: Multi-display support in TTY backend

**Features**: F20

**Done Criteria**:

- `dpms list` shows all displays via DRM enumeration
- `dpms off DP-1` works if DRM supports per-connector control
- Clear error message if multi-display not supported on hardware

**Note**: This phase is lower priority; TTY users typically have single display.

______________________________________________________________________

## Test Strategy

### New Unit Tests

| Test | Feature | Input | Expected Output |
|------|---------|-------|-----------------|
| `test_parse_on_no_args` | F12 | `["dpms", "on"]` | `Command::On { target: Default }` |
| `test_parse_on_with_display` | F12 | `["dpms", "on", "DP-1"]` | `Command::On { target: Named("DP-1") }` |
| `test_parse_off_all` | F12 | `["dpms", "off", "--all"]` | `Command::Off { target: All }` |
| `test_parse_toggle` | F12 | `["dpms", "toggle"]` | `Command::Toggle { target: Default }` |
| `test_parse_toggle_display` | F12 | `["dpms", "toggle", "eDP-1"]` | `Command::Toggle { target: Named("eDP-1") }` |
| `test_parse_list` | F12 | `["dpms", "list"]` | `Command::List { json: false, verbose: false }` |
| `test_parse_list_json` | F12 | `["dpms", "list", "--json"]` | `Command::List { json: true, verbose: false }` |
| `test_parse_list_verbose` | F12 | `["dpms", "list", "-v"]` | `Command::List { json: false, verbose: true }` |
| `test_parse_status_display` | F12 | `["dpms", "status", "DP-1"]` | `Command::Status { target: Named("DP-1"), ... }` |
| `test_parse_completion_bash` | F12 | `["dpms", "completion", "bash"]` | `Command::Completion { shell: Bash }` |
| `test_parse_completion_zsh` | F12 | `["dpms", "completion", "zsh"]` | `Command::Completion { shell: Zsh }` |
| `test_resolve_exact_match` | F13 | `"DP-1"`, `["DP-1", "eDP-1"]` | `Ok("DP-1")` |
| `test_resolve_partial_match` | F13 | `"DP"`, `["DP-1", "eDP-1"]` | `Ok("DP-1")` |
| `test_resolve_ambiguous` | F13 | `"DP"`, `["DP-1", "DP-2"]` | `Err(AmbiguousDisplay)` |
| `test_resolve_not_found` | F13 | `"HDMI"`, `["DP-1"]` | `Err(DisplayNotFound)` |
| `test_format_list_text` | F18 | `[("DP-1", On), ("eDP-1", Off)]` | `"DP-1: On\neDP-1: Off\n"` |
| `test_format_list_json` | F18 | `[("DP-1", On)]` | `[{"name":"DP-1","power":"on"}]` |
| `test_format_single_json` | F18 | `("DP-1", On)` | `{"name":"DP-1","power":"on"}` |
| `test_format_verbose` | F18 | `("DP-1", On, "Dell", "U2720Q")` | `"DP-1: On (Dell U2720Q)\n"` |
| `test_error_display_not_found` | F19 | `DisplayNotFound("X", ["A","B"])` | Contains "not found" and "A, B" |
| `test_error_ambiguous` | F19 | `AmbiguousDisplay("D", ["D1","D2"])` | Contains "ambiguous" and "D1, D2" |

### Integration Tests

| Test | Features | Environment | Description |
|------|----------|-------------|-------------|
| `test_list_wayland` | F15, F17 | Wayland | Verify list shows all outputs |
| `test_off_single_display` | F17 | Wayland | Turn off one display, verify others unaffected |
| `test_toggle_single` | F14, F17 | Wayland | Toggle one display |
| `test_backwards_compat_off` | F17 | Wayland | `dpms off` without args works as before |
| `test_completion_generates` | F16 | Any | Verify completion scripts are non-empty |

### Manual Test Checklist

- [ ] **CLI-001**: `dpms on` works (backwards compat)
- [ ] **CLI-002**: `dpms off` works (backwards compat)
- [ ] **CLI-003**: `dpms status` works (backwards compat)
- [ ] **CLI-004**: `dpms status --json` works (backwards compat)
- [ ] **MULTI-001**: `dpms list` shows all displays
- [ ] **MULTI-002**: `dpms list --json` produces valid JSON array
- [ ] **MULTI-003**: `dpms off DP-1` only affects DP-1
- [ ] **MULTI-004**: `dpms off --all` affects all displays
- [ ] **MULTI-005**: `dpms toggle DP-1` toggles only DP-1
- [ ] **MULTI-006**: `dpms status DP-1` shows only DP-1
- [ ] **MULTI-007**: `dpms off DP` matches `DP-1` (partial)
- [ ] **MULTI-008**: `dpms off INVALID` shows helpful error
- [ ] **COMP-001**: `dpms completion bash | bash` doesn't error
- [ ] **COMP-002**: `dpms completion zsh` generates valid script
- [ ] **COMP-003**: `dpms completion fish` generates valid script

### Coverage Targets

| Module | Target | Notes |
|--------|--------|-------|
| `cli.rs` | 100% | Extended parsing fully testable |
| `display.rs` | 100% | Pure matching logic |
| `output.rs` | 100% | Extended formatting |
| `error.rs` | 100% | New error variants |
| `wayland.rs` | 60% | Multi-output logic partially testable |

## Pseudocode

### Extended CLI Parsing

```
STRUCT Cli:
    command: Commands
    
ENUM Commands:
    On:
        display: Option<String>
        all: bool
    Off:
        display: Option<String>
        all: bool
    Toggle:
        display: Option<String>
        all: bool
    Status:
        display: Option<String>
        json: bool
    List:
        json: bool
        verbose: bool
    Completion:
        shell: Shell
    DaemonInternal  // hidden

FUNCTION parse() -> Command
    cli = Cli::parse()
    MATCH cli.command:
        Commands::On { display, all } =>
            Command::On { target: resolve_target(display, all) }
        Commands::Off { display, all } =>
            Command::Off { target: resolve_target(display, all) }
        Commands::Toggle { display, all } =>
            Command::Toggle { target: resolve_target(display, all) }
        Commands::Status { display, json } =>
            target = IF display.is_some() THEN Named(display) ELSE Default
            Command::Status { target, json }
        Commands::List { json, verbose } =>
            Command::List { json, verbose }
        Commands::Completion { shell } =>
            Command::Completion { shell }
        Commands::DaemonInternal =>
            Command::DaemonInternal
END FUNCTION

FUNCTION resolve_target(display: Option<String>, all: bool) -> DisplayTarget
    IF all THEN
        DisplayTarget::All
    ELSE IF display.is_some() THEN
        DisplayTarget::Named(display.unwrap())
    ELSE
        DisplayTarget::Default
    END IF
END FUNCTION
```

### Toggle Implementation

```
FUNCTION handle_toggle(backend: &mut dyn PowerBackend, target: DisplayTarget) -> Result<()>
    displays = backend.get_power(&target)?
    
    FOR display IN displays:
        new_state = IF display.power == On THEN Off ELSE On
        backend.set_power(&DisplayTarget::Named(display.name), new_state)?
    END FOR
    
    RETURN Ok(())
END FUNCTION
```

### List Implementation

```
FUNCTION handle_list(backend: &dyn PowerBackend, json: bool, verbose: bool) -> Result<()>
    displays = backend.list_displays()?
    
    IF displays.is_empty() THEN
        RETURN Err(Error::NoDisplayFound)
    END IF
    
    IF json THEN
        print(format_displays_json(&displays))
    ELSE
        FOR display IN displays:
            IF verbose AND display.make.is_some() THEN
                println!("{}: {} ({} {})", 
                    display.name, 
                    display.power,
                    display.make.unwrap_or_default(),
                    display.model.unwrap_or_default())
            ELSE
                println!("{}: {}", display.name, display.power)
            END IF
        END FOR
    END IF
    
    RETURN Ok(())
END FUNCTION
```

### Extended Wayland Backend

```
STRUCT WaylandBackend:
    connection: Connection
    power_manager: ZwlrOutputPowerManagerV1
    outputs: HashMap<ObjectId, OutputInfo>  // NEW: track all outputs

STRUCT OutputInfo:
    proxy: WlOutput
    name: Option<String>
    description: Option<String>
    make: Option<String>
    model: Option<String>
    power_state: Option<PowerState>

IMPL PowerBackend FOR WaylandBackend:
    FUNCTION set_power(&mut self, target: &DisplayTarget, state: PowerState) -> Result<()>
        targets = self.resolve_targets(target)?
        
        FOR output_info IN targets:
            power_control = self.power_manager.get_output_power(&output_info.proxy)
            mode = IF state == On THEN Mode::On ELSE Mode::Off
            power_control.set_mode(mode)
            power_control.destroy()
        END FOR
        
        self.roundtrip()?
        RETURN Ok(())
    END FUNCTION
    
    FUNCTION get_power(&self, target: &DisplayTarget) -> Result<Vec<DisplayInfo>>
        targets = self.resolve_targets(target)?
        result = Vec::new()
        
        FOR output_info IN targets:
            // Query power state for each output
            power_control = self.power_manager.get_output_power(&output_info.proxy)
            self.roundtrip()  // Receive mode event
            
            result.push(DisplayInfo {
                name: output_info.name.clone(),
                power: output_info.power_state.unwrap_or(On),
                description: output_info.description.clone(),
                make: output_info.make.clone(),
                model: output_info.model.clone(),
            })
            
            power_control.destroy()
        END FOR
        
        RETURN Ok(result)
    END FUNCTION
    
    FUNCTION list_displays(&self) -> Result<Vec<DisplayInfo>>
        self.get_power(&DisplayTarget::All)
    END FUNCTION
    
    FUNCTION resolve_targets(&self, target: &DisplayTarget) -> Result<Vec<&OutputInfo>>
        MATCH target:
            DisplayTarget::All | DisplayTarget::Default =>
                RETURN Ok(self.outputs.values().collect())
            
            DisplayTarget::Named(name) =>
                // Exact match
                IF let Some(output) = self.outputs.values().find(|o| o.name == Some(name)) THEN
                    RETURN Ok(vec![output])
                END IF
                
                // Partial match
                matches = self.outputs.values()
                    .filter(|o| o.name.as_ref().map(|n| n.starts_with(name)).unwrap_or(false))
                    .collect()
                
                IF matches.len() == 1 THEN
                    RETURN Ok(matches)
                ELSE IF matches.len() > 1 THEN
                    names = matches.iter().filter_map(|o| o.name.clone()).collect()
                    RETURN Err(Error::AmbiguousDisplay { name, candidates: names })
                ELSE
                    available = self.outputs.values().filter_map(|o| o.name.clone()).collect()
                    RETURN Err(Error::DisplayNotFound { name, available })
                END IF
        END MATCH
    END FUNCTION
END IMPL
```

### Shell Completion Generation

```
FUNCTION handle_completion(shell: Shell) -> Result<()>
    cmd = Cli::command()
    
    MATCH shell:
        Shell::Bash => generate(shells::Bash, &mut cmd, "dpms", &mut stdout())
        Shell::Zsh => generate(shells::Zsh, &mut cmd, "dpms", &mut stdout())
        Shell::Fish => generate(shells::Fish, &mut cmd, "dpms", &mut stdout())
        Shell::Elvish => generate(shells::Elvish, &mut cmd, "dpms", &mut stdout())
        Shell::PowerShell => generate(shells::PowerShell, &mut cmd, "dpms", &mut stdout())
    END MATCH
    
    RETURN Ok(())
END FUNCTION
```

### Extended Output Formatting

```
FUNCTION format_display_text(display: &DisplayInfo, verbose: bool) -> String
    power_str = IF display.power == On THEN "On" ELSE "Off"
    
    IF verbose AND (display.make.is_some() OR display.model.is_some()) THEN
        make = display.make.as_deref().unwrap_or("")
        model = display.model.as_deref().unwrap_or("")
        format!("{}: {} ({} {})\n", display.name, power_str, make, model).trim()
    ELSE
        format!("{}: {}\n", display.name, power_str)
    END IF
END FUNCTION

FUNCTION format_displays_json(displays: &[DisplayInfo]) -> String
    // Hand-crafted JSON to avoid serde dependency
    parts = displays.iter().map(|d| {
        power_str = IF d.power == On THEN "on" ELSE "off"
        format!("{{\"name\":\"{}\",\"power\":\"{}\"}}", d.name, power_str)
    }).collect::<Vec<_>>()
    
    format!("[{}]", parts.join(","))
END FUNCTION

FUNCTION format_single_display_json(display: &DisplayInfo) -> String
    power_str = IF display.power == On THEN "on" ELSE "off"
    format!("{{\"name\":\"{}\",\"power\":\"{}\"}}", display.name, power_str)
END FUNCTION
```

## Dependencies (New)

```toml
[dependencies]
# Existing dependencies unchanged

# NEW: Shell completions
clap_complete = "4"
```

**Note**: No serde dependency added. JSON output continues to be hand-crafted.

## Migration & Backwards Compatibility

### CLI Compatibility Matrix

| v1 Command | v2 Behavior | Notes |
|------------|-------------|-------|
| `dpms on` | Turns on all displays | Unchanged |
| `dpms off` | Turns off all displays | Unchanged |
| `dpms status` | Shows all displays | Extended output format |
| `dpms status --json` | JSON for all displays | Extended to array format |

### JSON Output Compatibility

**v1 output** (single display):
```json
{"power":"on"}
```

**v2 output** (backwards compat mode - when single display or no target specified):
```json
{"power":"on"}
```

**v2 output** (multi-display or explicit --all):
```json
[{"name":"DP-1","power":"on"},{"name":"eDP-1","power":"off"}]
```

**Decision**: When a single display is targeted or only one display exists, use v1 format. When multiple displays are involved, use array format. This maintains backwards compatibility for scripts parsing v1 output.

## Brief Compliance

**Coverage**: 100% (All discussion points addressed)

| Discussion Point | Spec Section |
|------------------|--------------|
| Multi-display selection | F12, F13, F17 |
| Toggle command | F14 |
| List command | F15 |
| Shell completions | F16 |
| Partial name matching | F13 |
| Verbose display info | F15, F18 |
| Backwards compatibility | Migration section |
| No serde dependency | Dependencies section |

**Scope Creep**: None. Explicitly excluded:
- Resolution/mode control (wlout territory)
- Position/move commands (wlout territory)
- Mirror functionality (wlout territory)
- X11 support (deferred, not in discussion scope)
