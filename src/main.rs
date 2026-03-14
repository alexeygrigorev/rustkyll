use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use rayon::prelude::*;

use rustkyll::collection::{self, CollectionItem};
use rustkyll::config::SiteConfig;
use rustkyll::data;
use rustkyll::feed::{self, FeedOptions};
use rustkyll::generator::{self, GeneratorError};
use rustkyll::incremental::{self, BuildManifest, IncrementalAction};
use rustkyll::sitemap;
use rustkyll::static_files;
use rustkyll::template::layout::LayoutEngine;

#[derive(Debug, Parser)]
#[command(
    name = "rustkyll",
    about = "A static site generator for DataTalks.Club"
)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Build the static site
    Build {
        /// Source directory (default: current directory)
        #[arg(long, default_value = ".")]
        source: PathBuf,

        /// Destination directory (default: _site)
        #[arg(long, default_value = "_site")]
        destination: PathBuf,

        /// Enable incremental builds (only rebuild changed pages)
        #[arg(long, default_value_t = false)]
        incremental: bool,

        /// Force a full rebuild, ignoring the incremental manifest
        #[arg(long, default_value_t = false)]
        force: bool,
    },

    /// Build and serve the site locally
    Serve {
        /// Source directory (default: current directory)
        #[arg(long, default_value = ".")]
        source: PathBuf,

        /// Destination directory (default: _site)
        #[arg(long, default_value = "_site")]
        destination: PathBuf,

        /// Port to serve on (default: 4000)
        #[arg(long, default_value_t = 4000)]
        port: u16,

        /// Enable live reload (default: true)
        #[arg(long, default_value_t = true)]
        livereload: bool,

        /// Disable live reload
        #[arg(long, default_value_t = false)]
        no_livereload: bool,
    },
}

