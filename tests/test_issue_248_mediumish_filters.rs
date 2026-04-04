//! Integration tests for issue #248: Mediumish Liquid filter support.
//!
//! Verifies that `url_escape` and `camelcase` filters work end-to-end.
//! Uses self-contained fixtures (no dependency on websites/ directory).

use std::fs;
use std::path::Path;
use std::process::Command;

/// Create a minimal Mediumish-style site fixture that exercises url_escape
/// and camelcase filters in a category sidebar.
fn create_mediumish_fixture(root: &Path) {
    // _config.yml
    fs::write(
        root.join("_config.yml"),
        "\
title: Mediumish Filter Test
permalink: /:categories/:title/
",
    )
    .unwrap();

    fs::create_dir_all(root.join("_layouts")).unwrap();
    fs::create_dir_all(root.join("_includes")).unwrap();
    fs::create_dir_all(root.join("_posts")).unwrap();

    // _includes/sidebar.html -- uses url_escape and camelcase filters
    fs::write(
        root.join("_includes/sidebar.html"),
        r#"<div class="fortags">
<ul class="tags">
{% assign categories = site.categories %}
{% for category in categories %}
<li><a class="tag" href="/categories#{{ category[0] | url_escape }}">{{ category[0] | camelcase }}</a></li>
{% endfor %}
</ul>
</div>
"#,
    )
    .unwrap();

    // _layouts/default.html -- includes the sidebar
    fs::write(
        root.join("_layouts/default.html"),
        r#"<!DOCTYPE html>
<html>
<head><title>{{ page.title }}</title></head>
<body>
{% include sidebar.html %}
<main>{{ content }}</main>
</body>
</html>
"#,
    )
    .unwrap();

    // _layouts/post.html
    fs::write(
        root.join("_layouts/post.html"),
        "---\nlayout: default\n---\n<article>{{ content }}</article>\n",
    )
    .unwrap();

    // index.html -- uses default layout
    fs::write(
        root.join("index.html"),
        "---\nlayout: default\ntitle: Home\n---\n<h1>Welcome</h1>\n",
    )
    .unwrap();

    // Posts with categories to populate site.categories
    fs::write(
        root.join("_posts/2023-01-01-hello-world.md"),
        "---\ntitle: Hello World\nlayout: post\ncategories: [Jekyll, tutorial]\n---\nHello world content.\n",
    )
    .unwrap();

    fs::write(
        root.join("_posts/2023-02-15-web-dev-tips.md"),
        "---\ntitle: Web Dev Tips\nlayout: post\ncategories: [web development]\n---\nWeb development tips with non-ASCII: café, naïve, über.\n",
    )
    .unwrap();
}

/// Build a fixture site and return the temp dir with the output.
fn build_fixture(fixture_root: &Path) -> tempfile::TempDir {
    let dest = tempfile::TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .args(["build", "--source"])
        .arg(fixture_root)
        .arg("--destination")
        .arg(dest.path())
        .output()
        .expect("failed to run rustkyll");
    assert!(
        output.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    dest
}

#[test]
fn test_mediumish_no_unknown_filter_warnings() {
    let fixture = tempfile::TempDir::new().unwrap();
    create_mediumish_fixture(fixture.path());

    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .args(["build", "--source"])
        .arg(fixture.path())
        .arg("--destination")
        .arg(fixture.path().join("_site"))
        .output()
        .expect("failed to run rustkyll");
    assert!(
        output.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
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
    let fixture = tempfile::TempDir::new().unwrap();
    create_mediumish_fixture(fixture.path());
    let dest = build_fixture(fixture.path());

    let index_path = dest.path().join("index.html");
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
    let fixture = tempfile::TempDir::new().unwrap();
    create_mediumish_fixture(fixture.path());
    let dest = build_fixture(fixture.path());

    let index_path = dest.path().join("index.html");
    assert!(index_path.exists(), "index.html should exist");
    let html = std::fs::read_to_string(&index_path).unwrap();

    // The category sidebar should have anchor tags with href to categories
    assert!(
        html.contains("/categories#"),
        "index.html should contain category anchor links (href to /categories#...)"
    );

    // Verify we have the fortags div with <a> tags
    let fortags_start = html.find("fortags");
    assert!(
        fortags_start.is_some(),
        "index.html should contain the 'fortags' category sidebar section"
    );

    let section = &html[fortags_start.unwrap()..];
    let section_end = section.find("</div>").unwrap_or(section.len());
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
    let fixture = tempfile::TempDir::new().unwrap();
    create_mediumish_fixture(fixture.path());
    let dest = build_fixture(fixture.path());

    let mut checked = 0;
    check_dir_for_raw_liquid(dest.path(), &mut checked);
    assert!(
        checked > 0,
        "Should have checked at least one HTML file in the build output"
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
