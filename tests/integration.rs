//! Integration tests for powermon CLI
//!
//! These tests verify CLI argument parsing and help output.
//! For full power cycle tests, use the shell script: tests/test_power_cycle.sh

use std::process::Command;

/// Get the path to the powermon binary
fn powermon_bin() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let mut path = std::path::PathBuf::from(manifest_dir);
    path.push("target");

    // Prefer release build
    let release_path = path.join("release").join("powermon");
    if release_path.exists() {
        return release_path;
    }

    path.push("debug");
    path.push("powermon");
    path
}

#[test]
fn test_help_command() {
    let output = Command::new(powermon_bin())
        .arg("--help")
        .output()
        .expect("Failed to execute powermon --help");

    assert!(output.status.success(), "powermon --help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Control monitor power state"),
        "Help should contain description"
    );
    assert!(stdout.contains("on"), "Help should mention 'on' command");
    assert!(stdout.contains("off"), "Help should mention 'off' command");
    assert!(
        stdout.contains("status"),
        "Help should mention 'status' command"
    );
}

#[test]
fn test_status_subcommand_help() {
    let output = Command::new(powermon_bin())
        .args(["status", "--help"])
        .output()
        .expect("Failed to execute powermon status --help");

    assert!(
        output.status.success(),
        "powermon status --help should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--json"),
        "Status help should mention --json flag"
    );
}

#[test]
fn test_on_subcommand_help() {
    let output = Command::new(powermon_bin())
        .args(["on", "--help"])
        .output()
        .expect("Failed to execute powermon on --help");

    assert!(output.status.success(), "powermon on --help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Turn display on"),
        "On help should describe the command"
    );
}

#[test]
fn test_off_subcommand_help() {
    let output = Command::new(powermon_bin())
        .args(["off", "--help"])
        .output()
        .expect("Failed to execute powermon off --help");

    assert!(
        output.status.success(),
        "powermon off --help should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Turn display off"),
        "Off help should describe the command"
    );
}

#[test]
fn test_invalid_command_fails() {
    let output = Command::new(powermon_bin())
        .arg("invalid")
        .output()
        .expect("Failed to execute powermon invalid");

    assert!(!output.status.success(), "Invalid command should fail");
}
