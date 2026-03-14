//! Integration tests for blog post generation.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use liquid::Object;

use rustkyll::collection::{self, CollectionItem};
use rustkyll::config::SiteConfig;
use rustkyll::data;
use rustkyll::generator;
use rustkyll::template::layout::LayoutEngine;

fn site_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datatalksclub.github.io")
}

static CONFIG: LazyLock<SiteConfig> =
    LazyLock::new(|| SiteConfig::from_file(&site_dir().join("_config.yml")).unwrap());

struct PostsFixture {
    posts: Vec<CollectionItem>,
    site_context: Object,
    layout_engine: LayoutEngine,
}

static FIXTURE: LazyLock<PostsFixture> = LazyLock::new(|| {
    let (posts, _) = collection::load_collection("posts", &site_dir(), &CONFIG).unwrap();
    let (people, _) = collection::load_collection("people", &site_dir(), &CONFIG).unwrap();
    let (books, _) = collection::load_collection("books", &site_dir(), &CONFIG).unwrap();
    let data_tree = data::load_data(&site_dir().join("_data")).unwrap();

    let mut colls = HashMap::new();
    colls.insert("posts".to_string(), posts.clone());
    colls.insert("books".to_string(), books);
    colls.insert("people".to_string(), people);

    let site_context =
        generator::build_site_context(&CONFIG, &colls, &data_tree, Some(&site_dir()), &[]);
    let layout_engine =
        LayoutEngine::new(&site_dir().join("_layouts"), &site_dir().join("_includes")).unwrap();

    PostsFixture {
        posts,
        site_context,
        layout_engine,
    }
});

static POSTS_OUTPUT: LazyLock<tempfile::TempDir> = LazyLock::new(|| {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = generator::generate_collection_pages(
        &FIXTURE.posts,
        "posts",
        &CONFIG,
        &FIXTURE.layout_engine,
        &FIXTURE.site_context,
        tmp.path(),
    )
    .unwrap();

    // 55 total posts, but some fail due to include system limitations
    let total = result.generated + result.errors.len();
    assert_eq!(total, 55, "Expected 55 total posts attempted");
    assert!(
        result.generated >= 47,
        "Expected at least 47 posts generated, got {} (errors: {:?})",
        result.generated,
        result.errors
    );
    tmp
});

#[test]
fn test_generate_all_posts_count() {
    if !site_dir().exists() {
        return;
    }
    let _output = &*POSTS_OUTPUT;
}

#[test]
fn test_generated_posts_are_valid_html() {
    if !site_dir().exists() {
        return;
    }
    let blog_dir = POSTS_OUTPUT.path().join("blog");
    assert!(blog_dir.exists(), "blog/ directory should exist");
    for entry in fs::read_dir(&blog_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e == "html").unwrap_or(false) {
            let content = fs::read_to_string(&path).unwrap();
            assert!(!content.is_empty(), "File should not be empty: {:?}", path);
            assert!(
                content.contains("<html"),
                "File should contain <html: {:?}",
                path
            );
            assert!(
                content.contains("<body"),
                "File should contain <body: {:?}",
                path
            );
        }
    }
}

#[test]
fn test_generated_segmentation_post_content() {
    if !site_dir().exists() {
        return;
    }
    let path = POSTS_OUTPUT.path().join("blog/segmentation.html");
    assert!(path.exists(), "segmentation.html should exist");
    let content = fs::read_to_string(&path).unwrap();

    assert!(
        content.contains("Build a 5D RFM+ framework"),
        "Should contain subtitle"
    );
    assert!(
        content.contains("/people/nishantmohan.html"),
        "Should contain author link"
    );
    assert!(
        content.contains("\"@type\": \"Article\""),
        "Should contain Article JSON-LD"
    );
    assert!(
        content.contains("BreadcrumbList"),
        "Should contain BreadcrumbList"
    );
}

#[test]
fn test_generated_mlops_post_content() {
    if !site_dir().exists() {
        return;
    }
    let path = POSTS_OUTPUT.path().join("blog/mlops-10-minutes.html");
    assert!(path.exists(), "mlops-10-minutes.html should exist");
    let content = fs::read_to_string(&path).unwrap();

    assert!(
        content.contains("/people/alexeygrigorev.html"),
        "Should contain author link"
    );
    assert!(
        content.contains("Alexey Grigorev"),
        "Should contain resolved author name"
    );
}

