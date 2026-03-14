//! Integration tests for the full site build pipeline (Issue 19).
//!
//! These tests exercise the complete build flow: config loading, collection
//! loading, template rendering, sitemap/feed generation, and static file copying.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use rustkyll::collection::{self, CollectionItem};
use rustkyll::config::SiteConfig;
use rustkyll::data;
use rustkyll::feed::{self, FeedOptions};
use rustkyll::generator;
use rustkyll::sitemap;
use rustkyll::static_files;
use rustkyll::template::layout::LayoutEngine;

fn site_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datatalksclub.github.io")
}

/// Run the full build pipeline programmatically, mirroring what the CLI does.
fn run_build(source: &Path, destination: &Path) -> (usize, usize, usize, usize, Vec<String>) {
    // Load config
    let config = SiteConfig::from_file(&source.join("_config.yml")).unwrap();

    // Load data
    let data_dir = source.join("_data");
    let data_tree = if data_dir.exists() {
        data::load_data(&data_dir).unwrap()
    } else {
        HashMap::new()
    };

    // Load collections
    let mut collections: HashMap<String, Vec<CollectionItem>> = HashMap::new();
    for collection_name in config.collections.keys() {
        let (items, _errors) =
            collection::load_collection(collection_name, source, &config).unwrap();
        collections.insert(collection_name.clone(), items);
    }

    // Load posts
    if !collections.contains_key("posts") {
        let (posts, _) = collection::load_collection("posts", source, &config).unwrap();
        collections.insert("posts".to_string(), posts);
    }

    // Load pages
    let (pages, _) = collection::load_pages(source).unwrap();

    // Build context
    let site_context =
        generator::build_site_context(&config, &collections, &data_tree, Some(source));

    // Create layout engine
    let layout_engine =
        LayoutEngine::new(&source.join("_layouts"), &source.join("_includes")).unwrap();

    // Create destination
    if destination.exists() {
        fs::remove_dir_all(destination).unwrap();
    }
    fs::create_dir_all(destination).unwrap();

    // Generate collection pages
    let mut collection_pages = 0;
    let mut all_errors = Vec::new();

    for (name, items) in &collections {
        let result = generator::generate_collection_pages(
            items,
            name,
            &config,
            &layout_engine,
            &site_context,
            destination,
        )
        .unwrap();
        collection_pages += result.generated;
        all_errors.extend(result.errors);
    }

    // Generate standalone pages
    let page_result = generator::generate_standalone_pages(
        &pages,
        &config,
        &layout_engine,
        &site_context,
        destination,
    )
    .unwrap();
    let standalone_pages = page_result.generated;
    all_errors.extend(page_result.errors);

    // Copy static files (before sitemap/feed so generated files take precedence)
    let static_count = static_files::copy_static_files(source, destination, &config).unwrap();

    // Generate sitemap (after static copy to overwrite any source sitemap)
    let collections_vec: Vec<(String, Vec<CollectionItem>)> = collections.into_iter().collect();
    let sitemap_count =
        sitemap::generate_sitemap(&config.url, &collections_vec, &pages, destination).unwrap();

    // Generate feed (after static copy to overwrite any source feed)
    let posts_for_feed: Vec<CollectionItem> = collections_vec
        .iter()
        .filter(|(name, _)| name == "posts")
        .flat_map(|(_, items)| items.clone())
        .collect();
    feed::write_atom_feed(
        &posts_for_feed,
        &config,
        &FeedOptions::default(),
        destination,
    )
    .unwrap();

    (
        collection_pages,
        standalone_pages,
        sitemap_count,
        static_count,
        all_errors,
    )
}

// ============================================================================
// Full build via library API -- single comprehensive test to avoid multiple
// slow parallel builds in debug mode
// ============================================================================

