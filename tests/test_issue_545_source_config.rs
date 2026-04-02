//! Tests for Issue 545: `source` config key support.
//!
//! Verifies that when `source: src` is set in `_config.yml`, rustkyll looks
//! for pages, posts, layouts, includes, and data inside the `src/` subdirectory.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Helper: run a rustkyll build on the given source directory, writing to dest.
fn build_site(source: &Path, dest: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .args(["build", "--source"])
        .arg(source)
        .arg("--destination")
        .arg(dest)
        .arg("--quiet")
        .output()
        .expect("failed to run rustkyll")
}

/// Build a minimal site with `source: src` in _config.yml.
/// Pages, posts, and layouts live under src/.
#[test]
fn test_source_config_src_discovers_content() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // _config.yml at root with source: src
    fs::write(
        root.join("_config.yml"),
        "source: src\ntitle: ソーステスト\npermalink: /:title.html\n",
    )
    .unwrap();

    // Content inside src/
    let src = root.join("src");
    fs::create_dir_all(src.join("_layouts")).unwrap();
    fs::create_dir_all(src.join("_posts")).unwrap();

    // A simple layout
    fs::write(
        src.join("_layouts/default.html"),
        "<html><body>{{ content }}</body></html>",
    )
    .unwrap();

    // A page at src/index.md
    fs::write(
        src.join("index.md"),
        "---\nlayout: default\npermalink: /index.html\ntitle: Home\n---\nHello from source config.\n",
    )
    .unwrap();

    // A post at src/_posts/
    fs::write(
        src.join("_posts/2024-01-15-test-post.md"),
        "---\nlayout: default\ntitle: Test Post\n---\nPost content héré.\n",
    )
    .unwrap();

    let dest = root.join("_site");
    let output = build_site(root, &dest);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "Build failed: {}", stderr);

    // Verify page was generated
    let index_html = dest.join("index.html");
    assert!(index_html.exists(), "index.html should exist in output");
    let content = fs::read_to_string(&index_html).unwrap();
    assert!(
        content.contains("Hello from source config"),
        "index.html should contain page content, got: {}",
        content
    );

    // Verify post was generated
    let post_html = dest.join("test-post.html");
    assert!(
        post_html.exists(),
        "Post test-post.html should exist in output"
    );
    let post_content = fs::read_to_string(&post_html).unwrap();
    assert!(
        post_content.contains("Post content"),
        "Post should contain content, got: {}",
        post_content
    );
}

/// Build a site with `source: .` -- this should be a no-op (same as default).
#[test]
fn test_source_config_dot_is_noop() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    fs::write(
        root.join("_config.yml"),
        "source: .\ntitle: Dot Test\npermalink: /:title.html\n",
    )
    .unwrap();

    fs::create_dir_all(root.join("_layouts")).unwrap();
    fs::write(
        root.join("_layouts/default.html"),
        "<html><body>{{ content }}</body></html>",
    )
    .unwrap();

    fs::write(
        root.join("index.md"),
        "---\nlayout: default\npermalink: /index.html\ntitle: Home\n---\nDot source works.\n",
    )
    .unwrap();

    let dest = root.join("_site");
    let output = build_site(root, &dest);

    assert!(
        output.status.success(),
        "Build with source: . failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let index_html = dest.join("index.html");
    assert!(index_html.exists(), "index.html should exist");
    let content = fs::read_to_string(&index_html).unwrap();
    assert!(
        content.contains("Dot source works"),
        "Content mismatch: {}",
        content
    );
}

/// Build a site with no source key -- should default to "." behavior.
#[test]
fn test_source_config_absent_defaults_to_dot() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    fs::write(
        root.join("_config.yml"),
        "title: No Source Key\npermalink: /:title.html\n",
    )
    .unwrap();

    fs::create_dir_all(root.join("_layouts")).unwrap();
    fs::write(
        root.join("_layouts/default.html"),
        "<html><body>{{ content }}</body></html>",
    )
    .unwrap();

    fs::write(
        root.join("index.md"),
        "---\nlayout: default\npermalink: /index.html\ntitle: Home\n---\nNo source key works.\n",
    )
    .unwrap();

    let dest = root.join("_site");
    let output = build_site(root, &dest);

    assert!(
        output.status.success(),
        "Build without source key failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let index_html = dest.join("index.html");
    assert!(index_html.exists(), "index.html should exist");
    let content = fs::read_to_string(&index_html).unwrap();
    assert!(
        content.contains("No source key works"),
        "Content mismatch: {}",
        content
    );
}