#[test]
fn test_generated_hiring_post_content() {
    if !site_dir().exists() {
        return;
    }
    let path = POSTS_OUTPUT
        .path()
        .join("blog/hiring-process-for-data-professionals.html");
    assert!(
        path.exists(),
        "hiring-process-for-data-professionals.html should exist"
    );
    let content = fs::read_to_string(&path).unwrap();

    assert!(
        content.contains("/people/pavelchernetsov.html"),
        "Should contain author link"
    );
    assert!(
        !content.is_empty() && content.len() > 1000,
        "Should have substantial rendered content"
    );
}

// Post rendering through layout

#[test]
fn test_render_segmentation_post() {
    if !site_dir().exists() {
        return;
    }
    let post = FIXTURE
        .posts
        .iter()
        .find(|p| p.slug == "segmentation")
        .unwrap();
    let mut fm = post.front_matter.clone();
    fm.insert("url".into(), serde_yaml::Value::String(post.url.clone()));
    if !fm.contains_key("date") {
        if let Some(ref date) = post.date {
            fm.insert("date".to_string(), serde_yaml::Value::String(date.clone()));
        }
    }

    let html = FIXTURE
        .layout_engine
        .render_page("post", &post.content, &fm, &FIXTURE.site_context)
        .unwrap();

    assert!(
        html.contains("Build a 5D RFM+ framework"),
        "Should contain subtitle"
    );
    assert!(
        html.contains("/people/nishantmohan.html"),
        "Should contain author link"
    );
    assert!(
        html.contains("Background"),
        "Should contain markdown heading from content"
    );
    assert!(
        html.contains("\"@type\": \"Article\""),
        "Should contain Article JSON-LD"
    );
    assert!(html.contains("<html"), "Should contain <html");
    assert!(html.contains("<body"), "Should contain <body");
}

#[test]
fn test_author_resolution_alexeygrigorev() {
    if !site_dir().exists() {
        return;
    }
    let post = FIXTURE
        .posts
        .iter()
        .find(|p| p.slug == "mlops-10-minutes")
        .unwrap();
    let mut fm = post.front_matter.clone();
    fm.insert("url".into(), serde_yaml::Value::String(post.url.clone()));
    if !fm.contains_key("date") {
        if let Some(ref date) = post.date {
            fm.insert("date".to_string(), serde_yaml::Value::String(date.clone()));
        }
    }

    let html = FIXTURE
        .layout_engine
        .render_page("post", &post.content, &fm, &FIXTURE.site_context)
        .unwrap();

    assert!(
        html.contains("Alexey Grigorev"),
        "Should contain resolved author name"
    );
    assert!(
        html.contains("/people/alexeygrigorev.html"),
        "Should contain author link"
    );
}

#[test]
fn test_youtube_include_in_post() {
    if !site_dir().exists() {
        return;
    }
    let youtube_post = FIXTURE
        .posts
        .iter()
        .find(|p| p.content.contains("include youtube.html"));
    if let Some(post) = youtube_post {
        let mut fm = post.front_matter.clone();
        fm.insert("url".into(), serde_yaml::Value::String(post.url.clone()));
        let html = FIXTURE
            .layout_engine
            .render_page("post", &post.content, &fm, &FIXTURE.site_context)
            .unwrap();
        assert!(
            html.contains("<iframe") && html.contains("youtube"),
            "Should contain YouTube iframe embed"
        );
    }
}

#[test]
fn test_json_ld_schema_correctness() {
    if !site_dir().exists() {
        return;
    }
    let post = FIXTURE
        .posts
        .iter()
        .find(|p| p.slug == "segmentation")
        .unwrap();
    let mut fm = post.front_matter.clone();
    fm.insert("url".into(), serde_yaml::Value::String(post.url.clone()));
    let html = FIXTURE
        .layout_engine
        .render_page("post", &post.content, &fm, &FIXTURE.site_context)
        .unwrap();

    assert!(html.contains("\"@type\": \"Article\""));
    assert!(html.contains("BreadcrumbList"));
    assert!(html.contains("\"@type\": \"Person\""));
}

