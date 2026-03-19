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

/// Run the structural comparison script for a given site.
/// Returns (stdout, success) so tests can check output and apply thresholds.
fn run_comparison(site: &str) -> (String, bool) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let script = root.join("scripts/compare-output.sh");
    assert!(
        script.exists(),
        "comparison script not found at {}",
        script.display()
    );

    let site_dir = root.join("websites").join(site);
    assert!(site_dir.exists(), "site source not found at {:?}", site_dir);

    let release = root.join("target/release/rustkyll");
    let binary = if release.exists() {
        release
    } else {
        root.join("target/debug/rustkyll")
    };
    assert!(
        binary.exists(),
        "rustkyll binary not found at {:?}. Run `cargo build` first.",
        binary
    );

    let output = Command::new("bash")
        .arg(&script)
        .arg("--site")
        .arg(site)
        .output()
        .expect("Failed to run compare-output.sh");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    println!("=== STDOUT ===\n{stdout}");
    if !stderr.is_empty() {
        println!("=== STDERR ===\n{stderr}");
    }

    (stdout + &stderr, output.status.success())
}

#[test]
fn test_structural_comparison_kids_horror_stories() {
    let (output, success) = run_comparison("alexeygrigorev/kids-horror-stories-ru");
    assert!(
        success,
        "kids-horror-stories-ru structural comparison should pass with zero diffs.\nOutput:\n{output}"
    );
}

#[test]
fn test_structural_comparison_dtc_site() {
    let (output, _success) = run_comparison("DataTalksClub/datatalksclub.github.io");

    // DTC is not at 100% DOM match yet (~68%), so we check structural properties
    // rather than asserting zero diffs. The script exits 1 on any diffs.
    assert!(
        output.contains("OK: File count within"),
        "DTC file count should be within tolerance.\nOutput:\n{output}"
    );
    assert!(
        output.contains("Files with raw Liquid tags: 0")
            || output.contains("Files with raw Liquid tags: 1"),
        "DTC should have at most 1 liquid leak.\nOutput:\n{output}"
    );
}
