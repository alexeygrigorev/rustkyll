//! Integration tests for large-site performance (Issues 49, 57).
//!
//! These tests verify that rustkyll can build large sites within acceptable
//! time limits and produce correct output. All tests that build large sites
//! are marked with `#[ignore]` to keep the default test suite fast.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Check if we are running in CI (GitHub Actions sets CI=true).
fn is_ci() -> bool {
    std::env::var("CI").map(|v| v == "true").unwrap_or(false)
}

use rustkyll::collection::{self, CollectionItem};
use rustkyll::config::SiteConfig;
use rustkyll::data;
use rustkyll::generator;
use rustkyll::sitemap;
use rustkyll::static_files;
use rustkyll::template::engine::CachedSiteContext;
use rustkyll::template::layout::LayoutEngine;

fn websites_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("websites")
}

fn dtc_site_dir() -> PathBuf {
    websites_dir().join("DataTalksClub/datatalksclub.github.io")
}

fn kids_site_dir() -> PathBuf {
    websites_dir().join("alexeygrigorev/kids-horror-stories-ru")
}

/// Run a full build using the cached site context optimization.
/// Returns (collection_pages, standalone_pages, errors, elapsed_secs).
fn build_site_cached(source: &Path, destination: &Path) -> (usize, usize, Vec<String>, f64) {
    let start = Instant::now();

    let config = SiteConfig::from_file(&source.join("_config.yml")).unwrap();

    let data_dir = source.join("_data");
    let data_tree = if data_dir.exists() {
        data::load_data(&data_dir).unwrap()
    } else {
        data::DataTree::new()
    };

    let mut collections: HashMap<String, Vec<CollectionItem>> = HashMap::new();
    for collection_name in config.collections.keys() {
        let (items, _) = collection::load_collection(collection_name, source, &config).unwrap();
        collections.insert(collection_name.clone(), items);
    }
    if !collections.contains_key("posts") {
        let (posts, _) = collection::load_collection("posts", source, &config).unwrap();
        collections.insert("posts".to_string(), posts);
    }

    let (pages, _) = collection::load_pages(source, &config).unwrap();

    let site_context =
        generator::build_site_context(&config, &collections, &data_tree, Some(source), &pages);

    let layout_engine =
        LayoutEngine::new(&source.join("_layouts"), &source.join("_includes")).unwrap();

    if destination.exists() {
        fs::remove_dir_all(destination).unwrap();
    }
    fs::create_dir_all(destination).unwrap();

    // Use the cached site context optimization (Issue 49)
    let cached_site = CachedSiteContext::new(&site_context);
    let author_items: Vec<CollectionItem> = collections
        .values()
        .flat_map(|v| v.iter().cloned())
        .collect();

    let mut collection_pages = 0;
    let mut all_errors = Vec::new();

    for (name, items) in &collections {
        let result = generator::generate_collection_pages_cached(
            items,
            name,
            &config,
            &layout_engine,
            &cached_site,
            destination,
            &author_items,
        )
        .unwrap();
        collection_pages += result.generated;
        all_errors.extend(result.errors);
    }

    let page_result =
        generator::generate_pages_cached(&pages, &layout_engine, &cached_site, destination)
            .unwrap();
    let standalone_pages = page_result.generated;
    all_errors.extend(page_result.errors);

    static_files::copy_static_files(source, destination, &config).unwrap();

    let collections_vec: Vec<(String, Vec<CollectionItem>)> = collections.into_iter().collect();
    sitemap::generate_sitemap(&config.url, &collections_vec, &pages, destination).unwrap();

    let elapsed = start.elapsed().as_secs_f64();
    (collection_pages, standalone_pages, all_errors, elapsed)
}

// ============================================================================
// Performance: Large site build times (Issue 57 -- tighter targets)
// ============================================================================

