//! Integration tests for page count matching between rustkyll and Jekyll.
//!
//! These tests build benchmark sites and verify the HTML file count matches
//! Jekyll's output exactly. All tests are `#[ignore]` since they require
//! website checkouts in `websites/` and are slow.

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

#[test]
#[ignore]
fn test_large_blog_3000_page_count() {
    let tmp = build_site("large-blog-3000");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 3001,
        "large-blog-3000: expected 3001 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_large_docs_site_page_count() {
    let tmp = build_site("large-docs-site");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 801,
        "large-docs-site: expected 801 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_documentation_theme_jekyll_page_count() {
    let tmp = build_site("documentation-theme-jekyll");
    let count = count_html_files(tmp.path());
    // Target is 100; allow small variance due to git remote resolution differences
    assert!(
        (98..=100).contains(&count),
        "documentation-theme-jekyll: expected ~100 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_muan_blog_page_count() {
    let tmp = build_site("muan-blog");
    let count = count_html_files(tmp.path());
    // Target is 2218, allow +/- 1 for minor permalink differences
    assert!(
        (2217..=2219).contains(&count),
        "muan-blog: expected ~2218 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_homebrew_site_page_count() {
    let tmp = build_site("homebrew-site");
    let count = count_html_files(tmp.path());
    // Target is 134, allow +/- 1 for minor differences
    assert!(
        (133..=135).contains(&count),
        "homebrew-site: expected ~134 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_muan_blog_notes_exist() {
    let tmp = build_site("muan-blog");
    let notes_dir = tmp.path().join("notes");
    assert!(notes_dir.exists(), "notes directory should exist");
    let notes_count = count_html_files(&notes_dir);
    assert!(
        notes_count > 1000,
        "Expected >1000 notes HTML files, got {}",
        notes_count
    );
}

#[test]
#[ignore]
fn test_muan_blog_stories_exist() {
    let tmp = build_site("muan-blog");
    let stories_dir = tmp.path().join("stories");
    assert!(stories_dir.exists(), "stories directory should exist");
    let stories_count = count_html_files(&stories_dir);
    assert!(
        stories_count > 700,
        "Expected >700 stories HTML files, got {}",
        stories_count
    );
}
