use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::{Parser, Subcommand};

use rustkyll::collection::{self, CollectionItem};
use rustkyll::config::SiteConfig;
use rustkyll::data;
use rustkyll::feed::{self, FeedOptions};
use rustkyll::generator::{self, GeneratorError};
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

/// Summary of a completed build.
#[derive(Debug, Default)]
struct BuildSummary {
    collection_pages: usize,
    standalone_pages: usize,
    sitemap_entries: usize,
    static_files: usize,
    errors: Vec<String>,
}

/// Run the full site build pipeline.
fn build_site(source: &Path, destination: &Path) -> Result<BuildSummary, BuildError> {
    let mut summary = BuildSummary::default();

    // 1. Load config
    let config_path = source.join("_config.yml");
    let config = SiteConfig::from_file(&config_path)?;

    // 2. Load data
    let data_dir = source.join("_data");
    let data_tree = if data_dir.exists() {
        data::load_data(&data_dir)?
    } else {
        HashMap::new()
    };

    // 3. Load all collections
    let mut collections: HashMap<String, Vec<CollectionItem>> = HashMap::new();
    let mut all_load_errors = Vec::new();

    for collection_name in config.collections.keys() {
        let (items, errors) = collection::load_collection(collection_name, source, &config)?;
        for err in &errors {
            all_load_errors.push(format!("collection '{}': {}", collection_name, err));
        }
        collections.insert(collection_name.clone(), items);
    }

    // Load posts separately (not in config.collections but always expected)
    if !collections.contains_key("posts") {
        let (posts, errors) = collection::load_collection("posts", source, &config)?;
        for err in &errors {
            all_load_errors.push(format!("collection 'posts': {}", err));
        }
        collections.insert("posts".to_string(), posts);
    }

    // 4. Load standalone pages
    let (pages, page_errors) = collection::load_pages(source)?;
    for err in &page_errors {
        all_load_errors.push(format!("pages: {}", err));
    }

    // 5. Build site context
    let site_context =
        generator::build_site_context(&config, &collections, &data_tree, Some(source));

    // 6. Create layout engine
    let layouts_dir = source.join("_layouts");
    let includes_dir = source.join("_includes");
    let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir)?;

    // 7. Clean and create destination directory
    if destination.exists() {
        std::fs::remove_dir_all(destination)?;
    }
    std::fs::create_dir_all(destination)?;

    // 8. Generate collection pages (in parallel across collections)
    let collection_names: Vec<String> = collections.keys().cloned().collect();
    for name in &collection_names {
        if let Some(items) = collections.get(name) {
            let result = generator::generate_collection_pages(
                items,
                name,
                &config,
                &layout_engine,
                &site_context,
                destination,
            )?;
            summary.collection_pages += result.generated;
            summary.errors.extend(result.errors);
        }
    }

    // 9. Generate standalone pages
    let page_result = generator::generate_standalone_pages(
        &pages,
        &config,
        &layout_engine,
        &site_context,
        destination,
    )?;
    summary.standalone_pages = page_result.generated;
    summary.errors.extend(page_result.errors);

    // 10. Copy static files (before sitemap/feed so generated files take precedence)
    let static_count = static_files::copy_static_files(source, destination, &config)?;
    summary.static_files = static_count;

    // 11. Generate sitemap.xml (after static copy to overwrite any source sitemap)
    let collections_vec: Vec<(String, Vec<CollectionItem>)> = collections.into_iter().collect();
    let sitemap_count =
        sitemap::generate_sitemap(&config.url, &collections_vec, &pages, destination)?;
    summary.sitemap_entries = sitemap_count;

    // 12. Generate feed.xml (from posts, after static copy to overwrite any source feed)
    let posts_for_feed: Vec<&CollectionItem> = collections_vec
        .iter()
        .filter(|(name, _)| name == "posts")
        .flat_map(|(_, items)| items.iter())
        .collect();
    let posts_owned: Vec<CollectionItem> = posts_for_feed.into_iter().cloned().collect();
    feed::write_atom_feed(&posts_owned, &config, &FeedOptions::default(), destination)?;

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
        }) => {
            let start = Instant::now();

            println!("Source:      {}", source.display());
            println!("Destination: {}", destination.display());
            println!();

            match build_site(&source, &destination) {
                Ok(summary) => {
                    let elapsed = start.elapsed();
                    let total_pages = summary.collection_pages + summary.standalone_pages;

                    println!("Build complete!");
                    println!("  Collection pages: {}", summary.collection_pages);
                    println!("  Standalone pages: {}", summary.standalone_pages);
                    println!("  Total pages:      {}", total_pages);
                    println!("  Sitemap entries:  {}", summary.sitemap_entries);
                    println!("  Static files:     {}", summary.static_files);
                    println!("  Time:             {:.2}s", elapsed.as_secs_f64());

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
                Err(e) => {
                    eprintln!("Build failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            println!("Hello from rustkyll! Use --help to see available commands.");
        }
    }
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
}
