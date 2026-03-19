//! Integration tests for pagination support (Issue 98).
//!
//! Tests the full pagination pipeline: config extraction, paginator variable
//! in templates, page generation, and prev/next navigation links.

use std::collections::{BTreeMap, HashMap};
use std::fs;

use rustkyll::collection::{self, CollectionItem, Page};
use rustkyll::config::SiteConfig;
use rustkyll::generator;
use rustkyll::pagination::{self, PaginationConfig};
use rustkyll::template::layout::LayoutEngine;

/// Create a temporary site with posts and pagination config, build it,
/// and return the output directory path.
fn build_paginated_site(
    num_posts: usize,
    per_page: usize,
    paginate_path: &str,
    index_content: &str,
    layouts: &HashMap<String, String>,
) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let site_dir = dir.path();

    // Create _config.yml
    let config_yaml = format!(
        "url: https://example.com\ntitle: Test Blog\npaginate: {}\npaginate_path: \"{}\"\n",
        per_page, paginate_path
    );
    fs::write(site_dir.join("_config.yml"), &config_yaml).unwrap();

    // Create _posts directory and posts
    let posts_dir = site_dir.join("_posts");
    fs::create_dir_all(&posts_dir).unwrap();
    for i in 1..=num_posts {
        let date = format!("2024-01-{:02}", i.min(28));
        let filename = format!("{}-post-{}.md", date, i);
        let content = format!("---\ntitle: Post {}\n---\n\nContent of post {}.\n", i, i);
        fs::write(posts_dir.join(&filename), &content).unwrap();
    }

    // Create index.html
    fs::write(site_dir.join("index.html"), index_content).unwrap();

    // Create _layouts directory
    let layouts_dir = site_dir.join("_layouts");
    fs::create_dir_all(&layouts_dir).unwrap();
    for (name, content) in layouts {
        fs::write(layouts_dir.join(format!("{}.html", name)), content).unwrap();
    }

    // Create _includes directory (empty but must exist)
    let includes_dir = site_dir.join("_includes");
    fs::create_dir_all(&includes_dir).unwrap();

    dir
}

#[test]
fn test_pagination_config_extraction() {
    let yaml = "paginate: 5\npaginate_path: /page:num/\n";
    let config = SiteConfig::from_yaml_str(yaml).unwrap();
    let pagination = PaginationConfig::from_config(&config).unwrap();
    assert_eq!(pagination.per_page, 5);
    assert_eq!(pagination.paginate_path, "/page:num/");
}

#[test]
fn test_pagination_config_default_path() {
    let yaml = "paginate: 10\n";
    let config = SiteConfig::from_yaml_str(yaml).unwrap();
    let pagination = PaginationConfig::from_config(&config).unwrap();
    assert_eq!(pagination.per_page, 10);
    assert_eq!(pagination.paginate_path, "/blog/page:num/");
}

#[test]
fn test_pagination_not_enabled_without_config() {
    let yaml = "url: https://example.com\n";
    let config = SiteConfig::from_yaml_str(yaml).unwrap();
    assert!(PaginationConfig::from_config(&config).is_none());
}

