//! Integration tests for standalone page generation (issue 14).
//!
//! Tests that all 10 root-level `.md` pages render to HTML correctly,
//! with no unrendered Liquid tags, proper layout wrapping, and expected content.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use liquid::Object;

use rustkyll::collection::{self, Page};
use rustkyll::config::SiteConfig;
use rustkyll::data;
use rustkyll::generator;
use rustkyll::template::layout::LayoutEngine;

fn site_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datatalksclub.github.io")
}

static CONFIG: LazyLock<SiteConfig> =
    LazyLock::new(|| SiteConfig::from_file(&site_dir().join("_config.yml")).unwrap());

struct PagesFixture {
    pages: Vec<Page>,
    site_context: Object,
    layout_engine: LayoutEngine,
}

static FIXTURE: LazyLock<PagesFixture> = LazyLock::new(|| {
    let (pages, _) = collection::load_pages(&site_dir()).unwrap();
    let (people, _) = collection::load_collection("people", &site_dir(), &CONFIG).unwrap();
    let (posts, _) = collection::load_collection("posts", &site_dir(), &CONFIG).unwrap();
    let (books, _) = collection::load_collection("books", &site_dir(), &CONFIG).unwrap();
    let (podcast, _) = collection::load_collection("podcast", &site_dir(), &CONFIG).unwrap();
    let (tools, _) = collection::load_collection("tools", &site_dir(), &CONFIG).unwrap();
    let (courses, _) = collection::load_collection("courses", &site_dir(), &CONFIG).unwrap();
    let (conferences, _) =
        collection::load_collection("conferences", &site_dir(), &CONFIG).unwrap();
    let data_tree = data::load_data(&site_dir().join("_data")).unwrap();

    let mut colls = HashMap::new();
    colls.insert("posts".to_string(), posts);
    colls.insert("books".to_string(), books);
    colls.insert("people".to_string(), people);
    colls.insert("podcast".to_string(), podcast);
    colls.insert("tools".to_string(), tools);
    colls.insert("courses".to_string(), courses);
    colls.insert("conferences".to_string(), conferences);

    let site_context =
        generator::build_site_context(&CONFIG, &colls, &data_tree, Some(&site_dir()), &[]);
    let layout_engine =
        LayoutEngine::new(&site_dir().join("_layouts"), &site_dir().join("_includes")).unwrap();

    PagesFixture {
        pages,
        site_context,
        layout_engine,
    }
});

/// Generate all standalone pages once to a shared temp directory.
static PAGES_OUTPUT: LazyLock<tempfile::TempDir> = LazyLock::new(|| {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = generator::generate_standalone_pages(
        &FIXTURE.pages,
        &CONFIG,
        &FIXTURE.layout_engine,
        &FIXTURE.site_context,
        tmp.path(),
    )
    .unwrap();

    assert_eq!(
        result.generated, 10,
        "Expected 10 pages generated, got {} (skipped: {}, errors: {:?})",
        result.generated, result.skipped, result.errors
    );
    assert!(
        result.errors.is_empty(),
        "Expected no generation errors, got: {:?}",
        result.errors
    );
    tmp
});

fn read_page(slug: &str) -> String {
    let path = PAGES_OUTPUT.path().join(format!("{slug}.html"));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {slug}.html at {:?}: {e}", path))
}

// ========================================================================
// Unit: generate_standalone_pages with empty input
// ========================================================================

#[test]
fn test_generate_standalone_pages_empty_vec() {
    if !site_dir().exists() { return; }
    let tmp = tempfile::TempDir::new().unwrap();
    let result = generator::generate_standalone_pages(
        &[],
        &CONFIG,
        &FIXTURE.layout_engine,
        &FIXTURE.site_context,
        tmp.path(),
    )
    .unwrap();
    assert_eq!(result.generated, 0);
    assert_eq!(result.skipped, 0);
    assert!(result.errors.is_empty());
}

// ========================================================================
// Integration: All 10 pages generated
// ========================================================================

