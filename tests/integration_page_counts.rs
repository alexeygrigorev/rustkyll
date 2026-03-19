//! Integration tests for page count matching between rustkyll and Jekyll.
//!
//! Most tests have been moved to integration_tests/tests/integration_page_counts.rs.
//! This file retains only the fast tests that don't require external website checkouts.

use std::path::Path;
use std::process::Command;

/// Count HTML files in a directory recursively.
fn count_html_files(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_html_files(&path);
            } else if path.extension().map(|e| e == "html").unwrap_or(false) {
                count += 1;
            }
        }
    }
    count
}

/// Build a site using rustkyll and return the output directory.
fn build_site(site_name: &str) -> tempfile::TempDir {
    let source = format!("websites/{}", site_name);
    let tmp = tempfile::TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .args(["build", "--source", &source, "--destination"])
        .arg(tmp.path())
        .output()
        .expect("failed to run rustkyll");
    assert!(
        output.status.success(),
        "Build failed for {}: {}",
        site_name,
        String::from_utf8_lossy(&output.stderr)
    );
    tmp
}

// ========================================================================
// Issue 123: alexeygrigorev.github.io build and content verification
// ========================================================================

#[test]
fn test_alexeygrigorev_site_builds_and_has_correct_content() {
    let source = std::path::Path::new("websites/alexeygrigorev/alexeygrigorev.github.io");
    if !source.exists() {
        return;
    }
    let tmp = build_site("alexeygrigorev/alexeygrigorev.github.io");
    let dest = tmp.path();

    // Verify expected pages exist
    assert!(dest.join("index.html").exists(), "index.html should exist");
    assert!(
        dest.join("courses.html").exists(),
        "courses.html should exist"
    );
    assert!(dest.join("cv.html").exists(), "cv.html should exist");
    assert!(
        dest.join("projects.html").exists(),
        "projects.html should exist"
    );
    assert!(
        dest.join("services.html").exists(),
        "services.html should exist"
    );

    // Verify correct page count (8 pages as per cross-site-results.md)
    let count = count_html_files(dest);
    assert_eq!(
        count, 8,
        "alexeygrigorev.github.io: expected 8 HTML files, got {}",
        count
    );

    // Verify homepage has Font Awesome CDN link (the CSS reference that matters)
    let index_html = std::fs::read_to_string(dest.join("index.html")).unwrap();
    assert!(
        index_html.contains("cdnjs.cloudflare.com/ajax/libs/font-awesome"),
        "Homepage should reference Font Awesome CSS from CDN"
    );

    // Verify homepage has the main.css link
    assert!(
        index_html.contains("/assets/css/main.css"),
        "Homepage should reference main.css"
    );

    // Verify content renders (not raw Liquid)
    assert!(
        !index_html.contains("{{ site.data"),
        "Homepage should not contain raw Liquid tags"
    );
    assert!(
        index_html.contains("Courses"),
        "Homepage should contain rendered Courses section"
    );
    assert!(
        index_html.contains("Projects"),
        "Homepage should contain rendered Projects section"
    );
    assert!(
        index_html.contains("Contribution activity"),
        "Homepage should contain Contribution activity section"
    );

    // Verify static CSS file is copied
    assert!(
        dest.join("assets/css/main.css").exists(),
        "main.css should be copied to output"
    );
}
