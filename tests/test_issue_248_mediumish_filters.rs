//! Integration tests for issue #248: Mediumish Liquid filter support.
//!
//! Verifies that `url_escape` and `camelcase` filters work end-to-end
//! in the Mediumish theme build. Both are passthrough filters that should
//! be recognized without warnings and produce rendered HTML output (not
//! raw Liquid markup).

use std::path::Path;
use std::process::Command;

/// Build the mediumish site using the rustkyll binary and return a temp dir
/// containing the output.
fn build_mediumish() -> tempfile::TempDir {
    let source = "websites/mediumish";
    assert!(
        Path::new(source).exists(),
        "Mediumish site source must exist at websites/mediumish"
    );
    let tmp = tempfile::TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .args(["build", "--source", source, "--destination"])
        .arg(tmp.path())
        .output()
        .expect("failed to run rustkyll");
    assert!(
        output.status.success(),
        "Mediumish build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    tmp
}

#[test]
fn test_mediumish_no_unknown_filter_warnings() {
    let source = "websites/mediumish";
    if !Path::new(source).exists() {
        panic!("Mediumish site source must exist at websites/mediumish");
    }
    let tmp = tempfile::TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .args(["build", "--source", source, "--destination"])
        .arg(tmp.path())
        .output()
        .expect("failed to run rustkyll");
    assert!(
        output.status.success(),
        "Mediumish build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Verify no warnings specifically about url_escape or camelcase filters
    // (other unknown filter warnings like 'document' or 'add' are separate issues)
    let has_url_escape_warning = stderr
        .lines()
        .any(|line| line.contains("unknown filter") && line.contains("url_escape"));
    assert!(
        !has_url_escape_warning,
        "Build stderr should not warn about url_escape filter, got: {}",
        stderr
    );
    let has_camelcase_warning = stderr
        .lines()
        .any(|line| line.contains("unknown filter") && line.contains("camelcase"));
    assert!(
        !has_camelcase_warning,
        "Build stderr should not warn about camelcase filter, got: {}",
        stderr
    );
}

#[test]
fn test_mediumish_category_sidebar_rendered() {
    let tmp = build_mediumish();
    let dest = tmp.path();

    // The index.html should exist and contain rendered category sidebar
    let index_path = dest.join("index.html");
    assert!(index_path.exists(), "index.html should exist");
    let html = std::fs::read_to_string(&index_path).unwrap();

    // The sidebar should contain rendered category links, not raw Liquid
    assert!(
        !html.contains("{{ category | url_escape"),
        "index.html should not contain raw Liquid url_escape filter markup"
    );
    assert!(
        !html.contains("{{ category | camelcase"),
        "index.html should not contain raw Liquid camelcase filter markup"
    );
    assert!(
        !html.contains("{{ category[0] | url_escape"),
        "index.html should not contain raw Liquid url_escape filter markup (array form)"
    );
    assert!(
        !html.contains("{{ category[0] | camelcase"),
        "index.html should not contain raw Liquid camelcase filter markup (array form)"
    );
}

#[test]
fn test_mediumish_category_sidebar_has_links() {
    let tmp = build_mediumish();
    let dest = tmp.path();

    let index_path = dest.join("index.html");
    assert!(index_path.exists(), "index.html should exist");
    let html = std::fs::read_to_string(&index_path).unwrap();

    // The category sidebar should have anchor tags with href to categories
    // The mediumish posts have categories: Jekyll, tutorial, web development
    assert!(
        html.contains("/categories#"),
        "index.html should contain category anchor links (href to /categories#...)"
    );

    // Verify we have actual <a> tags in the category sidebar section
    // The "fortags" div contains the category links
    let fortags_start = html.find("fortags");
    assert!(
        fortags_start.is_some(),
        "index.html should contain the 'fortags' category sidebar section"
    );

    // Extract the fortags section and verify it contains <a> tags
    let section = &html[fortags_start.unwrap()..];
    let section_end = section
        .find("</div>\n</div>\n</div>")
        .unwrap_or(section.len());
    let section = &section[..section_end];

    assert!(
        section.contains("<a "),
        "Category sidebar should contain <a> tags with rendered category links"
    );
    assert!(
        section.contains("href="),
        "Category links should have href attributes"
    );
}

#[test]
fn test_mediumish_no_raw_liquid_in_any_page() {
    let tmp = build_mediumish();
    let dest = tmp.path();

    // Walk all HTML files and check none contain raw url_escape or camelcase Liquid
    let mut checked = 0;
    check_dir_for_raw_liquid(dest, &mut checked);
    assert!(
        checked > 0,
        "Should have checked at least one HTML file in the mediumish build output"
    );
}

fn check_dir_for_raw_liquid(dir: &Path, count: &mut usize) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                check_dir_for_raw_liquid(&path, count);
            } else if path.extension().map(|e| e == "html").unwrap_or(false) {
                let html = std::fs::read_to_string(&path).unwrap();
                assert!(
                    !html.contains("| url_escape"),
                    "{}: should not contain raw '| url_escape' Liquid markup",
                    path.display()
                );
                assert!(
                    !html.contains("| camelcase"),
                    "{}: should not contain raw '| camelcase' Liquid markup",
                    path.display()
                );
                *count += 1;
            }
        }
    }
}