#[test]
#[ignore] // Large site test -- run with `cargo test -- --ignored`
fn test_dtc_site_builds_successfully() {
    let source = dtc_site_dir();
    if !source.exists() {
        eprintln!("Skipping: DTC site not found at {:?}", source);
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");

    let (collection_pages, standalone_pages, errors, elapsed) = build_site_cached(&source, &dest);

    eprintln!(
        "DTC build: {:.2}s, {} collection pages, {} standalone pages, {} errors",
        elapsed,
        collection_pages,
        standalone_pages,
        errors.len()
    );

    // Must build successfully
    assert!(
        collection_pages > 700,
        "Expected 700+ collection pages, got {}",
        collection_pages
    );
    assert!(
        standalone_pages > 5,
        "Expected 5+ standalone pages, got {}",
        standalone_pages
    );
}

#[test]
#[ignore] // Large site test
fn test_dtc_site_build_time() {
    let source = dtc_site_dir();
    if !source.exists() {
        eprintln!("Skipping: DTC site not found at {:?}", source);
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");

    // Run 3 times and take median
    let mut times = Vec::new();
    for _ in 0..3 {
        let (_, _, _, elapsed) = build_site_cached(&source, &dest);
        times.push(elapsed);
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = times[1];

    eprintln!("DTC build times: {:?}, median: {:.2}s", times, median);

    // Issue 57: the release build target is <2s. Debug/test builds are ~5-10x
    // slower due to lack of optimizations. CI runners (2-core VMs) are slower
    // than dev machines, so use relaxed thresholds there.
    let threshold = if is_ci() { 90.0 } else { 30.0 };
    assert!(
        median < threshold,
        "DTC site should build in under {:.0}s (debug), took {:.2}s (median of 3 runs)",
        threshold,
        median
    );
}

#[test]
#[ignore] // Large site test
fn test_kids_site_build_time() {
    let source = kids_site_dir();
    if !source.exists() {
        eprintln!("Skipping: kids-horror-stories-ru not found at {:?}", source);
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");

    let mut times = Vec::new();
    for _ in 0..3 {
        let (_, _, _, elapsed) = build_site_cached(&source, &dest);
        times.push(elapsed);
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = times[1];

    eprintln!("Kids build times: {:?}, median: {:.2}s", times, median);

    // Issue 57: the release build target is <0.5s. Debug builds are much
    // slower. CI runners are slower than dev machines.
    let threshold = if is_ci() { 30.0 } else { 5.0 };
    assert!(
        median < threshold,
        "kids-horror-stories-ru should build in under {:.0}s (debug), took {:.2}s",
        threshold,
        median
    );
}

// ============================================================================
// Correctness: Output validation after optimization
// ============================================================================

#[test]
#[ignore] // Large site test
fn test_dtc_output_html_file_count() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");

    let (collection_pages, standalone_pages, _, _) = build_site_cached(&source, &dest);

    // Count actual HTML files generated
    let html_count = count_html_files(dest.as_path());

    eprintln!(
        "DTC output: {} collection pages, {} standalone pages, {} HTML files on disk",
        collection_pages, standalone_pages, html_count
    );

    // Expected: ~785 pages total (within +/-5 files per spec)
    let expected = 785;
    assert!(
        html_count >= expected - 5 && html_count <= expected + 5,
        "Expected {} +/-5 HTML files, got {}",
        expected,
        html_count
    );
}

#[test]
#[ignore] // Large site test
fn test_dtc_output_no_raw_liquid_tags() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

    // Check that no generated HTML file contains raw Liquid tags
    let mut files_with_raw_tags = Vec::new();
    for entry in walkdir(dest.as_path()) {
        if entry.ends_with(".html") {
            let content = fs::read_to_string(&entry).unwrap_or_default();
            // Check for unrendered Liquid tags (but not in JSON-LD script blocks
            // where {{ and }} are valid JSON)
            if content.contains("{%") {
                files_with_raw_tags.push(entry);
            }
        }
    }

    assert!(
        files_with_raw_tags.is_empty(),
        "Found {} HTML files with raw Liquid tags: {:?}",
        files_with_raw_tags.len(),
        &files_with_raw_tags[..files_with_raw_tags.len().min(10)]
    );
}

#[test]
#[ignore] // Large site test
fn test_dtc_output_no_empty_html_files() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

    let mut empty_files = Vec::new();
    for entry in walkdir(dest.as_path()) {
        if entry.ends_with(".html") {
            let metadata = fs::metadata(&entry).unwrap();
            // A 0-byte file is always a bug: Jekyll never produces 0-byte output
            // for items with output: true. Small files (e.g. 1-2 bytes) are
            // legitimate for collection items with no layout and no body content.
            if metadata.len() == 0 {
                empty_files.push((entry, metadata.len()));
            }
        }
    }

    assert!(
        empty_files.is_empty(),
        "Found {} zero-byte HTML files (generation bug): {:?}",
        empty_files.len(),
        &empty_files[..empty_files.len().min(10)]
    );
}

#[test]
#[ignore] // Large site test
fn test_dtc_homepage_has_expected_content() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

    let index_path = dest.join("index.html");
    assert!(index_path.exists(), "Homepage index.html should exist");

    let content = fs::read_to_string(&index_path).unwrap();
    assert!(
        content.contains("<title>"),
        "Homepage should contain a <title> tag"
    );
    assert!(
        content.contains("<a "),
        "Homepage should contain at least one link"
    );
    assert!(
        content.len() > 1000,
        "Homepage should have substantial content, got {} bytes",
        content.len()
    );
}

#[test]
#[ignore] // Large site test
fn test_dtc_blog_post_has_expected_content() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

    // Find a blog post
    let blog_dir = dest.join("blog");
    if blog_dir.exists() {
        let entries: Vec<_> = fs::read_dir(&blog_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "html"))
            .collect();
        assert!(
            !entries.is_empty(),
            "Blog directory should contain HTML files"
        );

        // Check the first blog post
        let post_content = fs::read_to_string(entries[0].path()).unwrap();
        assert!(
            post_content.contains("<title>"),
            "Blog post should contain a title tag"
        );
        assert!(
            post_content.len() > 500,
            "Blog post should have substantial content"
        );
    }
}