#[test]
fn test_paginated_index_has_paginator_posts() {
    let index_content = r#"---
layout: default
title: Home
---
{% for post in paginator.posts %}
<div class="post">
  <h2><a href="{{ post.url }}">{{ post.title }}</a></h2>
</div>
{% endfor %}
"#;

    let mut layouts = HashMap::new();
    layouts.insert(
        "default".to_string(),
        "<html><body>{{ content }}</body></html>".to_string(),
    );

    let dir = build_paginated_site(7, 3, "/page:num/", index_content, &layouts);
    let site_dir = dir.path();
    let output_dir = site_dir.join("_site");
    fs::create_dir_all(&output_dir).unwrap();

    // Load config
    let config = SiteConfig::from_file(&site_dir.join("_config.yml")).unwrap();
    let pagination_config = PaginationConfig::from_config(&config).unwrap();

    // Load posts
    let (posts, _) = collection::load_collection("posts", site_dir, &config).unwrap();
    assert_eq!(posts.len(), 7);

    // Load pages
    let (pages, _) = collection::load_pages(site_dir, &config).unwrap();
    let index_page = pagination::find_index_page(&pages).unwrap();

    // Build site context
    let mut collections: HashMap<String, Vec<CollectionItem>> = HashMap::new();
    collections.insert("posts".to_string(), posts.clone());
    let data = BTreeMap::new();
    let site_context =
        generator::build_site_context(&config, &collections, &data, Some(site_dir), &pages);

    // Create layout engine
    let layout_engine =
        LayoutEngine::new(&site_dir.join("_layouts"), &site_dir.join("_includes")).unwrap();
    let cached_site = LayoutEngine::build_cached_site_context(&site_context);

    // Generate pagination
    let count = pagination::generate_pagination_pages(
        &posts,
        &pagination_config,
        index_page,
        &layout_engine,
        &cached_site,
        &config,
        &output_dir,
    )
    .unwrap();

    // 7 posts / 3 per page = 3 pages
    assert_eq!(count, 3, "Should generate 3 pagination pages");

    // Check page 1 (index.html) has 3 posts
    let index_html = fs::read_to_string(output_dir.join("index.html")).unwrap();
    let post_count = index_html.matches("<div class=\"post\">").count();
    assert_eq!(post_count, 3, "Page 1 should have 3 posts");

    // Check page 2 exists and has 3 posts
    let page2_html = fs::read_to_string(output_dir.join("page2/index.html")).unwrap();
    let post_count = page2_html.matches("<div class=\"post\">").count();
    assert_eq!(post_count, 3, "Page 2 should have 3 posts");

    // Check page 3 exists and has 1 post
    let page3_html = fs::read_to_string(output_dir.join("page3/index.html")).unwrap();
    let post_count = page3_html.matches("<div class=\"post\">").count();
    assert_eq!(post_count, 1, "Page 3 should have 1 post");

    // Page 4 should NOT exist
    assert!(
        !output_dir.join("page4/index.html").exists(),
        "Page 4 should not exist"
    );
}

