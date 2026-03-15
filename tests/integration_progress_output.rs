//! Integration tests for build progress output (Issue 92).
//!
//! These tests run the `rustkyll` binary as a subprocess and capture
//! stderr/stdout to verify progress output behavior.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Create a minimal site directory with config, layout, and an index page.
/// Returns the TempDir (must be kept alive for the duration of the test).
fn create_minimal_site() -> TempDir {
    let dir = TempDir::new().expect("failed to create temp dir");
    let root = dir.path();

    // _config.yml
    fs::write(
        root.join("_config.yml"),
        r#"
url: "https://example.com"
name: "Test Site"
title: "Test Site"
baseurl: ""
collections: {}
"#,
    )
    .unwrap();

    // _layouts/page.html
    fs::create_dir_all(root.join("_layouts")).unwrap();
    fs::write(root.join("_layouts").join("page.html"), "{{ content }}").unwrap();

    // index.md
    fs::write(
        root.join("index.md"),
        r#"---
layout: page
title: Home
---
Hello world
"#,
    )
    .unwrap();

    dir
}

/// Create a site with multiple posts and standalone pages for count verification.
fn create_site_with_known_page_count() -> TempDir {
    let dir = TempDir::new().expect("failed to create temp dir");
    let root = dir.path();

    // _config.yml with posts collection
    fs::write(
        root.join("_config.yml"),
        r#"
url: "https://example.com"
name: "Test Site"
title: "Test Site"
baseurl: ""
permalink: "/blog/:title.html"
collections: {}
"#,
    )
    .unwrap();

    // _layouts
    fs::create_dir_all(root.join("_layouts")).unwrap();
    fs::write(root.join("_layouts").join("page.html"), "{{ content }}").unwrap();
    fs::write(root.join("_layouts").join("post.html"), "{{ content }}").unwrap();

    // 3 posts
    fs::create_dir_all(root.join("_posts")).unwrap();
    for i in 1..=3 {
        fs::write(
            root.join("_posts")
                .join(format!("2024-01-0{i}-post-{i}.md")),
            format!(
                r#"---
layout: post
title: "Post {i}"
---
Content of post {i}
"#
            ),
        )
        .unwrap();
    }

    // 2 standalone pages
    fs::write(
        root.join("index.md"),
        r#"---
layout: page
title: Home
---
Home page
"#,
    )
    .unwrap();

    fs::write(
        root.join("about.md"),
        r#"---
layout: page
title: About
---
About page
"#,
    )
    .unwrap();

    dir
}

/// Run rustkyll build on a site directory, returning the Command output.
fn run_build(site_dir: &std::path::Path, extra_args: &[&str]) -> std::process::Output {
    let dest = site_dir.join("_site");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustkyll"));
    cmd.arg("build")
        .arg("--source")
        .arg(site_dir)
        .arg("--destination")
        .arg(&dest);
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.output().expect("failed to run rustkyll binary")
}

#[test]
fn progress_stderr_contains_phase_messages_in_normal_mode() {
    let site = create_minimal_site();
    let output = run_build(site.path(), &[]);

    assert!(
        output.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // In non-TTY mode (piped stderr), only phase_done() messages are printed
    // (phase() is a no-op in non-TTY). Verify key phase completion messages:
    assert!(
        stderr.contains("Loading data files"),
        "stderr should contain 'Loading data files', got: {stderr}"
    );
    assert!(
        stderr.contains("Loading collections"),
        "stderr should contain 'Loading collections', got: {stderr}"
    );
    assert!(
        stderr.contains("Copying static files"),
        "stderr should contain 'Copying static files', got: {stderr}"
    );
    assert!(
        stderr.contains("Generating sitemap"),
        "stderr should contain 'Generating sitemap', got: {stderr}"
    );
}

#[test]
fn progress_stderr_empty_in_quiet_mode() {
    let site = create_minimal_site();
    let output = run_build(site.path(), &["--quiet"]);

    assert!(
        output.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // In quiet mode, stderr should have no progress output.
    // It should be completely empty for a successful build.
    assert!(
        stderr.is_empty(),
        "stderr should be empty in quiet mode, got: {stderr}"
    );
}

#[test]
fn progress_stdout_does_not_contain_phase_indicators() {
    let site = create_minimal_site();
    let output = run_build(site.path(), &[]);

    assert!(
        output.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Progress phase messages go to stderr, not stdout.
    // stdout should contain only the build summary.
    assert!(
        !stdout.contains("Loading data files"),
        "stdout should NOT contain 'Loading data files', got: {stdout}"
    );
    assert!(
        !stdout.contains("Loading collections"),
        "stdout should NOT contain 'Loading collections', got: {stdout}"
    );
    assert!(
        !stdout.contains("Copying static files..."),
        "stdout should NOT contain 'Copying static files...', got: {stdout}"
    );
    assert!(
        !stdout.contains("Generating sitemap..."),
        "stdout should NOT contain 'Generating sitemap...', got: {stdout}"
    );

    // stdout should contain the summary
    assert!(
        stdout.contains("Build complete") || stdout.contains("Source:"),
        "stdout should contain build summary, got: {stdout}"
    );
}

#[test]
fn progress_count_matches_known_page_count() {
    let site = create_site_with_known_page_count();
    let output = run_build(site.path(), &[]);

    assert!(
        output.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // We created 3 posts + 2 standalone pages = 5 total pages.
    // The stdout summary should show the correct totals.
    assert!(
        stdout.contains("Total pages:      5"),
        "stdout should report 5 total pages, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn progress_non_tty_no_ansi_escape_codes() {
    let site = create_minimal_site();
    let output = run_build(site.path(), &[]);

    assert!(
        output.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // When stderr is piped (non-TTY), there should be no ANSI escape codes.
    assert!(
        !stderr.contains("\x1b["),
        "stderr should not contain ANSI escape codes in non-TTY mode, got: {stderr}"
    );

    // Also no carriage returns (used for in-place TTY updates)
    assert!(
        !stderr.contains('\r'),
        "stderr should not contain carriage returns in non-TTY mode, got: {stderr}"
    );
}
