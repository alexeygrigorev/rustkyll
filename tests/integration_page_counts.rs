//! Integration tests for page count matching between rustkyll and Jekyll.
//!
//! Uses self-contained fixtures (no dependency on websites/ directory).

use std::fs;
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

/// Build a site from a fixture directory and return the output temp dir.
fn build_fixture(source: &Path) -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .args(["build", "--source"])
        .arg(source)
        .arg("--destination")
        .arg(tmp.path())
        .output()
        .expect("failed to run rustkyll");
    assert!(
        output.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    tmp
}

/// Check if any subdirectory under `parent` contains an `index.html`.
fn has_subdir_with_index(parent: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("index.html").exists() {
                return true;
            }
        }
    }
    false
}

// ========================================================================
// Portfolio site build and content verification (replaces alexeygrigorev test)
// ========================================================================

/// Create a minimal portfolio-style fixture site with multiple pages,
/// static CSS, CDN links, and data-driven content.
fn create_portfolio_fixture(root: &Path) {
    fs::write(
        root.join("_config.yml"),
        "\
title: Portfolio Test
permalink: /:title.html
",
    )
    .unwrap();

    fs::create_dir_all(root.join("_layouts")).unwrap();
    fs::create_dir_all(root.join("_data")).unwrap();
    fs::create_dir_all(root.join("assets/css")).unwrap();

    // Static CSS file that should be copied to output
    fs::write(root.join("assets/css/main.css"), "body { margin: 0; }\n").unwrap();

    // Data file for data-driven rendering
    fs::write(
        root.join("_data/projects.yml"),
        "- name: Project Alpha\n  url: https://example.com/alpha\n- name: Проект Бета\n  url: https://example.com/beta\n",
    )
    .unwrap();

    // Default layout with Font Awesome CDN link
    fs::write(
        root.join("_layouts/default.html"),
        r#"<!DOCTYPE html>
<html>
<head>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.0.0/css/all.min.css">
<link rel="stylesheet" href="/assets/css/main.css">
<title>{{ page.title }}</title>
</head>
<body>
{{ content }}
</body>
</html>
"#,
    )
    .unwrap();

    // Post layout
    fs::write(
        root.join("_layouts/post.html"),
        "---\nlayout: default\n---\n<article>{{ content }}</article>\n",
    )
    .unwrap();

    // index.html with data-driven content
    fs::write(
        root.join("index.html"),
        r#"---
layout: default
title: Home
---
<h1>Welcome</h1>
<h2>Projects</h2>
<ul>
{% for project in site.data.projects %}
<li><a href="{{ project.url }}">{{ project.name }}</a></li>
{% endfor %}
</ul>
<h2>Courses</h2>
<h2>Contribution activity</h2>
"#,
    )
    .unwrap();

    // Additional pages
    for page in &["courses", "cv", "projects", "services", "about"] {
        fs::write(
            root.join(format!("{}.html", page)),
            format!(
                "---\nlayout: default\ntitle: {}\n---\n<h1>{}</h1>\n",
                page, page
            ),
        )
        .unwrap();
    }

    // Posts
    fs::create_dir_all(root.join("_posts")).unwrap();
    fs::write(
        root.join("_posts/2023-01-15-first-post.md"),
        "---\ntitle: First Post\nlayout: post\n---\nHello world.\n",
    )
    .unwrap();
    fs::write(
        root.join("_posts/2023-06-20-café-review.md"),
        "---\ntitle: Café Review\nlayout: post\n---\nA review of cafés with über quality.\n",
    )
    .unwrap();
}