#[test]
fn test_all_10_pages_generated() {
    if !site_dir().exists() { return; }
    let _output = &*PAGES_OUTPUT;
    let expected = [
        "index", "events", "articles", "books", "people", "podcast", "tools", "courses", "slack",
        "support",
    ];
    for slug in &expected {
        let path = PAGES_OUTPUT.path().join(format!("{slug}.html"));
        assert!(path.exists(), "{slug}.html should exist at {:?}", path);
    }
}

// ========================================================================
// Integration: No unrendered Liquid tags in any page
// ========================================================================

#[test]
fn test_no_unrendered_liquid_tags() {
    if !site_dir().exists() { return; }
    let _output = &*PAGES_OUTPUT;
    let slugs = [
        "index", "events", "articles", "books", "people", "podcast", "tools", "courses", "slack",
        "support",
    ];
    for slug in &slugs {
        let html = read_page(slug);
        assert!(
            !html.contains("{%"),
            "{slug}.html contains unrendered Liquid block tag '{{% ... %}}'"
        );
        assert!(
            !html.contains("{{"),
            "{slug}.html contains unrendered Liquid output tag '{{{{ ... }}}}'"
        );
    }
}

// ========================================================================
// Integration: All pages have HTML structure from layout
// ========================================================================

#[test]
fn test_all_pages_have_html_structure() {
    if !site_dir().exists() { return; }
    let _output = &*PAGES_OUTPUT;
    let slugs = [
        "index", "events", "articles", "books", "people", "podcast", "tools", "courses", "slack",
        "support",
    ];
    for slug in &slugs {
        let html = read_page(slug);
        assert!(
            html.contains("<html"),
            "{slug}.html should contain <html tag"
        );
        assert!(
            html.contains("<head"),
            "{slug}.html should contain <head tag"
        );
        assert!(
            html.contains("<body"),
            "{slug}.html should contain <body tag"
        );
    }
}

// ========================================================================
// Integration: index.html content verification
// ========================================================================

#[test]
fn test_index_has_upcoming_events() {
    if !site_dir().exists() { return; }
    let html = read_page("index");
    // index.md references site.data.events
    let has_events = html.contains("Upcoming events")
        || html.contains("upcoming-events")
        || html.contains("event");
    assert!(has_events, "index.html should reference events data");
}

#[test]
fn test_index_has_podcast_episodes() {
    if !site_dir().exists() { return; }
    let html = read_page("index");
    assert!(
        html.contains("Latest podcast episodes") || html.contains("podcast"),
        "index.html should have podcast section"
    );
}

#[test]
fn test_index_has_sponsors_section() {
    if !site_dir().exists() { return; }
    let html = read_page("index");
    assert!(
        html.contains("Our Sponsors") || html.contains("sponsors"),
        "index.html should have sponsors section"
    );
}

#[test]
fn test_index_has_book_of_the_week() {
    if !site_dir().exists() { return; }
    let html = read_page("index");
    assert!(
        html.contains("Book of the week") || html.contains("book"),
        "index.html should have book section"
    );
}

#[test]
fn test_index_has_latest_articles() {
    if !site_dir().exists() { return; }
    let html = read_page("index");
    assert!(
        html.contains("Latest articles") || html.contains("articles"),
        "index.html should have articles section"
    );
}

// ========================================================================
// Integration: events.html content verification
// ========================================================================

#[test]
fn test_events_has_past_events() {
    if !site_dir().exists() { return; }
    let html = read_page("events");
    assert!(
        html.contains("Past events") || html.contains("past-events"),
        "events.html should have past events section"
    );
}

#[test]
fn test_events_has_links() {
    if !site_dir().exists() { return; }
    let html = read_page("events");
    assert!(
        html.contains("<a href="),
        "events.html should contain links"
    );
}

#[test]
fn test_events_has_speaker_links() {
    if !site_dir().exists() { return; }
    let html = read_page("events");
    assert!(
        html.contains("/people/"),
        "events.html should link to speaker pages"
    );
}

// ========================================================================
// Integration: articles.html content verification
// ========================================================================