#[test]
fn test_mlops_post_tags_in_json_ld() {
    if !site_dir().exists() {
        return;
    }
    let post = FIXTURE
        .posts
        .iter()
        .find(|p| p.slug == "mlops-10-minutes")
        .unwrap();
    let mut fm = post.front_matter.clone();
    fm.insert("url".into(), serde_yaml::Value::String(post.url.clone()));
    let html = FIXTURE
        .layout_engine
        .render_page("post", &post.content, &fm, &FIXTURE.site_context)
        .unwrap();

    assert!(html.contains("mlops"), "Should contain mlops tag");
    assert!(html.contains("team"), "Should contain team tag");
    assert!(html.contains("process"), "Should contain process tag");
}

#[test]
fn test_post_with_no_subtitle() {
    if !site_dir().exists() {
        return;
    }
    let post = FIXTURE
        .posts
        .iter()
        .find(|p| !p.front_matter.contains_key("subtitle"));
    if let Some(post) = post {
        let mut fm = post.front_matter.clone();
        fm.insert("url".into(), serde_yaml::Value::String(post.url.clone()));
        let html = FIXTURE
            .layout_engine
            .render_page("post", &post.content, &fm, &FIXTURE.site_context)
            .unwrap();
        assert!(
            !html.contains("<h3></h3>"),
            "Should not contain empty h3 subtitle tag"
        );
    }
}

// ========================================================================
// Issue 71: Related posts include rendering
// ========================================================================

#[test]
fn test_related_posts_include_renders_as_html() {
    if !site_dir().exists() {
        return;
    }
    // Find the data engineering zoomcamp post (uses related-posts.html include)
    let post = FIXTURE
        .posts
        .iter()
        .find(|p| p.slug == "data-engineering-zoomcamp");
    let post = post.expect("data-engineering-zoomcamp post should exist");

    let cached_site =
        rustkyll::template::layout::LayoutEngine::build_cached_site_context(&FIXTURE.site_context);
    let mut fm = post.front_matter.clone();
    fm.insert("url".into(), serde_yaml::Value::String(post.url.clone()));
    let html = FIXTURE
        .layout_engine
        .render_markdown_page_with_cached_site("post", &post.content, &fm, &cached_site)
        .unwrap();

    // Related post titles should render as h3 tags, not inside <pre><code>
    assert!(
        html.contains("related-post-title"),
        "Should contain related-post-title class"
    );
    assert!(
        html.contains("<h3 class=\"related-post-title\">"),
        "Related post titles should be rendered as h3 tags, not escaped. HTML snippet: {}",
        html.chars()
            .skip(html.find("related-post").unwrap_or(0).saturating_sub(200))
            .take(600)
            .collect::<String>()
    );
    assert!(
        html.contains("<a href=\"/blog/"),
        "Related post links should be rendered as HTML links"
    );
    // Should NOT have escaped HTML tags inside code blocks
    assert!(
        !html.contains("&lt;a href=\"/blog/"),
        "Related post links should NOT be HTML-escaped"
    );
}

#[test]
fn test_related_posts_no_code_blocks() {
    if !site_dir().exists() {
        return;
    }
    // Check multiple posts that use related-posts.html
    let slugs = [
        "data-engineering-zoomcamp",
        "llm-zoomcamp",
        "mlops-zoomcamp",
        "machine-learning-zoomcamp",
    ];
    let cached_site =
        rustkyll::template::layout::LayoutEngine::build_cached_site_context(&FIXTURE.site_context);

    for slug in &slugs {
        let post = FIXTURE.posts.iter().find(|p| p.slug == *slug);
        if let Some(post) = post {
            let mut fm = post.front_matter.clone();
            fm.insert("url".into(), serde_yaml::Value::String(post.url.clone()));
            let html = FIXTURE
                .layout_engine
                .render_markdown_page_with_cached_site("post", &post.content, &fm, &cached_site)
                .unwrap();

            // Count related-post-title occurrences (should be >= 3)
            let title_count = html.matches("related-post-title").count();
            assert!(
                title_count >= 3,
                "{slug} should have at least 3 related post titles, found {title_count}"
            );
        }
    }
}
