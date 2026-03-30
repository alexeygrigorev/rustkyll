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
use rustkyll::pagination::{self, PaginationConfig};
use rustkyll::progress::ProgressReporter;
use rustkyll::sitemap;
use rustkyll::static_files;
use rustkyll::template::engine::CachedSiteContext;
use rustkyll::template::layout::LayoutEngine;

#[derive(Debug, Parser)]
#[command(
    name = "rustkyll",
    about = "A fast static site generator compatible with Jekyll"
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

        /// Suppress all progress output (only errors are shown)
        #[arg(long, default_value_t = false)]
        quiet: bool,
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

        /// Disable file watching (build once and serve, no rebuilds)
        #[arg(long, default_value_t = false)]
        no_watch: bool,

        /// Suppress all progress output (only errors are shown)
        #[arg(long, default_value_t = false)]
        quiet: bool,

        /// Do not open the browser automatically
        #[arg(long, default_value_t = false)]
        no_browser: bool,
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
    /// Whether to suppress all progress output.
    quiet: bool,
    /// Pre-determined changed file paths (relative to source) for serve-mode
    /// incremental rebuilds. When `Some`, skips manifest-based change detection
    /// and uses these paths directly.
    changed_paths: Option<Vec<String>>,
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

/// Convert a slug to a title by replacing hyphens with spaces and capitalizing
/// each word. Matches Jekyll's `Document#make_title_from_slug` behavior.
/// E.g., "rendering-process" -> "Rendering Process"
fn title_from_slug(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Run the full site build pipeline.
/// Extract redirect URLs from a page/post's front matter.
///
/// Check if a source path is the root index page (used for pagination).
fn is_index_page(source_path: &str) -> bool {
    source_path == "index.html" || source_path == "index.md" || source_path == "index.htm"
}

/// Supports both single string and array values for `redirect_from`:
/// - `redirect_from: /old-url/`
/// - `redirect_from: [/old-1/, /old-2/]`
fn extract_redirect_from(fm: &rustkyll::frontmatter::FrontMatter) -> Vec<String> {
    let mut redirects = Vec::new();
    if let Some(val) = fm.get("redirect_from") {
        match val {
            serde_yaml::Value::String(s) => {
                if !s.is_empty() {
                    redirects.push(s.clone());
                }
            }
            serde_yaml::Value::Sequence(seq) => {
                for item in seq {
                    if let Some(s) = item.as_str() {
                        if !s.is_empty() {
                            redirects.push(s.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    redirects
}

/// Generate a simple HTML redirect page (matches jekyll-redirect-from output).
///
/// When `site_url` is non-empty, produces absolute URLs by prepending
/// `site_url` + `site_baseurl` to the target path. Falls back to the
/// relative `to_url` when `site_url` is empty.
fn generate_redirect_html(
    _from_url: &str,
    to_url: &str,
    site_url: &str,
    site_baseurl: &str,
) -> String {
    let absolute_url = if site_url.is_empty() {
        to_url.to_string()
    } else {
        let base = site_url.trim_end_matches('/');
        let baseurl_part = site_baseurl.trim_end_matches('/');
        let path = if to_url.starts_with('/') {
            to_url.to_string()
        } else {
            format!("/{}", to_url)
        };
        format!("{}{}{}", base, baseurl_part, path)
    };
    format!(
        r#"<!DOCTYPE html>
<html lang="en-US">
  <meta charset="utf-8">
  <title>Redirecting&hellip;</title>
  <link rel="canonical" href="{to}">
  <script>location="{to}"</script>
  <meta http-equiv="refresh" content="0; url={to}">
  <meta name="robots" content="noindex">
  <h1>Redirecting&hellip;</h1>
  <a href="{to}">Click here if you are not redirected.</a>
</html>
"#,
        to = absolute_url
    )
}

fn build_site(
    source: &Path,
    destination: &Path,
    options: &BuildOptions,
) -> Result<BuildSummary, BuildError> {
    let progress = ProgressReporter::new(options.quiet);
    let mut summary = BuildSummary::default();

    // 1. Load config
    progress.phase("Loading config...");
    let phase_start = Instant::now();
    let config_path = source.join("_config.yml");
    let config = if config_path.exists() {
        SiteConfig::from_file(&config_path)?
    } else {
        // Jekyll builds sites without _config.yml using defaults.
        SiteConfig::default()
    };
    summary.timing.config = phase_start.elapsed();

    // 2. Load data
    progress.phase("Loading data files...");
    let phase_start = Instant::now();
    let data_dir = source.join("_data");
    let data_tree = if data_dir.exists() {
        data::load_data(&data_dir)?
    } else {
        data::DataTree::new()
    };
    let data_file_count = data_tree.len();
    progress.phase_done(&format!("Loading data files... {} files", data_file_count));
    summary.timing.data = phase_start.elapsed();

    // 3. Load all collections in parallel
    progress.phase("Loading collections...");
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

    // Jekyll assigns the build timestamp as the default `date` for collection
    // items that don't have an explicit date.  Generate once so every item in
    // this build gets the same value.
    // Determine timezone: explicit config > system timezone > None (UTC)
    let site_tz: Option<chrono_tz::Tz> = config
        .extras
        .get("timezone")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<chrono_tz::Tz>().ok())
        .or_else(|| {
            rustkyll::template::filters::get_system_timezone()
                .and_then(|name| name.parse::<chrono_tz::Tz>().ok())
        });
    let build_time = collection::build_timestamp(site_tz);
    // Issue 474: Jekyll lazily assigns site.time as the default date for all
    // collection documents, but only exposes it as page.date (in front matter)
    // for posts. Non-post items have page.date = nil unless explicitly set.
    for (name, items) in collections.iter_mut() {
        let is_posts = name == "posts";
        collection::backfill_default_dates(items, &build_time, is_posts);
    }

    // Issue 500: Jekyll infers title from slug for collection documents that have
    // no explicit title in front matter. E.g., "rendering-process" -> "Rendering Process".
    // This matches Jekyll's Document#make_title_from_slug behavior.
    for items in collections.values_mut() {
        for item in items.iter_mut() {
            if !item.front_matter.contains_key("title") {
                let title = title_from_slug(&item.slug);
                if !title.is_empty() {
                    item.front_matter
                        .insert("title".to_string(), serde_yaml::Value::String(title));
                }
            }
        }
    }

    // Issue 354: Filter out future-dated posts (Jekyll defaults to future: false)
    let allow_future = config
        .extras
        .get("future")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if let Some(posts) = collections.get_mut("posts") {
        collection::filter_future_posts(posts, allow_future);
    }

    let total_items: usize = collections.values().map(|v| v.len()).sum();
    progress.phase_done(&format!(
        "Loading collections... {} collections, {} items",
        collections.len(),
        total_items
    ));
    summary.timing.collections = phase_start.elapsed();

    // 4. Load standalone pages
    progress.phase("Loading pages...");
    let phase_start = Instant::now();
    let (pages, page_errors) = collection::load_pages(source, &config)?;
    for err in &page_errors {
        all_load_errors.push(format!("pages: {}", err));
    }
    summary.timing.pages = phase_start.elapsed();

    // 4b. Detect URL collisions (Issue 225)
    // Collect all (source_path, url) pairs from collections and pages, then
    // check for duplicates. Emit Jekyll-style "Conflict" warnings to stderr.
    {
        let mut url_entries: Vec<(String, String)> = Vec::new();
        for (name, items) in &collections {
            // Only check collections that produce output
            if name != "posts" {
                if let Some(coll_config) = config.collection(name) {
                    if !coll_config.output {
                        continue;
                    }
                }
            }
            for item in items {
                url_entries.push((item.source_path.clone(), item.url.clone()));
            }
        }
        for page in &pages {
            url_entries.push((page.source_path.clone(), page.url.clone()));
        }
        let collisions = collection::detect_url_collisions(&url_entries);
        for collision in &collisions {
            eprintln!("{}", collection::format_collision_warning(collision));
        }
    }

    // 5. Incremental build check
    let phase_start = Instant::now();
    let current_globals = incremental::collect_global_files(source);
    let all_source_paths = collect_all_source_paths(&collections, &pages);
    let current_sources = incremental::collect_source_files(source, &all_source_paths);

    // If the caller already determined which files changed (serve mode),
    // skip the manifest-based detection and use those paths directly.
    let action = if let Some(ref paths) = options.changed_paths {
        if paths.is_empty() {
            IncrementalAction::SkipAll
        } else {
            IncrementalAction::RebuildPartial(paths.clone())
        }
    } else {
        let do_incremental = options.incremental && !options.force;
        if do_incremental {
            match incremental::load_manifest(destination) {
                Some(prev_manifest) => incremental::determine_action(
                    &prev_manifest,
                    &current_globals,
                    &current_sources,
                ),
                None => IncrementalAction::FullRebuild, // no manifest = first build
            }
        } else {
            IncrementalAction::FullRebuild
        }
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
    // Collect static file paths first so they can be exposed as site.static_files
    let static_file_paths = static_files::collect_static_files(source, &config)?;

    // Set permalink patterns BEFORE building layout engine, because
    // preprocess_jekyll_tags (called during include partial compilation)
    // needs the permalink style to resolve {% link %} tags correctly.
    let expanded_permalink = rustkyll::collection::expand_permalink_style(&config.permalink);
    rustkyll::frontmatter::set_post_permalink_pattern(expanded_permalink);
    rustkyll::collection::set_page_permalink_style(&config.permalink);

    // Set collection-specific permalink suffixes so {% link _docs/file.md %} can
    // produce trailing slashes when the collection's permalink ends with `/`.
    for (name, coll_cfg) in &config.collections {
        let expanded = rustkyll::collection::expand_permalink_style(&coll_cfg.permalink);
        let suffix = if expanded.ends_with('/') { "/" } else { "" };
        rustkyll::collection::set_collection_permalink_suffix(name, suffix);
    }

    // Build site context and load layouts in parallel since they are independent.
    progress.phase("Building site context...");
    let phase_start_context = Instant::now();
    let layouts_dir = source.join("_layouts");
    let includes_dir = source.join("_includes");

    let (mut site_context, layout_result) = rayon::join(
        || {
            generator::build_site_context_with_static_files(
                &config,
                &collections,
                &data_tree,
                Some(source),
                &pages,
                &static_file_paths,
            )
        },
        || LayoutEngine::new(&layouts_dir, &includes_dir),
    );
    let mut layout_engine = layout_result?;

    // Issue 500: Expose config.repository as site.repository in Liquid context.
    // The `repository` field is a named SiteConfig field (not in extras), so
    // build_site_context doesn't automatically include it. Inject it here.
    if let Some(ref repo) = config.repository {
        site_context.insert(
            "repository".into(),
            liquid::model::Value::scalar(repo.clone()),
        );
    }

    summary.timing.context = phase_start_context.elapsed();
    // Layouts loaded in parallel, timing is subsumed by context.
    summary.timing.layouts = std::time::Duration::ZERO;

    // Issue 216: Set markdown processor mode based on config.
    // When the site uses a non-kramdown markdown processor (e.g., CommonMarkGhPages),
    // disable the kramdown-specific inline code class behavior.
    let is_kramdown = config
        .extras
        .get("markdown")
        .and_then(|v| v.as_str())
        .map(|m| m.eq_ignore_ascii_case("kramdown"))
        .unwrap_or(true); // kramdown is Jekyll's default
    layout_engine.set_kramdown_code_classes(is_kramdown);

    // Issue 314: Set markdownify filter list indentation mode.
    // CommonMark sites should NOT indent <li> elements in the markdownify path.
    rustkyll::frontmatter::set_markdownify_indent_lists(is_kramdown);

    // Set markdownify inline code class mode.
    // CommonMark sites should NOT add highlighter-rouge class to inline <code>.
    rustkyll::frontmatter::set_markdownify_code_classes(is_kramdown);

    // Issue 223: Enable HARDBREAKS if the site config has commonmark.options: ["HARDBREAKS"]
    layout_engine.set_hardbreaks(config.has_commonmark_hardbreaks());

    // Issue 294: Enable autolink if the site config has commonmark.extensions: ["autolink"]
    let has_autolink = config.has_commonmark_autolink();
    layout_engine.set_autolink(has_autolink);

    // Also enable autolink in the markdownify filter for CommonMark sites.
    rustkyll::frontmatter::set_markdownify_autolink(has_autolink);

    // 7b. Pre-render collection items that contain Liquid tags (e.g.,
    // `{% include links.html %}` in posts). This ensures `item.html_content`
    // contains fully rendered HTML BEFORE the site context is built, so that
    // aggregation pages (tag pages, news pages) see rendered content instead
    // of raw Liquid tags. (Issue 327, Category C)
    {
        let needs_liquid_prerender: bool = collections
            .values()
            .any(|items| items.iter().any(|item| item.content.contains("{% include")));
        // Only pre-render if few items need it (avoid expensive CachedSiteContext for large sites)
        let prerender_count: usize = collections
            .values()
            .map(|items| {
                items
                    .iter()
                    .filter(|item| item.content.contains("{% include"))
                    .count()
            })
            .sum();
        // Only pre-render for small sites (CachedSiteContext creation is expensive for large sites)
        let total_items: usize = collections.values().map(|items| items.len()).sum();
        if needs_liquid_prerender && prerender_count > 0 && total_items <= 100 {
            // Build a temporary cached site context for Liquid resolution
            let temp_cached_site = CachedSiteContext::new(&site_context);
            for items in collections.values_mut() {
                for item in items.iter_mut() {
                    if item.content.contains("{% include") {
                        let is_markdown_source = item.source_path.ends_with(".md")
                            || item.source_path.ends_with(".markdown");
                        if !is_markdown_source {
                            continue;
                        }
                        let mut page_fm = item.front_matter.clone();
                        page_fm
                            .entry("url".to_string())
                            .or_insert_with(|| serde_yaml::Value::String(item.url.clone()));
                        if !page_fm.contains_key("date") {
                            if let Some(ref date) = item.date {
                                page_fm.insert(
                                    "date".to_string(),
                                    serde_yaml::Value::String(date.clone()),
                                );
                            }
                        }
                        match layout_engine.render_markdown_content_with_cached_site(
                            &item.content,
                            &page_fm,
                            &temp_cached_site,
                        ) {
                            Ok(rendered) => {
                                item.html_content = rendered;
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: failed to pre-render Liquid in '{}': {}",
                                    item.source_path, e
                                );
                            }
                        }
                    }
                }
            }
            // Note: We do NOT rebuild the site context here.
            // The pre-rendered html_content is used during per-page generation
            // when each page renders its own content. Rebuilding the context
            // is expensive (~0.5s for DTC) and only needed if other pages
            // iterate over this collection's rendered content (rare).
        }
    }

    // 8. Clean and create destination directory (only for full rebuilds)
    // Optimization: rename the old directory and delete it in the background,
    // so generation can start immediately while the old files are being removed.
    let _cleanup_handle = if changed_set.is_none() && destination.exists() {
        // Rename to a temporary path for async removal
        let tmp_path = destination.with_extension("_old_cleanup");
        // Remove any leftover cleanup directory from a previous failed build
        let _ = std::fs::remove_dir_all(&tmp_path);
        if std::fs::rename(destination, &tmp_path).is_ok() {
            // Spawn background thread to remove the old directory
            let handle = std::thread::spawn(move || {
                let _ = std::fs::remove_dir_all(&tmp_path);
            });
            Some(handle)
        } else {
            // Rename failed (different filesystem?), fall back to synchronous removal
            std::fs::remove_dir_all(destination)?;
            None
        }
    } else {
        None
    };
    std::fs::create_dir_all(destination)?;

    // 9. Build cached site context ONCE for all page renders.
    // This is the key performance optimization: the LenientValue tree for the
    // site Object (with all posts, pages, collections) is built once and shared
    // across all collection and page renders, avoiding O(n) work per collection.
    progress.phase("Rendering pages...");
    let phase_start = Instant::now();
    let cached_site = CachedSiteContext::new(&site_context);

    // Pre-collect author items once (used for JSON-LD author resolution).
    // Only "people" collection items are needed for author slug resolution,
    // avoiding a full clone of all 777+ collection items.
    let empty_vec = Vec::new();
    let author_items: &[CollectionItem] = collections.get("people").unwrap_or(&empty_vec);

    // Count total renderable pages for progress bar
    let total_renderable: usize = {
        let mut count = 0usize;
        for (name, items) in &collections {
            if name != "posts" {
                if let Some(coll_config) = config.collection(name) {
                    if !coll_config.output {
                        continue;
                    }
                }
            }
            match &changed_set {
                Some(changed) => {
                    count += items
                        .iter()
                        .filter(|item| changed.contains(&item.source_path))
                        .count();
                }
                None => count += items.len(),
            }
        }
        match &changed_set {
            Some(changed) => {
                count += pages
                    .iter()
                    .filter(|page| changed.contains(&page.source_path))
                    .count();
            }
            None => count += pages.len(),
        }
        count
    };
    let render_progress = progress.render_progress(total_renderable as u64, "Rendering");

    // Process all collections in parallel to maximize thread utilization.
    // Each collection's items are also processed in parallel internally (nested par_iter).
    // This avoids idle threads between sequential collection processing.
    // Cache of rendered content for posts with Liquid tags (avoids redundant re-render for feed).
    let rendered_content_cache: HashMap<String, String>;
    {
        use std::sync::Mutex;

        // Pre-filter collections and items.
        let collection_tasks: Vec<(&str, &[CollectionItem])> = collections
            .iter()
            .filter_map(|(name, items)| {
                // Skip collections with output: false (except posts which always output).
                if name != "posts" {
                    if let Some(coll_config) = config.collection(name) {
                        if !coll_config.output {
                            return None;
                        }
                    }
                }
                if items.is_empty() {
                    return None;
                }
                Some((name.as_str(), items.as_slice()))
            })
            .collect();

        // For incremental builds, filter items per collection.
        let filtered_items: Vec<Vec<CollectionItem>> = match &changed_set {
            Some(changed) => collection_tasks
                .iter()
                .map(|(_, items)| {
                    items
                        .iter()
                        .filter(|item| changed.contains(&item.source_path))
                        .cloned()
                        .collect()
                })
                .collect(),
            None => Vec::new(),
        };

        let results: Mutex<Vec<Result<generator::GenerationResult, generator::GeneratorError>>> =
            Mutex::new(Vec::new());

        rayon::scope(|s| {
            for (i, (name, items)) in collection_tasks.iter().enumerate() {
                let items_slice: &[CollectionItem] = if !filtered_items.is_empty() {
                    if filtered_items[i].is_empty() {
                        continue;
                    }
                    &filtered_items[i]
                } else {
                    items
                };
                let config = &config;
                let layout_engine = &layout_engine;
                let cached_site = &cached_site;
                let render_progress = &render_progress;
                let results = &results;
                s.spawn(move |_| {
                    let result = generator::generate_collection_pages_cached_with_progress(
                        items_slice,
                        name,
                        config,
                        layout_engine,
                        cached_site,
                        destination,
                        author_items,
                        Some(render_progress),
                    );
                    results.lock().unwrap().push(result);
                });
            }
        });

        // Collect rendered content cache for feed optimization
        let mut cache = HashMap::new();
        for result in results.into_inner().unwrap() {
            let result = result?;
            summary.collection_pages += result.generated;
            summary.errors.extend(result.errors);
            cache.extend(result.rendered_content);
        }
        rendered_content_cache = cache;
    }

    // 9b. Update post html_content with rendered content from generation cache.
    // Must happen BEFORE pagination (step 10b) so that paginator.posts has
    // the Liquid-processed content (e.g., {% gist %} tags rendered to HTML).
    if let Some(posts) = collections.get_mut("posts") {
        for item in posts.iter_mut() {
            if let Some(rendered) = rendered_content_cache.get(&item.source_path) {
                item.html_content = rendered.clone();
            }
        }
    }

    // 10. Generate standalone pages (avoid cloning: pass slice directly)
    // When pagination is enabled, skip the index page from normal rendering
    // because it will be rendered with the paginator variable in step 10b.
    let pagination_config = PaginationConfig::from_config(&config);
    let has_pagination = pagination_config.is_some()
        && collections.contains_key("posts")
        && pagination::find_index_page(&pages).is_some();

    let filtered_pages: Vec<collection::Page>;
    let pages_slice: &[collection::Page] = match (&changed_set, has_pagination) {
        (Some(changed), true) => {
            filtered_pages = pages
                .iter()
                .filter(|page| {
                    changed.contains(&page.source_path) && !is_index_page(&page.source_path)
                })
                .cloned()
                .collect();
            &filtered_pages
        }
        (Some(changed), false) => {
            filtered_pages = pages
                .iter()
                .filter(|page| changed.contains(&page.source_path))
                .cloned()
                .collect();
            &filtered_pages
        }
        (None, true) => {
            filtered_pages = pages
                .iter()
                .filter(|page| !is_index_page(&page.source_path))
                .cloned()
                .collect();
            &filtered_pages
        }
        (None, false) => &pages,
    };

    if !pages_slice.is_empty() {
        let page_result = generator::generate_pages_cached_with_config_and_progress(
            pages_slice,
            &layout_engine,
            &cached_site,
            destination,
            Some(&config),
            Some(&render_progress),
            Some(source),
        )?;
        summary.standalone_pages = page_result.generated;
        summary.errors.extend(page_result.errors);
    }
    // 10b. Generate pagination pages (jekyll-paginate support)
    if let Some(ref pagination_config) = pagination_config {
        if let Some(posts) = collections.get("posts") {
            if let Some(index_page) = pagination::find_index_page(&pages) {
                let count = pagination::generate_pagination_pages(
                    posts,
                    pagination_config,
                    index_page,
                    &layout_engine,
                    &cached_site,
                    &config,
                    destination,
                )?;
                summary.standalone_pages += count;
            }
        }
    }

    // 10c. Generate redirect pages (jekyll-redirect-from support)
    {
        let mut redirect_count = 0;
        // Check collections for redirect_from front matter
        for items in collections.values() {
            for item in items {
                let redirects = extract_redirect_from(&item.front_matter);
                for redirect_url in &redirects {
                    let target_url = &item.url;
                    let html = generate_redirect_html(
                        redirect_url,
                        target_url,
                        &config.url,
                        &config.baseurl,
                    );
                    let out_path = generator::url_to_output_path(destination, redirect_url);
                    if let Some(parent) = out_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if std::fs::write(&out_path, &html).is_ok() {
                        redirect_count += 1;
                    }
                }
            }
        }
        // Check standalone pages for redirect_from front matter
        for page in &pages {
            let redirects = extract_redirect_from(&page.front_matter);
            for redirect_url in &redirects {
                let target_url = &page.url;
                let html =
                    generate_redirect_html(redirect_url, target_url, &config.url, &config.baseurl);
                let out_path = generator::url_to_output_path(destination, redirect_url);
                if let Some(parent) = out_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if std::fs::write(&out_path, &html).is_ok() {
                    redirect_count += 1;
                }
            }
        }
        summary.standalone_pages += redirect_count;
    }

    // 10c2. Handle redirect_to pages (jekyll-redirect-from support)
    // When a page has redirect_to in front matter, its output should be a
    // redirect HTML page pointing to the target URL.
    {
        let mut redirect_to_count = 0;
        // Check standalone pages for redirect_to front matter
        for page in &pages {
            if let Some(val) = page.front_matter.get("redirect_to") {
                // If the page has a layout that exists in the layout engine,
                // the normal rendering pipeline already rendered it correctly
                // using that layout (e.g., a custom "redirect" layout).
                // Skip the hardcoded redirect override in that case.
                if let Some(serde_yaml::Value::String(layout_name)) =
                    page.front_matter.get("layout")
                {
                    if layout_engine.has_layout(layout_name) {
                        continue;
                    }
                }
                let target = match val {
                    serde_yaml::Value::String(s) => Some(s.clone()),
                    _ => None,
                };
                if let Some(target_url) = target {
                    if !target_url.is_empty() {
                        // For redirect_to, the target URL is used directly (may be
                        // absolute or relative). If it's absolute, use as-is.
                        let absolute_url = if target_url.starts_with("http://")
                            || target_url.starts_with("https://")
                        {
                            target_url.clone()
                        } else {
                            let base = config.url.trim_end_matches('/');
                            let baseurl_part = config.baseurl.trim_end_matches('/');
                            let path = if target_url.starts_with('/') {
                                target_url.clone()
                            } else {
                                format!("/{}", target_url)
                            };
                            format!("{}{}{}", base, baseurl_part, path)
                        };
                        let html = format!(
                            r#"<!DOCTYPE html>
<html lang="en-US">
  <meta charset="utf-8">
  <title>Redirecting&hellip;</title>
  <link rel="canonical" href="{to}">
  <script>location="{to}"</script>
  <meta http-equiv="refresh" content="0; url={to}">
  <meta name="robots" content="noindex">
  <h1>Redirecting&hellip;</h1>
  <a href="{to}">Click here if you are not redirected.</a>
</html>
"#,
                            to = absolute_url
                        );
                        let out_path = generator::url_to_output_path(destination, &page.url);
                        if let Some(parent) = out_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if std::fs::write(&out_path, &html).is_ok() {
                            redirect_to_count += 1;
                        }
                    }
                }
            }
        }
        // Check collections for redirect_to front matter
        for items in collections.values() {
            for item in items {
                if let Some(val) = item.front_matter.get("redirect_to") {
                    // If the item has a layout that exists in the layout engine,
                    // the normal rendering pipeline already rendered it correctly.
                    if let Some(serde_yaml::Value::String(layout_name)) =
                        item.front_matter.get("layout")
                    {
                        if layout_engine.has_layout(layout_name) {
                            continue;
                        }
                    }
                    let target = match val {
                        serde_yaml::Value::String(s) => Some(s.clone()),
                        _ => None,
                    };
                    if let Some(target_url) = target {
                        if !target_url.is_empty() {
                            let absolute_url = if target_url.starts_with("http://")
                                || target_url.starts_with("https://")
                            {
                                target_url.clone()
                            } else {
                                let base = config.url.trim_end_matches('/');
                                let baseurl_part = config.baseurl.trim_end_matches('/');
                                let path = if target_url.starts_with('/') {
                                    target_url.clone()
                                } else {
                                    format!("/{}", target_url)
                                };
                                format!("{}{}{}", base, baseurl_part, path)
                            };
                            let html = format!(
                                r#"<!DOCTYPE html>
<html lang="en-US">
  <meta charset="utf-8">
  <title>Redirecting&hellip;</title>
  <link rel="canonical" href="{to}">
  <script>location="{to}"</script>
  <meta http-equiv="refresh" content="0; url={to}">
  <meta name="robots" content="noindex">
  <h1>Redirecting&hellip;</h1>
  <a href="{to}">Click here if you are not redirected.</a>
</html>
"#,
                                to = absolute_url
                            );
                            let out_path = generator::url_to_output_path(destination, &item.url);
                            if let Some(parent) = out_path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let _ = std::fs::write(&out_path, &html);
                        }
                    }
                }
            }
        }
        if redirect_to_count > 0 {
            // redirect_to pages replace existing output, no new pages added
        }
    }

    // 10d. Generate archive pages (jekyll-archives support)
    if let Some(ref archives_config) = rustkyll::archives::ArchivesConfig::from_config(&config) {
        if let Some(posts) = collections.get("posts") {
            let count = rustkyll::archives::generate_archive_pages(
                posts,
                archives_config,
                &layout_engine,
                &cached_site,
                &config,
                destination,
            )?;
            summary.standalone_pages += count;
        }
    }

    render_progress.finish();
    summary.timing.generation = phase_start.elapsed();

    // 11. Copy static files (before sitemap/feed so generated files take precedence)
    progress.phase("Copying static files...");
    let phase_start = Instant::now();
    let static_count = static_files::copy_static_files(source, destination, &config)?;
    summary.static_files = static_count;
    progress.phase_done(&format!("Copying static files... {} files", static_count));
    summary.timing.static_files = phase_start.elapsed();

    // 12. (moved to step 9b -- post html_content is already updated before pagination)

    // 13. Generate sitemap.xml and feed.xml
    progress.phase("Generating sitemap...");
    let phase_start = Instant::now();

    // Build collections_vec by iterating the HashMap directly (avoid extra collect)
    let collections_vec: Vec<(String, Vec<CollectionItem>)> = collections.into_iter().collect();
    let sitemap_count =
        sitemap::generate_sitemap(&config.url, &collections_vec, &pages, destination)?;
    summary.sitemap_entries = sitemap_count;
    progress.phase_done(&format!("Generating sitemap... {} entries", sitemap_count));

    // 14. Generate feed.xml (from posts)
    progress.phase("Generating feed...");
    if let Some((_, posts_vec)) = collections_vec.iter().find(|(name, _)| name == "posts") {
        feed::write_atom_feed(posts_vec, &config, &FeedOptions::default(), destination)?;
        progress.phase_done(&format!("Generating feed... {} posts", posts_vec.len()));
    } else {
        // No posts collection -- write empty feed
        feed::write_atom_feed(&[], &config, &FeedOptions::default(), destination)?;
        progress.phase_done("Generating feed... 0 posts");
    }
    summary.timing.sitemap_feed = phase_start.elapsed();

    // 15. Save the build manifest
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
            quiet,
        }) => {
            let start = Instant::now();

            if !quiet {
                println!("Source:      {}", source.display());
                println!("Destination: {}", destination.display());
                if incremental {
                    println!("Mode:        incremental");
                }
                if force {
                    println!("Mode:        force (full rebuild)");
                }
                println!();
            }

            let options = BuildOptions {
                incremental,
                force,
                quiet,
                changed_paths: None,
            };

            match build_site(&source, &destination, &options) {
                Ok(summary) if !quiet => {
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
                Ok(_) => {
                    // quiet mode: no summary output
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
            no_watch,
            quiet,
            no_browser,
        }) => {
            let auto_open_browser = !no_browser;
            let livereload_enabled = livereload && !no_livereload && !no_watch;

            // Build the site first
            if !quiet {
                println!("Building site before serving...");
            }
            let options = BuildOptions {
                incremental: false,
                force: false,
                quiet,
                changed_paths: None,
            };
            match build_site(&source, &destination, &options) {
                Ok(summary) if !quiet => {
                    let total_pages = summary.collection_pages + summary.standalone_pages;
                    println!("Build complete: {} pages generated.", total_pages);
                }
                Ok(_) => {}
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
                    let build_fn = Box::new(move |scope: rustkyll::livereload::RebuildScope| {
                        let opts = match &scope {
                            rustkyll::livereload::RebuildScope::Full => BuildOptions {
                                incremental: false,
                                force: false,
                                quiet,
                                changed_paths: None,
                            },
                            rustkyll::livereload::RebuildScope::Partial(paths) => BuildOptions {
                                incremental: true,
                                force: false,
                                quiet,
                                changed_paths: Some(paths.clone()),
                            },
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
                if let Err(e) = rustkyll::server::start_server_with_options(
                    &destination,
                    port,
                    Some(ws_port),
                    auto_open_browser,
                ) {
                    eprintln!("Server error: {}", e);
                    std::process::exit(1);
                }

                shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
                let _ = ws_handle.join();
                let _ = watcher_handle.join();
            } else {
                // No live reload or no watch -- just serve
                if no_watch && !quiet {
                    println!("File watching disabled.");
                }
                if !quiet {
                    println!("Live reload disabled.");
                }
                if let Err(e) = rustkyll::server::start_server_with_options(
                    &destination,
                    port,
                    None,
                    auto_open_browser,
                ) {
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

    #[test]
    fn test_cli_about_is_generic() {
        let result = Cli::try_parse_from(["rustkyll", "--help"]);
        let err = result.unwrap_err();
        let help_text = err.to_string();
        assert!(
            !help_text.contains("DataTalks"),
            "CLI help should not mention any specific organization"
        );
        assert!(
            help_text.contains("Jekyll"),
            "CLI help should mention Jekyll compatibility"
        );
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
                ..
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

    #[test]
    fn test_cli_parses_serve_no_watch_flag() {
        let cli = Cli::try_parse_from(["rustkyll", "serve", "--no-watch"]).unwrap();
        match cli.command {
            Some(Commands::Serve { no_watch, .. }) => {
                assert!(no_watch);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_parses_serve_no_watch_defaults_false() {
        let cli = Cli::try_parse_from(["rustkyll", "serve"]).unwrap();
        match cli.command {
            Some(Commands::Serve { no_watch, .. }) => {
                assert!(!no_watch);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_parses_serve_no_watch_and_no_livereload() {
        let cli =
            Cli::try_parse_from(["rustkyll", "serve", "--no-watch", "--no-livereload"]).unwrap();
        match cli.command {
            Some(Commands::Serve {
                no_watch,
                no_livereload,
                ..
            }) => {
                assert!(no_watch);
                assert!(no_livereload);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_parses_serve_no_watch_with_source() {
        let cli = Cli::try_parse_from(["rustkyll", "serve", "--no-watch", "--source", "/tmp/site"])
            .unwrap();
        match cli.command {
            Some(Commands::Serve {
                no_watch, source, ..
            }) => {
                assert!(no_watch);
                assert_eq!(source, PathBuf::from("/tmp/site"));
            }
            _ => panic!("Expected Serve command"),
        }
    }

    // --- Issue 55: serve/build defaults to current directory ---

    #[test]
    fn test_cli_build_relative_source_path() {
        let cli = Cli::try_parse_from(["rustkyll", "build", "--source", "../relative"]).unwrap();
        match cli.command {
            Some(Commands::Build { source, .. }) => {
                assert_eq!(source, PathBuf::from("../relative"));
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_cli_serve_default_source_is_dot() {
        let cli = Cli::try_parse_from(["rustkyll", "serve"]).unwrap();
        match cli.command {
            Some(Commands::Serve {
                source,
                destination,
                ..
            }) => {
                assert_eq!(source, PathBuf::from("."));
                assert_eq!(destination, PathBuf::from("_site"));
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_serve_with_explicit_source() {
        let cli = Cli::try_parse_from(["rustkyll", "serve", "--source", "/tmp/site"]).unwrap();
        match cli.command {
            Some(Commands::Serve { source, .. }) => {
                assert_eq!(source, PathBuf::from("/tmp/site"));
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_serve_with_explicit_destination() {
        let cli = Cli::try_parse_from(["rustkyll", "serve", "--destination", "/tmp/out"]).unwrap();
        match cli.command {
            Some(Commands::Serve { destination, .. }) => {
                assert_eq!(destination, PathBuf::from("/tmp/out"));
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_build_relative_dot_source() {
        let cli = Cli::try_parse_from(["rustkyll", "build", "--source", "."]).unwrap();
        match cli.command {
            Some(Commands::Build { source, .. }) => {
                assert_eq!(source, PathBuf::from("."));
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_build_site_with_dot_source() {
        // Create a minimal site in a temp directory and build with source = "."
        let tmp = tempfile::tempdir().unwrap();
        let site_root = tmp.path();

        // Minimal config
        std::fs::create_dir_all(site_root.join("_layouts")).unwrap();
        std::fs::create_dir_all(site_root.join("_includes")).unwrap();
        std::fs::write(
            site_root.join("_config.yml"),
            "url: \"https://example.com\"\ntitle: \"Dot Test\"\n",
        )
        .unwrap();
        std::fs::write(
            site_root.join("_layouts/page.html"),
            "<html><body>{{ content }}</body></html>",
        )
        .unwrap();
        std::fs::write(
            site_root.join("index.md"),
            "---\ntitle: Home\nlayout: page\npermalink: /index.html\n---\nHello dot source.",
        )
        .unwrap();

        let dest = site_root.join("_site");
        let options = BuildOptions {
            incremental: false,
            force: false,
            quiet: false,
            changed_paths: None,
        };

        // Build using the absolute path (equivalent to passing "." while CWD = site_root)
        let result = build_site(site_root, &dest, &options);
        assert!(
            result.is_ok(),
            "build_site should succeed: {:?}",
            result.err()
        );

        let summary = result.unwrap();
        assert!(
            summary.standalone_pages > 0,
            "Should generate at least one page"
        );
        assert!(
            dest.join("index.html").exists(),
            "index.html should be in _site/"
        );

        let content = std::fs::read_to_string(dest.join("index.html")).unwrap();
        assert!(
            content.contains("Hello dot source"),
            "Output should contain page content"
        );
    }

    #[test]
    fn test_build_site_missing_config_uses_defaults() {
        // Jekyll builds sites without _config.yml using defaults.
        let tmp = tempfile::tempdir().unwrap();
        let empty_dir = tmp.path();
        let dest = empty_dir.join("_site");
        let options = BuildOptions {
            incremental: false,
            force: false,
            quiet: false,
            changed_paths: None,
        };

        let result = build_site(empty_dir, &dest, &options);
        assert!(
            result.is_ok(),
            "Sites without _config.yml should build with defaults, got: {:?}",
            result.err()
        );
    }

    // --- Issue 91: --quiet flag tests ---

    #[test]
    fn test_cli_build_quiet_flag() {
        let cli = Cli::try_parse_from(["rustkyll", "build", "--quiet"]).unwrap();
        match cli.command {
            Some(Commands::Build { quiet, .. }) => {
                assert!(quiet, "--quiet flag should be true");
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_cli_build_no_quiet_flag_defaults_false() {
        let cli = Cli::try_parse_from(["rustkyll", "build"]).unwrap();
        match cli.command {
            Some(Commands::Build { quiet, .. }) => {
                assert!(!quiet, "--quiet should default to false");
            }
            _ => panic!("Expected Build command"),
        }
    }

    #[test]
    fn test_cli_serve_quiet_flag() {
        let cli = Cli::try_parse_from(["rustkyll", "serve", "--quiet"]).unwrap();
        match cli.command {
            Some(Commands::Serve { quiet, .. }) => {
                assert!(quiet, "--quiet flag should be true for serve");
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_serve_no_quiet_flag_defaults_false() {
        let cli = Cli::try_parse_from(["rustkyll", "serve"]).unwrap();
        match cli.command {
            Some(Commands::Serve { quiet, .. }) => {
                assert!(!quiet, "--quiet should default to false for serve");
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_serve_no_browser_flag() {
        let cli = Cli::try_parse_from(["rustkyll", "serve", "--no-browser"]).unwrap();
        match cli.command {
            Some(Commands::Serve { no_browser, .. }) => {
                assert!(no_browser, "--no-browser flag should be true");
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_serve_no_browser_defaults_false() {
        let cli = Cli::try_parse_from(["rustkyll", "serve"]).unwrap();
        match cli.command {
            Some(Commands::Serve { no_browser, .. }) => {
                assert!(!no_browser, "--no-browser should default to false");
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_serve_no_browser_with_other_flags() {
        let cli = Cli::try_parse_from([
            "rustkyll",
            "serve",
            "--no-browser",
            "--no-watch",
            "--port",
            "8080",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Serve {
                no_browser,
                no_watch,
                port,
                ..
            }) => {
                assert!(no_browser);
                assert!(no_watch);
                assert_eq!(port, 8080);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_build_quiet_mode_produces_no_progress() {
        // Build a minimal site in quiet mode; verify build succeeds
        let tmp = tempfile::tempdir().unwrap();
        let site_root = tmp.path();

        std::fs::create_dir_all(site_root.join("_layouts")).unwrap();
        std::fs::create_dir_all(site_root.join("_includes")).unwrap();
        std::fs::write(
            site_root.join("_config.yml"),
            "url: \"https://example.com\"\ntitle: \"Quiet Test\"\n",
        )
        .unwrap();
        std::fs::write(
            site_root.join("_layouts/page.html"),
            "<html><body>{{ content }}</body></html>",
        )
        .unwrap();
        std::fs::write(
            site_root.join("index.md"),
            "---\ntitle: Home\nlayout: page\npermalink: /index.html\n---\nHello quiet.",
        )
        .unwrap();

        let dest = site_root.join("_site");
        let options = BuildOptions {
            incremental: false,
            force: false,
            quiet: true,
            changed_paths: None,
        };

        let result = build_site(site_root, &dest, &options);
        assert!(result.is_ok(), "build should succeed in quiet mode");
        let summary = result.unwrap();
        assert!(summary.standalone_pages > 0, "Should generate pages");
    }

    #[test]
    fn test_redirect_html_absolute_url() {
        let html = generate_redirect_html("/old/", "/community/", "https://example.com", "");
        assert!(
            html.contains("href=\"https://example.com/community/\""),
            "Expected absolute URL in redirect HTML, got: {}",
            html
        );
    }

    #[test]
    fn test_redirect_html_with_baseurl() {
        let html = generate_redirect_html("/old/", "/page/", "https://example.com", "/docs");
        assert!(
            html.contains("href=\"https://example.com/docs/page/\""),
            "Expected absolute URL with baseurl in redirect HTML, got: {}",
            html
        );
    }

    #[test]
    fn test_redirect_html_all_elements_absolute() {
        let html = generate_redirect_html("/old/", "/community/", "https://example.com", "");
        let expected_url = "https://example.com/community/";
        // <link rel="canonical" href="...">
        assert!(
            html.contains(&format!("href=\"{}\"", expected_url)),
            "link canonical should use absolute URL"
        );
        // <meta http-equiv="refresh" content="0; url=...">
        assert!(
            html.contains(&format!("url={}", expected_url)),
            "meta refresh should use absolute URL"
        );
        // <script>location="..."</script>
        assert!(
            html.contains(&format!("location=\"{}\"", expected_url)),
            "script location should use absolute URL"
        );
        // <a href="...">
        assert!(
            html.contains(&format!("<a href=\"{}\">", expected_url)),
            "anchor href should use absolute URL"
        );
    }

    #[test]
    fn test_redirect_html_no_site_url_uses_relative() {
        let html = generate_redirect_html("/old/", "/community/", "", "");
        assert!(
            html.contains("href=\"/community/\""),
            "Should fall back to relative URL when site.url is empty, got: {}",
            html
        );
    }

    #[test]
    fn test_redirect_html_structure_unchanged() {
        let html = generate_redirect_html("/old/", "/community/", "https://example.com", "");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html lang=\"en-US\">"));
        assert!(html.contains("<meta charset=\"utf-8\">"));
        assert!(html.contains("<title>Redirecting&hellip;</title>"));
        assert!(html.contains("<meta name=\"robots\" content=\"noindex\">"));
        assert!(html.contains("<h1>Redirecting&hellip;</h1>"));
        assert!(html.contains("Click here if you are not redirected."));
    }

    // Issue 226 RC6: URL concatenation missing slash
    #[test]
    fn test_rc6_redirect_html_path_without_leading_slash() {
        // When to_url lacks a leading slash, the redirect should still produce
        // a valid URL with a / separator between site_url and path
        let html =
            generate_redirect_html("/old/", "no-permission/", "https://choosealicense.com", "");
        assert!(
            html.contains("https://choosealicense.com/no-permission/"),
            "URL should have / separator even when path lacks leading slash. Got: {}",
            html
        );
        assert!(
            !html.contains("https://choosealicense.comno-permission/"),
            "URL must NOT concatenate without separator. Got: {}",
            html
        );
    }
}