#[test]
fn test_paginator_navigation_links() {
    let index_content = r#"---
layout: default
title: Home
---
{% if paginator.previous_page %}
<a class="prev" href="{{ paginator.previous_page_path }}">Previous</a>
{% endif %}
<span class="current">Page {{ paginator.page }} of {{ paginator.total_pages }}</span>
{% if paginator.next_page %}
<a class="next" href="{{ paginator.next_page_path }}">Next</a>
{% endif %}
"#;

    let mut layouts = HashMap::new();
    layouts.insert(
        "default".to_string(),
        "<html><body>{{ content }}</body></html>".to_string(),
    );

    let dir = build_paginated_site(8, 3, "/page:num/", index_content, &layouts);
    let site_dir = dir.path();
    let output_dir = site_dir.join("_site");
    fs::create_dir_all(&output_dir).unwrap();

    let config = SiteConfig::from_file(&site_dir.join("_config.yml")).unwrap();
    let pagination_config = PaginationConfig::from_config(&config).unwrap();
    let (posts, _) = collection::load_collection("posts", site_dir, &config).unwrap();
    let (pages, _) = collection::load_pages(site_dir, &config).unwrap();
    let index_page = pagination::find_index_page(&pages).unwrap();

    let mut collections: HashMap<String, Vec<CollectionItem>> = HashMap::new();
    collections.insert("posts".to_string(), posts.clone());
    let data = BTreeMap::new();
    let site_context =
        generator::build_site_context(&config, &collections, &data, Some(site_dir), &pages);

    let layout_engine =
        LayoutEngine::new(&site_dir.join("_layouts"), &site_dir.join("_includes")).unwrap();
    let cached_site = LayoutEngine::build_cached_site_context(&site_context);

    pagination::generate_pagination_pages(
        &posts,
        &pagination_config,
        index_page,
        &layout_engine,
        &cached_site,
        &config,
        &output_dir,
    )
    .unwrap();

    // Page 1: no previous, has next
    let page1 = fs::read_to_string(output_dir.join("index.html")).unwrap();
    assert!(
        !page1.contains("class=\"prev\""),
        "Page 1 should not have prev link"
    );
    assert!(
        page1.contains("class=\"next\""),
        "Page 1 should have next link"
    );
    assert!(
        page1.contains("Page 1 of 3"),
        "Page 1 should show 'Page 1 of 3'"
    );
    assert!(
        page1.contains("/page2/"),
        "Page 1 next link should point to /page2/"
    );

    // Page 2: has previous and next
    let page2 = fs::read_to_string(output_dir.join("page2/index.html")).unwrap();
    assert!(
        page2.contains("class=\"prev\""),
        "Page 2 should have prev link"
    );
    assert!(
        page2.contains("class=\"next\""),
        "Page 2 should have next link"
    );
    assert!(
        page2.contains("Page 2 of 3"),
        "Page 2 should show 'Page 2 of 3'"
    );
    // Page 2's previous should be / (the root)
    assert!(
        page2.contains("href=\"/\""),
        "Page 2 prev link should point to /"
    );
    assert!(
        page2.contains("/page3/"),
        "Page 2 next link should point to /page3/"
    );

    // Page 3: has previous, no next
    let page3 = fs::read_to_string(output_dir.join("page3/index.html")).unwrap();
    assert!(
        page3.contains("class=\"prev\""),
        "Page 3 should have prev link"
    );
    assert!(
        !page3.contains("class=\"next\""),
        "Page 3 should not have next link"
    );
    assert!(
        page3.contains("Page 3 of 3"),
        "Page 3 should show 'Page 3 of 3'"
    );
    assert!(
        page3.contains("/page2/"),
        "Page 3 prev link should point to /page2/"
    );
}

#[test]
fn test_paginator_total_posts_and_per_page() {
    let index_content = r#"---
layout: default
title: Home
---
<span>total={{ paginator.total_posts }}</span>
<span>per_page={{ paginator.per_page }}</span>
"#;

    let mut layouts = HashMap::new();
    layouts.insert(
        "default".to_string(),
        "<html><body>{{ content }}</body></html>".to_string(),
    );

    let dir = build_paginated_site(12, 5, "/page:num/", index_content, &layouts);
    let site_dir = dir.path();
    let output_dir = site_dir.join("_site");
    fs::create_dir_all(&output_dir).unwrap();

    let config = SiteConfig::from_file(&site_dir.join("_config.yml")).unwrap();
    let pagination_config = PaginationConfig::from_config(&config).unwrap();
    let (posts, _) = collection::load_collection("posts", site_dir, &config).unwrap();
    let (pages, _) = collection::load_pages(site_dir, &config).unwrap();
    let index_page = pagination::find_index_page(&pages).unwrap();

    let mut collections: HashMap<String, Vec<CollectionItem>> = HashMap::new();
    collections.insert("posts".to_string(), posts.clone());
    let data = BTreeMap::new();
    let site_context =
        generator::build_site_context(&config, &collections, &data, Some(site_dir), &pages);

    let layout_engine =
        LayoutEngine::new(&site_dir.join("_layouts"), &site_dir.join("_includes")).unwrap();
    let cached_site = LayoutEngine::build_cached_site_context(&site_context);

    pagination::generate_pagination_pages(
        &posts,
        &pagination_config,
        index_page,
        &layout_engine,
        &cached_site,
        &config,
        &output_dir,
    )
    .unwrap();

    let page1 = fs::read_to_string(output_dir.join("index.html")).unwrap();
    assert!(page1.contains("total=12"), "Should show total_posts=12");
    assert!(page1.contains("per_page=5"), "Should show per_page=5");
}

