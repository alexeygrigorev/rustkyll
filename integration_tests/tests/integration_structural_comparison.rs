//! Integration tests for structural comparison of rustkyll vs Jekyll output.
//!
//! These tests require:
//! - The release binary to be built
//! - The website source directories to exist
//! - Jekyll to be installed for building reference output
//!
//! Run with: cargo test -p integration-tests --test integration_structural_comparison

use std::path::Path;
use std::process::Command;

/// Run the structural comparison script for a given site and assert exit code 0.
fn run_comparison(site: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let script = root.join("scripts/compare-output.sh");

    assert!(
        script.exists(),
        "Comparison script not found at {}",
        script.display()
    );

    let output = Command::new("bash")
        .arg(&script)
        .arg("--site")
        .arg(site)
        .output()
        .expect("Failed to run compare-output.sh");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("=== STDOUT ===\n{stdout}");
    if !stderr.is_empty() {
        println!("=== STDERR ===\n{stderr}");
    }

    assert!(
        output.status.success(),
        "compare-output.sh --site {site} failed with exit code {:?}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}",
        output.status.code()
    );
}

#[test]
fn test_structural_comparison_kids_horror_stories() {
    run_comparison("alexeygrigorev/kids-horror-stories-ru");
}

#[test]
fn test_structural_comparison_dtc_site() {
    run_comparison("DataTalksClub/datatalksclub.github.io");
}
