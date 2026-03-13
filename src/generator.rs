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

use liquid::model::Value as LiquidValue;
use liquid::Object;
use rayon::prelude::*;

use crate::collection::CollectionItem;
use crate::config::SiteConfig;
use crate::data::DataTree;
use crate::template::context::{normalize_arrays, yaml_to_liquid};
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
pub fn build_site_context(
    config: &SiteConfig,
    collections: &HashMap<String, Vec<CollectionItem>>,
    data: &DataTree,
    site_dir: Option<&Path>,
) -> Object {
    let mut site = Object::new();

    // Basic site fields
    site.insert("url".into(), LiquidValue::scalar(config.url.clone()));
    site.insert("name".into(), LiquidValue::scalar(config.name.clone()));
    site.insert("title".into(), LiquidValue::scalar(config.title.clone()));

    // site.time -- current build time as a string matching event time format
    let now = chrono::Local::now();
    site.insert(
        "time".into(),
        LiquidValue::scalar(now.format("%Y-%m-%d %H:%M:%S").to_string()),
    );

    // site.<collection_name> for each collection
    for (name, items) in collections {
        let arr: Vec<LiquidValue> = items.iter().map(collection_item_to_liquid).collect();
        site.insert(
            name.clone().into(),
            normalize_arrays(LiquidValue::Array(arr)),
        );
    }

    // site.twitter
    if let Some(ref twitter) = config.twitter {
        site.insert("twitter".into(), LiquidValue::scalar(twitter.clone()));
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

/// Convert a `CollectionItem` to a Liquid `Value` (object).
///
/// Includes all front matter fields plus computed fields like `url`, `id`,
/// `content`, `date`, and `slug`. Also ensures `short` is set from slug
/// if not present in front matter (needed for people author lookup).
fn collection_item_to_liquid(item: &CollectionItem) -> LiquidValue {
    let mut obj = Object::new();

    // Copy all front matter fields, normalizing arrays so that objects
    // in arrays have uniform keys (prevents "Unknown index" in Liquid for loops)
    for (key, value) in &item.front_matter {
        obj.insert(key.clone().into(), normalize_arrays(yaml_to_liquid(value)));
    }

    // Add computed fields
    obj.insert("url".into(), LiquidValue::scalar(item.url.clone()));
    obj.insert("slug".into(), LiquidValue::scalar(item.slug.clone()));
    obj.insert(
        "id".into(),
        LiquidValue::scalar(format!(
            "/{}",
            if item.url.ends_with(".html") {
                item.url.trim_end_matches(".html").trim_start_matches('/')
            } else {
                item.url.trim_start_matches('/')
            }
        )),
    );

    if let Some(ref date) = item.date {
        obj.insert("date".into(), LiquidValue::scalar(date.clone()));
    }

    obj.insert(
        "content".into(),
        LiquidValue::scalar(item.html_content.clone()),
    );

    // Ensure "short" is set from slug if not in front matter
    // (needed for author lookup via `site.people | where: "short", name`)
    if !item.front_matter.contains_key("short") {
        obj.insert("short".into(), LiquidValue::scalar(item.slug.clone()));
    }

    LiquidValue::Object(obj)
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
    let collection_out_dir = output_dir.join(collection_type);
    fs::create_dir_all(&collection_out_dir).map_err(|e| GeneratorError::WriteFile {
        path: collection_out_dir.display().to_string(),
        source: e,
    })?;

    let result = Mutex::new(GenerationResult {
        generated: 0,
        skipped: 0,
        errors: Vec::new(),
    });

    items.par_iter().for_each(|item| {
        let layout_name = match resolve_layout(item, config, collection_type) {
            Some(name) => name,
            None => {
                result.lock().unwrap().skipped += 1;
                return;
            }
        };

        // Build page front matter with the url field added
        let mut page_fm = item.front_matter.clone();
        page_fm.insert("url".into(), serde_yaml::Value::String(item.url.clone()));

        // Also ensure date is in front matter if available (needed for posts)
        if !page_fm.contains_key("date") {
            if let Some(ref date) = item.date {
                page_fm.insert("date".to_string(), serde_yaml::Value::String(date.clone()));
            }
        }

        // Determine which content to render: raw content for posts (may contain Liquid),
        // html_content for collections that have already been converted from markdown
        let render_content = if item.collection_name == "posts" {
            &item.content
        } else {
            &item.html_content
        };

        match layout_engine.render_page(&layout_name, render_content, &page_fm, site_context) {
            Ok(html) => {
                // Compute output path: use URL-based path for posts, standard path for others
                let out_path = if item.url.starts_with("/blog/") {
                    let relative = item.url.trim_start_matches('/');
                    output_dir.join(relative)
                } else {
                    output_path(output_dir, collection_type, &item.slug)
                };

                if let Some(parent) = out_path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        result.lock().unwrap().errors.push(format!(
                            "Failed to create dir for {}/{}: {}",
                            collection_type, item.slug, e
                        ));
                        return;
                    }
                }

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
                result.lock().unwrap().errors.push(format!(
                    "Failed to render {}/{}: {}",
                    collection_type, item.slug, e
                ));
            }
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
        // Resolve layout from front matter
        let layout_name = match page
            .front_matter
            .get("layout")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            Some(name) => name.to_string(),
            None => {
                result.lock().unwrap().skipped += 1;
                return;
            }
        };

        // Build page front matter with url added
        let mut page_fm = page.front_matter.clone();
        page_fm.insert("url".into(), serde_yaml::Value::String(page.url.clone()));

        // Use raw content (not html_content) because pages may contain Liquid tags
        // that must be resolved before the layout wraps them.
        match layout_engine.render_page(&layout_name, &page.content, &page_fm, site_context) {
            Ok(html) => {
                // Compute output path from URL
                let relative = page.url.trim_start_matches('/');
                let out_path = output_dir.join(relative);

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
                result
                    .lock()
                    .unwrap()
                    .errors
                    .push(format!("Failed to render page {}: {}", page.slug, e));
            }
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
    let _ = config; // available for future use (e.g. default layout resolution)
    generate_pages(pages, layout_engine, site_context, output_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use liquid::model::ValueView;
    use std::path::PathBuf;

    fn site_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datatalksclub.github.io")
    }

    fn test_config() -> SiteConfig {
        SiteConfig::from_file(&site_dir().join("_config.yml")).unwrap()
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
        let ctx = build_site_context(&config, &collections, &data, None);

        let posts_val = ctx.get("posts").expect("site should have posts");
        if let LiquidValue::Array(arr) = posts_val {
            assert!(arr.len() >= 50, "Expected 50+ posts, got {}", arr.len());
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
        let ctx = build_site_context(&config, &collections, &data, None);

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
        let ctx = build_site_context(&config, &collections, &data, None);

        let books_val = ctx.get("books").expect("site should have books");
        if let LiquidValue::Array(arr) = books_val {
            assert!(arr.len() >= 90, "Expected 90+ books, got {}", arr.len());
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
        let ctx = build_site_context(&config, &collections, &data, None);

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
        let ctx = build_site_context(&config, &collections, &data, None);

        let data_val = ctx.get("data").expect("site should have data");
        if let LiquidValue::Object(data_obj) = data_val {
            let events = data_obj.get("events").expect("data should have events");
            if let LiquidValue::Array(arr) = events {
                assert!(arr.len() > 100, "Expected 100+ events, got {}", arr.len());
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
        let ctx = build_site_context(&config, &collections, &data, None);

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
    // Unit: GitHub repository URL resolution
    // ========================================================================

    #[test]
    fn test_resolve_repo_url_from_config() {
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            twitter: None,
            repository: Some("owner/repo".to_string()),
            permalink: "/:title.html".to_string(),
            exclude: vec![],
            collections: HashMap::new(),
            defaults: vec![],
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
            twitter: None,
            repository: None,
            permalink: "/:title.html".to_string(),
            exclude: vec![],
            collections: HashMap::new(),
            defaults: vec![],
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
            twitter: None,
            repository: None,
            permalink: "/:title.html".to_string(),
            exclude: vec![],
            collections: HashMap::new(),
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: String::new(),
                    type_name: "people".to_string(),
                },
                values: crate::config::DefaultValues {
                    layout: "author".to_string(),
                },
            }],
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
                    layout: "custom".to_string(),
                },
            }],
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
}
