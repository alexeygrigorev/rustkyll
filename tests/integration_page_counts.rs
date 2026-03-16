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
    assert_eq!(
        count, 98,
        "documentation-theme-jekyll: expected 98 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_muan_blog_page_count() {
    let tmp = build_site("muan-blog");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2218,
        "muan-blog: expected 2218 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_homebrew_site_page_count() {
    let tmp = build_site("homebrew-site");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 134,
        "homebrew-site: expected 134 HTML files, got {}",
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

// ========================================================================
// Issue 82: New test sites page count verification
// ========================================================================

#[test]
#[ignore]
fn test_mojombo_blog_page_count() {
    let tmp = build_site("mojombo-blog");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 17,
        "mojombo-blog: expected 17 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_mojombo_blog_date_permalinks() {
    // Verifies the default permalink 'date' generates correct URLs
    let tmp = build_site("mojombo-blog");
    let post = tmp.path().join("2008/11/17/blogging-like-a-hacker.html");
    assert!(
        post.exists(),
        "mojombo-blog: post should be at date-based permalink path: {}",
        post.display()
    );
}

#[test]
#[ignore]
fn test_just_the_docs_page_count() {
    let tmp = build_site("just-the-docs");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 47,
        "just-the-docs: expected 47 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_cayman_theme_page_count() {
    let tmp = build_site("cayman-theme");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "cayman-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_slate_theme_page_count() {
    let tmp = build_site("slate-theme");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "slate-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_leap_day_theme_page_count() {
    let tmp = build_site("leap-day-theme");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "leap-day-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_midnight_theme_page_count() {
    let tmp = build_site("midnight-theme");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "midnight-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_hacker_theme_page_count() {
    let tmp = build_site("hacker-theme");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "hacker-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_architect_theme_page_count() {
    let tmp = build_site("architect-theme");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "architect-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_time_machine_theme_page_count() {
    let tmp = build_site("time-machine-theme");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "time-machine-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_merlot_theme_page_count() {
    let tmp = build_site("merlot-theme");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "merlot-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
#[ignore]
fn test_dinky_theme_page_count() {
    let tmp = build_site("dinky-theme");
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "dinky-theme: expected 2 HTML files, got {}",
        count
    );
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