/// Errors that can occur during the build pipeline.
#[derive(Debug, thiserror::Error)]
enum BuildError {
    #[error("config error: {0}")]
    Config(#[from] rustkyll::config::ConfigError),

    #[error("data error: {0}")]
    Data(#[from] rustkyll::data::DataError),

    #[error("collection error: {0}")]
    Collection(#[from] rustkyll::collection::CollectionError),

    #[error("generator error: {0}")]
    Generator(#[from] GeneratorError),

    #[error("template error: {0}")]
    Template(#[from] rustkyll::template::TemplateError),

    #[error("static file error: {0}")]
    StaticFile(#[from] rustkyll::static_files::StaticFileError),

    #[error("feed error: {0}")]
    Feed(#[from] rustkyll::feed::FeedError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Options controlling the build behavior.
#[derive(Debug, Clone)]
struct BuildOptions {
    /// Whether incremental builds are enabled.
    incremental: bool,
    /// Whether to force a full rebuild (overrides incremental).
    force: bool,
}

/// Per-phase timing information for build profiling.
#[derive(Debug, Default)]
struct PhaseTiming {
    config: Duration,
    data: Duration,
    collections: Duration,
    pages: Duration,
    incremental: Duration,
    context: Duration,
    layouts: Duration,
    generation: Duration,
    static_files: Duration,
    sitemap_feed: Duration,
    manifest: Duration,
}

/// Summary of a completed build.
#[derive(Debug, Default)]
struct BuildSummary {
    collection_pages: usize,
    standalone_pages: usize,
    sitemap_entries: usize,
    static_files: usize,
    errors: Vec<String>,
    /// Whether this was an incremental build that skipped everything.
    skipped_all: bool,
    /// Number of source files that triggered a rebuild.
    changed_sources: usize,
    /// Per-phase timing breakdown.
    timing: PhaseTiming,
}

/// Collect all source file relative paths from loaded collections and pages.
fn collect_all_source_paths(
    collections: &HashMap<String, Vec<CollectionItem>>,
    pages: &[collection::Page],
) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for items in collections.values() {
        for item in items {
            paths.push(PathBuf::from(&item.source_path));
        }
    }
    for page in pages {
        paths.push(PathBuf::from(&page.source_path));
    }
    paths
}

/// Run the full site build pipeline.
fn build_site(
    source: &Path,
    destination: &Path,
    options: &BuildOptions,
) -> Result<BuildSummary, BuildError> {
    let mut summary = BuildSummary::default();

    // 1. Load config
    let phase_start = Instant::now();
    let config_path = source.join("_config.yml");
    let config = SiteConfig::from_file(&config_path)?;
    summary.timing.config = phase_start.elapsed();

    // 2. Load data
    let phase_start = Instant::now();
    let data_dir = source.join("_data");
    let data_tree = if data_dir.exists() {
        data::load_data(&data_dir)?
    } else {
        HashMap::new()
    };
    summary.timing.data = phase_start.elapsed();

    // 3. Load all collections in parallel
    let phase_start = Instant::now();
    let mut all_load_errors = Vec::new();

    // Gather all collection names to load (including "posts" if not in config)
    let mut collection_names_to_load: Vec<String> = config.collections.keys().cloned().collect();
    if !collection_names_to_load.contains(&"posts".to_string()) {
        collection_names_to_load.push("posts".to_string());
    }

    // Load all collections in parallel using rayon
    type CollectionLoadResult = (
        String,
        Result<
            (Vec<CollectionItem>, Vec<collection::CollectionError>),
            collection::CollectionError,
        >,
    );
    let loaded: Vec<CollectionLoadResult> = collection_names_to_load
        .par_iter()
        .map(|name| {
            let result = collection::load_collection(name, source, &config);
            (name.clone(), result)
        })
        .collect();

    let mut collections: HashMap<String, Vec<CollectionItem>> = HashMap::new();
    for (name, result) in loaded {
        let (items, errors) = result?;
        for err in errors {
            all_load_errors.push(format!("collection '{}': {}", name, err));
        }
        collections.insert(name, items);
    }
    summary.timing.collections = phase_start.elapsed();

    // 4. Load standalone pages
    let phase_start = Instant::now();
    let (pages, page_errors) = collection::load_pages(source)?;
    for err in &page_errors {
        all_load_errors.push(format!("pages: {}", err));
    }
    summary.timing.pages = phase_start.elapsed();

    // 5. Incremental build check
    let phase_start = Instant::now();
    let current_globals = incremental::collect_global_files(source);
    let all_source_paths = collect_all_source_paths(&collections, &pages);
    let current_sources = incremental::collect_source_files(source, &all_source_paths);

    let do_incremental = options.incremental && !options.force;
    let action = if do_incremental {
        match incremental::load_manifest(destination) {
            Some(prev_manifest) => {
                incremental::determine_action(&prev_manifest, &current_globals, &current_sources)
            }
            None => IncrementalAction::FullRebuild, // no manifest = first build
        }
    } else {
        IncrementalAction::FullRebuild
    };
    summary.timing.incremental = phase_start.elapsed();

    // Handle skip-all case
    if action == IncrementalAction::SkipAll {
        summary.skipped_all = true;
        summary.errors.extend(all_load_errors);
        return Ok(summary);
    }

    // Determine which source files changed (for partial rebuilds)
    let changed_set: Option<std::collections::HashSet<String>> = match &action {
        IncrementalAction::RebuildPartial(changed) => {
            summary.changed_sources = changed.len();
            Some(changed.iter().cloned().collect())
        }
        _ => None,
    };

    // 6. Build site context (always uses full collections for cross-references)
    let phase_start = Instant::now();
    let site_context =
        generator::build_site_context(&config, &collections, &data_tree, Some(source), &pages);
    summary.timing.context = phase_start.elapsed();

    // 7. Create layout engine
    let phase_start = Instant::now();
    let layouts_dir = source.join("_layouts");
    let includes_dir = source.join("_includes");
    let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir)?;
    summary.timing.layouts = phase_start.elapsed();

    // 8. Clean and create destination directory (only for full rebuilds)
    if changed_set.is_none() {
        // Full rebuild: wipe destination
        if destination.exists() {
            std::fs::remove_dir_all(destination)?;
        }
    }
    std::fs::create_dir_all(destination)?;

    // 9. Generate collection pages (avoid cloning: pass slices directly)
    let phase_start = Instant::now();
    for (name, items) in &collections {
        // For partial rebuilds, filter to only changed items
        let filtered: Vec<CollectionItem>;
        let items_slice: &[CollectionItem] = match &changed_set {
            Some(changed) => {
                filtered = items
                    .iter()
                    .filter(|item| changed.contains(&item.source_path))
                    .cloned()
                    .collect();
                &filtered
            }
            None => items,
        };

        if !items_slice.is_empty() {
            // For collections that need JSON-LD with author resolution (e.g., books),
            // pass the people collection for lookup.
            let people_items: &[CollectionItem] = collections
                .get("people")
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let result = generator::generate_collection_pages_with_people(
                items_slice,
                name,
                &config,
                &layout_engine,
                &site_context,
                destination,
                people_items,
            )?;
            summary.collection_pages += result.generated;
            summary.errors.extend(result.errors);
        }
    }

    // 10. Generate standalone pages (avoid cloning: pass slice directly)
    let filtered_pages: Vec<collection::Page>;
    let pages_slice: &[collection::Page] = match &changed_set {
        Some(changed) => {
            filtered_pages = pages
                .iter()
                .filter(|page| changed.contains(&page.source_path))
                .cloned()
                .collect();
            &filtered_pages
        }
        None => &pages,
    };

    if !pages_slice.is_empty() {
        let page_result = generator::generate_standalone_pages(
            pages_slice,
            &config,
            &layout_engine,
            &site_context,
            destination,
        )?;
        summary.standalone_pages = page_result.generated;
        summary.errors.extend(page_result.errors);
    }
    summary.timing.generation = phase_start.elapsed();

    // 11. Copy static files (before sitemap/feed so generated files take precedence)
    let phase_start = Instant::now();
    let static_count = static_files::copy_static_files(source, destination, &config)?;
    summary.static_files = static_count;
    summary.timing.static_files = phase_start.elapsed();

    // 12. Generate sitemap.xml and feed.xml
    let phase_start = Instant::now();

    // Build collections_vec by iterating the HashMap directly (avoid extra collect)
    let collections_vec: Vec<(String, Vec<CollectionItem>)> = collections.into_iter().collect();
    let sitemap_count =
        sitemap::generate_sitemap(&config.url, &collections_vec, &pages, destination)?;
    summary.sitemap_entries = sitemap_count;

    // 13. Generate feed.xml (from posts)
    if let Some((_, posts_vec)) = collections_vec.iter().find(|(name, _)| name == "posts") {
        feed::write_atom_feed(posts_vec, &config, &FeedOptions::default(), destination)?;
    } else {
        // No posts collection -- write empty feed
        feed::write_atom_feed(&[], &config, &FeedOptions::default(), destination)?;
    }
    summary.timing.sitemap_feed = phase_start.elapsed();

    // 14. Save the build manifest
    let phase_start = Instant::now();
    let mut output_map = HashMap::new();
    for (_, items) in &collections_vec {
        for item in items {
            let output_path = item.url.trim_start_matches('/').to_string();
            output_map.insert(item.source_path.clone(), output_path);
        }
    }
    for page in &pages {
        let output_path = page.url.trim_start_matches('/').to_string();
        output_map.insert(page.source_path.clone(), output_path);
    }

    let manifest = BuildManifest {
        source_files: current_sources,
        output_map,
        global_files: current_globals,
    };
    incremental::save_manifest(destination, &manifest)?;
    summary.timing.manifest = phase_start.elapsed();

    // Include load errors in summary
    summary.errors.extend(all_load_errors);

    Ok(summary)
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Build {
            source,
            destination,
            incremental,
            force,
        }) => {
            let start = Instant::now();

            println!("Source:      {}", source.display());
            println!("Destination: {}", destination.display());
            if incremental {
                println!("Mode:        incremental");
            }
            if force {
                println!("Mode:        force (full rebuild)");
            }
            println!();

            let options = BuildOptions { incremental, force };

            match build_site(&source, &destination, &options) {
                Ok(summary) => {
                    let elapsed = start.elapsed();

                    if summary.skipped_all {
                        println!("Nothing to do -- all files up to date.");
                        println!("  Time: {:.2}s", elapsed.as_secs_f64());
                    } else {
                        let total_pages = summary.collection_pages + summary.standalone_pages;

                        println!("Build complete!");
                        if summary.changed_sources > 0 {
                            println!(
                                "  Changed sources:  {} (incremental)",
                                summary.changed_sources
                            );
                        }
                        println!("  Collection pages: {}", summary.collection_pages);
                        println!("  Standalone pages: {}", summary.standalone_pages);
                        println!("  Total pages:      {}", total_pages);
                        println!("  Sitemap entries:  {}", summary.sitemap_entries);
                        println!("  Static files:     {}", summary.static_files);
                        println!("  Time:             {:.2}s", elapsed.as_secs_f64());

                        // Per-phase timing breakdown
                        let t = &summary.timing;
                        println!();
                        println!("Phase timing:");
                        println!("  Config:       {:.3}s", t.config.as_secs_f64());
                        println!("  Data:         {:.3}s", t.data.as_secs_f64());
                        println!("  Collections:  {:.3}s", t.collections.as_secs_f64());
                        println!("  Pages:        {:.3}s", t.pages.as_secs_f64());
                        println!("  Incremental:  {:.3}s", t.incremental.as_secs_f64());
                        println!("  Context:      {:.3}s", t.context.as_secs_f64());
                        println!("  Layouts:      {:.3}s", t.layouts.as_secs_f64());
                        println!("  Generation:   {:.3}s", t.generation.as_secs_f64());
                        println!("  Static files: {:.3}s", t.static_files.as_secs_f64());
                        println!("  Sitemap/Feed: {:.3}s", t.sitemap_feed.as_secs_f64());
                        println!("  Manifest:     {:.3}s", t.manifest.as_secs_f64());

                        if !summary.errors.is_empty() {
                            println!();
                            println!("Warnings ({}):", summary.errors.len());
                            for (i, err) in summary.errors.iter().enumerate() {
                                if i < 20 {
                                    println!("  - {}", err);
                                }
                            }
                            if summary.errors.len() > 20 {
                                println!("  ... and {} more", summary.errors.len() - 20);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Build failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Serve {
            source,
            destination,
            port,
            livereload,
            no_livereload,
        }) => {
            let livereload_enabled = livereload && !no_livereload;

            // Build the site first
            println!("Building site before serving...");
            let options = BuildOptions {
                incremental: false,
                force: false,
            };
            match build_site(&source, &destination, &options) {
                Ok(summary) => {
                    let total_pages = summary.collection_pages + summary.standalone_pages;
                    println!("Build complete: {} pages generated.", total_pages);
                }
                Err(e) => {
                    eprintln!("Build failed: {}", e);
                    std::process::exit(1);
                }
            }

            let ws_port = port + 1; // WebSocket on port+1

            if livereload_enabled {
                let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

                // Set up Ctrl+C handler
                let shutdown_ctrlc = std::sync::Arc::clone(&shutdown);
                let _ = ctrlc_flag(&shutdown_ctrlc);

                // Channel for reload signals
                let (reload_tx, reload_rx) = std::sync::mpsc::channel();

                // Start WebSocket server thread
                let shutdown_ws = std::sync::Arc::clone(&shutdown);
                let ws_handle = std::thread::spawn(move || {
                    rustkyll::livereload::start_websocket_server(ws_port, reload_rx, shutdown_ws);
                });

                // Start file watcher thread
                let source_clone = source.clone();
                let destination_clone = destination.clone();
                let shutdown_watcher = std::sync::Arc::clone(&shutdown);
                let watcher_handle = std::thread::spawn(move || {
                    let src = source_clone.clone();
                    let dst = destination_clone.clone();
                    let build_fn = Box::new(move || {
                        let opts = BuildOptions {
                            incremental: false,
                            force: false,
                        };
                        build_site(&src, &dst, &opts)
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    });
                    if let Err(e) = rustkyll::livereload::start_file_watcher(
                        source_clone,
                        destination_clone,
                        reload_tx,
                        shutdown_watcher,
                        build_fn,
                    ) {
                        eprintln!("Watcher error: {}", e);
                    }
                });

                // Start HTTP server (blocks)
                if let Err(e) = rustkyll::server::start_server(&destination, port, Some(ws_port)) {
                    eprintln!("Server error: {}", e);
                    std::process::exit(1);
                }

                shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
                let _ = ws_handle.join();
                let _ = watcher_handle.join();
            } else {
                // No live reload -- just serve
                println!("Live reload disabled.");
                if let Err(e) = rustkyll::server::start_server(&destination, port, None) {
                    eprintln!("Server error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            println!("Hello from rustkyll! Use --help to see available commands.");
        }
    }
}

/// Set a flag to `true` when Ctrl+C is pressed.
fn ctrlc_flag(_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<(), std::io::Error> {
    // Since tiny_http blocks on incoming_requests, the process will
    // terminate naturally on SIGINT/SIGTERM
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parses_no_args() {
        let cli = Cli::try_parse_from(["rustkyll"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parses_build_subcommand() {
        let cli = Cli::try_parse_from(["rustkyll", "build"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Build {
                source: _,
                destination: _,
                ..
            })
        ));
    }

    #[test]
    fn test_cli_parses_build_with_source() {
        let cli = Cli::try_parse_from(["rustkyll", "build", "--source", "/tmp/site"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Build { source, .. }) => {
                assert_eq!(source, PathBuf::from("/tmp/site"));
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_cli_parses_build_with_destination() {
        let cli = Cli::try_parse_from(["rustkyll", "build", "--destination", "/tmp/output"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Build { destination, .. }) => {
                assert_eq!(destination, PathBuf::from("/tmp/output"));
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_cli_parses_build_with_both_args() {
        let cli = Cli::try_parse_from([
            "rustkyll",
            "build",
            "--source",
            "/src",
            "--destination",
            "/dst",
        ]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Build {
                source,
                destination,
                ..
            }) => {
                assert_eq!(source, PathBuf::from("/src"));
                assert_eq!(destination, PathBuf::from("/dst"));
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_cli_build_default_values() {
        let cli = Cli::try_parse_from(["rustkyll", "build"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Build {
                source,
                destination,
                ..
            }) => {
                assert_eq!(source, PathBuf::from("."));
                assert_eq!(destination, PathBuf::from("_site"));
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_cli_rejects_unknown_flag() {
        let result = Cli::try_parse_from(["rustkyll", "--nonexistent"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_help_flag() {
        let result = Cli::try_parse_from(["rustkyll", "--help"]);
        // --help causes clap to return an error (it's a special exit)
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }

    // --- Serve command CLI tests ---

    #[test]
    fn test_cli_parses_serve_defaults() {
        let cli = Cli::try_parse_from(["rustkyll", "serve"]).unwrap();
        match cli.command {
            Some(Commands::Serve {
                source,
                destination,
                port,
                livereload,
                no_livereload,
            }) => {
                assert_eq!(source, PathBuf::from("."));
                assert_eq!(destination, PathBuf::from("_site"));
                assert_eq!(port, 4000);
                assert!(livereload);
                assert!(!no_livereload);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_parses_serve_with_port() {
        let cli = Cli::try_parse_from(["rustkyll", "serve", "--port", "8080"]).unwrap();
        match cli.command {
            Some(Commands::Serve { port, .. }) => {
                assert_eq!(port, 8080);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_parses_serve_with_source_and_destination() {
        let cli = Cli::try_parse_from([
            "rustkyll",
            "serve",
            "--source",
            "/tmp/site",
            "--destination",
            "/tmp/out",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Serve {
                source,
                destination,
                ..
            }) => {
                assert_eq!(source, PathBuf::from("/tmp/site"));
                assert_eq!(destination, PathBuf::from("/tmp/out"));
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_parses_serve_all_flags() {
        let cli = Cli::try_parse_from([
            "rustkyll",
            "serve",
            "--port",
            "3000",
            "--source",
            "/src",
            "--destination",
            "/dst",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Serve {
                source,
                destination,
                port,
                ..
            }) => {
                assert_eq!(port, 3000);
                assert_eq!(source, PathBuf::from("/src"));
                assert_eq!(destination, PathBuf::from("/dst"));
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_parses_serve_livereload_enabled_by_default() {
        let cli = Cli::try_parse_from(["rustkyll", "serve"]).unwrap();
        match cli.command {
            Some(Commands::Serve {
                livereload,
                no_livereload,
                ..
            }) => {
                assert!(livereload);
                assert!(!no_livereload);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_parses_serve_no_livereload() {
        let cli = Cli::try_parse_from(["rustkyll", "serve", "--no-livereload"]).unwrap();
        match cli.command {
            Some(Commands::Serve { no_livereload, .. }) => {
                assert!(no_livereload);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_parses_serve_explicit_livereload() {
        let cli = Cli::try_parse_from(["rustkyll", "serve", "--livereload"]).unwrap();
        match cli.command {
            Some(Commands::Serve { livereload, .. }) => {
                assert!(livereload);
            }
            _ => panic!("Expected Serve command"),
        }
    }
}
