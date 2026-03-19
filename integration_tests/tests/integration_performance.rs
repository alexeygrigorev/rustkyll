//! Integration tests for large-site performance (Issues 49, 57).
//!
//! These tests verify that rustkyll can build large sites within acceptable
//! time limits and produce correct output.
//!
//! Run with: cargo test -p integration-tests --test integration_performance

use std::collections::HashMap;
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

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn websites_dir() -> PathBuf {
    project_root().join("websites")
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
fn test_dtc_site_build_time() {
    let source = dtc_site_dir();
    if !source.exists() {
        eprintln!("Skipping: DTC site not found at {:?}", source);
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

    eprintln!("DTC build times: {:?}, median: {:.2}s", times, median);

    let threshold = if is_ci() { 90.0 } else { 30.0 };
    assert!(
        median < threshold,
        "DTC site should build in under {:.0}s (debug), took {:.2}s (median of 3 runs)",
        threshold,
        median
    );
}

#[test]
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
fn test_dtc_output_html_file_count() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");

    let (collection_pages, standalone_pages, _, _) = build_site_cached(&source, &dest);

    let html_count = count_html_files(dest.as_path());

    eprintln!(
        "DTC output: {} collection pages, {} standalone pages, {} HTML files on disk",
        collection_pages, standalone_pages, html_count
    );

    let expected = 785;
    assert!(
        html_count >= expected - 5 && html_count <= expected + 5,
        "Expected {} +/-5 HTML files, got {}",
        expected,
        html_count
    );
}

#[test]
fn test_dtc_output_no_raw_liquid_tags() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

    let mut files_with_raw_tags = Vec::new();
    for entry in walkdir(dest.as_path()) {
        if entry.ends_with(".html") {
            let content = fs::read_to_string(&entry).unwrap_or_default();
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
fn test_dtc_blog_post_has_expected_content() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

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
fn test_dtc_podcast_page_has_expected_content() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

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
fn test_dtc_person_page_has_expected_content() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

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
fn test_dtc_output_file_tree_complete() {
    let source = dtc_site_dir();
    if !source.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");
    build_site_cached(&source, &dest);

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
    assert!(dest.join("index.html").exists(), "index.html should exist");
    assert!(
        dest.join("sitemap.xml").exists(),
        "sitemap.xml should exist"
    );
}

#[test]
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
