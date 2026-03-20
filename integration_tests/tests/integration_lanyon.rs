//! Integration tests for Lanyon theme support.
//!
//! Verifies that rustkyll correctly builds the Lanyon Jekyll theme,
//! including layouts, includes, pagination, sidebar, syntax highlighting,
//! date filters, absolute_url filter, related posts, and static assets.
//!
//! Run with: cargo test -p integration-tests --test integration_lanyon

use std::path::{Path, PathBuf};
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rustkyll_binary() -> PathBuf {
    let release = project_root().join("target/release/rustkyll");
    if release.exists() {
        return release;
    }
    project_root().join("target/debug/rustkyll")
}

fn build_lanyon() -> tempfile::TempDir {
    let binary = rustkyll_binary();
    assert!(
        binary.exists(),
        "rustkyll binary not found at {:?}. Run `cargo build` first.",
        binary
    );
    let source = project_root().join("websites/lanyon");
    assert!(source.exists(), "lanyon source not found at {:?}", source);
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
        "Lanyon build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    tmp
}

fn read_file(dir: &Path, relative: &str) -> String {
    let path = dir.join(relative);
    assert!(path.exists(), "Expected file not found: {}", path.display());
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e))
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

// --- Page count and file existence ---

#[test]
fn test_lanyon_html_page_count() {
    let tmp = build_lanyon();
    let count = count_html_files(tmp.path());
    assert_eq!(count, 6, "Expected 6 HTML files, got {}", count);
}

#[test]
fn test_lanyon_all_expected_files_exist() {
    let tmp = build_lanyon();
    let expected = [
        "index.html",
        "about/index.html",
        "404.html",
        "2020/04/01/whats-jekyll/index.html",
        "2020/04/02/example-content/index.html",
        "2020/04/03/introducing-lanyon/index.html",
        "atom.xml",
        "feed.xml",
        "sitemap.xml",
    ];
    for file in &expected {
        let path = tmp.path().join(file);
        assert!(path.exists(), "Missing expected file: {}", file);
    }
}

#[test]
fn test_lanyon_static_assets_copied() {
    let tmp = build_lanyon();
    let assets = [
        "public/css/lanyon.css",
        "public/css/poole.css",
        "public/css/syntax.css",
        "public/js/script.js",
        "public/favicon.ico",
        "public/apple-touch-icon-precomposed.png",
    ];
    for asset in &assets {
        let path = tmp.path().join(asset);
        assert!(path.exists(), "Missing static asset: {}", asset);
    }
}

// --- Post page content ---

