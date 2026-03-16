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
        data::DataTree::new()
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
    let (pages, _) = collection::load_pages(source, &config).unwrap();

    // Build context
    let site_context =
        generator::build_site_context(&config, &collections, &data_tree, Some(source), &pages);

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
    if !site_dir().exists() {
        return;
    }
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
    if !site_dir().exists() {
        return;
    }
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
    if !site_dir().exists() {
        return;
    }
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
    if !site_dir().exists() {
        return;
    }
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
// Issue 55: Build with default source (current directory)
// ============================================================================

/// Helper: create a minimal synthetic site in the given directory.
fn create_minimal_site(site_dir: &Path) {
    fs::create_dir_all(site_dir.join("_layouts")).unwrap();
    fs::create_dir_all(site_dir.join("_includes")).unwrap();
    fs::create_dir_all(site_dir.join("_data")).unwrap();

    fs::write(
        site_dir.join("_config.yml"),
        r#"
url: "https://example.com"
name: "Default Source Test"
title: "Default Source Test"
permalink: "/blog/:title.html"
"#,
    )
    .unwrap();

    fs::write(
        site_dir.join("_layouts/page.html"),
        "<html><body>{{ content }}</body></html>",
    )
    .unwrap();

    fs::write(
        site_dir.join("index.md"),
        "---\ntitle: Home\nlayout: page\npermalink: /index.html\n---\nWelcome default source.",
    )
    .unwrap();

    // A data file to verify data loading works
    fs::write(site_dir.join("_data/info.yml"), "key: value\n").unwrap();

    // A static file
    fs::write(site_dir.join("CNAME"), "example.com").unwrap();
}

#[test]
fn test_cli_build_no_source_flag_uses_cwd() {
    if !site_dir().exists() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let site = tmp.path().join("mysite");
    fs::create_dir_all(&site).unwrap();
    create_minimal_site(&site);

    // Run rustkyll build with no --source flag, with CWD set to the site directory.
    // Destination is explicitly set to a temp path to avoid polluting CWD.
    let dest = tmp.path().join("output");
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .current_dir(&site)
        .output()
        .expect("failed to run rustkyll binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Build with default source should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    assert!(
        stdout.contains("Build complete!"),
        "Should show completion message"
    );
    assert!(
        dest.join("index.html").exists(),
        "index.html should be generated"
    );
    assert!(dest.join("CNAME").exists(), "Static files should be copied");

    let index = fs::read_to_string(dest.join("index.html")).unwrap();
    assert!(
        index.contains("Welcome default source"),
        "Output should contain page content"
    );
}

