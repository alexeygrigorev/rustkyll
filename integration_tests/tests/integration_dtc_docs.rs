//! Integration tests for DTC/docs (just-the-docs theme) - Issue 233.
//!
//! These tests verify that the DataTalks.Club/docs site builds correctly
//! with rustkyll, including navigation, favicon, and JSON-LD output.
//!
//! Run with: cargo test -p integration-tests --test integration_dtc_docs

use std::path::PathBuf;
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn docs_source() -> PathBuf {
    project_root().join("websites/DataTalksClub/docs")
}

fn rustkyll_binary() -> PathBuf {
    let release = project_root().join("target/release/rustkyll");
    if release.exists() {
        return release;
    }
    project_root().join("target/debug/rustkyll")
}

fn build_dtc_docs(dest: &std::path::Path) -> String {
    let binary = rustkyll_binary();
    assert!(
        binary.exists(),
        "rustkyll binary not found at {:?}. Run `cargo build` first.",
        binary
    );
    let source = docs_source();
    assert!(source.exists(), "DTC/docs source not found at {:?}", source);
    if dest.exists() {
        std::fs::remove_dir_all(dest).unwrap();
    }
    let output = Command::new(&binary)
        .arg("build")
        .arg("--source")
        .arg(&source)
        .arg("--destination")
        .arg(dest)
        .output()
        .expect("failed to run rustkyll");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        panic!(
            "rustkyll build failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }
    stdout + &stderr
}

#[test]
fn test_dtc_docs_nav_items_count() {
    let dest = std::env::temp_dir().join("dtc-docs-test-nav");
    build_dtc_docs(&dest);

    let index = std::fs::read_to_string(dest.join("index.html")).expect("should have index.html");
    let nav_count = index.matches("nav-list-item").count();
    // The just-the-docs navigation should contain all 59 nav items (4 top-level
    // pages plus all their children, plus 1 external link).
    assert!(
        nav_count >= 50,
        "Expected at least 50 nav-list-item occurrences, got {}",
        nav_count
    );
}

#[test]
fn test_dtc_docs_nav_activation_css() {
    let dest = std::env::temp_dir().join("dtc-docs-test-css");
    build_dtc_docs(&dest);

    // Check a non-homepage page for nth-child CSS activation
    let page = std::fs::read_to_string(dest.join("courses/data-engineering-zoomcamp/index.html"))
        .expect("should have courses/data-engineering-zoomcamp/index.html");

    assert!(
        page.contains("jtd-nav-activation"),
        "Should have jtd-nav-activation style tag"
    );
    assert!(
        page.contains(":nth-child("),
        "Non-homepage should have :nth-child CSS selectors, not the fallback rule"
    );
    // The fallback rule is `.site-nav ul li a { ... }` without :nth-child
    // If only the fallback is present, it means activation failed.
}

#[test]
fn test_dtc_docs_favicon_link() {
    let dest = std::env::temp_dir().join("dtc-docs-test-favicon");
    build_dtc_docs(&dest);

    let index = std::fs::read_to_string(dest.join("index.html")).expect("should have index.html");
    assert!(
        index.contains(r#"<link rel="icon" href="/favicon.ico""#),
        "Should have favicon link in head. Got:\n{}",
        &index[..2000.min(index.len())]
    );
}

#[test]
fn test_dtc_docs_jsonld_publisher() {
    let dest = std::env::temp_dir().join("dtc-docs-test-jsonld");
    build_dtc_docs(&dest);

    let index = std::fs::read_to_string(dest.join("index.html")).expect("should have index.html");
    assert!(
        index.contains(r#""publisher":{"@type":"Organization","logo":{"@type":"ImageObject""#),
        "Should have publisher field in JSON-LD when site.logo is configured"
    );
}

#[test]
fn test_dtc_docs_no_render_failures() {
    let dest = std::env::temp_dir().join("dtc-docs-test-failures");
    let output = build_dtc_docs(&dest);

    let failure_count = output.matches("failed to render page").count();
    assert_eq!(
        failure_count, 0,
        "Should have zero page render failures. Found {}:\n{}",
        failure_count, output
    );
}

#[test]
fn test_dtc_docs_all_pages_rendered() {
    let dest = std::env::temp_dir().join("dtc-docs-test-pages");
    let output = build_dtc_docs(&dest);

    // The upstream docs repo grows over time, so assert a lower bound on the
    // actual parsed count rather than substring-matching a literal number
    // (which would spuriously match any value containing those digits).
    let standalone = output
        .lines()
        .find_map(|l| l.trim().strip_prefix("Standalone pages:"))
        .and_then(|n| n.trim().parse::<usize>().ok())
        .unwrap_or_else(|| panic!("Build output missing 'Standalone pages:' line.\n{}", output));

    assert!(
        standalone >= 59,
        "Should generate at least 59 standalone pages, got {}. Output:\n{}",
        standalone,
        output
    );
}
