//! Page generation orchestration.
//!
//! This module provides functions that wire together collection loading,
//! site context building, template rendering, and HTML output writing.
//! It is designed to be fully generic -- no collection-type-specific logic.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use crate::progress::RenderProgress;

use liquid::model::Value as LiquidValue;
use liquid::Object;
use rayon::prelude::*;

use crate::collection::{CollectionItem, Page};
use crate::config::SiteConfig;
use crate::data::DataTree;
use crate::jsonld;
use crate::template::context::{normalize_arrays, yaml_to_liquid};
use crate::template::engine::CachedSiteContext;
use crate::template::layout::LayoutEngine;
use crate::template::TemplateError;

/// Errors that can occur during page generation.
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("template error: {0}")]
    Template(#[from] TemplateError),

    #[error("collection error: {0}")]
    Collection(#[from] crate::collection::CollectionError),

    #[error("data error: {0}")]
    Data(#[from] crate::data::DataError),

    #[error("config error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("I/O error writing {path}: {source}")]
    WriteFile {
        path: String,
        source: std::io::Error,
    },
}

/// Compile SCSS source code to CSS using the grass compiler.
///
/// Jekyll compiles `.scss` files with front matter into CSS. This function
/// replicates that behavior using the `grass` crate (a pure-Rust SCSS compiler).
/// The output style is compressed to match Jekyll's default SCSS output.
fn compile_scss(scss_source: &str) -> Result<String, String> {
    let options = grass::Options::default().style(grass::OutputStyle::Compressed);
    grass::from_string(scss_source.to_string(), &options).map_err(|e| e.to_string())
}

/// Result of generating pages for a collection.
#[derive(Debug)]
pub struct GenerationResult {
    /// Number of pages successfully generated.
    pub generated: usize,
    /// Number of pages skipped (e.g., no layout found).
    pub skipped: usize,
    /// Non-fatal errors encountered during generation.
    pub errors: Vec<String>,
}

/// Build a Liquid `Object` representing the `site` namespace.
///
/// This object is passed as the site context to template rendering.
/// It includes:
/// - `site.<collection_name>` for each provided collection
/// - `site.data.*` from data files
/// - `site.url`, `site.name`, `site.title`, `site.time`
/// - `site.github.repository_url` (from config or git remote)
/// - `site.related_posts` (10 most recent posts by date descending)
/// - `site.pages` (standalone page objects)
pub fn build_site_context(
    config: &SiteConfig,
    collections: &HashMap<String, Vec<CollectionItem>>,
    data: &DataTree,
    site_dir: Option<&Path>,
    pages: &[Page],
) -> Object {
    let mut site = Object::new();

    // Extra config keys first (so named fields can override)
    for (key, value) in &config.extras {
        site.insert(key.clone().into(), yaml_to_liquid(value));
    }

    // Default timezone: if not set in _config.yml, use the system timezone.
    // This matches Jekyll's behavior: when no timezone is configured, Jekyll
    // uses the system's local timezone for formatting naive dates.
    if !config.extras.contains_key("timezone") {
        if let Some(tz_name) = crate::template::filters::get_system_timezone() {
            site.insert("timezone".into(), LiquidValue::scalar(tz_name));
        }
    }

    // Basic site fields
    site.insert("url".into(), LiquidValue::scalar(config.url.clone()));
    site.insert(
        "baseurl".into(),
        LiquidValue::scalar(config.baseurl.clone()),
    );
    site.insert("name".into(), LiquidValue::scalar(config.name.clone()));
    site.insert("title".into(), LiquidValue::scalar(config.title.clone()));

    // site.time -- current build time as a string matching event time format
    let now = chrono::Local::now();
    site.insert(
        "time".into(),
        LiquidValue::scalar(now.format("%Y-%m-%d %H:%M:%S").to_string()),
    );

    // site.<collection_name> for each collection.
    // Use slim conversion to exclude large array fields (like transcript with
    // 400+ entries) from site context objects. These fields are only needed
    // when rendering the current page (via page.X) and are never accessed
    // from site-level cross-references. Excluding them dramatically reduces
    // the cost of `where`, `sort`, and other filters that clone objects.
    for (name, items) in collections {
        let mut arr: Vec<LiquidValue> = items.iter().map(collection_item_to_liquid_slim).collect();
        // Jekyll exposes site.posts in reverse chronological order (newest first).
        // Other collections are kept in their load order (date ascending).
        if name == "posts" {
            arr.reverse();
        }
        site.insert(
            name.clone().into(),
            normalize_arrays(LiquidValue::Array(arr)),
        );
    }

    // site.categories and site.tags -- built from posts only (Jekyll behavior)
    let (categories_map, tags_map) = build_categories_and_tags(collections);
    site.insert("categories".into(), categories_map);
    site.insert("tags".into(), tags_map);

    // site.twitter -- convert yaml Value to liquid Value for both string and map support
    if let Some(ref twitter) = config.twitter {
        site.insert("twitter".into(), yaml_to_liquid(twitter));
    }

    // site.github -- dynamic repository URL resolution
    let repo_url = resolve_repository_url(config, site_dir);
    let mut github = Object::new();
    github.insert("repository_url".into(), repo_url);
    site.insert("github".into(), LiquidValue::Object(github));

    // site.data -- data tree
    let mut data_obj = Object::new();
    for (key, value) in data {
        let liquid_val = normalize_arrays(yaml_to_liquid(value));
        data_obj.insert(key.clone().into(), liquid_val);
    }
    site.insert("data".into(), LiquidValue::Object(data_obj));

    // site.related_posts -- 10 most recent posts sorted by date descending
    let related_posts = build_related_posts(collections);
    site.insert(
        "related_posts".into(),
        normalize_arrays(LiquidValue::Array(related_posts)),
    );

    // site.pages -- standalone page objects
    let pages_arr: Vec<LiquidValue> = pages.iter().map(page_to_liquid).collect();
    site.insert(
        "pages".into(),
        normalize_arrays(LiquidValue::Array(pages_arr)),
    );

    site
}

/// Resolve the GitHub repository URL dynamically.
///
/// Priority:
/// 1. Config `repository` field (format: `owner/repo`) -> `https://github.com/{repository}`
/// 2. Git remote origin URL from the site directory
/// 3. Nil if neither is available
fn resolve_repository_url(config: &SiteConfig, site_dir: Option<&Path>) -> LiquidValue {
    // 1. Check config.repository
    if let Some(ref repo) = config.repository {
        return LiquidValue::scalar(format!("https://github.com/{repo}"));
    }

    // 2. Try git remote origin URL
    if let Some(dir) = site_dir {
        if let Ok(output) = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(dir)
            .output()
        {
            if output.status.success() {
                let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // Convert SSH URLs to HTTPS
                let https_url = if url.starts_with("git@github.com:") {
                    let path = url
                        .trim_start_matches("git@github.com:")
                        .trim_end_matches(".git");
                    format!("https://github.com/{path}")
                } else if url.ends_with(".git") {
                    url.trim_end_matches(".git").to_string()
                } else {
                    url
                };
                return LiquidValue::scalar(https_url);
            }
        }
    }

    // 3. No repository info available
    LiquidValue::Nil
}

/// Convert a `CollectionItem` to a Liquid `Value` for site context arrays.
///
/// Includes all front matter fields plus computed fields like `url`, `id`,
/// `content`, `date`, and `slug`. Also ensures `short` is set from slug
/// if not present in front matter (needed for author lookup in JSON-LD).
///
/// Large array-valued front matter fields (more than 10 elements) are excluded
/// to reduce clone costs when filters like `where` and `sort` operate on
/// site-level collection arrays (`site.posts`, `site.podcast`, etc.).
/// Fields like `transcript` (which can have 400+ entries) are only needed
/// when rendering the current page (via `page.transcript`) and are never
/// accessed from site-level cross-references. The current page always has
/// its full front matter available through the `page` variable.
fn collection_item_to_liquid_slim(item: &CollectionItem) -> LiquidValue {
    let mut obj = Object::new();

    // Copy front matter fields, normalizing arrays so that objects
    // in arrays have uniform keys (prevents "Unknown index" in Liquid for loops).
    // Skip large array fields (e.g., transcript with 400+ entries) that are only
    // needed when rendering the current page, not for cross-references.
    for (key, value) in &item.front_matter {
        if let serde_yaml::Value::Sequence(seq) = value {
            if seq.len() > 10 {
                continue;
            }
        }
        obj.insert(key.clone().into(), normalize_arrays(yaml_to_liquid(value)));
    }

    // Add computed fields
    obj.insert("url".into(), LiquidValue::scalar(item.url.clone()));
    obj.insert("slug".into(), LiquidValue::scalar(item.slug.clone()));
    obj.insert("id".into(), LiquidValue::scalar(item.id.clone()));

    if let Some(ref date) = item.date {
        obj.insert("date".into(), LiquidValue::scalar(date.clone()));
    }

    obj.insert(
        "content".into(),
        LiquidValue::scalar(item.html_content.clone()),
    );

    // Ensure "short" is set from slug if not in front matter
    // (needed for author lookup via `site.<collection> | where: "short", name`)
    if !item.front_matter.contains_key("short") {
        obj.insert("short".into(), LiquidValue::scalar(item.slug.clone()));
    }

    LiquidValue::Object(obj)
}

/// Build `site.related_posts` -- the 10 most recent posts sorted by date descending.
///
/// In Jekyll, `site.related_posts` defaults to the 10 most recent posts
/// (unless LSI is enabled, which we do not support). Each entry has the same
/// structure as entries in `site.posts`.
fn build_related_posts(collections: &HashMap<String, Vec<CollectionItem>>) -> Vec<LiquidValue> {
    let Some(posts) = collections.get("posts") else {
        return Vec::new();
    };

    // Sort posts by date descending, take up to 10
    let mut sorted: Vec<&CollectionItem> = posts.iter().collect();
    sorted.sort_by(|a, b| {
        let date_a = a.date.as_deref().unwrap_or("");
        let date_b = b.date.as_deref().unwrap_or("");
        date_b
            .cmp(date_a) // descending
            .then_with(|| a.slug.cmp(&b.slug))
    });

    sorted
        .into_iter()
        .take(10)
        .map(collection_item_to_liquid_slim)
        .collect()
}

/// Convert a standalone `Page` to a Liquid `Value` object.
///
/// Exposes front matter fields, `title`, `url`, and `content`,
/// matching Jekyll's page object structure.
fn page_to_liquid(page: &Page) -> LiquidValue {
    let mut obj = Object::new();

    // Copy all front matter fields
    for (key, value) in &page.front_matter {
        obj.insert(key.clone().into(), normalize_arrays(yaml_to_liquid(value)));
    }

    // Add computed fields (may override front matter)
    obj.insert("url".into(), LiquidValue::scalar(page.url.clone()));
    obj.insert("slug".into(), LiquidValue::scalar(page.slug.clone()));
    obj.insert(
        "content".into(),
        LiquidValue::scalar(page.html_content.clone()),
    );

    // page.name -- the source filename (e.g. "index.md"), matching Jekyll's behavior
    // This is needed for templates that check page.name to customize output
    // (e.g., the DTC site's head.html uses {% if page.name == 'index.md' %})
    let name = std::path::Path::new(&page.source_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    obj.insert("name".into(), LiquidValue::scalar(name));

    // page.path -- the relative source path (e.g. "index.md" or "books.md")
    obj.insert("path".into(), LiquidValue::scalar(page.source_path.clone()));

    LiquidValue::Object(obj)
}

/// Build `site.categories` and `site.tags` Liquid objects from post collections.
///
/// Only posts are included (not other custom collections).
/// Returns `(categories_liquid_value, tags_liquid_value)` where each is a
/// Liquid object mapping category/tag name to an array of post objects.
fn build_categories_and_tags(
    collections: &HashMap<String, Vec<CollectionItem>>,
) -> (LiquidValue, LiquidValue) {
    let mut categories: HashMap<String, Vec<LiquidValue>> = HashMap::new();
    let mut tags: HashMap<String, Vec<LiquidValue>> = HashMap::new();

    if let Some(posts) = collections.get("posts") {
        for post in posts {
            let liquid_post = collection_item_to_liquid_slim(post);

            let post_categories = crate::collection::extract_categories(&post.front_matter);
            for cat in post_categories {
                categories.entry(cat).or_default().push(liquid_post.clone());
            }

            let post_tags = crate::collection::extract_tags(&post.front_matter);
            for tag in post_tags {
                tags.entry(tag).or_default().push(liquid_post.clone());
            }
        }
    }

    let categories_obj = categories
        .into_iter()
        .map(|(k, v)| (k.into(), LiquidValue::Array(v)))
        .collect::<Object>();

    let tags_obj = tags
        .into_iter()
        .map(|(k, v)| (k.into(), LiquidValue::Array(v)))
        .collect::<Object>();

    (
        LiquidValue::Object(categories_obj),
        LiquidValue::Object(tags_obj),
    )
}

/// Resolve the layout name for a collection item.
///
/// First checks the item's own front matter for a `layout` key.
/// If absent, falls back to the config defaults for the given collection type.
/// Returns `None` if no layout is configured anywhere.
pub fn resolve_layout<'a>(
    item: &'a CollectionItem,
    config: &'a SiteConfig,
    collection_type: &str,
) -> Option<String> {
    // Check the item's own front matter first
    if let Some(layout_val) = item.front_matter.get("layout") {
        if let Some(layout_str) = layout_val.as_str() {
            if !layout_str.is_empty() {
                return Some(layout_str.to_string());
            }
        }
    }

    // Fall back to config defaults
    config
        .default_layout_for(collection_type)
        .map(|s| s.to_string())
}

/// Compute the output file path for a collection item.
///
/// Given a slug and output directory, produces `<output_dir>/<collection>/<slug>.html`.
pub fn output_path(output_dir: &Path, collection: &str, slug: &str) -> std::path::PathBuf {
    output_dir.join(collection).join(format!("{slug}.html"))
}

/// Compute the output file path from a URL.
///
/// URLs ending with `/` produce `<output_dir>/<url>/index.html` (pretty URLs).
/// URLs with a recognized file extension produce `<output_dir>/<url>` directly.
/// Other URLs get `.html` appended.
pub fn url_to_output_path(output_dir: &Path, url: &str) -> std::path::PathBuf {
    let relative = url.trim_start_matches('/');
    if relative.is_empty() {
        return output_dir.join("index.html");
    }
    if relative.ends_with('/') {
        output_dir.join(relative).join("index.html")
    } else if url_has_file_extension(relative) {
        // URL already has a recognized file extension (e.g., .html, .xml, .json)
        output_dir.join(relative)
    } else {
        output_dir.join(format!("{relative}.html"))
    }
}

/// Normalize a front matter field to an array if it's a string.
///
/// Jekyll always exposes `categories` and `tags` as arrays, even when specified
/// as a single string in front matter (e.g., `categories: food` becomes `["food"]`).
/// Also handles space-separated strings (e.g., `categories: "foo bar"` -> `["foo", "bar"]`).
fn normalize_fm_to_array(fm: &mut crate::frontmatter::FrontMatter, key: &str) {
    if let Some(val) = fm.get(key) {
        match val {
            serde_yaml::Value::String(s) => {
                if s.is_empty() {
                    fm.insert(key.to_string(), serde_yaml::Value::Sequence(Vec::new()));
                } else {
                    // Split on spaces for multi-category strings
                    let items: Vec<serde_yaml::Value> = s
                        .split_whitespace()
                        .map(|w| serde_yaml::Value::String(w.to_string()))
                        .collect();
                    fm.insert(key.to_string(), serde_yaml::Value::Sequence(items));
                }
            }
            serde_yaml::Value::Null => {
                fm.insert(key.to_string(), serde_yaml::Value::Sequence(Vec::new()));
            }
            _ => {} // Already an array or other type, leave as-is
        }
    }
}

/// Check if a URL path already has a recognized file extension.
fn url_has_file_extension(path: &str) -> bool {
    if let Some(dot_pos) = path.rfind('.') {
        let ext = &path[dot_pos..];
        matches!(
            ext,
            ".html"
                | ".htm"
                | ".xml"
                | ".json"
                | ".txt"
                | ".rss"
                | ".atom"
                | ".css"
                | ".js"
                | ".svg"
        )
    } else {
        false
    }
}

/// Convert a `CollectionItem` into a `serde_yaml::Value::Mapping` suitable
/// for injection as `page.previous` or `page.next` in a post's front matter.
///
/// The resulting mapping contains all front matter fields plus computed fields
/// (`url`, `slug`, `date`) so templates can access e.g. `page.next.title`.
fn item_to_yaml_mapping(item: &CollectionItem) -> serde_yaml::Value {
    let mut map = serde_yaml::Mapping::new();

    // Copy front matter fields, skipping large arrays (e.g., transcript with
    // 400+ entries) that are never accessed via page.previous/page.next.
    for (key, value) in &item.front_matter {
        if let serde_yaml::Value::Sequence(seq) = value {
            if seq.len() > 10 {
                continue;
            }
        }
        map.insert(serde_yaml::Value::String(key.clone()), value.clone());
    }

    // Add computed fields
    map.insert(
        serde_yaml::Value::String("url".to_string()),
        serde_yaml::Value::String(item.url.clone()),
    );
    map.insert(
        serde_yaml::Value::String("slug".to_string()),
        serde_yaml::Value::String(item.slug.clone()),
    );
    if let Some(ref date) = item.date {
        map.entry(serde_yaml::Value::String("date".to_string()))
            .or_insert(serde_yaml::Value::String(date.clone()));
    }

    serde_yaml::Value::Mapping(map)
}

/// Build a map from post slug to (Option<previous>, Option<next>) YAML values.
///
/// Posts are sorted by date ascending (oldest first), with a secondary sort by
/// slug for deterministic ordering when dates are equal. This matches Jekyll's
/// behavior where `page.previous` is the older post and `page.next` is the
/// newer post.
pub fn build_prev_next_map(
    items: &[CollectionItem],
) -> HashMap<String, (Option<serde_yaml::Value>, Option<serde_yaml::Value>)> {
    let mut sorted: Vec<&CollectionItem> = items.iter().collect();
    sorted.sort_by(|a, b| {
        let date_a = a.date.as_deref().unwrap_or("");
        let date_b = b.date.as_deref().unwrap_or("");
        date_a.cmp(date_b).then_with(|| a.slug.cmp(&b.slug))
    });

    let mut result = HashMap::new();
    for (i, item) in sorted.iter().enumerate() {
        let prev = if i > 0 {
            Some(item_to_yaml_mapping(sorted[i - 1]))
        } else {
            None
        };
        let next = if i + 1 < sorted.len() {
            Some(item_to_yaml_mapping(sorted[i + 1]))
        } else {
            None
        };
        result.insert(item.slug.clone(), (prev, next));
    }
    result
}

/// Generate HTML pages for a collection using parallel rendering.
///
/// This is the main orchestration function. For each item in `items`:
/// 1. Resolve the layout from front matter or config defaults
/// 2. Render through `LayoutEngine::render_page`
/// 3. Write the result to `<output_dir>/<collection_name>/<slug>.html`
///
/// Items with no resolvable layout are skipped.
/// Uses rayon for parallel rendering of pages.
pub fn generate_collection_pages(
    items: &[CollectionItem],
    collection_type: &str,
    config: &SiteConfig,
    layout_engine: &LayoutEngine,
    site_context: &Object,
    output_dir: &Path,
) -> Result<GenerationResult, GeneratorError> {
    generate_collection_pages_with_authors(
        items,
        collection_type,
        config,
        layout_engine,
        site_context,
        output_dir,
        &[],
    )
}

/// Generate HTML pages for collection items, with access to author items
/// for JSON-LD author resolution.
///
/// This is the full version that supports JSON-LD post-processing injection.
/// The `author_items` slice is used to resolve author slugs to full names in
/// structured data blocks (e.g., Book JSON-LD). Any collection items can serve
/// as author sources, regardless of which collection name is used for author
/// data (e.g., "people", "authors", "team").
pub fn generate_collection_pages_with_authors(
    items: &[CollectionItem],
    collection_type: &str,
    config: &SiteConfig,
    layout_engine: &LayoutEngine,
    site_context: &Object,
    output_dir: &Path,
    author_items: &[CollectionItem],
) -> Result<GenerationResult, GeneratorError> {
    let cached_site = LayoutEngine::build_cached_site_context(site_context);
    generate_collection_pages_cached(
        items,
        collection_type,
        config,
        layout_engine,
        &cached_site,
        output_dir,
        author_items,
    )
}

/// Generate HTML pages for collection items using a pre-built cached site context.
///
/// This is the performance-optimized version. The caller should build the
/// `CachedSiteContext` ONCE and pass it to all collection page generations,
/// avoiding redundant O(n) LenientValue tree construction per collection.
pub fn generate_collection_pages_cached(
    items: &[CollectionItem],
    collection_type: &str,
    config: &SiteConfig,
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    output_dir: &Path,
    author_items: &[CollectionItem],
) -> Result<GenerationResult, GeneratorError> {
    generate_collection_pages_cached_with_progress(
        items,
        collection_type,
        config,
        layout_engine,
        cached_site,
        output_dir,
        author_items,
        None,
    )
}

/// Like `generate_collection_pages_cached` but accepts an optional progress tracker.
///
/// When provided, the progress bar is incremented after each page is rendered,
/// updating in real time during the rayon parallel loop.
#[allow(clippy::too_many_arguments)]
pub fn generate_collection_pages_cached_with_progress(
    items: &[CollectionItem],
    collection_type: &str,
    config: &SiteConfig,
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    output_dir: &Path,
    author_items: &[CollectionItem],
    progress: Option<&RenderProgress>,
) -> Result<GenerationResult, GeneratorError> {
    let collection_out_dir = output_dir.join(collection_type);
    fs::create_dir_all(&collection_out_dir).map_err(|e| GeneratorError::WriteFile {
        path: collection_out_dir.display().to_string(),
        source: e,
    })?;

    // Pre-create all needed output directories before the parallel loop.
    // This avoids redundant `create_dir_all` syscalls in each thread.
    {
        let mut dirs = std::collections::HashSet::new();
        for item in items {
            let out_path = url_to_output_path(output_dir, &item.url);
            if let Some(parent) = out_path.parent() {
                dirs.insert(parent.to_path_buf());
            }
        }
        for dir in &dirs {
            fs::create_dir_all(dir).map_err(|e| GeneratorError::WriteFile {
                path: dir.display().to_string(),
                source: e,
            })?;
        }
    }

    // Pre-compute prev/next references only if templates actually use them.
    // Building prev/next maps clones front matter for all adjacent items, which
    // is expensive for large collections (e.g., 427 people). Skip if unused.
    let prev_next = if layout_engine.uses_prev_next() {
        build_prev_next_map(items)
    } else {
        HashMap::new()
    };

    let result = Mutex::new(GenerationResult {
        generated: 0,
        skipped: 0,
        errors: Vec::new(),
    });

    items.par_iter().for_each(|item| {
        let layout_name = resolve_layout(item, config, collection_type);

        // Build page front matter: start with defaults, then overlay item's own front matter
        let mut page_fm = item.front_matter.clone();

        // Apply defaults from config (only for keys not already in front matter)
        let defaults = config.defaults_for(collection_type, &item.source_path);
        for (key, value) in defaults {
            page_fm.entry(key).or_insert(value);
        }

        // Normalize categories and tags to arrays (Jekyll always exposes them as arrays).
        // A front matter `categories: food` (string) must become `["food"]` so that
        // Liquid filters like `join` work correctly.
        normalize_fm_to_array(&mut page_fm, "categories");
        normalize_fm_to_array(&mut page_fm, "tags");

        page_fm.insert("url".into(), serde_yaml::Value::String(item.url.clone()));

        // Also ensure date is in front matter if available (needed for posts)
        if !page_fm.contains_key("date") {
            if let Some(ref date) = item.date {
                page_fm.insert("date".to_string(), serde_yaml::Value::String(date.clone()));
            }
        }

        // Inject previous/next for posts
        if let Some((prev, next)) = prev_next.get(&item.slug) {
            match prev {
                Some(val) => {
                    page_fm.insert("previous".to_string(), val.clone());
                }
                None => {
                    page_fm.insert("previous".to_string(), serde_yaml::Value::Null);
                }
            }
            match next {
                Some(val) => {
                    page_fm.insert("next".to_string(), val.clone());
                }
                None => {
                    page_fm.insert("next".to_string(), serde_yaml::Value::Null);
                }
            }
        }

        // Determine HTML output: render through layout if available,
        // otherwise output raw content (Jekyll outputs items without layout too).
        let html_result = if let Some(ref layout) = layout_name {
            // Jekyll processes Liquid first, then markdown for ALL markdown-sourced files.
            // Use the Liquid-first-then-markdown pipeline for any collection item whose
            // source content contains Liquid tags and comes from a markdown file. This
            // prevents markdown-to-HTML conversion from encoding operators (< > >= <=)
            // inside Liquid expressions like where_exp date comparisons.
            let is_markdown_source =
                item.source_path.ends_with(".md") || item.source_path.ends_with(".markdown");
            let has_liquid_tags = item.content.contains("{{") || item.content.contains("{%");
            let render_result = if is_markdown_source && has_liquid_tags {
                layout_engine.render_markdown_page_with_cached_site(
                    layout,
                    &item.content,
                    &page_fm,
                    cached_site,
                )
            } else if is_markdown_source {
                // Markdown without Liquid: use pre-converted HTML (faster)
                layout_engine.render_page_with_cached_site(
                    layout,
                    &item.html_content,
                    &page_fm,
                    cached_site,
                )
            } else {
                layout_engine.render_page_with_cached_site(
                    layout,
                    &item.html_content,
                    &page_fm,
                    cached_site,
                )
            };

            match render_result {
                Ok(html) => {
                    // Post-process: inject JSON-LD structured data if applicable.
                    // Only book pages get JSON-LD; skip the clone for other layouts.
                    if layout == "book" {
                        Ok(jsonld::inject_jsonld(
                            &html,
                            layout,
                            &page_fm,
                            config,
                            author_items,
                        ))
                    } else {
                        Ok(html)
                    }
                }
                Err(e) => Err(e),
            }
        } else {
            // No layout: output just the rendered HTML content (matches Jekyll behavior).
            // If the body is empty, output a newline -- Jekyll never produces 0-byte files
            // for collection items with output: true.
            if item.html_content.is_empty() {
                Ok("\n".to_string())
            } else {
                Ok(item.html_content.clone())
            }
        };

        match html_result {
            Ok(html) => {
                // Compute output path from the item's URL (respects permalink patterns)
                let out_path = url_to_output_path(output_dir, &item.url);

                // Directories were pre-created before the parallel loop.
                match fs::write(&out_path, &html) {
                    Ok(()) => {
                        result.lock().unwrap().generated += 1;
                    }
                    Err(e) => {
                        result.lock().unwrap().errors.push(format!(
                            "Failed to write {}/{}: {}",
                            collection_type, item.slug, e
                        ));
                    }
                }
            }
            Err(e) => {
                // On render failure, fall back to writing the HTML content as-is.
                // This matches Jekyll's behavior of always producing output files,
                // and ensures page counts match even when templates have issues.
                eprintln!(
                    "Warning: failed to render {}/{}, writing fallback: {}",
                    collection_type, item.slug, e
                );
                let out_path = url_to_output_path(output_dir, &item.url);
                // Directories were pre-created before the parallel loop.
                let fallback_content = if item.html_content.is_empty() {
                    "\n"
                } else {
                    &item.html_content
                };
                match fs::write(&out_path, fallback_content) {
                    Ok(()) => {
                        result.lock().unwrap().generated += 1;
                    }
                    Err(write_e) => {
                        result.lock().unwrap().errors.push(format!(
                            "Failed to write fallback {}/{}: {}",
                            collection_type, item.slug, write_e
                        ));
                    }
                }
            }
        }
        // Increment progress bar after each page is processed (real-time update)
        if let Some(p) = progress {
            p.inc(&item.slug);
        }
    });

    Ok(result.into_inner().unwrap())
}

/// Generate HTML pages for a named collection from the site directory.
///
/// This is the single generic entry point for generating any collection type.
/// It loads the specified collection and renders all items through their layouts.
///
/// The `site_context` and `layout_engine` should be pre-built and shared
/// across all collection generations for efficiency.
pub fn generate_site_collection(
    collection_name: &str,
    site_dir: &Path,
    config: &SiteConfig,
    layout_engine: &LayoutEngine,
    site_context: &Object,
    output_dir: &Path,
) -> Result<GenerationResult, GeneratorError> {
    let (items, load_errors) =
        crate::collection::load_collection(collection_name, site_dir, config)?;

    let mut result = generate_collection_pages(
        &items,
        collection_name,
        config,
        layout_engine,
        site_context,
        output_dir,
    )?;

    // Add collection loading errors to the result
    for err in &load_errors {
        result.errors.push(format!("collection load error: {err}"));
    }

    Ok(result)
}

/// Generate HTML for standalone pages (root-level `.md` files like `events.md`).
///
/// Each page's raw content is rendered through the template engine (resolving
/// Liquid tags like `{% assign %}`, `{% for %}`, `{% include %}`) and then
/// wrapped in the layout specified by the page's front matter.
///
/// Output files are written to `<output_dir>/<slug>.html` (or the page's
/// permalink if specified in front matter).
///
/// Pages without a `layout` in their front matter are skipped.
pub fn generate_pages(
    pages: &[crate::collection::Page],
    layout_engine: &LayoutEngine,
    site_context: &Object,
    output_dir: &Path,
) -> Result<GenerationResult, GeneratorError> {
    let cached_site = LayoutEngine::build_cached_site_context(site_context);
    generate_pages_cached(pages, layout_engine, &cached_site, output_dir)
}

/// Generate HTML for standalone pages using a pre-built cached site context.
///
/// Performance-optimized version of `generate_pages`. The caller should
/// build the `CachedSiteContext` once and share it across all generation calls.
pub fn generate_pages_cached(
    pages: &[crate::collection::Page],
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    output_dir: &Path,
) -> Result<GenerationResult, GeneratorError> {
    generate_pages_cached_with_config(pages, layout_engine, cached_site, output_dir, None)
}

/// Generate HTML for standalone pages using a pre-built cached site context,
/// with optional config for applying front-matter defaults.
///
/// When `config` is provided, front-matter defaults from `_config.yml` are applied
/// to each page before checking for a layout. This is essential for sites that
/// use `defaults` with `type: "pages"` or `path:` scoping to assign layouts
/// to standalone pages (e.g., documentation-theme-jekyll, large-docs-site).
pub fn generate_pages_cached_with_config(
    pages: &[crate::collection::Page],
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    output_dir: &Path,
    config: Option<&SiteConfig>,
) -> Result<GenerationResult, GeneratorError> {
    generate_pages_cached_with_config_and_progress(
        pages,
        layout_engine,
        cached_site,
        output_dir,
        config,
        None,
    )
}

/// Like `generate_pages_cached_with_config` but accepts an optional progress tracker.
pub fn generate_pages_cached_with_config_and_progress(
    pages: &[crate::collection::Page],
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    output_dir: &Path,
    config: Option<&SiteConfig>,
    progress: Option<&RenderProgress>,
) -> Result<GenerationResult, GeneratorError> {
    fs::create_dir_all(output_dir).map_err(|e| GeneratorError::WriteFile {
        path: output_dir.display().to_string(),
        source: e,
    })?;

    let result = Mutex::new(GenerationResult {
        generated: 0,
        skipped: 0,
        errors: Vec::new(),
    });

    pages.par_iter().for_each(|page| {
        // Build page front matter: start with page's own front matter,
        // then apply config defaults for keys not already present.
        let mut page_fm = page.front_matter.clone();
        if let Some(cfg) = config {
            // Apply defaults matching type "pages" and path-scoped defaults
            let defaults = cfg.defaults_for_page(&page.source_path);
            for (key, value) in defaults {
                page_fm.entry(key).or_insert(value);
            }
        }

        // Resolve layout from front matter (after defaults are applied).
        // Three cases:
        //   1. `layout: "some_layout"` -> render with that layout
        //   2. `layout: null` (explicit null) -> render through Liquid, no layout wrapping
        //   3. No `layout` key at all -> skip page (no rendering)
        let layout_value = page_fm.get("layout");
        let layout_name: Option<String> = layout_value
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        // Jekyll processes ALL files with front matter regardless of layout.
        // Pages without a layout are rendered through Liquid without wrapping.
        page_fm.insert("url".into(), serde_yaml::Value::String(page.url.clone()));

        // page.name -- the source filename (e.g. "index.md"), matching Jekyll's behavior.
        // Needed for templates like DTC's head.html that check page.name.
        let page_name = std::path::Path::new(&page.source_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        page_fm
            .entry("name".into())
            .or_insert_with(|| serde_yaml::Value::String(page_name));

        // page.path -- the relative source path
        page_fm
            .entry("path".into())
            .or_insert_with(|| serde_yaml::Value::String(page.source_path.clone()));

        // Use raw content (not html_content) because pages may contain Liquid tags
        // that must be resolved before the layout wraps them.
        let is_markdown = page.source_path.ends_with(".md");
        let render_result = if let Some(ref layout) = layout_name {
            // Has a named layout -> render with layout wrapping
            if is_markdown {
                layout_engine.render_markdown_page_with_cached_site(
                    layout,
                    &page.content,
                    &page_fm,
                    cached_site,
                )
            } else {
                layout_engine.render_page_with_cached_site(
                    layout,
                    &page.content,
                    &page_fm,
                    cached_site,
                )
            }
        } else {
            // layout: null -> render through Liquid without layout wrapping
            if is_markdown {
                layout_engine.render_markdown_content_with_cached_site(
                    &page.content,
                    &page_fm,
                    cached_site,
                )
            } else {
                layout_engine.render_content_only_with_cached_site(
                    &page.content,
                    &page_fm,
                    cached_site,
                )
            }
        };
        match render_result {
            Ok(html) => {
                // If the source is an SCSS file, compile to CSS
                let html = if page.source_path.ends_with(".scss") {
                    match compile_scss(&html) {
                        Ok(css) => css,
                        Err(e) => {
                            result.lock().unwrap().errors.push(format!(
                                "Failed to compile SCSS for page {}: {}",
                                page.slug, e
                            ));
                            return;
                        }
                    }
                } else {
                    html
                };

                // Compute output path from URL (handles trailing-slash pretty URLs)
                let out_path = url_to_output_path(output_dir, &page.url);

                if let Some(parent) = out_path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        result.lock().unwrap().errors.push(format!(
                            "Failed to create dir for page {}: {}",
                            page.slug, e
                        ));
                        return;
                    }
                }

                match fs::write(&out_path, &html) {
                    Ok(()) => {
                        result.lock().unwrap().generated += 1;
                    }
                    Err(e) => {
                        result
                            .lock()
                            .unwrap()
                            .errors
                            .push(format!("Failed to write page {}: {}", page.slug, e));
                    }
                }
            }
            Err(e) => {
                // On render failure, fall back to writing the content as-is
                // (with markdown->HTML conversion). This matches Jekyll's behavior
                // of always producing output files, and ensures page counts match.
                eprintln!(
                    "Warning: failed to render page '{}', writing fallback: {}",
                    page.slug, e
                );
                let fallback = if page.source_path.ends_with(".md") {
                    crate::frontmatter::markdown_to_html(&page.content)
                } else {
                    page.content.clone()
                };
                let out_path = url_to_output_path(output_dir, &page.url);
                if let Some(parent) = out_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                match fs::write(&out_path, &fallback) {
                    Ok(()) => {
                        result.lock().unwrap().generated += 1;
                    }
                    Err(write_e) => {
                        result.lock().unwrap().errors.push(format!(
                            "Failed to write fallback page {}: {}",
                            page.slug, write_e
                        ));
                    }
                }
            }
        }
        // Increment progress bar after each page is processed (real-time update)
        if let Some(p) = progress {
            p.inc(&page.slug);
        }
    });

    Ok(result.into_inner().unwrap())
}

/// Generate HTML for standalone pages (alias for `generate_pages`).
///
/// This is the public entry point specified by issue 14. It delegates to
/// [`generate_pages`] which renders each root-level `.md` page through
/// the template engine and wraps it in the layout from front matter.
pub fn generate_standalone_pages(
    pages: &[crate::collection::Page],
    config: &SiteConfig,
    layout_engine: &LayoutEngine,
    site_context: &Object,
    output_dir: &Path,
) -> Result<GenerationResult, GeneratorError> {
    let cached_site = LayoutEngine::build_cached_site_context(site_context);
    generate_pages_cached_with_config(pages, layout_engine, &cached_site, output_dir, Some(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use liquid::model::ValueView;
    use std::path::PathBuf;

    fn site_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    fn test_config() -> SiteConfig {
        SiteConfig::from_file(&site_dir().join("_config.yml")).unwrap()
    }

    // ========================================================================
    // Unit: url_to_output_path
    // ========================================================================

    #[test]
    fn test_url_to_output_path_trailing_slash() {
        let out = PathBuf::from("/out");
        assert_eq!(
            url_to_output_path(&out, "/stories/my-story/"),
            PathBuf::from("/out/stories/my-story/index.html")
        );
    }

    #[test]
    fn test_url_to_output_path_html_extension() {
        let out = PathBuf::from("/out");
        assert_eq!(
            url_to_output_path(&out, "/blog/my-post.html"),
            PathBuf::from("/out/blog/my-post.html")
        );
    }

    #[test]
    fn test_url_to_output_path_root_slash() {
        let out = PathBuf::from("/out");
        assert_eq!(
            url_to_output_path(&out, "/"),
            PathBuf::from("/out/index.html")
        );
    }

    #[test]
    fn test_url_to_output_path_no_extension() {
        let out = PathBuf::from("/out");
        assert_eq!(
            url_to_output_path(&out, "/about"),
            PathBuf::from("/out/about.html")
        );
    }

    #[test]
    fn test_url_to_output_path_nested_trailing_slash() {
        let out = PathBuf::from("/out");
        assert_eq!(
            url_to_output_path(&out, "/2024/01/15/my-post/"),
            PathBuf::from("/out/2024/01/15/my-post/index.html")
        );
    }

    // ========================================================================
    // Unit: Site context building
    // ========================================================================

    #[test]
    fn test_build_site_context_has_posts() {
        let config = test_config();
        let (posts, _) = crate::collection::load_collection("posts", &site_dir(), &config).unwrap();
        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), posts);
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let posts_val = ctx.get("posts").expect("site should have posts");
        if let LiquidValue::Array(arr) = posts_val {
            assert!(arr.len() >= 2, "Expected 2+ posts, got {}", arr.len());
        } else {
            panic!("Expected posts to be an array");
        }
    }

    #[test]
    fn test_build_site_context_posts_have_required_fields() {
        let config = test_config();
        let (posts, _) = crate::collection::load_collection("posts", &site_dir(), &config).unwrap();
        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), posts);
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        if let Some(LiquidValue::Array(arr)) = ctx.get("posts") {
            // Check the first post has title and url
            let first = &arr[0];
            if let LiquidValue::Object(obj) = first {
                assert!(obj.get("title").is_some(), "Post should have title");
                assert!(obj.get("url").is_some(), "Post should have url");
            } else {
                panic!("Expected post to be an object");
            }
        }
    }

    #[test]
    fn test_build_site_context_has_books() {
        let config = test_config();
        let (books, _) = crate::collection::load_collection("books", &site_dir(), &config).unwrap();
        let mut collections = HashMap::new();
        collections.insert("books".to_string(), books);
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let books_val = ctx.get("books").expect("site should have books");
        if let LiquidValue::Array(arr) = books_val {
            assert!(arr.len() >= 2, "Expected 2+ books, got {}", arr.len());
        } else {
            panic!("Expected books to be an array");
        }
    }

    #[test]
    fn test_build_site_context_books_have_required_fields() {
        let config = test_config();
        let (books, _) = crate::collection::load_collection("books", &site_dir(), &config).unwrap();
        let mut collections = HashMap::new();
        collections.insert("books".to_string(), books);
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        if let Some(LiquidValue::Array(arr)) = ctx.get("books") {
            let first = &arr[0];
            if let LiquidValue::Object(obj) = first {
                assert!(obj.get("title").is_some(), "Book should have title");
                assert!(obj.get("id").is_some(), "Book should have id");
                assert!(obj.get("authors").is_some(), "Book should have authors");
            } else {
                panic!("Expected book to be an object");
            }
        }
    }

    #[test]
    fn test_build_site_context_has_data_events() {
        let config = test_config();
        let data_dir = site_dir().join("_data");
        let data = crate::data::load_data(&data_dir).unwrap();
        let collections = HashMap::new();
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let data_val = ctx.get("data").expect("site should have data");
        if let LiquidValue::Object(data_obj) = data_val {
            let events = data_obj.get("events").expect("data should have events");
            if let LiquidValue::Array(arr) = events {
                assert!(!arr.is_empty(), "Expected 1+ events, got {}", arr.len());
            } else {
                panic!("Expected events to be an array");
            }
        } else {
            panic!("Expected data to be an object");
        }
    }

    #[test]
    fn test_build_site_context_has_url_and_name() {
        let config = test_config();
        let data = DataTree::new();
        let collections = HashMap::new();
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        assert_eq!(
            ctx.get("url"),
            Some(&LiquidValue::scalar("https://datatalks.club"))
        );
        assert_eq!(
            ctx.get("name"),
            Some(&LiquidValue::scalar("DataTalks.Club"))
        );
    }

    // ========================================================================
    // Unit: site.categories mapping
    // ========================================================================

    #[test]
    fn test_build_site_context_categories_mapping() {
        let config = SiteConfig::default();
        let data = DataTree::new();

        // Create 3 posts: A with categories [ml, python], B with category ml, C with none
        let post_a = CollectionItem {
            slug: "post-a".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Post A".to_string()),
                );
                fm.insert(
                    "categories".to_string(),
                    serde_yaml::Value::Sequence(vec![
                        serde_yaml::Value::String("ml".to_string()),
                        serde_yaml::Value::String("python".to_string()),
                    ]),
                );
                fm
            },
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/blog/post-a.html".to_string(),
            date: Some("2021-01-01".to_string()),
            collection_name: "posts".to_string(),
            source_path: "_posts/2021-01-01-post-a.md".to_string(),
            id: String::new(),
        };

        let post_b = CollectionItem {
            slug: "post-b".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Post B".to_string()),
                );
                fm.insert(
                    "category".to_string(),
                    serde_yaml::Value::String("ml".to_string()),
                );
                fm
            },
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/blog/post-b.html".to_string(),
            date: Some("2021-02-01".to_string()),
            collection_name: "posts".to_string(),
            source_path: "_posts/2021-02-01-post-b.md".to_string(),
            id: String::new(),
        };

        let post_c = CollectionItem {
            slug: "post-c".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Post C".to_string()),
                );
                fm
            },
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/blog/post-c.html".to_string(),
            date: Some("2021-03-01".to_string()),
            collection_name: "posts".to_string(),
            source_path: "_posts/2021-03-01-post-c.md".to_string(),
            id: String::new(),
        };

        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), vec![post_a, post_b, post_c]);

        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let categories = ctx.get("categories").expect("should have categories");
        if let LiquidValue::Object(cats) = categories {
            // "ml" should have 2 posts
            let ml = cats.get("ml").expect("should have ml category");
            if let LiquidValue::Array(arr) = ml {
                assert_eq!(arr.len(), 2, "ml category should have 2 posts");
            } else {
                panic!("Expected ml to be an array");
            }

            // "python" should have 1 post
            let python = cats.get("python").expect("should have python category");
            if let LiquidValue::Array(arr) = python {
                assert_eq!(arr.len(), 1, "python category should have 1 post");
            } else {
                panic!("Expected python to be an array");
            }
        } else {
            panic!("Expected categories to be an object");
        }
    }

    // ========================================================================
    // Unit: site.tags mapping
    // ========================================================================

    #[test]
    fn test_build_site_context_tags_mapping() {
        let config = SiteConfig::default();
        let data = DataTree::new();

        let post_a = CollectionItem {
            slug: "post-a".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Post A".to_string()),
                );
                fm.insert(
                    "tags".to_string(),
                    serde_yaml::Value::Sequence(vec![
                        serde_yaml::Value::String("data-science".to_string()),
                        serde_yaml::Value::String("career".to_string()),
                    ]),
                );
                fm
            },
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/blog/post-a.html".to_string(),
            date: Some("2021-01-01".to_string()),
            collection_name: "posts".to_string(),
            source_path: "_posts/2021-01-01-post-a.md".to_string(),
            id: String::new(),
        };

        let post_b = CollectionItem {
            slug: "post-b".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Post B".to_string()),
                );
                fm.insert(
                    "tags".to_string(),
                    serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(
                        "data-science".to_string(),
                    )]),
                );
                fm
            },
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/blog/post-b.html".to_string(),
            date: Some("2021-02-01".to_string()),
            collection_name: "posts".to_string(),
            source_path: "_posts/2021-02-01-post-b.md".to_string(),
            id: String::new(),
        };

        let post_c = CollectionItem {
            slug: "post-c".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Post C".to_string()),
                );
                fm
            },
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/blog/post-c.html".to_string(),
            date: Some("2021-03-01".to_string()),
            collection_name: "posts".to_string(),
            source_path: "_posts/2021-03-01-post-c.md".to_string(),
            id: String::new(),
        };

        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), vec![post_a, post_b, post_c]);

        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let tags = ctx.get("tags").expect("should have tags");
        if let LiquidValue::Object(tag_map) = tags {
            let ds = tag_map
                .get("data-science")
                .expect("should have data-science tag");
            if let LiquidValue::Array(arr) = ds {
                assert_eq!(arr.len(), 2, "data-science tag should have 2 posts");
            } else {
                panic!("Expected data-science to be an array");
            }

            let career = tag_map.get("career").expect("should have career tag");
            if let LiquidValue::Array(arr) = career {
                assert_eq!(arr.len(), 1, "career tag should have 1 post");
            } else {
                panic!("Expected career to be an array");
            }
        } else {
            panic!("Expected tags to be an object");
        }
    }

    #[test]
    fn test_build_site_context_empty_posts_empty_categories_tags() {
        let config = SiteConfig::default();
        let data = DataTree::new();
        let collections = HashMap::new();

        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let categories = ctx.get("categories").expect("should have categories");
        if let LiquidValue::Object(cats) = categories {
            assert!(cats.is_empty(), "categories should be empty with no posts");
        } else {
            panic!("Expected categories to be an object");
        }

        let tags = ctx.get("tags").expect("should have tags");
        if let LiquidValue::Object(tag_map) = tags {
            assert!(tag_map.is_empty(), "tags should be empty with no posts");
        } else {
            panic!("Expected tags to be an object");
        }
    }

    #[test]
    fn test_non_post_collections_excluded_from_tags() {
        let config = SiteConfig::default();
        let data = DataTree::new();

        // People item with tags -- should NOT appear in site.tags
        let person = CollectionItem {
            slug: "alice".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "tags".to_string(),
                    serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(
                        "expert".to_string(),
                    )]),
                );
                fm
            },
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/people/alice.html".to_string(),
            date: None,
            collection_name: "people".to_string(),
            source_path: "_people/alice.md".to_string(),
            id: String::new(),
        };

        let mut collections = HashMap::new();
        collections.insert("people".to_string(), vec![person]);

        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let tags = ctx.get("tags").expect("should have tags");
        if let LiquidValue::Object(tag_map) = tags {
            assert!(
                tag_map.is_empty(),
                "people tags should not appear in site.tags"
            );
        } else {
            panic!("Expected tags to be an object");
        }
    }

    #[test]
    fn test_build_site_context_dtc_tags() {
        let config = test_config();
        let (posts, _) = crate::collection::load_collection("posts", &site_dir(), &config).unwrap();
        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), posts);
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        // The segmentation post has tags: [analytics, clustering]
        let tags = ctx.get("tags").expect("should have tags");
        if let LiquidValue::Object(tag_map) = tags {
            assert!(!tag_map.is_empty(), "DTC posts should produce some tags");
            let analytics = tag_map.get("analytics");
            assert!(
                analytics.is_some(),
                "Should have 'analytics' tag from segmentation post"
            );
        } else {
            panic!("Expected tags to be an object");
        }

        // DTC posts don't use categories
        let categories = ctx.get("categories").expect("should have categories");
        if let LiquidValue::Object(cats) = categories {
            assert!(
                cats.is_empty(),
                "DTC posts should have empty categories (none use categories in front matter)"
            );
        } else {
            panic!("Expected categories to be an object");
        }
    }

    // ========================================================================
    // Unit: GitHub repository URL resolution
    // ========================================================================

    #[test]
    fn test_resolve_repo_url_from_config() {
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            repository: Some("owner/repo".to_string()),
            ..Default::default()
        };
        let url = resolve_repository_url(&config, None);
        assert_eq!(url, LiquidValue::scalar("https://github.com/owner/repo"));
    }

    #[test]
    fn test_resolve_repo_url_from_git_remote() {
        let config = test_config();
        let url = resolve_repository_url(&config, Some(&site_dir()));
        // Should resolve from git remote
        assert_ne!(url, LiquidValue::Nil, "Should resolve from git remote");
        let url_str = url.to_kstr().to_string();
        assert!(
            url_str.contains("github.com"),
            "Should be a GitHub URL: {}",
            url_str
        );
    }

    #[test]
    fn test_resolve_repo_url_nil_when_no_info() {
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            ..Default::default()
        };
        // Pass a non-existent directory to avoid git remote resolving
        let url = resolve_repository_url(&config, Some(Path::new("/nonexistent")));
        assert_eq!(url, LiquidValue::Nil);
    }

    // ========================================================================
    // Unit: Front matter defaults merging
    // ========================================================================

    #[test]
    fn test_resolve_layout_from_config_defaults() {
        let config = test_config();
        let item = CollectionItem {
            slug: "test".to_string(),
            front_matter: HashMap::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/people/test.html".to_string(),
            date: None,
            collection_name: "people".to_string(),
            source_path: "_people/test.md".to_string(),
            id: String::new(),
        };
        assert_eq!(
            resolve_layout(&item, &config, "people"),
            Some("author".to_string())
        );
    }

    #[test]
    fn test_resolve_layout_front_matter_overrides_default() {
        let config = test_config();
        let mut fm = HashMap::new();
        fm.insert(
            "layout".to_string(),
            serde_yaml::Value::String("custom".to_string()),
        );
        let item = CollectionItem {
            slug: "test".to_string(),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/people/test.html".to_string(),
            date: None,
            collection_name: "people".to_string(),
            source_path: "_people/test.md".to_string(),
            id: String::new(),
        };
        assert_eq!(
            resolve_layout(&item, &config, "people"),
            Some("custom".to_string())
        );
    }

    #[test]
    fn test_resolve_layout_no_default_no_front_matter() {
        let config = test_config();
        let item = CollectionItem {
            slug: "test".to_string(),
            front_matter: HashMap::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/courses/test.html".to_string(),
            date: None,
            collection_name: "courses".to_string(),
            source_path: "_courses/test.md".to_string(),
            id: String::new(),
        };
        // courses has no default layout
        assert_eq!(resolve_layout(&item, &config, "courses"), None);
    }

    // ========================================================================
    // Unit: Output path generation
    // ========================================================================

    #[test]
    fn test_output_path_people() {
        let path = output_path(Path::new("/tmp/site"), "people", "alexeygrigorev");
        assert_eq!(path, PathBuf::from("/tmp/site/people/alexeygrigorev.html"));
    }

    #[test]
    fn test_output_path_chiphuyen() {
        let path = output_path(Path::new("/tmp/site"), "people", "chiphuyen");
        assert_eq!(path, PathBuf::from("/tmp/site/people/chiphuyen.html"));
    }

    #[test]
    fn test_book_output_path() {
        let out = output_path(Path::new("/tmp/site"), "books", "20201214-ml-bookcamp");
        assert_eq!(
            out,
            PathBuf::from("/tmp/site/books/20201214-ml-bookcamp.html")
        );
    }

    #[test]
    fn test_output_path_podcast_agentic() {
        let path = output_path(
            Path::new("/tmp/site"),
            "podcast",
            "building-agentic-ai-engineering-tooling-retrieval-evaluation",
        );
        assert_eq!(
            path,
            PathBuf::from(
                "/tmp/site/podcast/building-agentic-ai-engineering-tooling-retrieval-evaluation.html"
            )
        );
    }

    // ========================================================================
    // Unit: url_to_output_path for non-HTML extensions
    // ========================================================================

    #[test]
    fn test_url_to_output_path_xml() {
        let path = url_to_output_path(Path::new("/tmp/site"), "/podcast.xml");
        assert_eq!(path, PathBuf::from("/tmp/site/podcast.xml"));
    }

    #[test]
    fn test_url_to_output_path_json() {
        let path = url_to_output_path(Path::new("/tmp/site"), "/data.json");
        assert_eq!(path, PathBuf::from("/tmp/site/data.json"));
    }

    #[test]
    fn test_url_to_output_path_html_unchanged() {
        let path = url_to_output_path(Path::new("/tmp/site"), "/about.html");
        assert_eq!(path, PathBuf::from("/tmp/site/about.html"));
    }

    #[test]
    fn test_url_to_output_path_no_ext_gets_html() {
        let path = url_to_output_path(Path::new("/tmp/site"), "/about");
        assert_eq!(path, PathBuf::from("/tmp/site/about.html"));
    }

    #[test]
    fn test_url_to_output_path_trailing_slash_pretty() {
        let path = url_to_output_path(Path::new("/tmp/site"), "/about/");
        assert_eq!(path, PathBuf::from("/tmp/site/about/index.html"));
    }

    #[test]
    fn test_url_to_output_path_txt() {
        let path = url_to_output_path(Path::new("/tmp/site"), "/robots.txt");
        assert_eq!(path, PathBuf::from("/tmp/site/robots.txt"));
    }

    // ========================================================================
    // Integration: Render a single person page with simplified layout
    // ========================================================================

    #[test]
    fn test_render_single_person_minimal_layout() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "author".to_string(),
            crate::template::Layout {
                source: "{{ page.title }} {{ content }}".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let mut fm = HashMap::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Alice Smith".to_string()),
        );
        fm.insert(
            "short".to_string(),
            serde_yaml::Value::String("alicesmith".to_string()),
        );

        let site_context = Object::new();
        let html_content = "<p>Alice is a data scientist.</p>";

        let result = engine
            .render_page("author", html_content, &fm, &site_context)
            .unwrap();

        assert!(result.contains("Alice Smith"), "Should contain name");
        assert!(
            result.contains("Alice is a data scientist."),
            "Should contain bio"
        );
    }

    #[test]
    fn test_render_person_with_social_links() {
        let layout_source = r#"
{{ page.title }}
{% if page.twitter %}<a href="https://twitter.com/{{ page.twitter }}">twitter</a>{% endif %}
{% if page.linkedin %}<a href="https://linkedin.com/in/{{ page.linkedin }}">linkedin</a>{% endif %}
{% if page.github %}<a href="https://github.com/{{ page.github }}">github</a>{% endif %}
{% if page.web %}<a href="{{ page.web }}">web</a>{% endif %}
{{ content }}
"#;
        let mut layouts = HashMap::new();
        layouts.insert(
            "author".to_string(),
            crate::template::Layout {
                source: layout_source.to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let mut fm = HashMap::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Bob".to_string()),
        );
        fm.insert(
            "twitter".to_string(),
            serde_yaml::Value::String("bob_t".to_string()),
        );
        fm.insert(
            "linkedin".to_string(),
            serde_yaml::Value::String("bob_l".to_string()),
        );
        fm.insert(
            "github".to_string(),
            serde_yaml::Value::String("bob_g".to_string()),
        );
        fm.insert(
            "web".to_string(),
            serde_yaml::Value::String("https://bob.com".to_string()),
        );

        let site_context = Object::new();
        let result = engine
            .render_page("author", "<p>bio</p>", &fm, &site_context)
            .unwrap();

        assert!(result.contains("https://twitter.com/bob_t"));
        assert!(result.contains("https://linkedin.com/in/bob_l"));
        assert!(result.contains("https://github.com/bob_g"));
        assert!(result.contains("https://bob.com"));
    }

    #[test]
    fn test_render_person_no_social_links() {
        let layout_source = r#"
{{ page.title }}
{% if page.twitter %}<a href="https://twitter.com/{{ page.twitter }}">twitter</a>{% endif %}
{% if page.linkedin %}<a href="https://linkedin.com/in/{{ page.linkedin }}">linkedin</a>{% endif %}
{% if page.github %}<a href="https://github.com/{{ page.github }}">github</a>{% endif %}
{% if page.web %}<a href="{{ page.web }}">web</a>{% endif %}
"#;
        let mut layouts = HashMap::new();
        layouts.insert(
            "author".to_string(),
            crate::template::Layout {
                source: layout_source.to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let mut fm = HashMap::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("NoLinks".to_string()),
        );

        let site_context = Object::new();
        let result = engine
            .render_page("author", "", &fm, &site_context)
            .unwrap();

        assert!(result.contains("NoLinks"));
        assert!(!result.contains("twitter.com"));
        assert!(!result.contains("linkedin.com"));
        assert!(!result.contains("github.com"));
    }

    // ========================================================================
    // Integration: Render with related content
    // ========================================================================

    #[test]
    fn test_render_with_related_posts() {
        let layout_source = r#"
{% assign articles = site.posts | where_exp: "post", "post.authors contains page.short" %}
{% if articles.size > 0 %}<h3>Articles</h3>
<ul>{% for post in articles %}<li><a href="{{ post.url }}">{{ post.title }}</a></li>{% endfor %}</ul>
{% endif %}
"#;
        let mut layouts = HashMap::new();
        layouts.insert(
            "author".to_string(),
            crate::template::Layout {
                source: layout_source.to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        // Build a site context with a post that contains our person
        let mut post = Object::new();
        post.insert("title".into(), LiquidValue::scalar("My Great Article"));
        post.insert(
            "url".into(),
            LiquidValue::scalar("/blog/great-article.html"),
        );
        post.insert(
            "authors".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("alice")]),
        );
        let mut site_context = Object::new();
        site_context.insert(
            "posts".into(),
            LiquidValue::Array(vec![LiquidValue::Object(post)]),
        );

        let mut fm = HashMap::new();
        fm.insert(
            "short".to_string(),
            serde_yaml::Value::String("alice".to_string()),
        );

        let result = engine
            .render_page("author", "", &fm, &site_context)
            .unwrap();

        assert!(
            result.contains("<h3>Articles</h3>"),
            "Should have Articles section"
        );
        assert!(
            result.contains("My Great Article"),
            "Should contain post title"
        );
        assert!(
            result.contains("/blog/great-article.html"),
            "Should contain post URL"
        );
    }

    #[test]
    fn test_render_with_related_events() {
        let layout_source = r#"
{% assign events = site.data.events | where_exp: "event", "event.speakers contains page.short" %}
{% if events.size > 0 %}<h3>Events</h3>
<ul>{% for event in events %}<li>{{ event.title }}</li>{% endfor %}</ul>
{% endif %}
"#;
        let mut layouts = HashMap::new();
        layouts.insert(
            "author".to_string(),
            crate::template::Layout {
                source: layout_source.to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let mut event = Object::new();
        event.insert("title".into(), LiquidValue::scalar("Cool Event"));
        event.insert(
            "speakers".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("bob")]),
        );
        let mut data_obj = Object::new();
        data_obj.insert(
            "events".into(),
            LiquidValue::Array(vec![LiquidValue::Object(event)]),
        );
        let mut site_context = Object::new();
        site_context.insert("data".into(), LiquidValue::Object(data_obj));

        let mut fm = HashMap::new();
        fm.insert(
            "short".to_string(),
            serde_yaml::Value::String("bob".to_string()),
        );

        let result = engine
            .render_page("author", "", &fm, &site_context)
            .unwrap();

        assert!(
            result.contains("<h3>Events</h3>"),
            "Should have Events section"
        );
        assert!(result.contains("Cool Event"), "Should contain event title");
    }

    #[test]
    fn test_render_with_related_books() {
        let layout_source = r#"
{% assign books = site.books | where_exp: "book", "book.authors contains page.short" %}
{% if books.size > 0 %}<h3>Books</h3>
<ul>{% for book in books %}<li>{{ book.title }}</li>{% endfor %}</ul>
{% endif %}
"#;
        let mut layouts = HashMap::new();
        layouts.insert(
            "author".to_string(),
            crate::template::Layout {
                source: layout_source.to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let mut book = Object::new();
        book.insert("title".into(), LiquidValue::scalar("ML Bookcamp"));
        book.insert(
            "authors".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("carol")]),
        );
        book.insert("id".into(), LiquidValue::scalar("/books/ml-bookcamp"));
        let mut site_context = Object::new();
        site_context.insert(
            "books".into(),
            LiquidValue::Array(vec![LiquidValue::Object(book)]),
        );

        let mut fm = HashMap::new();
        fm.insert(
            "short".to_string(),
            serde_yaml::Value::String("carol".to_string()),
        );

        let result = engine
            .render_page("author", "", &fm, &site_context)
            .unwrap();

        assert!(
            result.contains("<h3>Books</h3>"),
            "Should have Books section"
        );
        assert!(result.contains("ML Bookcamp"), "Should contain book title");
    }

    #[test]
    fn test_render_with_no_related_content() {
        let layout_source = r#"
{% assign articles = site.posts | where_exp: "post", "post.authors contains page.short" %}
{% if articles.size > 0 %}<h3>Articles</h3>{% endif %}
{% assign events = site.data.events | where_exp: "event", "event.speakers contains page.short" %}
{% if events.size > 0 %}<h3>Events</h3>{% endif %}
{% assign books = site.books | where_exp: "book", "book.authors contains page.short" %}
{% if books.size > 0 %}<h3>Books</h3>{% endif %}
"#;
        let mut layouts = HashMap::new();
        layouts.insert(
            "author".to_string(),
            crate::template::Layout {
                source: layout_source.to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let mut site_context = Object::new();
        site_context.insert("posts".into(), LiquidValue::Array(vec![]));
        site_context.insert("books".into(), LiquidValue::Array(vec![]));
        let mut data_obj = Object::new();
        data_obj.insert("events".into(), LiquidValue::Array(vec![]));
        site_context.insert("data".into(), LiquidValue::Object(data_obj));

        let mut fm = HashMap::new();
        fm.insert(
            "short".to_string(),
            serde_yaml::Value::String("nobody".to_string()),
        );

        let result = engine
            .render_page("author", "", &fm, &site_context)
            .unwrap();

        assert!(
            !result.contains("<h3>Articles</h3>"),
            "No Articles section expected"
        );
        assert!(
            !result.contains("<h3>Events</h3>"),
            "No Events section expected"
        );
        assert!(
            !result.contains("<h3>Books</h3>"),
            "No Books section expected"
        );
    }

    // ========================================================================
    // Edge cases
    // ========================================================================

    #[test]
    fn test_person_with_empty_content() {
        let mut layouts = HashMap::new();
        layouts.insert(
            "author".to_string(),
            crate::template::Layout {
                source: "{{ page.title }} | {{ content }}".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let mut fm = HashMap::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Empty Person".to_string()),
        );
        fm.insert(
            "twitter".to_string(),
            serde_yaml::Value::String("empty_t".to_string()),
        );

        let site_context = Object::new();
        let result = engine
            .render_page("author", "", &fm, &site_context)
            .unwrap();

        assert!(
            result.contains("Empty Person"),
            "Name should appear even with empty content"
        );
    }

    #[test]
    fn test_person_with_no_short_field() {
        let layout_source = r#"
{% assign articles = site.posts | where_exp: "post", "post.authors contains page.short" %}
{% if articles.size > 0 %}<h3>Articles</h3>{% endif %}
DONE
"#;
        let mut layouts = HashMap::new();
        layouts.insert(
            "author".to_string(),
            crate::template::Layout {
                source: layout_source.to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let mut post = Object::new();
        post.insert("title".into(), LiquidValue::scalar("Some Article"));
        post.insert(
            "authors".into(),
            LiquidValue::Array(vec![LiquidValue::scalar("someone")]),
        );
        let mut site_context = Object::new();
        site_context.insert(
            "posts".into(),
            LiquidValue::Array(vec![LiquidValue::Object(post)]),
        );

        // No "short" field in front matter
        let fm = HashMap::new();

        let result = engine
            .render_page("author", "", &fm, &site_context)
            .unwrap();

        // Should not crash, and should not show Articles since page.short is nil
        assert!(!result.contains("<h3>Articles</h3>"));
        assert!(result.contains("DONE"));
    }

    #[test]
    fn test_person_with_partial_social_links() {
        let layout_source = r#"
{% if page.twitter %}<a href="https://twitter.com/{{ page.twitter }}">T</a>{% endif %}
{% if page.linkedin %}<a href="https://linkedin.com/in/{{ page.linkedin }}">L</a>{% endif %}
{% if page.github %}<a href="https://github.com/{{ page.github }}">G</a>{% endif %}
{% if page.web %}<a href="{{ page.web }}">W</a>{% endif %}
"#;
        let mut layouts = HashMap::new();
        layouts.insert(
            "author".to_string(),
            crate::template::Layout {
                source: layout_source.to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let mut fm = HashMap::new();
        fm.insert(
            "twitter".to_string(),
            serde_yaml::Value::String("only_t".to_string()),
        );
        fm.insert(
            "github".to_string(),
            serde_yaml::Value::String("only_g".to_string()),
        );

        let site_context = Object::new();
        let result = engine
            .render_page("author", "", &fm, &site_context)
            .unwrap();

        assert!(result.contains("twitter.com/only_t"));
        assert!(result.contains("github.com/only_g"));
        assert!(!result.contains("linkedin.com"));
        assert!(!result.contains("page.web"));
    }

    // ========================================================================
    // Integration: generate_collection_pages with temp dir
    // ========================================================================

    #[test]
    fn test_generate_collection_pages_writes_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "author".to_string(),
            crate::template::Layout {
                source: "{{ page.title }}".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: String::new(),
                    type_name: "people".to_string(),
                },
                values: crate::config::DefaultValues {
                    values: {
                        let mut m = HashMap::new();
                        m.insert(
                            "layout".to_string(),
                            serde_yaml::Value::String("author".to_string()),
                        );
                        m
                    },
                },
            }],
            ..Default::default()
        };

        let items = vec![
            CollectionItem {
                slug: "alice".to_string(),
                front_matter: {
                    let mut fm = HashMap::new();
                    fm.insert(
                        "title".to_string(),
                        serde_yaml::Value::String("Alice".to_string()),
                    );
                    fm
                },
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                url: "/people/alice.html".to_string(),
                date: None,
                collection_name: "people".to_string(),
                source_path: "_people/alice.md".to_string(),
                id: String::new(),
            },
            CollectionItem {
                slug: "bob".to_string(),
                front_matter: {
                    let mut fm = HashMap::new();
                    fm.insert(
                        "title".to_string(),
                        serde_yaml::Value::String("Bob".to_string()),
                    );
                    fm
                },
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                url: "/people/bob.html".to_string(),
                date: None,
                collection_name: "people".to_string(),
                source_path: "_people/bob.md".to_string(),
                id: String::new(),
            },
        ];

        let site_context = Object::new();
        let result = generate_collection_pages(
            &items,
            "people",
            &config,
            &engine,
            &site_context,
            output_dir,
        )
        .unwrap();

        assert_eq!(result.generated, 2);
        assert_eq!(result.skipped, 0);
        assert!(result.errors.is_empty());

        assert!(output_dir.join("people/alice.html").exists());
        assert!(output_dir.join("people/bob.html").exists());

        let alice_html = fs::read_to_string(output_dir.join("people/alice.html")).unwrap();
        assert!(alice_html.contains("Alice"));

        let bob_html = fs::read_to_string(output_dir.join("people/bob.html")).unwrap();
        assert!(bob_html.contains("Bob"));
    }

    // ========================================================================
    // Unit: Generic collection generation with mock data
    // ========================================================================

    #[test]
    fn test_generic_generation_mock_collection() {
        let tmp = tempfile::TempDir::new().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "custom".to_string(),
            crate::template::Layout {
                source: "<h1>{{ page.title }}</h1>{{ content }}".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            twitter: None,
            repository: None,
            permalink: "/:title.html".to_string(),
            exclude: vec![],
            collections: HashMap::new(),
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: String::new(),
                    type_name: "widgets".to_string(),
                },
                values: crate::config::DefaultValues {
                    values: {
                        let mut m = HashMap::new();
                        m.insert(
                            "layout".to_string(),
                            serde_yaml::Value::String("custom".to_string()),
                        );
                        m
                    },
                },
            }],
            ..Default::default()
        };

        let items = vec![
            CollectionItem {
                slug: "widget-a".to_string(),
                front_matter: {
                    let mut fm = HashMap::new();
                    fm.insert(
                        "title".to_string(),
                        serde_yaml::Value::String("Widget A".to_string()),
                    );
                    fm
                },
                content: String::new(),
                html_content: "<p>Description A</p>".to_string(),
                excerpt: None,
                url: "/widgets/widget-a.html".to_string(),
                date: None,
                collection_name: "widgets".to_string(),
                source_path: "_widgets/widget-a.md".to_string(),
                id: String::new(),
            },
            CollectionItem {
                slug: "widget-b".to_string(),
                front_matter: {
                    let mut fm = HashMap::new();
                    fm.insert(
                        "title".to_string(),
                        serde_yaml::Value::String("Widget B".to_string()),
                    );
                    fm
                },
                content: String::new(),
                html_content: "<p>Description B</p>".to_string(),
                excerpt: None,
                url: "/widgets/widget-b.html".to_string(),
                date: None,
                collection_name: "widgets".to_string(),
                source_path: "_widgets/widget-b.md".to_string(),
                id: String::new(),
            },
        ];

        let site_context = Object::new();
        let result = generate_collection_pages(
            &items,
            "widgets",
            &config,
            &engine,
            &site_context,
            output_dir,
        )
        .unwrap();

        assert_eq!(result.generated, 2);
        assert!(output_dir.join("widgets/widget-a.html").exists());
        assert!(output_dir.join("widgets/widget-b.html").exists());

        let a_html = fs::read_to_string(output_dir.join("widgets/widget-a.html")).unwrap();
        assert!(a_html.contains("Widget A"));
        assert!(a_html.contains("Description A"));
    }

    // ========================================================================
    // Unit: where filter in template engine
    // ========================================================================

    #[test]
    fn test_where_filter_in_template() {
        let engine = crate::template::TemplateEngine::new().unwrap();
        let mut ctx = Object::new();

        let people = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("short".into(), LiquidValue::scalar("alice"));
                o.insert("title".into(), LiquidValue::scalar("Alice Smith"));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("short".into(), LiquidValue::scalar("bob"));
                o.insert("title".into(), LiquidValue::scalar("Bob Jones"));
                o
            }),
        ]);

        let mut site = Object::new();
        site.insert("people".into(), people);
        ctx.insert("site".into(), LiquidValue::Object(site));

        let template = r#"{% assign author = site.people | where: "short", "alice" | first %}{{ author.title }}"#;
        let output = engine.parse_and_render(template, &ctx).unwrap();
        assert_eq!(output, "Alice Smith");
    }

    #[test]
    fn test_where_filter_with_variable() {
        let engine = crate::template::TemplateEngine::new().unwrap();
        let mut ctx = Object::new();

        let people = LiquidValue::Array(vec![LiquidValue::Object({
            let mut o = Object::new();
            o.insert("short".into(), LiquidValue::scalar("alice"));
            o.insert("title".into(), LiquidValue::scalar("Alice Smith"));
            o
        })]);

        let mut site = Object::new();
        site.insert("people".into(), people);
        ctx.insert("site".into(), LiquidValue::Object(site));
        ctx.insert("a".into(), LiquidValue::scalar("alice"));

        let template =
            r#"{% assign author = site.people | where: "short", a | first %}{{ author.title }}"#;
        let output = engine.parse_and_render(template, &ctx).unwrap();
        assert_eq!(output, "Alice Smith");
    }

    // ========================================================================
    // Unit: LenientValue recursive array leniency
    // ========================================================================

    #[test]
    fn test_normalized_array_objects_missing_keys() {
        // Objects inside arrays should return Nil for missing keys, not error.
        // normalize_arrays() ensures all objects in an array share the same keys.
        let engine = crate::template::TemplateEngine::new().unwrap();
        let mut ctx = Object::new();

        // Array of objects where some lack certain keys
        let items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("name".into(), LiquidValue::scalar("Alice"));
                // no "role" key
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("name".into(), LiquidValue::scalar("Bob"));
                o.insert("role".into(), LiquidValue::scalar("Engineer"));
                o
            }),
        ]);
        ctx.insert("items".into(), normalize_arrays(items));

        let template = "{% for item in items %}{{ item.name }}:{{ item.role }};{% endfor %}";
        let output = engine.parse_and_render(template, &ctx).unwrap();
        assert_eq!(output, "Alice:;Bob:Engineer;");
    }

    #[test]
    fn test_normalized_nested_arrays_of_objects() {
        // Arrays of objects containing arrays of objects -- 2 levels deep.
        // normalize_arrays() recursively normalizes at all nesting levels.
        // Some inner objects have "y" and some do not -- normalization pads with Nil.
        let engine = crate::template::TemplateEngine::new().unwrap();
        let mut ctx = Object::new();

        let inner_items = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("x".into(), LiquidValue::scalar("found"));
                // no "y" key
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("x".into(), LiquidValue::scalar("also"));
                o.insert("y".into(), LiquidValue::scalar("deep"));
                o
            }),
        ]);

        let outer_items = LiquidValue::Array(vec![LiquidValue::Object({
            let mut o = Object::new();
            o.insert("children".into(), inner_items);
            o
        })]);
        ctx.insert("items".into(), normalize_arrays(outer_items));

        let template = "{% for item in items %}{% for child in item.children %}{{ child.x }}{{ child.y }};{% endfor %}{% endfor %}";
        let output = engine.parse_and_render(template, &ctx).unwrap();
        assert_eq!(output, "found;alsodeep;");
    }

    #[test]
    fn test_lenient_value_array_iteration_renders_empty_for_missing() {
        let engine = crate::template::TemplateEngine::new().unwrap();
        let mut ctx = Object::new();

        let transcript = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("header".into(), LiquidValue::scalar("Introduction"));
                // no "who" or "line"
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("who".into(), LiquidValue::scalar("Alexey"));
                o.insert("line".into(), LiquidValue::scalar("Hello!"));
                // no "header"
                o
            }),
        ]);

        let mut page = Object::new();
        page.insert("transcript".into(), transcript);
        ctx.insert("page".into(), LiquidValue::Object(page));

        let template = r#"{% for item in page.transcript %}{% if item.header %}[{{ item.header }}]{% endif %}{% if item.who %}<b>{{ item.who }}</b>: {{ item.line }}{% endif %}{% endfor %}"#;
        let output = engine.parse_and_render(template, &ctx).unwrap();
        assert!(output.contains("[Introduction]"));
        assert!(output.contains("<b>Alexey</b>: Hello!"));
    }

    // ========================================================================
    // Issue 23: Site context population with extras
    // ========================================================================

    #[test]
    fn test_site_context_empty_url_default() {
        let config = SiteConfig::default();
        let colls = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &colls, &data, None, &[]);
        assert_eq!(ctx.get("url"), Some(&LiquidValue::scalar("")));
    }

    #[test]
    fn test_site_context_extras_populated() {
        let mut config = SiteConfig::default();
        config.extras.insert(
            "locale".to_string(),
            serde_yaml::Value::String("en".to_string()),
        );
        config.extras.insert(
            "author".to_string(),
            serde_yaml::Value::String("Alice".to_string()),
        );
        let colls = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &colls, &data, None, &[]);
        assert_eq!(ctx.get("locale"), Some(&LiquidValue::scalar("en")));
        assert_eq!(ctx.get("author"), Some(&LiquidValue::scalar("Alice")));
    }

    #[test]
    fn test_site_context_twitter_map() {
        let mut mapping = serde_yaml::Mapping::new();
        mapping.insert(
            serde_yaml::Value::String("username".to_string()),
            serde_yaml::Value::String("handle".to_string()),
        );
        let config = SiteConfig {
            twitter: Some(serde_yaml::Value::Mapping(mapping)),
            ..Default::default()
        };
        let colls = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &colls, &data, None, &[]);
        let twitter = ctx.get("twitter").expect("should have twitter");
        if let LiquidValue::Object(obj) = twitter {
            let username = obj.get("username").expect("should have username");
            assert_eq!(username.to_kstr().as_str(), "handle");
        } else {
            panic!("Expected twitter to be an object, got {:?}", twitter);
        }
    }

    #[test]
    fn test_site_context_nested_extra() {
        let yaml = "sass:\n  style: compressed\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let colls = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &colls, &data, None, &[]);
        let sass = ctx.get("sass").expect("should have sass");
        if let LiquidValue::Object(obj) = sass {
            let style = obj.get("style").expect("should have style");
            assert_eq!(style.to_kstr().as_str(), "compressed");
        } else {
            panic!("Expected sass to be an object");
        }
    }

    // ========================================================================
    // Issue 24: Site context includes baseurl
    // ========================================================================

    #[test]
    fn test_site_context_baseurl_populated() {
        let config = SiteConfig {
            baseurl: "/blog".to_string(),
            ..Default::default()
        };
        let colls = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &colls, &data, None, &[]);
        assert_eq!(ctx.get("baseurl"), Some(&LiquidValue::scalar("/blog")));
    }

    #[test]
    fn test_site_context_baseurl_default_empty() {
        let config = SiteConfig::default();
        let colls = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &colls, &data, None, &[]);
        assert_eq!(ctx.get("baseurl"), Some(&LiquidValue::scalar("")));
    }

    // ========================================================================
    // Issue 28: Applying defaults to front matter in generation
    // ========================================================================

    #[test]
    fn test_generate_applies_defaults_to_page_fm() {
        // Create a config with defaults that set comments: true for posts
        let yaml = r#"
url: "https://example.com"
name: "Test"
title: "Test"
permalink: "/blog/:title.html"
defaults:
  - scope:
      type: "posts"
    values:
      layout: "post"
      comments: true
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();

        // Create a layout that renders page.comments
        let mut layouts = HashMap::new();
        layouts.insert(
            "post".to_string(),
            crate::template::Layout {
                source: "Comments: {{ page.comments }}".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        // Create a post with NO comments in front matter
        let item = CollectionItem {
            slug: "test-post".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Test Post".to_string()),
                );
                fm
            },
            content: "body".to_string(),
            html_content: "<p>body</p>".to_string(),
            excerpt: None,
            url: "/blog/test-post.html".to_string(),
            date: Some("2021-01-01".to_string()),
            collection_name: "posts".to_string(),
            source_path: "_posts/2021-01-01-test-post.md".to_string(),
            id: String::new(),
        };

        let dir = tempfile::TempDir::new().unwrap();
        let site_context = Object::new();
        let result = generate_collection_pages(
            &[item],
            "posts",
            &config,
            &engine,
            &site_context,
            dir.path(),
        )
        .unwrap();

        assert_eq!(result.generated, 1);
        assert_eq!(result.skipped, 0);

        // Read the generated file and check that comments default was applied
        let content = fs::read_to_string(dir.path().join("blog/test-post.html")).unwrap();
        assert!(
            content.contains("Comments: true"),
            "Default comments: true should be applied. Got: {}",
            content
        );
    }

    #[test]
    fn test_generate_front_matter_overrides_defaults() {
        let yaml = r#"
url: "https://example.com"
name: "Test"
title: "Test"
permalink: "/blog/:title.html"
defaults:
  - scope:
      type: "posts"
    values:
      layout: "post"
      comments: true
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();

        let mut layouts = HashMap::new();
        layouts.insert(
            "post".to_string(),
            crate::template::Layout {
                source: "Comments: {{ page.comments }}".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        // Create a post WITH comments: false in front matter (should override default)
        let item = CollectionItem {
            slug: "test-post".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Test Post".to_string()),
                );
                fm.insert("comments".to_string(), serde_yaml::Value::Bool(false));
                fm
            },
            content: "body".to_string(),
            html_content: "<p>body</p>".to_string(),
            excerpt: None,
            url: "/blog/test-post.html".to_string(),
            date: Some("2021-01-01".to_string()),
            collection_name: "posts".to_string(),
            source_path: "_posts/2021-01-01-test-post.md".to_string(),
            id: String::new(),
        };

        let dir = tempfile::TempDir::new().unwrap();
        let site_context = Object::new();
        let result = generate_collection_pages(
            &[item],
            "posts",
            &config,
            &engine,
            &site_context,
            dir.path(),
        )
        .unwrap();

        assert_eq!(result.generated, 1);

        let content = fs::read_to_string(dir.path().join("blog/test-post.html")).unwrap();
        assert!(
            content.contains("Comments: false"),
            "Front matter comments: false should override default. Got: {}",
            content
        );
    }

    fn make_post(slug: &str, date: &str, title: &str) -> CollectionItem {
        let mut fm = crate::frontmatter::FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String(title.to_string()),
        );
        CollectionItem {
            slug: slug.to_string(),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: format!("/blog/{slug}.html"),
            date: Some(date.to_string()),
            collection_name: "posts".to_string(),
            source_path: format!("_posts/{date}-{slug}.md"),
            id: format!("/blog/{slug}"),
        }
    }

    #[test]
    fn test_prev_next_three_posts() {
        let posts = vec![
            make_post("post-a", "2024-01-01", "Post A"),
            make_post("post-b", "2024-02-01", "Post B"),
            make_post("post-c", "2024-03-01", "Post C"),
        ];
        let map = build_prev_next_map(&posts);

        // Post A: no previous, next is B
        let (prev_a, next_a) = map.get("post-a").unwrap();
        assert!(prev_a.is_none());
        let next_a_map = next_a.as_ref().unwrap().as_mapping().unwrap();
        assert_eq!(
            next_a_map
                .get(serde_yaml::Value::String("title".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Post B"
        );

        // Post B: previous is A, next is C
        let (prev_b, next_b) = map.get("post-b").unwrap();
        let prev_b_map = prev_b.as_ref().unwrap().as_mapping().unwrap();
        assert_eq!(
            prev_b_map
                .get(serde_yaml::Value::String("title".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Post A"
        );
        let next_b_map = next_b.as_ref().unwrap().as_mapping().unwrap();
        assert_eq!(
            next_b_map
                .get(serde_yaml::Value::String("title".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Post C"
        );

        // Post C: previous is B, no next
        let (prev_c, next_c) = map.get("post-c").unwrap();
        let prev_c_map = prev_c.as_ref().unwrap().as_mapping().unwrap();
        assert_eq!(
            prev_c_map
                .get(serde_yaml::Value::String("title".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Post B"
        );
        assert!(next_c.is_none());
    }

    #[test]
    fn test_prev_next_single_post() {
        let posts = vec![make_post("only", "2024-01-01", "Only Post")];
        let map = build_prev_next_map(&posts);
        let (prev, next) = map.get("only").unwrap();
        assert!(prev.is_none());
        assert!(next.is_none());
    }

    #[test]
    fn test_prev_next_two_posts() {
        let posts = vec![
            make_post("first", "2024-01-01", "First"),
            make_post("second", "2024-02-01", "Second"),
        ];
        let map = build_prev_next_map(&posts);

        let (prev_first, next_first) = map.get("first").unwrap();
        assert!(prev_first.is_none());
        let next_map = next_first.as_ref().unwrap().as_mapping().unwrap();
        assert_eq!(
            next_map
                .get(serde_yaml::Value::String("url".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "/blog/second.html"
        );

        let (prev_second, next_second) = map.get("second").unwrap();
        assert!(next_second.is_none());
        let prev_map = prev_second.as_ref().unwrap().as_mapping().unwrap();
        assert_eq!(
            prev_map
                .get(serde_yaml::Value::String("url".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "/blog/first.html"
        );
    }

    #[test]
    fn test_prev_next_url_and_title_correct() {
        let posts = vec![
            make_post("alpha", "2024-01-01", "Alpha Post"),
            make_post("beta", "2024-02-01", "Beta Post"),
        ];
        let map = build_prev_next_map(&posts);

        let (_, next) = map.get("alpha").unwrap();
        let next_map = next.as_ref().unwrap().as_mapping().unwrap();
        assert_eq!(
            next_map
                .get(serde_yaml::Value::String("url".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "/blog/beta.html"
        );
        assert_eq!(
            next_map
                .get(serde_yaml::Value::String("title".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Beta Post"
        );
    }

    #[test]
    fn test_prev_next_out_of_order_dates_sorted() {
        // Posts provided in wrong order should still be sorted by date
        let posts = vec![
            make_post("newest", "2024-03-01", "Newest"),
            make_post("oldest", "2024-01-01", "Oldest"),
            make_post("middle", "2024-02-01", "Middle"),
        ];
        let map = build_prev_next_map(&posts);

        // Oldest should have no previous
        let (prev, next) = map.get("oldest").unwrap();
        assert!(prev.is_none());
        let next_map = next.as_ref().unwrap().as_mapping().unwrap();
        assert_eq!(
            next_map
                .get(serde_yaml::Value::String("title".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Middle"
        );

        // Newest should have no next
        let (prev_n, next_n) = map.get("newest").unwrap();
        assert!(next_n.is_none());
        let prev_map = prev_n.as_ref().unwrap().as_mapping().unwrap();
        assert_eq!(
            prev_map
                .get(serde_yaml::Value::String("title".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Middle"
        );
    }

    #[test]
    fn test_prev_next_same_date_deterministic_by_slug() {
        let posts = vec![
            make_post("beta", "2024-01-01", "Beta"),
            make_post("alpha", "2024-01-01", "Alpha"),
        ];
        let map = build_prev_next_map(&posts);

        // alpha sorts before beta by slug
        let (prev_alpha, next_alpha) = map.get("alpha").unwrap();
        assert!(prev_alpha.is_none());
        let next_map = next_alpha.as_ref().unwrap().as_mapping().unwrap();
        assert_eq!(
            next_map
                .get(serde_yaml::Value::String("title".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Beta"
        );
    }

    #[test]
    fn test_prev_next_empty_collection() {
        let posts: Vec<CollectionItem> = vec![];
        let map = build_prev_next_map(&posts);
        assert!(map.is_empty());
    }

    #[test]
    fn test_prev_next_contains_date() {
        let posts = vec![
            make_post("a", "2024-01-15", "A"),
            make_post("b", "2024-02-20", "B"),
        ];
        let map = build_prev_next_map(&posts);
        let (_, next) = map.get("a").unwrap();
        let next_map = next.as_ref().unwrap().as_mapping().unwrap();
        assert_eq!(
            next_map
                .get(serde_yaml::Value::String("date".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "2024-02-20"
        );
    }

    #[test]
    fn test_prev_next_not_injected_for_non_posts() {
        // build_prev_next_map is only called for posts collection.
        // For non-post collections, the map should be empty (tested by
        // verifying generate_collection_pages only calls it for "posts").
        // Here we verify the map itself works correctly even if called
        // with non-post items -- the function is agnostic to collection type.
        let items = vec![make_post("person", "2024-01-01", "Person")];
        let map = build_prev_next_map(&items);
        assert_eq!(map.len(), 1); // It builds a map regardless
                                  // The guard in generate_collection_pages prevents injection for non-posts
    }

    // ========================================================================
    // Issue 42: site.related_posts and site.pages
    // ========================================================================

    fn make_test_post(slug: &str, date: &str, title: &str) -> CollectionItem {
        let mut fm = HashMap::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String(title.to_string()),
        );
        CollectionItem {
            slug: slug.to_string(),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: format!("/blog/{}.html", slug),
            date: Some(date.to_string()),
            collection_name: "posts".to_string(),
            source_path: format!("_posts/{}-{}.md", date, slug),
            id: format!("/blog/{}", slug),
        }
    }

    fn make_test_page(slug: &str, title: &str) -> Page {
        let mut fm = HashMap::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String(title.to_string()),
        );
        Page {
            slug: slug.to_string(),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            url: format!("/{}.html", slug),
            source_path: format!("{}.md", slug),
        }
    }

    #[test]
    fn test_related_posts_with_15_posts() {
        let config = SiteConfig::default();
        let data = DataTree::new();

        let mut posts: Vec<CollectionItem> = (1..=15)
            .map(|i| {
                make_test_post(
                    &format!("post-{}", i),
                    &format!("2024-{:02}-01", i.min(12)),
                    &format!("Post {}", i),
                )
            })
            .collect();
        // Give posts 13-15 dates in 2025 to make them the most recent
        posts[12].date = Some("2025-01-01".to_string());
        posts[13].date = Some("2025-02-01".to_string());
        posts[14].date = Some("2025-03-01".to_string());

        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), posts);
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let related = ctx.get("related_posts").expect("should have related_posts");
        if let LiquidValue::Array(arr) = related {
            assert_eq!(arr.len(), 10, "Should have exactly 10 related posts");
        } else {
            panic!("Expected related_posts to be an array");
        }
    }

    #[test]
    fn test_related_posts_sorted_descending() {
        let config = SiteConfig::default();
        let data = DataTree::new();

        let posts = vec![
            make_test_post("old", "2020-01-01", "Old Post"),
            make_test_post("mid", "2022-06-15", "Mid Post"),
            make_test_post("new", "2024-12-01", "New Post"),
        ];

        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), posts);
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let related = ctx.get("related_posts").expect("should have related_posts");
        if let LiquidValue::Array(arr) = related {
            assert_eq!(arr.len(), 3, "Should have all 3 posts");

            // First should be newest
            if let LiquidValue::Object(first) = &arr[0] {
                let title = first.get("title").unwrap();
                assert_eq!(
                    title,
                    &LiquidValue::scalar("New Post"),
                    "First should be most recent"
                );
            }
            // Last should be oldest
            if let LiquidValue::Object(last) = &arr[2] {
                let title = last.get("title").unwrap();
                assert_eq!(
                    title,
                    &LiquidValue::scalar("Old Post"),
                    "Last should be oldest"
                );
            }
        } else {
            panic!("Expected related_posts to be an array");
        }
    }

    #[test]
    fn test_related_posts_with_5_posts() {
        let config = SiteConfig::default();
        let data = DataTree::new();

        let posts: Vec<CollectionItem> = (1..=5)
            .map(|i| {
                make_test_post(
                    &format!("post-{}", i),
                    &format!("2024-{:02}-01", i),
                    &format!("Post {}", i),
                )
            })
            .collect();

        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), posts);
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let related = ctx.get("related_posts").expect("should have related_posts");
        if let LiquidValue::Array(arr) = related {
            assert_eq!(arr.len(), 5, "Should have all 5 posts");
        } else {
            panic!("Expected related_posts to be an array");
        }
    }

    #[test]
    fn test_related_posts_with_zero_posts() {
        let config = SiteConfig::default();
        let data = DataTree::new();
        let collections = HashMap::new(); // No posts collection at all

        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let related = ctx.get("related_posts").expect("should have related_posts");
        if let LiquidValue::Array(arr) = related {
            assert!(arr.is_empty(), "Should be empty array with no posts");
        } else {
            panic!("Expected related_posts to be an array");
        }
    }

    #[test]
    fn test_related_posts_entries_have_required_fields() {
        let config = SiteConfig::default();
        let data = DataTree::new();

        let posts = vec![make_test_post("test", "2024-01-01", "Test Post")];

        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), posts);
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let related = ctx.get("related_posts").expect("should have related_posts");
        if let LiquidValue::Array(arr) = related {
            if let LiquidValue::Object(obj) = &arr[0] {
                assert!(obj.get("title").is_some(), "Should have title");
                assert!(obj.get("url").is_some(), "Should have url");
                assert!(obj.get("date").is_some(), "Should have date");
            } else {
                panic!("Expected post to be an object");
            }
        }
    }

    #[test]
    fn test_site_pages_with_pages() {
        let config = SiteConfig::default();
        let data = DataTree::new();
        let collections = HashMap::new();
        let pages = vec![
            make_test_page("about", "About"),
            make_test_page("contact", "Contact"),
            make_test_page("index", "Home"),
        ];

        let ctx = build_site_context(&config, &collections, &data, None, &pages);

        let pages_val = ctx.get("pages").expect("should have pages");
        if let LiquidValue::Array(arr) = pages_val {
            assert_eq!(arr.len(), 3, "Should have 3 pages");
        } else {
            panic!("Expected pages to be an array");
        }
    }

    #[test]
    fn test_site_pages_entries_have_required_fields() {
        let config = SiteConfig::default();
        let data = DataTree::new();
        let collections = HashMap::new();
        let pages = vec![make_test_page("about", "About Us")];

        let ctx = build_site_context(&config, &collections, &data, None, &pages);

        let pages_val = ctx.get("pages").expect("should have pages");
        if let LiquidValue::Array(arr) = pages_val {
            if let LiquidValue::Object(obj) = &arr[0] {
                assert_eq!(
                    obj.get("title"),
                    Some(&LiquidValue::scalar("About Us")),
                    "Should have title"
                );
                assert!(obj.get("url").is_some(), "Should have url");
            } else {
                panic!("Expected page to be an object");
            }
        }
    }

    #[test]
    fn test_site_pages_empty_when_no_pages() {
        let config = SiteConfig::default();
        let data = DataTree::new();
        let collections = HashMap::new();

        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let pages_val = ctx.get("pages").expect("should have pages");
        if let LiquidValue::Array(arr) = pages_val {
            assert!(arr.is_empty(), "Should be empty array with no pages");
        } else {
            panic!("Expected pages to be an array");
        }
    }

    #[test]
    fn test_site_posts_unchanged_with_related_posts() {
        let config = SiteConfig::default();
        let data = DataTree::new();

        let posts: Vec<CollectionItem> = (1..=15)
            .map(|i| {
                make_test_post(
                    &format!("post-{}", i),
                    &format!("2024-{:02}-01", i.min(12)),
                    &format!("Post {}", i),
                )
            })
            .collect();

        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), posts);
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        // site.posts should still have all 15, not just 10
        let all_posts = ctx.get("posts").expect("should have posts");
        if let LiquidValue::Array(arr) = all_posts {
            assert_eq!(
                arr.len(),
                15,
                "site.posts should contain ALL posts, got {}",
                arr.len()
            );
        } else {
            panic!("Expected posts to be an array");
        }

        // site.related_posts should have only 10
        let related = ctx.get("related_posts").expect("should have related_posts");
        if let LiquidValue::Array(arr) = related {
            assert_eq!(
                arr.len(),
                10,
                "site.related_posts should have 10, got {}",
                arr.len()
            );
        }
    }

    // ========================================================================
    // Unit: Empty-body collection items (Issue 80)
    // ========================================================================

    #[test]
    fn test_empty_body_no_layout_produces_newline() {
        // A collection item with no layout and empty body should produce "\n"
        let item = CollectionItem {
            slug: "modelstore".to_string(),
            url: "/tools/modelstore.html".to_string(),
            date: None,
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("ModelStore".to_string()),
                );
                fm
            },
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            collection_name: "tools".to_string(),
            source_path: "_tools/modelstore.md".to_string(),
            id: String::new(),
        };

        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path();

        // Create minimal layout engine (no layouts needed for this test)
        let layouts_dir = tmp.path().join("_layouts");
        let includes_dir = tmp.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();
        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();

        let config = SiteConfig::default();
        let site_ctx = liquid::Object::new();
        let cached_site = CachedSiteContext::new(&site_ctx);

        let result = generate_collection_pages_cached(
            &[item],
            "tools",
            &config,
            &layout_engine,
            &cached_site,
            output_dir,
            &[],
        )
        .unwrap();

        assert_eq!(result.generated, 1);
        assert!(result.errors.is_empty());

        let output_path = output_dir.join("tools/modelstore.html");
        assert!(output_path.exists(), "Output file should exist");

        let content = fs::read_to_string(&output_path).unwrap();
        assert_eq!(
            content, "\n",
            "Empty-body item with no layout should produce a newline"
        );
        assert!(!content.is_empty(), "Output must not be 0 bytes");
    }

    #[test]
    fn test_nonempty_body_no_layout_produces_content() {
        // A collection item with no layout but non-empty body should produce the body content
        let item = CollectionItem {
            slug: "sometool".to_string(),
            url: "/tools/sometool.html".to_string(),
            date: None,
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("SomeTool".to_string()),
                );
                fm
            },
            content: "This is a tool.".to_string(),
            html_content: "<p>This is a tool.</p>\n".to_string(),
            excerpt: None,
            collection_name: "tools".to_string(),
            source_path: "_tools/sometool.md".to_string(),
            id: String::new(),
        };

        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path();

        let layouts_dir = tmp.path().join("_layouts");
        let includes_dir = tmp.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();
        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();

        let config = SiteConfig::default();
        let site_ctx = liquid::Object::new();
        let cached_site = CachedSiteContext::new(&site_ctx);

        let result = generate_collection_pages_cached(
            &[item],
            "tools",
            &config,
            &layout_engine,
            &cached_site,
            output_dir,
            &[],
        )
        .unwrap();

        assert_eq!(result.generated, 1);

        let output_path = output_dir.join("tools/sometool.html");
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(
            content.contains("<p>This is a tool.</p>"),
            "Non-empty body should be rendered as-is, got: {}",
            content
        );
    }

    #[test]
    fn test_empty_body_with_layout_renders_through_layout() {
        // A collection item with a layout but empty body should render through the layout
        // (not produce just a newline)
        let item = CollectionItem {
            slug: "emptytool".to_string(),
            url: "/tools/emptytool.html".to_string(),
            date: None,
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("EmptyTool".to_string()),
                );
                fm.insert(
                    "layout".to_string(),
                    serde_yaml::Value::String("tool".to_string()),
                );
                fm
            },
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            collection_name: "tools".to_string(),
            source_path: "_tools/emptytool.md".to_string(),
            id: String::new(),
        };

        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path();

        let layouts_dir = tmp.path().join("_layouts");
        let includes_dir = tmp.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        // Create a minimal "tool" layout
        fs::write(
            layouts_dir.join("tool.html"),
            "<html><body>{{ content }}</body></html>",
        )
        .unwrap();

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();

        let config = SiteConfig::default();
        let site_ctx = liquid::Object::new();
        let cached_site = CachedSiteContext::new(&site_ctx);

        let result = generate_collection_pages_cached(
            &[item],
            "tools",
            &config,
            &layout_engine,
            &cached_site,
            output_dir,
            &[],
        )
        .unwrap();

        assert_eq!(result.generated, 1);

        let output_path = output_dir.join("tools/emptytool.html");
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(
            content.contains("<html>"),
            "Item with layout should render through layout, got: {}",
            content
        );
    }
}
