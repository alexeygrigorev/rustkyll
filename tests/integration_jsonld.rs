//! Integration tests for JSON-LD structured data across all page types.
//!
//! Verifies that JSON-LD blocks are present, parseable as valid JSON,
//! and contain the expected Schema.org types for each collection.

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

struct FullFixture {
    posts: Vec<CollectionItem>,
    people: Vec<CollectionItem>,
    books: Vec<CollectionItem>,
    episodes: Vec<CollectionItem>,
    site_context: Object,
    layout_engine: LayoutEngine,
}

static FIXTURE: LazyLock<FullFixture> = LazyLock::new(|| {
    let (posts, _) = collection::load_collection("posts", &site_dir(), &CONFIG).unwrap();
    let (people, _) = collection::load_collection("people", &site_dir(), &CONFIG).unwrap();
    let (books, _) = collection::load_collection("books", &site_dir(), &CONFIG).unwrap();
    let (episodes, _) = collection::load_collection("podcast", &site_dir(), &CONFIG).unwrap();
    let data_tree = data::load_data(&site_dir().join("_data")).unwrap();

    let mut colls = HashMap::new();
    colls.insert("posts".to_string(), posts.clone());
    colls.insert("people".to_string(), people.clone());
    colls.insert("books".to_string(), books.clone());
    colls.insert("podcast".to_string(), episodes.clone());

    let site_context =
        generator::build_site_context(&CONFIG, &colls, &data_tree, Some(&site_dir()), &[]);
    let layout_engine =
        LayoutEngine::new(&site_dir().join("_layouts"), &site_dir().join("_includes")).unwrap();

    FullFixture {
        posts,
        people,
        books,
        episodes,
        site_context,
        layout_engine,
    }
});

/// Extract all JSON-LD blocks from an HTML string and parse each one.
/// Returns a vec of parsed serde_json::Value objects.
fn extract_jsonld_blocks(html: &str) -> Vec<serde_json::Value> {
    let marker = r#"<script type="application/ld+json">"#;
    let end_marker = "</script>";
    let mut results = Vec::new();
    let mut search_from = 0;

    while let Some(start_pos) = html[search_from..].find(marker) {
        let abs_start = search_from + start_pos + marker.len();
        if let Some(end_pos) = html[abs_start..].find(end_marker) {
            let json_str = html[abs_start..abs_start + end_pos].trim();
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                results.push(val);
            }
            search_from = abs_start + end_pos + end_marker.len();
        } else {
            break;
        }
    }
    results
}

/// Check that a JSON-LD value (or its @graph) contains a given @type.
fn jsonld_contains_type(val: &serde_json::Value, schema_type: &str) -> bool {
    // Check top-level @type
    if val.get("@type").and_then(|t| t.as_str()) == Some(schema_type) {
        return true;
    }
    // Check @graph array
    if let Some(graph) = val.get("@graph").and_then(|g| g.as_array()) {
        for item in graph {
            if item.get("@type").and_then(|t| t.as_str()) == Some(schema_type) {
                return true;
            }
        }
    }
    false
}

fn render_item(layout: &str, item: &CollectionItem) -> String {
    let mut fm = item.front_matter.clone();
    fm.insert("url".into(), serde_yaml::Value::String(item.url.clone()));
    if !fm.contains_key("date") {
        if let Some(ref date) = item.date {
            fm.insert("date".to_string(), serde_yaml::Value::String(date.clone()));
        }
    }
    let content = if !item.html_content.is_empty() {
        &item.html_content
    } else {
        &item.content
    };
    let html = FIXTURE
        .layout_engine
        .render_page(layout, content, &fm, &FIXTURE.site_context)
        .unwrap();
    // Apply JSON-LD post-processing (same as generate_collection_pages does)
    rustkyll::jsonld::inject_jsonld(&html, layout, &fm, &CONFIG, &FIXTURE.people)
}

// ========================================================================
// Post JSON-LD tests
// ========================================================================

#[test]
fn test_post_jsonld_is_valid_json() {
    let post = FIXTURE
        .posts
        .iter()
        .find(|p| p.slug == "segmentation")
        .unwrap();
    let html = render_item("post", post);
    let blocks = extract_jsonld_blocks(&html);

    assert!(!blocks.is_empty(), "Post should contain JSON-LD block(s)");
    for block in &blocks {
        assert!(
            block.get("@context").is_some(),
            "JSON-LD should have @context"
        );
    }
}