#[test]
fn test_full_build_real_site() {
    let source = site_dir();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");

    let start = Instant::now();
    let (collection_pages, standalone_pages, sitemap_entries, static_file_count, errors) =
        run_build(&source, &dest);
    let elapsed = start.elapsed();

    // Print summary for debugging
    eprintln!("Build completed in {:.2}s", elapsed.as_secs_f64());
    eprintln!("  Collection pages: {}", collection_pages);
    eprintln!("  Standalone pages: {}", standalone_pages);
    eprintln!("  Sitemap entries:  {}", sitemap_entries);
    eprintln!("  Static files:     {}", static_file_count);
    eprintln!("  Errors:           {}", errors.len());

    // -- Page counts --
    assert!(
        collection_pages > 100,
        "Expected 100+ collection pages, got {}",
        collection_pages
    );
    assert!(
        standalone_pages > 0,
        "Expected some standalone pages, got {}",
        standalone_pages
    );
    assert!(
        sitemap_entries > 100,
        "Expected 100+ sitemap entries, got {}",
        sitemap_entries
    );
    assert!(
        static_file_count > 10,
        "Expected 10+ static files, got {}",
        static_file_count
    );

    // -- Sitemap --
    let sitemap_path = dest.join("sitemap.xml");
    assert!(sitemap_path.exists(), "sitemap.xml should exist");
    let sitemap_content = fs::read_to_string(&sitemap_path).unwrap();
    assert!(
        sitemap_content.starts_with("<?xml"),
        "sitemap.xml should start with XML declaration, got: {}",
        &sitemap_content[..50.min(sitemap_content.len())]
    );
    assert!(sitemap_content.contains("<urlset"));
    assert!(sitemap_content.contains("https://datatalks.club/"));

    // -- Feed --
    let feed_path = dest.join("feed.xml");
    assert!(feed_path.exists(), "feed.xml should exist");
    let feed_content = fs::read_to_string(&feed_path).unwrap();
    assert!(feed_content.starts_with("<?xml"));
    assert!(feed_content.contains("xmlns=\"http://www.w3.org/2005/Atom\""));
    assert!(feed_content.contains("<entry>"));

    // -- Static files --
    assert!(
        dest.join("assets/styles.css").exists(),
        "assets/styles.css should be copied"
    );
    assert!(dest.join("CNAME").exists(), "CNAME should be copied");
    assert!(
        dest.join("robots.txt").exists(),
        "robots.txt should be copied"
    );
    assert!(
        dest.join("favicon.ico").exists(),
        "favicon.ico should be copied"
    );

    // -- Collection directories --
    assert!(
        dest.join("people").is_dir(),
        "people/ directory should exist"
    );
    assert!(dest.join("books").is_dir(), "books/ directory should exist");
    assert!(
        dest.join("podcast").is_dir(),
        "podcast/ directory should exist"
    );
    assert!(
        dest.join("people/alexeygrigorev.html").exists(),
        "alexeygrigorev.html should exist"
    );

    // -- No underscore dirs in output --
    for entry in fs::read_dir(&dest).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        assert!(
            !name.starts_with('_'),
            "Output should not contain underscore-prefixed entry: {}",
            name
        );
    }
}

// ============================================================================
// CLI integration tests
// ============================================================================

#[test]
fn test_cli_build_with_real_site() {
    let source = site_dir();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");

    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg(source.to_str().unwrap())
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .output()
        .expect("failed to run rustkyll binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Build should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // Verify summary output
    assert!(
        stdout.contains("Build complete!"),
        "Should print completion message, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Collection pages:"),
        "Should print collection page count"
    );
    assert!(
        stdout.contains("Standalone pages:"),
        "Should print standalone page count"
    );
    assert!(stdout.contains("Time:"), "Should print timing info");

    // Verify output files exist
    assert!(dest.join("sitemap.xml").exists());
    assert!(dest.join("feed.xml").exists());
}

#[test]
fn test_cli_build_invalid_source_fails() {
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg("/nonexistent/path")
        .output()
        .expect("failed to run rustkyll binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Build failed"));
}

// ============================================================================
// Minimal synthetic site build
// ============================================================================

