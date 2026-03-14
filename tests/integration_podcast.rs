//! Integration tests for podcast page generation.

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

struct PodcastFixture {
    episodes: Vec<CollectionItem>,
    site_context: Object,
    layout_engine: LayoutEngine,
}

static FIXTURE: LazyLock<PodcastFixture> = LazyLock::new(|| {
    let (episodes, _) = collection::load_collection("podcast", &site_dir(), &CONFIG).unwrap();
    let (people, _) = collection::load_collection("people", &site_dir(), &CONFIG).unwrap();
    let (posts, _) = collection::load_collection("posts", &site_dir(), &CONFIG).unwrap();
    let (books, _) = collection::load_collection("books", &site_dir(), &CONFIG).unwrap();
    let data_tree = data::load_data(&site_dir().join("_data")).unwrap();

    let mut colls = HashMap::new();
    colls.insert("posts".to_string(), posts);
    colls.insert("books".to_string(), books);
    colls.insert("people".to_string(), people);
    colls.insert("podcast".to_string(), episodes.clone());

    let site_context =
        generator::build_site_context(&CONFIG, &colls, &data_tree, Some(&site_dir()), &[]);
    let layout_engine =
        LayoutEngine::new(&site_dir().join("_layouts"), &site_dir().join("_includes")).unwrap();

    PodcastFixture {
        episodes,
        site_context,
        layout_engine,
    }
});

fn render_episode(slug: &str) -> String {
    let episode = FIXTURE.episodes.iter().find(|e| e.slug == slug).unwrap();
    let mut fm = episode.front_matter.clone();
    fm.insert(
        "url".to_string(),
        serde_yaml::Value::String(episode.url.clone()),
    );
    FIXTURE
        .layout_engine
        .render_page("podcast", &episode.content, &fm, &FIXTURE.site_context)
        .unwrap()
}

static PODCAST_OUTPUT: LazyLock<tempfile::TempDir> = LazyLock::new(|| {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = generator::generate_collection_pages(
        &FIXTURE.episodes,
        "podcast",
        &CONFIG,
        &FIXTURE.layout_engine,
        &FIXTURE.site_context,
        tmp.path(),
    )
    .unwrap();

    assert!(
        result.generated >= 193,
        "Expected at least 193 podcast pages generated, got {} (errors: {})",
        result.generated,
        result.errors.len()
    );
    tmp
});

// ========================================================================
// Full generation
// ========================================================================

#[test]
fn test_generate_all_podcast_pages_count() {
    if !site_dir().exists() {
        return;
    }
    let _output = &*PODCAST_OUTPUT;
}

#[test]
fn test_generated_podcast_files_are_valid_html() {
    if !site_dir().exists() {
        return;
    }
    let podcast_dir = PODCAST_OUTPUT.path().join("podcast");
    let mut checked = 0;
    for entry in fs::read_dir(&podcast_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e == "html").unwrap_or(false) {
            let content = fs::read_to_string(&path).unwrap();
            assert!(!content.is_empty(), "File should not be empty: {:?}", path);
            assert!(content.contains("<html"), "Missing <html: {:?}", path);
            assert!(content.contains("<body"), "Missing <body: {:?}", path);
            assert!(content.contains("</body>"), "Missing </body>: {:?}", path);
            checked += 1;
        }
    }
    assert!(
        checked >= 193,
        "Should check 193+ files, checked {}",
        checked
    );
}