#[test]
fn test_portfolio_site_builds_and_has_correct_content() {
    let fixture = tempfile::TempDir::new().unwrap();
    create_portfolio_fixture(fixture.path());
    let dest = build_fixture(fixture.path());

    // Verify expected pages exist
    assert!(
        dest.path().join("index.html").exists(),
        "index.html should exist"
    );
    assert!(
        dest.path().join("courses.html").exists(),
        "courses.html should exist"
    );
    assert!(dest.path().join("cv.html").exists(), "cv.html should exist");
    assert!(
        dest.path().join("projects.html").exists(),
        "projects.html should exist"
    );
    assert!(
        dest.path().join("services.html").exists(),
        "services.html should exist"
    );

    // Verify correct page count: 5 pages + index + about + 2 posts = 8
    let count = count_html_files(dest.path());
    assert_eq!(count, 8, "expected 8 HTML files, got {}", count);

    // Verify homepage has Font Awesome CDN link
    let index_html = fs::read_to_string(dest.path().join("index.html")).unwrap();
    assert!(
        index_html.contains("cdnjs.cloudflare.com/ajax/libs/font-awesome"),
        "Homepage should reference Font Awesome CSS from CDN"
    );

    // Verify homepage has main.css link
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

    // Verify data-driven content renders
    assert!(
        index_html.contains("Project Alpha"),
        "Homepage should render data-driven project names"
    );
    assert!(
        index_html.contains("Проект Бета"),
        "Homepage should render non-ASCII data-driven content"
    );

    // Verify static CSS file is copied
    assert!(
        dest.path().join("assets/css/main.css").exists(),
        "main.css should be copied to output"
    );
}

// ========================================================================
// Chirpy-style site with tag/category generators
// ========================================================================

/// Create a minimal chirpy-style fixture that generates tag and category
/// archive pages from posts.
fn create_chirpy_fixture(root: &Path) {
    fs::write(
        root.join("_config.yml"),
        "\
title: Chirpy Test
permalink: /posts/:title/
",
    )
    .unwrap();

    fs::create_dir_all(root.join("_layouts")).unwrap();
    fs::create_dir_all(root.join("_posts")).unwrap();

    // Default layout
    fs::write(
        root.join("_layouts/default.html"),
        "<!DOCTYPE html><html><body>{{ content }}</body></html>\n",
    )
    .unwrap();

    // Post layout
    fs::write(
        root.join("_layouts/post.html"),
        "---\nlayout: default\n---\n<article>{{ content }}</article>\n",
    )
    .unwrap();

    // index.html
    fs::write(
        root.join("index.html"),
        "---\nlayout: default\ntitle: Home\n---\n<h1>Posts</h1>\n{% for post in site.posts %}<a href=\"{{ post.url }}\">{{ post.title }}</a>\n{% endfor %}\n",
    )
    .unwrap();

    // Several posts with categories and tags
    fs::write(
        root.join("_posts/2023-01-01-getting-started.md"),
        "---\ntitle: Getting Started\nlayout: post\ncategories: [tutorial]\ntags: [beginner, setup]\n---\nA getting started guide.\n",
    )
    .unwrap();
    fs::write(
        root.join("_posts/2023-03-15-advanced-tips.md"),
        "---\ntitle: Advanced Tips\nlayout: post\ncategories: [tutorial]\ntags: [advanced]\n---\nAdvanced tips and tricks.\n",
    )
    .unwrap();
    fs::write(
        root.join("_posts/2023-06-20-news-update.md"),
        "---\ntitle: News Update\nlayout: post\ncategories: [news]\ntags: [update, announcement]\n---\nLatest news with émojis and naïve text.\n",
    )
    .unwrap();
}

#[test]
fn test_chirpy_style_site_builds_with_posts() {
    let fixture = tempfile::TempDir::new().unwrap();
    create_chirpy_fixture(fixture.path());
    let dest = build_fixture(fixture.path());

    // Verify HTML files are generated: index + 3 posts = 4
    let count = count_html_files(dest.path());
    assert!(
        count >= 4,
        "expected at least 4 HTML files (index + 3 posts), got {}",
        count
    );

    // Verify key pages exist
    assert!(
        dest.path().join("index.html").exists(),
        "index.html should exist"
    );

    // Verify at least one post page exists under posts/*/index.html
    let posts_dir = dest.path().join("posts");
    if posts_dir.exists() {
        assert!(
            has_subdir_with_index(&posts_dir),
            "At least one post should exist under posts/*/index.html"
        );
    }

    // Verify index.html is rendered (not raw Liquid)
    let index_html = fs::read_to_string(dest.path().join("index.html")).unwrap();
    assert!(
        !index_html.contains("{{ site."),
        "index.html should not contain raw Liquid output tags"
    );
    assert!(
        !index_html.contains("{% "),
        "index.html should not contain raw Liquid block tags"
    );

    // Verify post titles appear in the rendered index
    assert!(
        index_html.contains("Getting Started"),
        "index should list 'Getting Started' post"
    );
    assert!(
        index_html.contains("News Update"),
        "index should list 'News Update' post"
    );
}
