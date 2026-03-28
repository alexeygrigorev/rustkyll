//! TDD tests for issue 357: page.id must be set for collection items so that
//! conditional resource loading patterns like `{% if site.theme_settings.katex and page.id %}`
//! work correctly (as used in type-theme's _includes/head.html).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Create a site fixture that mimics type-theme's conditional KaTeX loading.
/// The include uses `{% if site.theme_settings.katex and page.id %}` to decide
/// whether to inject KaTeX CSS/JS on post pages.
fn create_type_theme_fixture(tmp: &Path) {
    fs::write(
        tmp.join("_config.yml"),
        "\
title: Type Theme Test
permalink: /:year/:month/:day/:title.html
theme_settings:
  katex: true
  title: Type Theme Test
",
    )
    .unwrap();

    fs::create_dir_all(tmp.join("_layouts")).unwrap();
    fs::create_dir_all(tmp.join("_includes")).unwrap();

    // _includes/head.html -- conditional KaTeX loading like type-theme
    fs::write(
        tmp.join("_includes/head.html"),
        "\
<head>
<meta charset=\"utf-8\">
{% if site.theme_settings.katex and page.id %}
<link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/npm/katex@0.10.2/dist/katex.min.css\">
<script src=\"https://cdn.jsdelivr.net/npm/katex@0.10.2/dist/katex.min.js\"></script>
{% endif %}
</head>
",
    )
    .unwrap();

    // post layout includes head.html
    fs::write(
        tmp.join("_layouts/post.html"),
        "{% include head.html %}\n<body>{{ content }}</body>\n",
    )
    .unwrap();

    // default layout (for standalone pages) also includes head.html
    fs::write(
        tmp.join("_layouts/default.html"),
        "{% include head.html %}\n<body>{{ content }}</body>\n",
    )
    .unwrap();

    // A post that should get KaTeX
    fs::create_dir_all(tmp.join("_posts")).unwrap();
    fs::write(
        tmp.join("_posts/2014-11-30-sample-post.md"),
        "---\nlayout: post\ntitle: Sample Post\n---\nHello from the sample post.\n",
    )
    .unwrap();

    // A standalone page that should NOT get KaTeX (no page.id)
    fs::write(
        tmp.join("index.md"),
        "---\nlayout: default\ntitle: Home\n---\nWelcome home.\n",
    )
    .unwrap();
}

#[test]
fn test_page_id_enables_conditional_katex_on_posts() {
    let tmp = tempfile::TempDir::new().unwrap();
    create_type_theme_fixture(tmp.path());

    let config = rustkyll::config::SiteConfig::from_file(&tmp.path().join("_config.yml")).unwrap();
    let (posts, _) = rustkyll::collection::load_collection("posts", tmp.path(), &config).unwrap();

    assert!(!posts.is_empty(), "Should have at least one post");
    let post = &posts[0];
    assert!(
        !post.id.is_empty(),
        "Post should have a non-empty id, got: {:?}",
        post.id
    );
    assert!(
        post.id.contains("sample-post"),
        "Post id should contain slug 'sample-post', got: {:?}",
        post.id
    );

    // Build site and generate pages
    let data_tree = rustkyll::data::DataTree::new();
    let mut colls = HashMap::new();
    colls.insert("posts".to_string(), posts.clone());
    let site_context =
        rustkyll::generator::build_site_context(&config, &colls, &data_tree, None, &[]);
    let layout_engine = rustkyll::template::layout::LayoutEngine::new(
        &tmp.path().join("_layouts"),
        &tmp.path().join("_includes"),
    )
    .unwrap();

    let output_dir = tempfile::TempDir::new().unwrap();
    let result = rustkyll::generator::generate_collection_pages(
        &posts,
        "posts",
        &config,
        &layout_engine,
        &site_context,
        output_dir.path(),
    )
    .unwrap();

    assert!(
        result.generated >= 1,
        "Should generate at least 1 post page"
    );

    // Read the generated post HTML
    let post_html_path = output_dir.path().join("2014/11/30/sample-post.html");
    assert!(
        post_html_path.exists(),
        "Post HTML should exist at {:?}",
        post_html_path
    );

    let html = fs::read_to_string(&post_html_path).unwrap();

    // The critical check: page.id is truthy AND site.theme_settings.katex is true,
    // so the KaTeX link and script tags should be present.
    assert!(
        html.contains("katex.min.css"),
        "Post page should include KaTeX CSS (page.id is truthy), but got:\n{}",
        html
    );
    assert!(
        html.contains("katex.min.js"),
        "Post page should include KaTeX JS (page.id is truthy), but got:\n{}",
        html
    );
}