#[test]
fn test_build_minimal_synthetic_site() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("site");
    let dest = tmp.path().join("output");

    // Create minimal site structure
    fs::create_dir_all(source.join("_layouts")).unwrap();
    fs::create_dir_all(source.join("_includes")).unwrap();
    fs::create_dir_all(source.join("_data")).unwrap();
    fs::create_dir_all(source.join("_people")).unwrap();

    // Config
    fs::write(
        source.join("_config.yml"),
        r#"
url: "https://example.com"
name: "Test Site"
title: "Test Site"
permalink: "/blog/:title.html"
collections:
  people:
    output: true
    permalink: "/:collection/:title.html"
defaults:
  - scope:
      type: people
    values:
      layout: person
"#,
    )
    .unwrap();

    // Layout
    fs::write(
        source.join("_layouts/person.html"),
        "<html><body>{{ content }}</body></html>",
    )
    .unwrap();

    fs::write(
        source.join("_layouts/page.html"),
        "<html><body>{{ content }}</body></html>",
    )
    .unwrap();

    // Data file
    fs::write(
        source.join("_data/items.yml"),
        "- name: thing1\n- name: thing2\n",
    )
    .unwrap();

    // Collection item
    fs::write(
        source.join("_people/alice.md"),
        "---\ntitle: Alice\n---\nHello from Alice.",
    )
    .unwrap();

    fs::write(
        source.join("_people/bob.md"),
        "---\ntitle: Bob\n---\nHello from Bob.",
    )
    .unwrap();

    // Standalone page
    fs::write(
        source.join("index.md"),
        "---\ntitle: Home\nlayout: page\npermalink: /index.html\n---\nWelcome!",
    )
    .unwrap();

    // Static file
    fs::write(source.join("CNAME"), "example.com").unwrap();

    // Run build via CLI
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg(source.to_str().unwrap())
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .output()
        .expect("failed to run rustkyll binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Build should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // Verify outputs
    assert!(
        dest.join("people/alice.html").exists(),
        "alice.html should exist"
    );
    assert!(
        dest.join("people/bob.html").exists(),
        "bob.html should exist"
    );
    assert!(dest.join("index.html").exists(), "index.html should exist");
    assert!(
        dest.join("sitemap.xml").exists(),
        "sitemap.xml should exist"
    );
    assert!(dest.join("feed.xml").exists(), "feed.xml should exist");
    assert!(dest.join("CNAME").exists(), "CNAME should be copied");

    // Verify HTML content
    let alice_html = fs::read_to_string(dest.join("people/alice.html")).unwrap();
    assert!(
        alice_html.contains("Hello from Alice"),
        "Alice page should contain content"
    );
    assert!(
        alice_html.contains("<html>"),
        "Alice page should be wrapped in layout"
    );

    // Verify sitemap content
    let sitemap = fs::read_to_string(dest.join("sitemap.xml")).unwrap();
    assert!(sitemap.contains("https://example.com/"));
    assert!(sitemap.contains("/people/alice.html"));
    assert!(sitemap.contains("/people/bob.html"));

    // Verify summary output
    assert!(stdout.contains("Collection pages: 2"));
    assert!(stdout.contains("Standalone pages: 1"));
}

// ============================================================================
// Phase timing tests (Issue 31)
// ============================================================================

#[test]
fn test_cli_build_prints_phase_timing() {
    let source = site_dir();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");

    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg(source.to_str().unwrap())
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .output()
        .expect("failed to run rustkyll binary");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "Build should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify phase timing output
    assert!(
        stdout.contains("Phase timing:"),
        "Should print phase timing header"
    );
    assert!(
        stdout.contains("Collections:"),
        "Should print collections timing"
    );
    assert!(
        stdout.contains("Generation:"),
        "Should print generation timing"
    );
    assert!(
        stdout.contains("Static files:"),
        "Should print static files timing"
    );
    assert!(
        stdout.contains("Sitemap/Feed:"),
        "Should print sitemap/feed timing"
    );
}

// ============================================================================
// Parallel collection loading test (Issue 31)
// ============================================================================

#[test]
fn test_parallel_collection_loading() {
    use rayon::prelude::*;

    let source = site_dir();
    let config = SiteConfig::from_file(&source.join("_config.yml")).unwrap();

    // Load all collection names
    let mut collection_names: Vec<String> = config.collections.keys().cloned().collect();
    if !collection_names.contains(&"posts".to_string()) {
        collection_names.push("posts".to_string());
    }

    // Load in parallel
    let loaded: Vec<(String, Vec<CollectionItem>)> = collection_names
        .par_iter()
        .map(|name| {
            let (items, _errors) = collection::load_collection(name, &source, &config).unwrap();
            (name.clone(), items)
        })
        .collect();

    // Verify all collections loaded
    let loaded_names: Vec<&str> = loaded.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        loaded_names.contains(&"posts"),
        "Should load posts collection"
    );
    assert!(
        loaded_names.contains(&"people"),
        "Should load people collection"
    );
    assert!(
        loaded_names.contains(&"books"),
        "Should load books collection"
    );

    // Verify each collection has items
    for (name, items) in &loaded {
        assert!(!items.is_empty(), "Collection '{}' should have items", name);
    }
}