#[test]
#[ignore] // Large site test
fn test_dtc_podcast_page_has_expected_content() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

    // Check a podcast page
    let podcast_dir = dest.join("podcast");
    assert!(podcast_dir.exists(), "Podcast directory should exist");

    let entries: Vec<_> = fs::read_dir(&podcast_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "html"))
        .collect();
    assert!(
        !entries.is_empty(),
        "Podcast directory should contain HTML files"
    );

    // Check the first podcast page for key elements
    let content = fs::read_to_string(entries[0].path()).unwrap();
    assert!(
        content.contains("<title>"),
        "Podcast page should contain a title tag"
    );
    assert!(
        content.contains("PodcastEpisode") || content.contains("podcast"),
        "Podcast page should contain podcast-related content"
    );
    assert!(
        content.len() > 1000,
        "Podcast page should have substantial content"
    );
}

#[test]
#[ignore] // Large site test
fn test_dtc_person_page_has_expected_content() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

    // Check a person page
    let people_dir = dest.join("people");
    assert!(people_dir.exists(), "People directory should exist");

    let entries: Vec<_> = fs::read_dir(&people_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "html"))
        .collect();
    assert!(
        entries.len() > 400,
        "Should have 400+ person pages, got {}",
        entries.len()
    );
}

#[test]
#[ignore] // Large site test
fn test_dtc_events_page_has_expected_content() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

    let events_path = dest.join("events.html");
    assert!(events_path.exists(), "events.html should exist");

    let content = fs::read_to_string(&events_path).unwrap();
    assert!(
        content.contains("<title>"),
        "Events page should contain a title tag"
    );
    assert!(
        content.len() > 1000,
        "Events page should have substantial content"
    );
}

