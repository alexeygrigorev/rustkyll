//! Integration tests for page count matching between rustkyll and Jekyll.
//!
//! These tests build benchmark sites and verify the HTML file count matches
//! Jekyll's output exactly. They require website checkouts in `websites/`.
//!
//! Run with: cargo test -p integration-tests --test integration_page_counts

use std::path::Path;
use std::process::Command;

fn project_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

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
/// Returns `None` if the binary or source directory is missing (graceful skip).
fn build_site(site_name: &str) -> Option<tempfile::TempDir> {
    let binary = project_root().join("target/debug/rustkyll");
    if !binary.exists() {
        eprintln!(
            "SKIPPING: rustkyll binary not found at {:?}. Run `cargo build` first.",
            binary
        );
        return None;
    }
    let source = project_root().join("websites").join(site_name);
    if !source.exists() {
        eprintln!("SKIPPING: site source not found at {:?}", source);
        return None;
    }
    let tmp = tempfile::TempDir::new().unwrap();
    let output = Command::new(&binary)
        .args(["build", "--source"])
        .arg(&source)
        .arg("--destination")
        .arg(tmp.path())
        .output()
        .expect("failed to run rustkyll");
    assert!(
        output.status.success(),
        "Build failed for {}: {}",
        site_name,
        String::from_utf8_lossy(&output.stderr)
    );
    Some(tmp)
}

#[test]
fn test_large_blog_3000_page_count() {
    let tmp = match build_site("large-blog-3000") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 3001,
        "large-blog-3000: expected 3001 HTML files, got {}",
        count
    );
}

#[test]
fn test_large_docs_site_page_count() {
    let tmp = match build_site("large-docs-site") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 801,
        "large-docs-site: expected 801 HTML files, got {}",
        count
    );
}

#[test]
fn test_documentation_theme_jekyll_page_count() {
    let tmp = match build_site("documentation-theme-jekyll") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 98,
        "documentation-theme-jekyll: expected 98 HTML files, got {}",
        count
    );
}

#[test]
fn test_muan_blog_page_count() {
    let tmp = match build_site("muan-blog") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2218,
        "muan-blog: expected 2218 HTML files, got {}",
        count
    );
}

#[test]
fn test_homebrew_site_page_count() {
    let tmp = match build_site("homebrew-site") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 134,
        "homebrew-site: expected 134 HTML files, got {}",
        count
    );
}

#[test]
fn test_muan_blog_notes_exist() {
    let tmp = match build_site("muan-blog") {
        Some(t) => t,
        None => return,
    };
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
fn test_muan_blog_stories_exist() {
    let tmp = match build_site("muan-blog") {
        Some(t) => t,
        None => return,
    };
    let stories_dir = tmp.path().join("stories");
    assert!(stories_dir.exists(), "stories directory should exist");
    let stories_count = count_html_files(&stories_dir);
    assert!(
        stories_count > 700,
        "Expected >700 stories HTML files, got {}",
        stories_count
    );
}

#[test]
fn test_mojombo_blog_page_count() {
    let tmp = match build_site("mojombo-blog") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 17,
        "mojombo-blog: expected 17 HTML files, got {}",
        count
    );
}

#[test]
fn test_mojombo_blog_date_permalinks() {
    let tmp = match build_site("mojombo-blog") {
        Some(t) => t,
        None => return,
    };
    let post = tmp.path().join("2008/11/17/blogging-like-a-hacker.html");
    assert!(
        post.exists(),
        "mojombo-blog: post should be at date-based permalink path: {}",
        post.display()
    );
}

#[test]
fn test_just_the_docs_page_count() {
    let tmp = match build_site("just-the-docs") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 47,
        "just-the-docs: expected 47 HTML files, got {}",
        count
    );
}

#[test]
fn test_cayman_theme_page_count() {
    let tmp = match build_site("cayman-theme") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "cayman-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
fn test_slate_theme_page_count() {
    let tmp = match build_site("slate-theme") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "slate-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
fn test_leap_day_theme_page_count() {
    let tmp = match build_site("leap-day-theme") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "leap-day-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
fn test_midnight_theme_page_count() {
    let tmp = match build_site("midnight-theme") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "midnight-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
fn test_hacker_theme_page_count() {
    let tmp = match build_site("hacker-theme") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "hacker-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
fn test_architect_theme_page_count() {
    let tmp = match build_site("architect-theme") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "architect-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
fn test_time_machine_theme_page_count() {
    let tmp = match build_site("time-machine-theme") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "time-machine-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
fn test_merlot_theme_page_count() {
    let tmp = match build_site("merlot-theme") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "merlot-theme: expected 2 HTML files, got {}",
        count
    );
}

#[test]
fn test_dinky_theme_page_count() {
    let tmp = match build_site("dinky-theme") {
        Some(t) => t,
        None => return,
    };
    let count = count_html_files(tmp.path());
    assert_eq!(
        count, 2,
        "dinky-theme: expected 2 HTML files, got {}",
        count
    );
}