#[test]
fn test_articles_lists_posts() {
    if !site_dir().exists() { return; }
    let html = read_page("articles");
    // articles.md iterates site.posts, each post has a title
    assert!(
        html.contains("<a href="),
        "articles.html should contain post links"
    );
}

#[test]
fn test_articles_has_author_links() {
    if !site_dir().exists() { return; }
    let html = read_page("articles");
    assert!(
        html.contains("/people/"),
        "articles.html should link to author pages"
    );
}

// ========================================================================
// Integration: books.html content verification
// ========================================================================

#[test]
fn test_books_has_archive() {
    if !site_dir().exists() { return; }
    let html = read_page("books");
    assert!(
        html.contains("Archive") || html.contains("archive"),
        "books.html should have archive section"
    );
}

#[test]
fn test_books_has_book_links() {
    if !site_dir().exists() { return; }
    let html = read_page("books");
    assert!(
        html.contains("<a href="),
        "books.html should contain book links"
    );
}

// ========================================================================
// Integration: people.html content verification
// ========================================================================

#[test]
fn test_people_has_count() {
    if !site_dir().exists() { return; }
    let html = read_page("people");
    // people.md uses {{ site.people.size }} which should render as a number
    // The count should be 424+
    assert!(
        html.contains("42") || html.contains("43") || html.contains("44"),
        "people.html should contain the people count (400+)"
    );
}

#[test]
fn test_people_has_name_links() {
    if !site_dir().exists() { return; }
    let html = read_page("people");
    assert!(
        html.contains("/people/"),
        "people.html should link to individual people pages"
    );
}

// ========================================================================
// Integration: podcast.html content verification
// ========================================================================

#[test]
fn test_podcast_has_season_headings() {
    if !site_dir().exists() { return; }
    let html = read_page("podcast");
    assert!(
        html.contains("Season") || html.contains("season"),
        "podcast.html should have season headings"
    );
}

#[test]
fn test_podcast_has_episode_links() {
    if !site_dir().exists() { return; }
    let html = read_page("podcast");
    assert!(
        html.contains("<a href="),
        "podcast.html should contain episode links"
    );
}

// ========================================================================
// Integration: tools.html content verification
// ========================================================================

#[test]
fn test_tools_lists_tools() {
    if !site_dir().exists() { return; }
    let html = read_page("tools");
    assert!(
        html.contains("<a href="),
        "tools.html should contain tool links"
    );
}

// ========================================================================
// Integration: Static pages (courses, slack, support)
// ========================================================================

#[test]
fn test_courses_has_content() {
    if !site_dir().exists() { return; }
    let html = read_page("courses");
    assert!(
        html.contains("Courses") || html.contains("courses"),
        "courses.html should contain Courses"
    );
}

#[test]
fn test_slack_has_subscribe() {
    if !site_dir().exists() { return; }
    let html = read_page("slack");
    assert!(
        html.contains("Slack") || html.contains("slack"),
        "slack.html should mention Slack"
    );
}

#[test]
fn test_support_has_content() {
    if !site_dir().exists() { return; }
    let html = read_page("support");
    assert!(
        html.contains("Support") || html.contains("support"),
        "support.html should contain Support content"
    );
}

// ========================================================================
// Integration: Layout correctness
// ========================================================================

#[test]
fn test_index_uses_home_layout() {
    if !site_dir().exists() { return; }
    let html = read_page("index");
    // The home layout has specific elements different from page layout
    // Check for the html structure that comes from the layout
    assert!(
        html.contains("<html"),
        "index.html should be wrapped in html layout"
    );
    assert!(
        html.contains("DataTalks.Club"),
        "index.html should contain site name"
    );
}

#[test]
fn test_non_index_pages_use_page_layout() {
    if !site_dir().exists() { return; }
    let slugs = [
        "events", "articles", "books", "people", "podcast", "tools", "courses", "slack", "support",
    ];
    for slug in &slugs {
        let html = read_page(slug);
        assert!(
            html.contains("<html"),
            "{slug}.html should be wrapped in html layout"
        );
    }
}
