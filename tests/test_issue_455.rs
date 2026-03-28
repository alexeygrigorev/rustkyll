//! TDD tests for issue 455: page.id must be truthy for collection items in Liquid.
//!
//! beautiful-jekyll uses `{% if page.id %}` to distinguish posts from pages.
//! Jekyll sets page.id for collection documents (e.g., `/2020/02/26/flake-it-till-you-make-it`).
//! This must evaluate as truthy in Liquid templates.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Create a minimal site fixture for testing page.id in collection items.
fn create_minimal_site(tmp: &Path) {
    // _config.yml
    fs::write(
        tmp.join("_config.yml"),
        "title: Test Site\npermalink: /:year/:month/:day/:title/\n",
    )
    .unwrap();

    // _layouts/post.html -- uses {% if page.id %} like beautiful-jekyll
    fs::create_dir_all(tmp.join("_layouts")).unwrap();
    fs::write(
        tmp.join("_layouts/post.html"),
        r#"{% if page.id %}<meta property="og:type" content="article">{% else %}<meta property="og:type" content="website">{% endif %}
{{ content }}"#,
    )
    .unwrap();

    // A post
    fs::create_dir_all(tmp.join("_posts")).unwrap();
    fs::write(
        tmp.join("_posts/2020-02-26-flake-it-till-you-make-it.md"),
        "---\nlayout: post\ntitle: Flake it till you make it\n---\nHello world.\n",
    )
    .unwrap();
}

#[test]
fn test_page_id_truthy_in_post_template() {
    let tmp = tempfile::TempDir::new().unwrap();
    create_minimal_site(tmp.path());

    let config = rustkyll::config::SiteConfig::from_file(&tmp.path().join("_config.yml")).unwrap();
    let (posts, _warnings) =
        rustkyll::collection::load_collection("posts", tmp.path(), &config).unwrap();

    assert!(!posts.is_empty(), "Should have at least one post");

    // Verify the CollectionItem has a non-empty id
    let post = &posts[0];
    assert!(
        !post.id.is_empty(),
        "Post should have a non-empty id, got: {:?}",
        post.id
    );
    assert!(
        post.id.contains("flake-it-till-you-make-it"),
        "Post id should contain slug, got: {:?}",
        post.id
    );

    // Build site context and layout engine
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

    // Generate collection pages
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
        "Should generate at least 1 post, got {} (errors: {:?})",
        result.generated,
        result.errors
    );

    // Find the generated HTML file
    let post_html_path = output_dir
        .path()
        .join("2020/02/26/flake-it-till-you-make-it/index.html");
    assert!(
        post_html_path.exists(),
        "Post HTML should exist at {:?}",
        post_html_path
    );

    let html = fs::read_to_string(&post_html_path).unwrap();

    // The critical check: page.id should be truthy, so og:type should be "article"
    assert!(
        html.contains(r#"og:type" content="article"#),
        "Post should have og:type=article (page.id truthy), but got:\n{}",
        html
    );
    assert!(
        !html.contains(r#"og:type" content="website"#),
        "Post should NOT have og:type=website (page.id falsy), but got:\n{}",
        html
    );
}

#[test]
fn test_page_id_value_in_template() {
    // Test that page.id renders the correct value, not just that it's truthy
    let tmp = tempfile::TempDir::new().unwrap();

    fs::write(
        tmp.path().join("_config.yml"),
        "title: Test\npermalink: /:year/:month/:day/:title/\n",
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join("_layouts")).unwrap();
    fs::write(
        tmp.path().join("_layouts/post.html"),
        "ID={{ page.id }}\n{{ content }}",
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join("_posts")).unwrap();
    fs::write(
        tmp.path()
            .join("_posts/2020-02-26-flake-it-till-you-make-it.md"),
        "---\nlayout: post\ntitle: Flake it\n---\nContent.\n",
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

    let html = fs::read_to_string(
        output_dir
            .path()
            .join("2020/02/26/flake-it-till-you-make-it/index.html"),
    )
    .unwrap();

    // page.id should render as the post's id path
    assert!(
        html.contains("ID=/2020/02/26/flake-it-till-you-make-it"),
        "page.id should render as the post ID path, got:\n{}",
        html
    );
}

#[test]
fn test_page_id_unicode_post() {
    // Test with non-ASCII content to catch encoding issues
    let tmp = tempfile::TempDir::new().unwrap();

    fs::write(
        tmp.path().join("_config.yml"),
        "title: Test\npermalink: /:year/:month/:day/:title/\n",
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join("_layouts")).unwrap();
    fs::write(
        tmp.path().join("_layouts/post.html"),
        "{% if page.id %}TRUTHY{% else %}FALSY{% endif %}\n{{ content }}",
    )
    .unwrap();

    fs::create_dir_all(tmp.path().join("_posts")).unwrap();
    fs::write(
        tmp.path().join("_posts/2021-05-10-cafe-uebersicht.md"),
        "---\nlayout: post\ntitle: Caf\u{00E9} \u{00DC}bersicht\n---\nInhalt mit Umlauten: \u{00E4}\u{00F6}\u{00FC}\n",
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

    let html = fs::read_to_string(
        output_dir
            .path()
            .join("2021/05/10/cafe-uebersicht/index.html"),
    )
    .unwrap();

    assert!(
        html.contains("TRUTHY"),
        "Unicode post should have truthy page.id, got:\n{}",
        html
    );
}