#[test]
fn test_post_jsonld_contains_article_type() {
    let post = FIXTURE
        .posts
        .iter()
        .find(|p| p.slug == "segmentation")
        .unwrap();
    let html = render_item("post", post);
    let blocks = extract_jsonld_blocks(&html);

    let has_article = blocks.iter().any(|b| jsonld_contains_type(b, "Article"));
    assert!(has_article, "Post JSON-LD should contain Article type");
}

#[test]
fn test_post_jsonld_contains_breadcrumb() {
    let post = FIXTURE
        .posts
        .iter()
        .find(|p| p.slug == "segmentation")
        .unwrap();
    let html = render_item("post", post);
    let blocks = extract_jsonld_blocks(&html);

    let has_breadcrumb = blocks
        .iter()
        .any(|b| jsonld_contains_type(b, "BreadcrumbList"));
    assert!(
        has_breadcrumb,
        "Post JSON-LD should contain BreadcrumbList type"
    );
}

#[test]
fn test_post_jsonld_article_fields() {
    let post = FIXTURE
        .posts
        .iter()
        .find(|p| p.slug == "segmentation")
        .unwrap();
    let html = render_item("post", post);
    let blocks = extract_jsonld_blocks(&html);

    let article = blocks
        .iter()
        .flat_map(|b| {
            if let Some(graph) = b.get("@graph").and_then(|g| g.as_array()) {
                graph.to_vec()
            } else if b.get("@type").and_then(|t| t.as_str()) == Some("Article") {
                vec![b.clone()]
            } else {
                vec![]
            }
        })
        .find(|item| item.get("@type").and_then(|t| t.as_str()) == Some("Article"));

    assert!(article.is_some(), "Should find Article in JSON-LD");
    let article = article.unwrap();

    assert!(
        article.get("headline").is_some(),
        "Article should have headline"
    );
    assert!(
        article.get("datePublished").is_some(),
        "Article should have datePublished"
    );
    assert!(
        article.get("publisher").is_some(),
        "Article should have publisher"
    );
    assert!(article.get("url").is_some(), "Article should have url");
}