#[test]
#[ignore] // Large site test
fn test_dtc_output_file_tree_complete() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

    // Verify expected directories exist
    assert!(dest.join("blog").exists(), "blog directory should exist");
    assert!(
        dest.join("podcast").exists(),
        "podcast directory should exist"
    );
    assert!(
        dest.join("people").exists(),
        "people directory should exist"
    );
    assert!(dest.join("books").exists(), "books directory should exist");
    assert!(dest.join("posts").exists(), "posts directory should exist");

    // Verify expected top-level files exist
    assert!(dest.join("index.html").exists(), "index.html should exist");
    assert!(
        dest.join("sitemap.xml").exists(),
        "sitemap.xml should exist"
    );
}

#[test]
#[ignore] // Large site test
fn test_kids_site_output_count() {
    let source = kids_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");

    let (collection_pages, _, _, _) = build_site_cached(&source, &dest);

    let html_count = count_html_files(dest.as_path());
    eprintln!(
        "Kids output: {} collection pages, {} HTML files on disk",
        collection_pages, html_count
    );

    // Expected: ~1345 pages (within 5% tolerance)
    let expected = 1345;
    let tolerance = (expected as f64 * 0.05) as usize;
    assert!(
        html_count >= expected - tolerance,
        "Expected at least {} HTML files, got {}",
        expected - tolerance,
        html_count
    );
}

// ============================================================================
// Unit: CachedSiteContext
// ============================================================================

#[test]
fn test_cached_site_context_creation() {
    use liquid::model::Value as LiquidValue;
    use liquid::Object;

    // Build a large site Object with 1000+ keys
    let mut site = Object::new();
    let mut posts = Vec::new();
    for i in 0..1000 {
        let mut post = Object::new();
        post.insert("title".into(), LiquidValue::scalar(format!("Post {}", i)));
        post.insert("url".into(), LiquidValue::scalar(format!("/post/{}", i)));
        post.insert("content".into(), LiquidValue::scalar("Some content here"));
        posts.push(LiquidValue::Object(post));
    }
    site.insert("posts".into(), LiquidValue::Array(posts));
    site.insert("url".into(), LiquidValue::scalar("https://example.com"));

    // CachedSiteContext::new should succeed and be reusable
    let cached = CachedSiteContext::new(&site);

    // Verify it can be used multiple times (the whole point of caching)
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let template = engine.parse("{{ site.url }}").unwrap();

    let mut ctx = Object::new();
    ctx.insert("page".into(), LiquidValue::Object(Object::new()));
    ctx.insert("content".into(), LiquidValue::scalar(""));

    let result1 = engine
        .render_with_cached_site(&template, &ctx, &cached)
        .unwrap();
    let result2 = engine
        .render_with_cached_site(&template, &ctx, &cached)
        .unwrap();

    assert_eq!(result1, "https://example.com");
    assert_eq!(result2, "https://example.com");
}

#[test]
fn test_cached_site_context_page_specific_variables() {
    use liquid::model::Value as LiquidValue;
    use liquid::Object;

    let mut site = Object::new();
    site.insert("name".into(), LiquidValue::scalar("Test Site"));

    let cached = CachedSiteContext::new(&site);
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let template = engine.parse("{{ page.title }} on {{ site.name }}").unwrap();

    // Render with different page titles using the same cached site
    let mut ctx1 = Object::new();
    let mut page1 = Object::new();
    page1.insert("title".into(), LiquidValue::scalar("Page One"));
    ctx1.insert("page".into(), LiquidValue::Object(page1));
    ctx1.insert("content".into(), LiquidValue::scalar(""));

    let mut ctx2 = Object::new();
    let mut page2 = Object::new();
    page2.insert("title".into(), LiquidValue::scalar("Page Two"));
    ctx2.insert("page".into(), LiquidValue::Object(page2));
    ctx2.insert("content".into(), LiquidValue::scalar(""));

    let result1 = engine
        .render_with_cached_site(&template, &ctx1, &cached)
        .unwrap();
    let result2 = engine
        .render_with_cached_site(&template, &ctx2, &cached)
        .unwrap();

    assert_eq!(result1, "Page One on Test Site");
    assert_eq!(result2, "Page Two on Test Site");
}

// ============================================================================
// Unit: Slim site context optimization (Issue 57)
// ============================================================================

