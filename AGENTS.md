# AGENTS.md

## OVERVIEW
DPMS - Display Power Management System for Rust. CLI tool for monitor power control via Wayland and TTY backends.

## STRUCTURE
- **src/**: Core implementation (11 files, 6104 lines)
- **tests/**: Test suite (2 files)  
- **docs/**: Project documentation (6 files, 4 subdirs)
- **target/**: Build artifacts (ignore)

## WHERE TO LOOK

### Power Management Logic
- `src/backend.rs` - PowerBackend trait definition
- `src/wayland.rs` - Wayland compositor protocol (440 lines)
- `src/tty.rs` - TTY backend with daemon support
- `src/daemon.rs` - Daemon process for persistent state (439 lines)
- `src/drm_ops.rs` - DRM atomic operations (374 lines)

### Entry Points
- `src/main.rs` - Main CLI dispatch and application entry
- `src/cli.rs` - Command-line argument parsing

### System Integration
- `src/env.rs` - Environment detection and configuration
- `src/error.rs` - Error handling and types
- `src/display.rs` - Display device abstraction
- `src/output.rs` - Output formatting and utilities

## CODE MAP

### Multi-Backend Architecture
```
PowerBackend trait
├── WaylandBackend (wayland.rs)
│   ├── Wayland protocol communication
│   ├── Auto socket discovery for SSH
│   └── Direct compositor integration
└── TTYBackend (tty.rs)
    ├── Daemon process (daemon.rs)
    ├── DRM operations (drm_ops.rs)  
    └── Atomic commit support
```

### CLI Flow
```
main() → cli::parse() → match command
├── power on/off → backend.set_power()
├── status → backend.get_status()
├── list → backend.list_displays()
└── cycle → backend.cycle_power()
```

## CONVENTIONS

### Code Organization
- **Single binary**: All code in main.rs with inline modules (no lib.rs)
- **Trait-based**: PowerBackend trait for multiple backend implementations
- **Hand-crafted JSON**: No serde dependency to minimize binary size
- **Environment-driven**: Auto-detection of Wayland vs TTY environments

### Error Handling
- Custom error types in error.rs
- Comprehensive error context preservation
- Graceful fallback between backends when possible

### System Integration
- Rust 2024 edition with minimal dependencies
- Release profile optimized with LTO
- Standard Cargo build system

## ANTI-PATTERNS

### Large Files
- No files >500 lines detected
- Well-balanced module sizes (374-440 lines largest modules)

### Dependencies
- Minimal external dependencies (wayland, rustix, signal-hook, thiserror)
- Avoids serde for JSON to reduce binary size
- Uses rustix for system calls instead of libc directly

## UNIQUE STYLES

### Hybrid Backend Approach
- Wayland: Direct compositor protocol for modern desktops
- TTY: Daemon-based approach for console/server environments
- Automatic backend selection based on environment

### Remote Session Support
- Wayland socket auto-discovery for SSH sessions
- Daemon persistence for TTY backend across reboots

### Performance Decisions
- Trade dependency size for hand-crafted JSON parsing
- LTO enabled in release builds for optimization
- Atomic DRM commits for display state changes

## COMMANDS

### Development
```bash
cargo build                    # Debug build
cargo build --release          # Optimized build  
cargo run -- --help            # CLI help
cargo test                     # Run tests
```

### Testing
```bash
cargo test                    # Rust integration tests
./tests/test_power_cycle.sh   # Bash shell script tests
```

## NOTES

### Architecture Decisions
- Single binary approach acceptable for CLI tool simplicity
- Daemon approach necessary for TTY display state persistence
- PowerBackend trait enables clean backend abstraction

### Implementation Details
- Wayland backend uses raw protocol for minimal dependencies
- DRM operations require atomic commit support
- Error handling preserves context across backend boundaries

### Build System
- Standard Cargo configuration, no CI/CD automation
- Target directory contains build artifacts
- Rust 2024 edition with modern language features