#[test]
fn test_lanyon_post_title_and_date() {
    let tmp = build_lanyon();
    let html = read_file(tmp.path(), "2020/04/03/introducing-lanyon/index.html");
    assert!(
        html.contains(r#"<h1 class="post-title">Introducing Lanyon</h1>"#),
        "Post page should contain post title in h1"
    );
    assert!(
        html.contains(r#"<span class="post-date">03 Apr 2020</span>"#),
        "Post page should contain formatted date via date_to_string filter"
    );
}

#[test]
fn test_lanyon_post_layout_chain() {
    let tmp = build_lanyon();
    let html = read_file(tmp.path(), "2020/04/03/introducing-lanyon/index.html");
    // post.html layout wraps content in div.post
    assert!(
        html.contains(r#"class="post""#),
        "Post page should have div.post from post.html layout"
    );
    // default.html layout wraps in body structure with sidebar
    assert!(
        html.contains(r#"class="wrap""#) || html.contains(r#"class="container"#),
        "Post page should have container/wrap from default.html layout"
    );
}

#[test]
fn test_lanyon_related_posts() {
    let tmp = build_lanyon();
    let html = read_file(tmp.path(), "2020/04/03/introducing-lanyon/index.html");
    assert!(
        html.contains(r#"class="related-posts""#),
        "Post page should contain related-posts section"
    );
    // Should have links to other posts
    assert!(
        html.contains("Example content") || html.contains("example-content"),
        "Related posts should link to other posts"
    );
}

// --- About page ---

#[test]
fn test_lanyon_about_page_layout() {
    let tmp = build_lanyon();
    let html = read_file(tmp.path(), "about/index.html");
    // page.html layout wraps content in div.page
    assert!(
        html.contains(r#"class="page""#),
        "About page should have div.page from page.html layout"
    );
    assert!(
        html.contains("About"),
        "About page should contain the word 'About'"
    );
}

// --- Sidebar ---

#[test]
fn test_lanyon_sidebar_navigation() {
    let tmp = build_lanyon();
    let html = read_file(tmp.path(), "index.html");
    assert!(
        html.contains(r#"class="sidebar-nav-item"#),
        "Pages should have sidebar navigation items"
    );
    assert!(
        html.contains("About"),
        "Sidebar should contain link to About page"
    );
}

#[test]
fn test_lanyon_sidebar_on_post_page() {
    let tmp = build_lanyon();
    let html = read_file(tmp.path(), "2020/04/01/whats-jekyll/index.html");
    assert!(
        html.contains(r#"class="sidebar-nav-item"#),
        "Post pages should also have sidebar navigation"
    );
}

// --- Index page / pagination ---

#[test]
fn test_lanyon_index_shows_all_posts() {
    let tmp = build_lanyon();
    let html = read_file(tmp.path(), "index.html");
    // Should contain all 3 post titles
    assert!(
        html.contains("Introducing Lanyon"),
        "Index should show 'Introducing Lanyon' post"
    );
    assert!(
        html.contains("Example content"),
        "Index should show 'Example content' post"
    );
    assert!(
        html.contains("What's Jekyll?")
            || html.contains("What&#39;s Jekyll?")
            || html.contains("What&rsquo;s Jekyll?"),
        "Index should show 'What's Jekyll?' post"
    );
}

#[test]
fn test_lanyon_index_has_post_links() {
    let tmp = build_lanyon();
    let html = read_file(tmp.path(), "index.html");
    assert!(
        html.contains("/2020/04/03/introducing-lanyon/"),
        "Index should link to introducing-lanyon post"
    );
    assert!(
        html.contains("/2020/04/02/example-content/"),
        "Index should link to example-content post"
    );
    assert!(
        html.contains("/2020/04/01/whats-jekyll/"),
        "Index should link to whats-jekyll post"
    );
}

// --- Syntax highlighting ---

#[test]
fn test_lanyon_syntax_highlighting_on_post_page() {
    let tmp = build_lanyon();
    let html = read_file(tmp.path(), "2020/04/02/example-content/index.html");
    assert!(
        html.contains(r#"class="highlight"#),
        "Example content post should have syntax highlighting"
    );
    assert!(
        html.contains("<code") && html.contains("language-js"),
        "Highlight block should contain code element with language-js class"
    );
}

// --- absolute_url filter ---

#[test]
fn test_lanyon_absolute_url_filter() {
    let tmp = build_lanyon();
    let html = read_file(tmp.path(), "index.html");
    assert!(
        html.contains("http://lanyon.getpoole.com"),
        "Pages should use absolute_url filter with configured url"
    );
}

// --- Feed and sitemap ---

#[test]
fn test_lanyon_feed_xml_has_rendered_content() {
    let tmp = build_lanyon();
    let xml = read_file(tmp.path(), "feed.xml");
    // Auto-generated feed should have rendered HTML, not raw Markdown
    assert!(
        xml.contains("<p>") || xml.contains("&lt;p&gt;"),
        "feed.xml should contain rendered HTML (p tags), not raw Markdown"
    );
    assert!(
        xml.contains("Introducing Lanyon") || xml.contains("Lanyon"),
        "feed.xml should contain post titles"
    );
}

#[test]
fn test_lanyon_sitemap_lists_all_pages() {
    let tmp = build_lanyon();
    let xml = read_file(tmp.path(), "sitemap.xml");
    // Sitemap should reference all pages
    assert!(
        xml.contains("introducing-lanyon"),
        "Sitemap should list introducing-lanyon post"
    );
    assert!(
        xml.contains("example-content"),
        "Sitemap should list example-content post"
    );
    assert!(
        xml.contains("whats-jekyll"),
        "Sitemap should list whats-jekyll post"
    );
    assert!(xml.contains("about"), "Sitemap should list about page");
}

// --- Unicode content (per project conventions) ---

#[test]
fn test_lanyon_handles_smart_quotes_in_title() {
    let tmp = build_lanyon();
    let html = read_file(tmp.path(), "2020/04/01/whats-jekyll/index.html");
    // The post title "What's Jekyll?" should render (may use smart quote or HTML entity)
    let has_title = html.contains("What's Jekyll?")
        || html.contains("What&#39;s Jekyll?")
        || html.contains("What&rsquo;s Jekyll?")
        || html.contains("What\u{2019}s Jekyll?");
    assert!(
        has_title,
        "What's Jekyll post should render title with apostrophe"
    );
}