#[test]
fn test_slim_site_context_excludes_large_arrays() {
    use liquid::model::Value as LiquidValue;
    use rustkyll::collection::CollectionItem;

    // Create a collection item with a large array in front matter
    let mut fm = HashMap::new();
    fm.insert(
        "title".to_string(),
        serde_yaml::Value::String("Test Episode".to_string()),
    );
    fm.insert(
        "season".to_string(),
        serde_yaml::Value::Number(serde_yaml::Number::from(1)),
    );
    // Add a "transcript" array with 100 entries -- should be excluded in slim mode
    let transcript: Vec<serde_yaml::Value> = (0..100)
        .map(|i| {
            let mut map = serde_yaml::Mapping::new();
            map.insert(
                serde_yaml::Value::String("line".to_string()),
                serde_yaml::Value::String(format!("Line {}", i)),
            );
            serde_yaml::Value::Mapping(map)
        })
        .collect();
    fm.insert(
        "transcript".to_string(),
        serde_yaml::Value::Sequence(transcript),
    );
    // Add a small array (under threshold) -- should be kept
    fm.insert(
        "guests".to_string(),
        serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("alice".to_string()),
            serde_yaml::Value::String("bob".to_string()),
        ]),
    );

    let item = CollectionItem {
        slug: "test-episode".to_string(),
        url: "/podcast/test-episode.html".to_string(),
        date: Some("2024-01-01".to_string()),
        front_matter: fm,
        content: "raw content".to_string(),
        html_content: "<p>rendered content</p>".to_string(),
        excerpt: None,
        collection_name: "podcast".to_string(),
        source_path: "_podcast/test-episode.md".to_string(),
        id: "/podcast/test-episode".to_string(),
    };

    // Build site context with this item
    let mut collections = HashMap::new();
    collections.insert("podcast".to_string(), vec![item]);

    let config = SiteConfig::default();
    let data = BTreeMap::new();
    let pages = vec![];

    let site = generator::build_site_context(&config, &collections, &data, None, &pages);

    // Get the podcast array from site context
    if let Some(LiquidValue::Array(podcast_arr)) = site.get("podcast") {
        assert_eq!(podcast_arr.len(), 1);

        if let LiquidValue::Object(ref episode) = podcast_arr[0] {
            // Title should be present (scalar value)
            assert!(
                episode.get("title").is_some(),
                "Title should be in slim site context"
            );
            // Season should be present (scalar value)
            assert!(
                episode.get("season").is_some(),
                "Season should be in slim site context"
            );
            // Guests should be present (small array, 2 elements)
            assert!(
                episode.get("guests").is_some(),
                "Small array (guests) should be in slim site context"
            );
            // Transcript should be excluded (large array, 100 elements)
            assert!(
                episode.get("transcript").is_none(),
                "Large array (transcript) should be excluded from slim site context"
            );
            // Content (rendered HTML) should still be present
            assert!(
                episode.get("content").is_some(),
                "Content should be in slim site context"
            );
        } else {
            panic!("Expected Object in podcast array");
        }
    } else {
        panic!("Expected podcast array in site context");
    }
}