#[test]
fn test_cli_build_default_destination_is_site_in_cwd() {
    if !site_dir().exists() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let site = tmp.path().join("mysite");
    fs::create_dir_all(&site).unwrap();
    create_minimal_site(&site);

    // Run rustkyll build with no --source and no --destination, CWD = site dir.
    // Default destination should be _site/ relative to CWD.
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .current_dir(&site)
        .output()
        .expect("failed to run rustkyll binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Build with all defaults should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // _site/ should appear inside the site directory (CWD)
    let default_dest = site.join("_site");
    assert!(
        default_dest.exists(),
        "_site/ should be created in CWD ({})",
        default_dest.display()
    );
    assert!(
        default_dest.join("index.html").exists(),
        "index.html should be in _site/"
    );
}

#[test]
fn test_cli_build_relative_source_path() {
    if !site_dir().exists() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let site = tmp.path().join("sites").join("mysite");
    fs::create_dir_all(&site).unwrap();
    create_minimal_site(&site);

    let dest = tmp.path().join("output");

    // Run from tmp.path()/sites/ with --source ../sites/mysite (relative)
    let parent = tmp.path().join("sites");
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg("mysite")
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .current_dir(&parent)
        .output()
        .expect("failed to run rustkyll binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Build with relative source should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    assert!(
        dest.join("index.html").exists(),
        "index.html should be generated"
    );
}

#[test]
fn test_cli_build_dotdot_relative_source_path() {
    if !site_dir().exists() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let site = tmp.path().join("mysite");
    fs::create_dir_all(&site).unwrap();
    create_minimal_site(&site);

    let run_dir = tmp.path().join("other");
    fs::create_dir_all(&run_dir).unwrap();

    let dest = tmp.path().join("output");

    // Run from other/ with --source ../mysite
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg("../mysite")
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .current_dir(&run_dir)
        .output()
        .expect("failed to run rustkyll binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Build with ../relative source should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    assert!(
        dest.join("index.html").exists(),
        "index.html should be generated"
    );
}

#[test]
fn test_cli_build_missing_config_graceful_error() {
    if !site_dir().exists() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let empty_dir = tmp.path();
    let dest = tmp.path().join("output");

    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .current_dir(empty_dir)
        .output()
        .expect("failed to run rustkyll binary");

    assert!(
        !output.status.success(),
        "Build should fail when _config.yml is missing"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Build failed"),
        "Should print failure message, got: {}",
        stderr
    );
}

#[test]
fn test_cli_build_explicit_source_override() {
    if !site_dir().exists() {
        return;
    }
    // Verify --source flag overrides default CWD behavior
    let tmp = tempfile::tempdir().unwrap();
    let site = tmp.path().join("actual_site");
    fs::create_dir_all(&site).unwrap();
    create_minimal_site(&site);

    let dest = tmp.path().join("output");

    // Run from tmp root (which has no _config.yml) but --source points to actual_site
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg(site.to_str().unwrap())
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .current_dir(tmp.path())
        .output()
        .expect("failed to run rustkyll binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Build with explicit --source should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    assert!(
        dest.join("index.html").exists(),
        "index.html should be generated"
    );
}

// ============================================================================
// Phase timing tests (Issue 31)
// ============================================================================

#[test]
fn test_cli_build_prints_phase_timing() {
    if !site_dir().exists() {
        return;
    }
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
    if !site_dir().exists() {
        return;
    }
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

// ============================================================================
// Progress output duplicate-line tests (Issue 96)
// ============================================================================

#[test]
fn test_cli_build_no_duplicate_phase_lines() {
    if !site_dir().exists() {
        return;
    }
    let source = site_dir();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");

    // When running as a subprocess, stderr is non-TTY, so phase() is a no-op
    // and phase_done() prints exactly once. This ensures each phase appears
    // as exactly one line.
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg(source.to_str().unwrap())
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .output()
        .expect("failed to run rustkyll binary");

    assert!(
        output.status.success(),
        "Build should succeed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Phase names that have both phase() and phase_done() calls.
    // Each should appear exactly ONCE in the output (not twice).
    let phase_patterns = [
        "Loading data files...",
        "Loading collections...",
        "Copying static files...",
        "Generating sitemap...",
        "Generating feed...",
    ];

    for pattern in &phase_patterns {
        let count = stderr.matches(pattern).count();
        assert_eq!(
            count, 1,
            "Phase '{}' should appear exactly once in stderr, but appeared {} times.\nFull stderr:\n{}",
            pattern, count, stderr
        );
    }
}

#[test]
fn test_cli_build_quiet_no_phase_output() {
    if !site_dir().exists() {
        return;
    }
    let source = site_dir();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("_site");

    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg(source.to_str().unwrap())
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .arg("--quiet")
        .output()
        .expect("failed to run rustkyll binary");

    assert!(
        output.status.success(),
        "Build should succeed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // In quiet mode, no phase messages should appear
    let phase_patterns = [
        "Loading data files...",
        "Loading collections...",
        "Loading config...",
        "Copying static files...",
        "Generating sitemap...",
        "Generating feed...",
    ];

    for pattern in &phase_patterns {
        assert!(
            !stderr.contains(pattern),
            "Quiet mode should not contain '{}' in stderr.\nFull stderr:\n{}",
            pattern,
            stderr
        );
    }
}