#[test]
fn test_post_jsonld_author_person_type() {
    let post = FIXTURE
        .posts
        .iter()
        .find(|p| p.slug == "segmentation")
        .unwrap();
    let html = render_item("post", post);
    let blocks = extract_jsonld_blocks(&html);

    let article = blocks
        .iter()
        .flat_map(|b| {
            b.get("@graph")
                .and_then(|g| g.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .find(|item| item.get("@type").and_then(|t| t.as_str()) == Some("Article"));

    if let Some(article) = article {
        if let Some(authors) = article.get("author").and_then(|a| a.as_array()) {
            for author in authors {
                assert_eq!(
                    author.get("@type").and_then(|t| t.as_str()),
                    Some("Person"),
                    "Author should be Person type"
                );
                assert!(
                    author.get("name").is_some(),
                    "Author should have name field"
                );
            }
        }
    }
}

#[test]
fn test_multiple_posts_jsonld_parseable() {
    let slugs = ["segmentation", "mlops-10-minutes"];
    for slug in &slugs {
        let post = FIXTURE.posts.iter().find(|p| p.slug == *slug);
        if let Some(post) = post {
            let html = render_item("post", post);
            let blocks = extract_jsonld_blocks(&html);
            assert!(
                !blocks.is_empty(),
                "Post {} should have parseable JSON-LD",
                slug
            );
        }
    }
}

// ========================================================================
// People JSON-LD tests
// ========================================================================

#[test]
fn test_person_jsonld_is_valid_json() {
    let person = FIXTURE
        .people
        .iter()
        .find(|p| p.slug == "alexeygrigorev")
        .unwrap();
    let html = render_item("author", person);
    let blocks = extract_jsonld_blocks(&html);

    assert!(
        !blocks.is_empty(),
        "Person page should contain JSON-LD block(s)"
    );
}

#[test]
fn test_person_jsonld_contains_person_type() {
    let person = FIXTURE
        .people
        .iter()
        .find(|p| p.slug == "alexeygrigorev")
        .unwrap();
    let html = render_item("author", person);
    let blocks = extract_jsonld_blocks(&html);

    let has_person = blocks.iter().any(|b| jsonld_contains_type(b, "Person"));
    assert!(has_person, "Person JSON-LD should contain Person type");
}

#[test]
fn test_person_jsonld_fields() {
    let person = FIXTURE
        .people
        .iter()
        .find(|p| p.slug == "alexeygrigorev")
        .unwrap();
    let html = render_item("author", person);
    let blocks = extract_jsonld_blocks(&html);

    let person_block = blocks
        .iter()
        .find(|b| b.get("@type").and_then(|t| t.as_str()) == Some("Person"));
    assert!(person_block.is_some(), "Should find Person in JSON-LD");
    let person_block = person_block.unwrap();

    assert!(
        person_block.get("name").is_some(),
        "Person should have name"
    );
    assert!(person_block.get("url").is_some(), "Person should have url");
    assert!(
        person_block.get("description").is_some(),
        "Person should have description"
    );
}

#[test]
fn test_person_jsonld_same_as_links() {
    let person = FIXTURE
        .people
        .iter()
        .find(|p| p.slug == "alexeygrigorev")
        .unwrap();
    let html = render_item("author", person);
    let blocks = extract_jsonld_blocks(&html);

    let person_block = blocks
        .iter()
        .find(|b| b.get("@type").and_then(|t| t.as_str()) == Some("Person"));
    assert!(person_block.is_some(), "Should find Person in JSON-LD");
    let person_block = person_block.unwrap();

    let same_as = person_block.get("sameAs").and_then(|s| s.as_array());
    assert!(same_as.is_some(), "Person should have sameAs links");
    let same_as = same_as.unwrap();
    assert!(
        !same_as.is_empty(),
        "sameAs should have at least one social link"
    );

    // Alexey has linkedin, twitter, github, web
    let links_str: Vec<&str> = same_as.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        links_str.iter().any(|l| l.contains("linkedin.com")),
        "sameAs should include LinkedIn"
    );
    assert!(
        links_str.iter().any(|l| l.contains("twitter.com")),
        "sameAs should include Twitter"
    );
    assert!(
        links_str.iter().any(|l| l.contains("github.com")),
        "sameAs should include GitHub"
    );
}

#[test]
fn test_multiple_people_jsonld_parseable() {
    let slugs = ["alexeygrigorev", "chiphuyen"];
    for slug in &slugs {
        let person = FIXTURE.people.iter().find(|p| p.slug == *slug);
        if let Some(person) = person {
            let html = render_item("author", person);
            let blocks = extract_jsonld_blocks(&html);
            assert!(
                !blocks.is_empty(),
                "Person {} should have parseable JSON-LD",
                slug
            );
        }
    }
}

// ========================================================================
// Book JSON-LD tests
// ========================================================================

#[test]
fn test_book_jsonld_is_valid_json() {
    let book = FIXTURE
        .books
        .iter()
        .find(|b| b.slug == "20201214-ml-bookcamp")
        .unwrap();
    let html = render_item("book", book);
    let blocks = extract_jsonld_blocks(&html);

    assert!(
        !blocks.is_empty(),
        "Book page should contain JSON-LD block(s)"
    );
}

#[test]
fn test_book_jsonld_contains_book_type() {
    let book = FIXTURE
        .books
        .iter()
        .find(|b| b.slug == "20201214-ml-bookcamp")
        .unwrap();
    let html = render_item("book", book);
    let blocks = extract_jsonld_blocks(&html);

    let has_book = blocks.iter().any(|b| jsonld_contains_type(b, "Book"));
    assert!(has_book, "Book JSON-LD should contain Book type");
}

#[test]
fn test_book_jsonld_contains_breadcrumb() {
    let book = FIXTURE
        .books
        .iter()
        .find(|b| b.slug == "20201214-ml-bookcamp")
        .unwrap();
    let html = render_item("book", book);
    let blocks = extract_jsonld_blocks(&html);

    let has_breadcrumb = blocks
        .iter()
        .any(|b| jsonld_contains_type(b, "BreadcrumbList"));
    assert!(
        has_breadcrumb,
        "Book JSON-LD should contain BreadcrumbList type"
    );
}

#[test]
fn test_book_jsonld_fields() {
    let book = FIXTURE
        .books
        .iter()
        .find(|b| b.slug == "20201214-ml-bookcamp")
        .unwrap();
    let html = render_item("book", book);
    let blocks = extract_jsonld_blocks(&html);

    let book_block = blocks
        .iter()
        .flat_map(|b| {
            b.get("@graph")
                .and_then(|g| g.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .find(|item| item.get("@type").and_then(|t| t.as_str()) == Some("Book"));

    assert!(book_block.is_some(), "Should find Book in JSON-LD");
    let book_block = book_block.unwrap();

    assert!(book_block.get("name").is_some(), "Book should have name");
    assert!(book_block.get("url").is_some(), "Book should have url");
    assert!(
        book_block.get("publisher").is_some(),
        "Book should have publisher"
    );
    assert!(book_block.get("image").is_some(), "Book should have image");
}

#[test]
fn test_book_jsonld_author_resolution() {
    let book = FIXTURE
        .books
        .iter()
        .find(|b| b.slug == "20201214-ml-bookcamp")
        .unwrap();
    let html = render_item("book", book);
    let blocks = extract_jsonld_blocks(&html);

    let book_block = blocks
        .iter()
        .flat_map(|b| {
            b.get("@graph")
                .and_then(|g| g.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .find(|item| item.get("@type").and_then(|t| t.as_str()) == Some("Book"));

    assert!(book_block.is_some(), "Should find Book in JSON-LD");
    let book_block = book_block.unwrap();

    let authors = book_block.get("author").and_then(|a| a.as_array());
    assert!(authors.is_some(), "Book should have author array");
    let authors = authors.unwrap();

    assert!(!authors.is_empty(), "Book should have at least one author");
    let first_author = &authors[0];
    assert_eq!(
        first_author.get("@type").and_then(|t| t.as_str()),
        Some("Person"),
        "Book author should be Person type"
    );
    assert!(
        first_author
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .contains("Alexey Grigorev"),
        "Author name should be resolved to Alexey Grigorev"
    );
}

#[test]
fn test_book_jsonld_title_matches() {
    let book = FIXTURE
        .books
        .iter()
        .find(|b| b.slug == "20201214-ml-bookcamp")
        .unwrap();
    let html = render_item("book", book);
    let blocks = extract_jsonld_blocks(&html);

    let book_block = blocks
        .iter()
        .flat_map(|b| {
            b.get("@graph")
                .and_then(|g| g.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .find(|item| item.get("@type").and_then(|t| t.as_str()) == Some("Book"));

    assert!(book_block.is_some());
    let book_val = book_block.unwrap();
    let name = book_val.get("name").and_then(|n| n.as_str()).unwrap_or("");
    assert!(
        name.contains("Machine Learning Bookcamp"),
        "Book name should be Machine Learning Bookcamp, got: {}",
        name
    );
}

#[test]
fn test_multiple_books_jsonld_parseable() {
    let slugs = ["20201214-ml-bookcamp", "20210201-data-teams"];
    for slug in &slugs {
        let book = FIXTURE.books.iter().find(|b| b.slug == *slug);
        if let Some(book) = book {
            let html = render_item("book", book);
            let blocks = extract_jsonld_blocks(&html);
            assert!(
                !blocks.is_empty(),
                "Book {} should have parseable JSON-LD",
                slug
            );
        }
    }
}

// ========================================================================
// Podcast JSON-LD tests
// ========================================================================

#[test]
fn test_podcast_jsonld_is_valid_json() {
    let episode = FIXTURE
        .episodes
        .iter()
        .find(|e| e.slug == "building-agentic-ai-engineering-tooling-retrieval-evaluation")
        .unwrap();
    let html = render_item("podcast", episode);
    let blocks = extract_jsonld_blocks(&html);

    assert!(
        !blocks.is_empty(),
        "Podcast page should contain JSON-LD block(s)"
    );
}

#[test]
fn test_podcast_jsonld_contains_episode_type() {
    let episode = FIXTURE
        .episodes
        .iter()
        .find(|e| e.slug == "building-agentic-ai-engineering-tooling-retrieval-evaluation")
        .unwrap();
    let html = render_item("podcast", episode);
    let blocks = extract_jsonld_blocks(&html);

    let has_episode = blocks
        .iter()
        .any(|b| jsonld_contains_type(b, "PodcastEpisode"));
    assert!(
        has_episode,
        "Podcast JSON-LD should contain PodcastEpisode type"
    );
}

#[test]
fn test_podcast_jsonld_contains_season_and_series() {
    let episode = FIXTURE
        .episodes
        .iter()
        .find(|e| e.slug == "building-agentic-ai-engineering-tooling-retrieval-evaluation")
        .unwrap();
    let html = render_item("podcast", episode);
    let blocks = extract_jsonld_blocks(&html);

    let has_season = blocks
        .iter()
        .any(|b| jsonld_contains_type(b, "PodcastSeason"));
    assert!(
        has_season,
        "Podcast JSON-LD should contain PodcastSeason type"
    );

    // PodcastSeries is nested in partOfSeries, check the raw HTML
    assert!(
        html.contains("PodcastSeries"),
        "Podcast JSON-LD should reference PodcastSeries"
    );
}

#[test]
fn test_podcast_jsonld_episode_fields() {
    let episode = FIXTURE
        .episodes
        .iter()
        .find(|e| e.slug == "building-agentic-ai-engineering-tooling-retrieval-evaluation")
        .unwrap();
    let html = render_item("podcast", episode);
    let blocks = extract_jsonld_blocks(&html);

    let ep_block = blocks
        .iter()
        .flat_map(|b| {
            b.get("@graph")
                .and_then(|g| g.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .find(|item| item.get("@type").and_then(|t| t.as_str()) == Some("PodcastEpisode"));

    assert!(ep_block.is_some(), "Should find PodcastEpisode in JSON-LD");
    let ep = ep_block.unwrap();

    assert!(ep.get("name").is_some(), "Episode should have name");
    assert!(
        ep.get("description").is_some(),
        "Episode should have description"
    );
    assert!(ep.get("url").is_some(), "Episode should have url");
    assert!(
        ep.get("datePublished").is_some(),
        "Episode should have datePublished"
    );
    assert!(
        ep.get("publisher").is_some(),
        "Episode should have publisher"
    );
}

#[test]
fn test_podcast_jsonld_video_object() {
    let episode = FIXTURE
        .episodes
        .iter()
        .find(|e| e.slug == "building-agentic-ai-engineering-tooling-retrieval-evaluation")
        .unwrap();
    let html = render_item("podcast", episode);
    let blocks = extract_jsonld_blocks(&html);

    let has_video = blocks
        .iter()
        .any(|b| jsonld_contains_type(b, "VideoObject"));
    assert!(
        has_video,
        "Podcast with YouTube link should contain VideoObject"
    );
}

#[test]
fn test_podcast_jsonld_breadcrumb() {
    let episode = FIXTURE
        .episodes
        .iter()
        .find(|e| e.slug == "building-agentic-ai-engineering-tooling-retrieval-evaluation")
        .unwrap();
    let html = render_item("podcast", episode);
    let blocks = extract_jsonld_blocks(&html);

    let has_breadcrumb = blocks
        .iter()
        .any(|b| jsonld_contains_type(b, "BreadcrumbList"));
    assert!(
        has_breadcrumb,
        "Podcast JSON-LD should contain BreadcrumbList"
    );
}

// ========================================================================
// Cross-collection: bulk parseability check
// ========================================================================

#[test]
fn test_all_generated_books_have_parseable_jsonld() {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = generator::generate_collection_pages_with_authors(
        &FIXTURE.books,
        "books",
        &CONFIG,
        &FIXTURE.layout_engine,
        &FIXTURE.site_context,
        tmp.path(),
        &FIXTURE.people,
    )
    .unwrap();

    assert!(result.generated >= 97, "Should generate 97+ books");

    let books_dir = tmp.path().join("books");
    let mut checked = 0;
    for entry in fs::read_dir(&books_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e == "html").unwrap_or(false) {
            let content = fs::read_to_string(&path).unwrap();
            let blocks = extract_jsonld_blocks(&content);
            assert!(
                !blocks.is_empty(),
                "Book {:?} should have JSON-LD",
                path.file_name()
            );
            // Verify the Book type is present
            let has_book = blocks.iter().any(|b| jsonld_contains_type(b, "Book"));
            assert!(
                has_book,
                "Book {:?} should have Book schema type",
                path.file_name()
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 97,
        "Should check 97+ book files, checked {}",
        checked
    );
}

#[test]
fn test_sample_posts_have_parseable_jsonld() {
    // Check a sample of posts (not all, to keep tests fast)
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

    assert!(result.generated >= 46, "Should generate 46+ posts");

    let blog_dir = tmp.path().join("blog");
    let mut checked = 0;
    for entry in fs::read_dir(&blog_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e == "html").unwrap_or(false) {
            let content = fs::read_to_string(&path).unwrap();
            let blocks = extract_jsonld_blocks(&content);
            assert!(
                !blocks.is_empty(),
                "Post {:?} should have JSON-LD",
                path.file_name()
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 46,
        "Should check 46+ post files, checked {}",
        checked
    );
}

#[test]
fn test_sample_podcast_episodes_have_parseable_jsonld() {
    // Check a few specific episodes for parseability
    let slugs = [
        "building-agentic-ai-engineering-tooling-retrieval-evaluation",
        "technical-writing-for-data-scientists",
        "ab-testing-and-product-experimentation",
    ];

    for slug in &slugs {
        let episode = FIXTURE.episodes.iter().find(|e| e.slug == *slug);
        if let Some(episode) = episode {
            let html = render_item("podcast", episode);
            let blocks = extract_jsonld_blocks(&html);
            assert!(
                !blocks.is_empty(),
                "Podcast episode {} should have parseable JSON-LD",
                slug
            );
            let has_episode = blocks
                .iter()
                .any(|b| jsonld_contains_type(b, "PodcastEpisode"));
            assert!(
                has_episode,
                "Episode {} should have PodcastEpisode type",
                slug
            );
        }
    }
}