#[test]
fn test_no_pagination_without_posts() {
    let config = SiteConfig::from_yaml_str("paginate: 5\n").unwrap();
    let pagination_config = PaginationConfig::from_config(&config).unwrap();

    let index_page = Page {
        slug: "index".to_string(),
        front_matter: HashMap::new(),
        content: String::new(),
        html_content: String::new(),
        url: "/".to_string(),
        source_path: "index.html".to_string(),
    };

    let dir = tempfile::TempDir::new().unwrap();
    let output_dir = dir.path();

    let layouts_dir = dir.path().join("_layouts");
    let includes_dir = dir.path().join("_includes");
    fs::create_dir_all(&layouts_dir).unwrap();
    fs::create_dir_all(&includes_dir).unwrap();
    let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
    let site_context = liquid::Object::new();
    let cached_site = LayoutEngine::build_cached_site_context(&site_context);

    let empty_posts: Vec<CollectionItem> = vec![];
    let count = pagination::generate_pagination_pages(
        &empty_posts,
        &pagination_config,
        &index_page,
        &layout_engine,
        &cached_site,
        &config,
        output_dir,
    )
    .unwrap();

    assert_eq!(count, 0, "No pages should be generated without posts");
}

#[test]
fn test_single_page_pagination() {
    // When all posts fit on one page, pagination should still work
    // (generates page 1 with the paginator variable)
    let index_content = r#"---
layout: default
title: Home
---
{% for post in paginator.posts %}
<div class="post">{{ post.title }}</div>
{% endfor %}
<span>total_pages={{ paginator.total_pages }}</span>
"#;

    let mut layouts = HashMap::new();
    layouts.insert(
        "default".to_string(),
        "<html><body>{{ content }}</body></html>".to_string(),
    );

    let dir = build_paginated_site(3, 10, "/page:num/", index_content, &layouts);
    let site_dir = dir.path();
    let output_dir = site_dir.join("_site");
    fs::create_dir_all(&output_dir).unwrap();

    let config = SiteConfig::from_file(&site_dir.join("_config.yml")).unwrap();
    let pagination_config = PaginationConfig::from_config(&config).unwrap();
    let (posts, _) = collection::load_collection("posts", site_dir, &config).unwrap();
    let (pages, _) = collection::load_pages(site_dir, &config).unwrap();
    let index_page = pagination::find_index_page(&pages).unwrap();

    let mut collections: HashMap<String, Vec<CollectionItem>> = HashMap::new();
    collections.insert("posts".to_string(), posts.clone());
    let data = BTreeMap::new();
    let site_context =
        generator::build_site_context(&config, &collections, &data, Some(site_dir), &pages);

    let layout_engine =
        LayoutEngine::new(&site_dir.join("_layouts"), &site_dir.join("_includes")).unwrap();
    let cached_site = LayoutEngine::build_cached_site_context(&site_context);

    let count = pagination::generate_pagination_pages(
        &posts,
        &pagination_config,
        index_page,
        &layout_engine,
        &cached_site,
        &config,
        &output_dir,
    )
    .unwrap();

    assert_eq!(
        count, 1,
        "Should generate 1 page even with all posts fitting"
    );

    let index_html = fs::read_to_string(output_dir.join("index.html")).unwrap();
    assert_eq!(
        index_html.matches("<div class=\"post\">").count(),
        3,
        "Page 1 should have all 3 posts"
    );
    assert!(
        index_html.contains("total_pages=1"),
        "Should show total_pages=1"
    );

    // No page 2 should exist
    assert!(
        !output_dir.join("page2/index.html").exists(),
        "Page 2 should not exist"
    );
}

// Full site pagination tests (hyde, beautiful-jekyll) have been moved to
// integration_tests/tests/integration_pagination.rs