#[test]
fn test_page_id_absent_on_standalone_pages_skips_katex() {
    // Standalone pages (non-collection) should NOT have page.id set,
    // so the conditional KaTeX block should not render.
    let tmp = tempfile::TempDir::new().unwrap();
    create_type_theme_fixture(tmp.path());

    let config = rustkyll::config::SiteConfig::from_file(&tmp.path().join("_config.yml")).unwrap();
    let (posts, _) = rustkyll::collection::load_collection("posts", tmp.path(), &config).unwrap();

    let data_tree = rustkyll::data::DataTree::new();
    let mut colls = HashMap::new();
    colls.insert("posts".to_string(), posts.clone());
    let site_context =
        rustkyll::generator::build_site_context(&config, &colls, &data_tree, None, &[]);
    let layout_engine = rustkyll::template::layout::LayoutEngine::new(
        &tmp.path().join("_layouts"),
        &tmp.path().join("_includes"),
    )
    .unwrap();

    let output_dir = tempfile::TempDir::new().unwrap();

    // Load and generate standalone pages
    let (pages, _errs) = rustkyll::collection::load_pages(tmp.path(), &config).unwrap();
    assert!(
        !pages.is_empty(),
        "Should have at least one standalone page"
    );

    rustkyll::generator::generate_pages(&pages, &layout_engine, &site_context, output_dir.path())
        .unwrap();

    let index_path = output_dir.path().join("index.html");
    assert!(
        index_path.exists(),
        "Index page should exist at {:?}",
        index_path
    );

    let html = fs::read_to_string(&index_path).unwrap();

    // Standalone pages do NOT have page.id, so KaTeX should NOT appear
    assert!(
        !html.contains("katex.min.css"),
        "Standalone page should NOT include KaTeX CSS (no page.id), but got:\n{}",
        html
    );
    assert!(
        !html.contains("katex.min.js"),
        "Standalone page should NOT include KaTeX JS (no page.id), but got:\n{}",
        html
    );
}

#[test]
fn test_page_id_format_matches_jekyll_date_based_path() {
    // Verify the exact page.id format: /YYYY/MM/DD/slug (no .html extension)
    let tmp = tempfile::TempDir::new().unwrap();

    fs::write(
        tmp.path().join("_config.yml"),
        "title: Test\npermalink: /:year/:month/:day/:title.html\n",
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join("_layouts")).unwrap();
    fs::write(
        tmp.path().join("_layouts/post.html"),
        "ID=[{{ page.id }}]\n{{ content }}",
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join("_posts")).unwrap();
    fs::write(
        tmp.path().join("_posts/2014-11-30-sample-post.md"),
        "---\nlayout: post\ntitle: Sample Post\n---\nContent.\n",
    )
    .unwrap();

    let config = rustkyll::config::SiteConfig::from_file(&tmp.path().join("_config.yml")).unwrap();
    let (posts, _) = rustkyll::collection::load_collection("posts", tmp.path(), &config).unwrap();

    let data_tree = rustkyll::data::DataTree::new();
    let mut colls = HashMap::new();
    colls.insert("posts".to_string(), posts.clone());
    let site_context =
        rustkyll::generator::build_site_context(&config, &colls, &data_tree, None, &[]);
    let layout_engine = rustkyll::template::layout::LayoutEngine::new(
        &tmp.path().join("_layouts"),
        &tmp.path().join("_includes"),
    )
    .unwrap();

    let output_dir = tempfile::TempDir::new().unwrap();
    rustkyll::generator::generate_collection_pages(
        &posts,
        "posts",
        &config,
        &layout_engine,
        &site_context,
        output_dir.path(),
    )
    .unwrap();

    let html = fs::read_to_string(output_dir.path().join("2014/11/30/sample-post.html")).unwrap();

    // Jekyll page.id format is /YYYY/MM/DD/slug (no extension)
    assert!(
        html.contains("ID=[/2014/11/30/sample-post]"),
        "page.id should be /2014/11/30/sample-post, got:\n{}",
        html
    );
}

#[test]
fn test_page_id_unicode_slug() {
    // Verify page.id works with non-ASCII characters in slug
    let tmp = tempfile::TempDir::new().unwrap();

    fs::write(
        tmp.path().join("_config.yml"),
        "title: Test\npermalink: /:year/:month/:day/:title.html\n",
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join("_layouts")).unwrap();
    fs::write(
        tmp.path().join("_layouts/post.html"),
        "{% if page.id %}HAS_ID{% else %}NO_ID{% endif %}\n{{ content }}",
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join("_posts")).unwrap();
    fs::write(
        tmp.path().join("_posts/2023-01-15-caf\u{00e9}-review.md"),
        "---\nlayout: post\ntitle: Caf\u{00e9} Review\n---\nGreat caf\u{00e9}.\n",
    )
    .unwrap();

    let config = rustkyll::config::SiteConfig::from_file(&tmp.path().join("_config.yml")).unwrap();
    let (posts, _) = rustkyll::collection::load_collection("posts", tmp.path(), &config).unwrap();

    assert!(!posts.is_empty(), "Should load unicode post");
    assert!(
        !posts[0].id.is_empty(),
        "Unicode post should have non-empty id"
    );

    let data_tree = rustkyll::data::DataTree::new();
    let mut colls = HashMap::new();
    colls.insert("posts".to_string(), posts.clone());
    let site_context =
        rustkyll::generator::build_site_context(&config, &colls, &data_tree, None, &[]);
    let layout_engine = rustkyll::template::layout::LayoutEngine::new(
        &tmp.path().join("_layouts"),
        &tmp.path().join("_includes"),
    )
    .unwrap();

    let output_dir = tempfile::TempDir::new().unwrap();
    rustkyll::generator::generate_collection_pages(
        &posts,
        "posts",
        &config,
        &layout_engine,
        &site_context,
        output_dir.path(),
    )
    .unwrap();

    // Find the generated file (URL might be percent-encoded)
    let possible_paths = [
        output_dir.path().join("2023/01/15/caf\u{00e9}-review.html"),
        output_dir.path().join("2023/01/15/caf%C3%A9-review.html"),
    ];

    let html = possible_paths
        .iter()
        .find_map(|p| fs::read_to_string(p).ok())
        .expect("Unicode post HTML should exist at one of the expected paths");

    assert!(
        html.contains("HAS_ID"),
        "Unicode post should have truthy page.id, got:\n{}",
        html
    );
}