#[test]
fn test_generated_agentic_episode_content() {
    if !site_dir().exists() {
        return;
    }
    let path = PODCAST_OUTPUT
        .path()
        .join("podcast/building-agentic-ai-engineering-tooling-retrieval-evaluation.html");
    assert!(path.exists());
    let content = fs::read_to_string(&path).unwrap();

    assert!(content.contains("Season 22, Episode 1"));
    assert!(content.contains("Building Agentic AI"));
    assert!(content.contains("x2AAjqz2XmM"));
    assert!(content.contains(r#"<script type="application/ld+json">"#));
    assert!(content.contains("\"@type\": \"PodcastEpisode\""));
    assert!(content.contains("Ranjitha Kulkarni"));
}

#[test]
fn test_generated_transcript_episode_content() {
    if !site_dir().exists() {
        return;
    }
    let path = PODCAST_OUTPUT
        .path()
        .join("podcast/technical-writing-for-data-scientists.html");
    assert!(path.exists());
    let content = fs::read_to_string(&path).unwrap();

    assert!(content.contains("data-tab=\"transcript\""));
    assert!(content.contains("transcript-header"));
    assert!(content.contains("<b>Alexey</b>"));
    assert!(content.contains("data-time="));
}

#[test]
fn test_generated_ab_testing_episode_clips() {
    if !site_dir().exists() {
        return;
    }
    let path = PODCAST_OUTPUT
        .path()
        .join("podcast/ab-testing-and-product-experimentation.html");
    assert!(path.exists());
    let content = fs::read_to_string(&path).unwrap();

    assert!(content.contains("\"@type\": \"Clip\"") || content.contains("\"@type\":\"Clip\""),);
    assert!(content.contains("startOffset"));
}

// ========================================================================
// Single episode rendering
// ========================================================================

#[test]
fn test_render_single_podcast_episode() {
    if !site_dir().exists() {
        return;
    }
    let html = render_episode("building-agentic-ai-engineering-tooling-retrieval-evaluation");

    assert!(html.contains("Building Agentic AI"));
    assert!(html.contains("Season 22, Episode 1"));
    assert!(html.contains("x2AAjqz2XmM"));
    assert!(html.contains("<iframe"));
    assert!(html.contains(r#"<script type="application/ld+json">"#));
    assert!(html.contains("\"@type\": \"PodcastEpisode\""));
}

#[test]
fn test_podcast_guest_resolution() {
    if !site_dir().exists() {
        return;
    }
    let html = render_episode("building-agentic-ai-engineering-tooling-retrieval-evaluation");

    assert!(html.contains("Ranjitha Kulkarni"));
    assert!(html.contains("guest-bio-card"));
}

#[test]
fn test_podcast_guest_bio_card_details() {
    if !site_dir().exists() {
        return;
    }
    let html = render_episode("building-agentic-ai-engineering-tooling-retrieval-evaluation");

    assert!(html.contains("guest-bio-img"));
    assert!(html.contains("guest-bio-name"));
    assert!(html.contains("guest-bio-links"));
}

#[test]
fn test_podcast_episode_navigation_mid_season() {
    if !site_dir().exists() {
        return;
    }
    let episode = FIXTURE
        .episodes
        .iter()
        .find(|e| {
            let season = e.front_matter.get("season").and_then(|v| v.as_u64());
            let ep_num = e.front_matter.get("episode").and_then(|v| v.as_u64());
            matches!((season, ep_num), (Some(_), Some(n)) if n >= 2)
        })
        .unwrap();

    let mut fm = episode.front_matter.clone();
    fm.insert(
        "url".to_string(),
        serde_yaml::Value::String(episode.url.clone()),
    );

    let html = FIXTURE
        .layout_engine
        .render_page("podcast", &episode.content, &fm, &FIXTURE.site_context)
        .unwrap();

    assert!(html.contains("Previous episode"));
    assert!(html.contains("episode-nav-link"));
}

#[test]
fn test_podcast_transcript_rendering() {
    if !site_dir().exists() {
        return;
    }
    let html = render_episode("technical-writing-for-data-scientists");

    assert!(html.contains("data-tab=\"transcript\""));
    assert!(html.contains("<h3") && html.contains("transcript-header"));
    assert!(html.contains("<b>Alexey</b>"));
    assert!(html.contains("<b>Eugene</b>"));
    assert!(html.contains("data-time=\"0\""));
    assert!(html.contains("data-time=\"19\""));
    assert!(html.contains("data-tab=\"timestamps\""));
}

#[test]
fn test_podcast_episode_without_transcript() {
    if !site_dir().exists() {
        return;
    }
    let episode = FIXTURE
        .episodes
        .iter()
        .find(|e| !e.front_matter.contains_key("transcript"))
        .unwrap();

    let mut fm = episode.front_matter.clone();
    fm.insert(
        "url".to_string(),
        serde_yaml::Value::String(episode.url.clone()),
    );

    let html = FIXTURE
        .layout_engine
        .render_page("podcast", &episode.content, &fm, &FIXTURE.site_context)
        .unwrap();

    assert!(!html.contains("data-tab=\"transcript\""));
    assert!(html.contains("Timestamps coming soon..."));
}

#[test]
fn test_podcast_todo_link_filtering() {
    if !site_dir().exists() {
        return;
    }
    let html = render_episode("technical-writing-for-data-scientists");

    assert!(!html.contains("Apple Podcasts"));
    assert!(!html.contains("icon-platform-spotify"));
}

#[test]
fn test_podcast_quotable_clips_json_ld() {
    if !site_dir().exists() {
        return;
    }
    let html = render_episode("ab-testing-and-product-experimentation");

    assert!(html.contains("\"@type\": \"Clip\"") || html.contains("\"@type\":\"Clip\""),);
    assert!(html.contains("startOffset"));
    assert!(html.contains("endOffset"));
}

#[test]
fn test_podcast_related_episodes() {
    if !site_dir().exists() {
        return;
    }
    let html = render_episode("building-agentic-ai-engineering-tooling-retrieval-evaluation");

    assert!(html.contains("Related episodes"));
    assert!(html.contains("related-episode-card"));
}

#[test]
fn test_podcast_inline_javascript() {
    if !site_dir().exists() {
        return;
    }
    let episode = FIXTURE.episodes.first().unwrap();
    let mut fm = episode.front_matter.clone();
    fm.insert(
        "url".to_string(),
        serde_yaml::Value::String(episode.url.clone()),
    );

    let html = FIXTURE
        .layout_engine
        .render_page("podcast", &episode.content, &fm, &FIXTURE.site_context)
        .unwrap();

    assert!(html.contains("Tab switching functionality"));
    assert!(html.contains("podcast-tab-button"));
}

#[test]
fn test_podcast_newsletter_cta() {
    if !site_dir().exists() {
        return;
    }
    let episode = FIXTURE.episodes.first().unwrap();
    let mut fm = episode.front_matter.clone();
    fm.insert(
        "url".to_string(),
        serde_yaml::Value::String(episode.url.clone()),
    );

    let html = FIXTURE
        .layout_engine
        .render_page("podcast", &episode.content, &fm, &FIXTURE.site_context)
        .unwrap();

    assert!(html.contains("mc-embedded-subscribe-form"));
}

#[test]
fn test_podcast_json_ld_season_and_series() {
    if !site_dir().exists() {
        return;
    }
    let html = render_episode("building-agentic-ai-engineering-tooling-retrieval-evaluation");

    assert!(html.contains("\"seasonNumber\": 22"));
    assert!(html.contains("DataTalks.Club Podcast"));
    assert!(html.contains("partOfSeries"));
}

#[test]
fn test_podcast_episode_no_guests_no_crash() {
    if !site_dir().exists() {
        return;
    }
    let episode = FIXTURE
        .episodes
        .iter()
        .find(|e| !e.front_matter.contains_key("guests"));
    if let Some(episode) = episode {
        let mut fm = episode.front_matter.clone();
        fm.insert(
            "url".to_string(),
            serde_yaml::Value::String(episode.url.clone()),
        );

        let result = FIXTURE.layout_engine.render_page(
            "podcast",
            &episode.content,
            &fm,
            &FIXTURE.site_context,
        );
        assert!(
            result.is_ok(),
            "Episode without guests should render: {:?}",
            result.err()
        );
        let html = result.unwrap();
        assert!(!html.contains("guest-bio-card"));
    }
}

#[test]
fn test_podcast_json_ld_parseable() {
    if !site_dir().exists() {
        return;
    }
    let files_to_check = [
        "building-agentic-ai-engineering-tooling-retrieval-evaluation.html",
        "technical-writing-for-data-scientists.html",
        "ab-testing-and-product-experimentation.html",
    ];

    for filename in &files_to_check {
        let path = PODCAST_OUTPUT.path().join("podcast").join(filename);
        let content = fs::read_to_string(&path).unwrap();

        let json_start = content.find(r#"<script type="application/ld+json">"#);
        assert!(json_start.is_some(), "Should find JSON-LD in {}", filename);
        let start = json_start.unwrap() + r#"<script type="application/ld+json">"#.len();
        let end = content[start..].find("</script>").unwrap() + start;
        let json_str = content[start..end].trim();

        let parsed: Result<serde_json::Value, _> = serde_json::from_str(json_str);
        assert!(
            parsed.is_ok(),
            "JSON-LD should be valid JSON in {}: {:?}\nJSON: {}",
            filename,
            parsed.err(),
            &json_str[..json_str.len().min(500)]
        );
    }
}

// ========================================================================
// Podcast context building
// ========================================================================

#[test]
fn test_podcast_site_context_has_podcast_and_people() {
    if !site_dir().exists() {
        return;
    }
    // Already tested via FIXTURE, but verify explicitly
    assert!(
        FIXTURE.site_context.get("podcast").is_some(),
        "Should have site.podcast"
    );
    assert!(
        FIXTURE.site_context.get("people").is_some(),
        "Should have site.people"
    );
}