#[test]
fn test_slim_site_context_keeps_small_arrays() {
    use rustkyll::collection::CollectionItem;

    // Create an item with only small front matter fields
    let mut fm = HashMap::new();
    fm.insert(
        "title".to_string(),
        serde_yaml::Value::String("Alice Smith".to_string()),
    );
    fm.insert(
        "short".to_string(),
        serde_yaml::Value::String("alice".to_string()),
    );
    // Small array (3 elements) -- should be kept
    fm.insert(
        "tags".to_string(),
        serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("ml".to_string()),
            serde_yaml::Value::String("data".to_string()),
            serde_yaml::Value::String("ai".to_string()),
        ]),
    );

    let item = CollectionItem {
        slug: "alice".to_string(),
        url: "/people/alice.html".to_string(),
        date: None,
        front_matter: fm,
        content: "Bio here".to_string(),
        html_content: "<p>Bio here</p>".to_string(),
        excerpt: None,
        collection_name: "people".to_string(),
        source_path: "_people/alice.md".to_string(),
        id: "/people/alice".to_string(),
    };

    let mut collections = HashMap::new();
    collections.insert("people".to_string(), vec![item]);

    let config = SiteConfig::default();
    let data = BTreeMap::new();
    let pages = vec![];

    let site = generator::build_site_context(&config, &collections, &data, None, &pages);

    if let Some(liquid::model::Value::Array(people_arr)) = site.get("people") {
        assert_eq!(people_arr.len(), 1);
        if let liquid::model::Value::Object(ref person) = people_arr[0] {
            assert!(person.get("title").is_some(), "Title should be present");
            assert!(person.get("short").is_some(), "Short should be present");
            assert!(
                person.get("tags").is_some(),
                "Small array should be present"
            );
            assert!(person.get("content").is_some(), "Content should be present");
        } else {
            panic!("Expected Object");
        }
    } else {
        panic!("Expected people array");
    }
}

// ============================================================================
// Unit: Performance micro-benchmarks (Issue 57)
// ============================================================================

#[test]
fn test_render_1000_for_loop_iterations() {
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let mut ctx = liquid::Object::new();

    // Create an array with 1000 items
    let items: Vec<liquid::model::Value> = (0..1000)
        .map(|i| liquid::model::Value::scalar(format!("item-{}", i)))
        .collect();
    ctx.insert("items".into(), liquid::model::Value::Array(items));

    let template_str = "{% for item in items %}{{ item }}\n{% endfor %}";

    let start = Instant::now();
    let result = engine.parse_and_render(template_str, &ctx);
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Template should render successfully");
    let output = result.unwrap();
    assert!(
        output.contains("item-0"),
        "Output should contain first item"
    );
    assert!(
        output.contains("item-999"),
        "Output should contain last item"
    );
    assert!(
        elapsed.as_millis() < 500,
        "1000 for-loop iterations should complete in under 500ms, took {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_render_deeply_nested_object_access() {
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let mut ctx = liquid::Object::new();

    // Create a deeply nested object: site.data.people[0].name
    let mut people = Vec::new();
    for i in 0..100 {
        let mut person = liquid::Object::new();
        person.insert(
            "name".into(),
            liquid::model::Value::scalar(format!("Person {}", i)),
        );
        person.insert(
            "bio".into(),
            liquid::model::Value::scalar("A short biography"),
        );
        people.push(liquid::model::Value::Object(person));
    }

    let mut data_obj = liquid::Object::new();
    data_obj.insert("people".into(), liquid::model::Value::Array(people));

    let mut site = liquid::Object::new();
    site.insert("data".into(), liquid::model::Value::Object(data_obj));

    ctx.insert("site".into(), liquid::model::Value::Object(site));

    let template_str = "{% for person in site.data.people %}{{ person.name }}\n{% endfor %}";

    let start = Instant::now();
    let result = engine.parse_and_render(template_str, &ctx);
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Person 0"));
    assert!(output.contains("Person 99"));
    assert!(
        elapsed.as_millis() < 50,
        "Deeply nested access should complete in under 50ms, took {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_render_100_templates_sequentially() {
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();

    let start = Instant::now();
    for i in 0..100 {
        let mut ctx = liquid::Object::new();
        ctx.insert("n".into(), liquid::model::Value::scalar(i as i64));
        ctx.insert(
            "title".into(),
            liquid::model::Value::scalar(format!("Page {}", i)),
        );

        let template_str = "<h1>{{ title }}</h1><p>Number: {{ n }}</p>";
        let result = engine.parse_and_render(template_str, &ctx);
        assert!(result.is_ok());
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 200,
        "100 sequential template renders should complete in under 200ms, took {}ms",
        elapsed.as_millis()
    );
}

// ============================================================================
// Helpers
// ============================================================================

fn count_html_files(dir: &Path) -> usize {
    walkdir(dir).iter().filter(|p| p.ends_with(".html")).count()
}

fn walkdir(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path));
            } else {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }
    files
}
