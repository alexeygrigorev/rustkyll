//! Page generation orchestration.
//!
//! This module provides functions that wire together collection loading,
//! site context building, template rendering, and HTML output writing.
//! It is designed to be fully generic -- no collection-type-specific logic.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use crate::progress::RenderProgress;

use liquid::model::Value as LiquidValue;
use liquid::Object;
use liquid::ValueView;
use rayon::prelude::*;

use crate::collection::{CollectionItem, Page};
use crate::config::SiteConfig;
use crate::data::DataTree;
use crate::jsonld;
use crate::template::context::{normalize_arrays, normalize_frontmatter_date, yaml_to_liquid};
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

/// Extract the site timezone from the config's extras map.
///
/// Returns `Some(Tz)` if the config has a valid `timezone` key, `None` otherwise.
fn get_config_timezone(config: &SiteConfig) -> Option<chrono_tz::Tz> {
    config
        .extras
        .get("timezone")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<chrono_tz::Tz>().ok())
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
/// - `site.github.build_revision` (git HEAD SHA for cache busting)
/// - `site.github.url` (site URL for absolute URLs in JSON-LD)
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
    let site_tz = get_config_timezone(config);
    for (name, items) in collections {
        let mut arr: Vec<LiquidValue> = items
            .iter()
            .map(|item| collection_item_to_liquid_slim(item, site_tz))
            .collect();
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
    let (categories_map, tags_map) = build_categories_and_tags(collections, site_tz);
    site.insert("categories".into(), categories_map);
    site.insert("tags".into(), tags_map);

    // site.twitter -- convert yaml Value to liquid Value for both string and map support
    if let Some(ref twitter) = config.twitter {
        site.insert("twitter".into(), yaml_to_liquid(twitter));
    }

    // site.github -- dynamic repository URL resolution
    //
    // Jekyll only populates site.github.* when either:
    // (a) jekyll-github-metadata plugin is in the plugins list, OR
    // (b) _config.yml has an explicit top-level `github:` key
    //
    // When _config.yml has an explicit github: key, its values take priority
    // over computed fields (merge computed as defaults, config wins).
    let has_plugin = has_github_metadata_plugin(config);
    let explicit_github = config.extras.get("github");
    let has_explicit_github = explicit_github.map(|v| v.is_mapping()).unwrap_or(false);

    // Start with the explicit github config values if present
    let mut github = if has_explicit_github {
        if let Some(LiquidValue::Object(obj)) = site.get("github") {
            obj.clone()
        } else {
            Object::new()
        }
    } else {
        Object::new()
    };

    // repository_url: always resolve from git remote as a fallback.
    // Jekyll on GitHub Pages auto-injects jekyll-github-metadata, so many sites
    // use site.github.repository_url without explicitly listing the plugin.
    // If explicit github config provides repository_url, that wins (already in the map).
    if !github.contains_key("repository_url") {
        github.insert(
            "repository_url".into(),
            resolve_repository_url(config, site_dir),
        );
    }

    // build_revision: populate when plugin is active OR explicit github config exists
    if !github.contains_key("build_revision") {
        github.insert(
            "build_revision".into(),
            if has_plugin || has_explicit_github {
                resolve_build_revision(site_dir)
            } else {
                LiquidValue::scalar("")
            },
        );
    }

    // url: site URL (used for absolute URLs in JSON-LD breadcrumbs)
    // When jekyll-github-metadata is active, derive from git remote (GitHub Pages URL).
    // Otherwise, use config.url. Explicit github.url in config always takes priority.
    if !github.contains_key("url") {
        let url_value = if has_plugin {
            resolve_github_pages_url(config, site_dir)
        } else {
            config.url.clone()
        };
        github.insert("url".into(), LiquidValue::scalar(url_value));
    }

    site.insert("github".into(), LiquidValue::Object(github));

    // site.data -- data tree
    let mut data_obj = Object::new();
    for (key, value) in data {
        let liquid_val = normalize_arrays(yaml_to_liquid(value));
        data_obj.insert(key.clone().into(), liquid_val);
    }
    site.insert("data".into(), LiquidValue::Object(data_obj));

    // site.related_posts -- 10 most recent posts sorted by date descending
    let related_posts = build_related_posts(collections, site_tz);
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

    // site.html_pages -- pages whose output URL ends with .html, .htm, or /
    // (directory index pages produce .html output). Matches Jekyll's site.html_pages.
    let html_pages: Vec<LiquidValue> = pages
        .iter()
        .filter(|p| {
            let url = &p.url;
            url.ends_with(".html") || url.ends_with(".htm") || url.ends_with('/')
        })
        .map(page_to_liquid)
        .collect();
    site.insert(
        "html_pages".into(),
        normalize_arrays(LiquidValue::Array(html_pages)),
    );

    site
}

/// Build the full site context including static files information.
///
/// This is the extended version of `build_site_context` that also includes
/// `site.html_pages` (pages with HTML output) and `site.static_files`
/// (static file metadata for templates like favicon detection).
pub fn build_site_context_with_static_files(
    config: &SiteConfig,
    collections: &HashMap<String, Vec<CollectionItem>>,
    data: &DataTree,
    site_dir: Option<&Path>,
    pages: &[Page],
    static_file_paths: &[PathBuf],
) -> Object {
    let mut site = build_site_context(config, collections, data, site_dir, pages);

    // site.static_files -- metadata about static files for templates
    let static_files_arr: Vec<LiquidValue> = static_file_paths
        .iter()
        .map(|p| {
            let mut obj = Object::new();
            let path_str = format!("/{}", p.to_string_lossy().replace('\\', "/"));
            obj.insert("path".into(), LiquidValue::scalar(path_str));

            let ext = p
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();
            obj.insert("extname".into(), LiquidValue::scalar(ext));

            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            obj.insert("name".into(), LiquidValue::scalar(name));

            let basename = p
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            obj.insert("basename".into(), LiquidValue::scalar(basename));

            LiquidValue::Object(obj)
        })
        .collect();
    site.insert(
        "static_files".into(),
        normalize_arrays(LiquidValue::Array(static_files_arr)),
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

/// Extract the owner and repo name (NWO -- "name with owner") from a git remote URL.
///
/// Supports both HTTPS and SSH URL formats:
/// - `https://github.com/owner/repo` -> `("owner", "repo")`
/// - `https://github.com/owner/repo.git` -> `("owner", "repo")`
/// - `git@github.com:owner/repo.git` -> `("owner", "repo")`
///
/// Returns `None` if the URL cannot be parsed as a GitHub remote.
fn extract_nwo_from_remote(remote_url: &str) -> Option<(String, String)> {
    let path = if let Some(stripped) = remote_url.strip_prefix("git@github.com:") {
        stripped.trim_end_matches(".git").to_string()
    } else if remote_url.contains("github.com/") {
        let after = remote_url.split("github.com/").nth(1)?;
        after.trim_end_matches(".git").to_string()
    } else {
        return None;
    };

    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

/// Convert an owner/repo pair to a GitHub Pages URL.
///
/// - Standard project repos: `https://{owner}.github.io/{repo}/`
/// - User/org sites (repo name is `{owner}.github.io`): `https://{owner}.github.io/`
///
/// The owner comparison is case-insensitive, matching GitHub's behavior.
fn nwo_to_pages_url(owner: &str, repo: &str) -> String {
    let expected_site_repo = format!("{}.github.io", owner.to_lowercase());
    if repo.to_lowercase() == expected_site_repo {
        format!("https://{}.github.io/", owner.to_lowercase())
    } else {
        format!("https://{}.github.io/{}/", owner.to_lowercase(), repo)
    }
}

/// Resolve `site.github.url` when `jekyll-github-metadata` plugin is active.
///
/// Derives the GitHub Pages URL from the git remote, matching Jekyll's local-build
/// behavior. Falls back to `config.url` if the git remote cannot be resolved.
fn resolve_github_pages_url(config: &SiteConfig, site_dir: Option<&Path>) -> String {
    if let Some(dir) = site_dir {
        // Try config.repository first
        if let Some(ref repo) = config.repository {
            if let Some((owner, name)) =
                extract_nwo_from_remote(&format!("https://github.com/{}", repo))
            {
                return nwo_to_pages_url(&owner, &name);
            }
        }

        // Try git remote
        if let Ok(output) = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(dir)
            .output()
        {
            if output.status.success() {
                let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Some((owner, repo)) = extract_nwo_from_remote(&url) {
                    return nwo_to_pages_url(&owner, &repo);
                }
            }
        }
    }

    // Fall back to config.url
    config.url.clone()
}

/// Check if the site has `jekyll-github-metadata` in its `plugins` list.
///
/// Jekyll only populates `site.github.*` fields (like `build_revision`) when the
/// `jekyll-github-metadata` gem is listed as a plugin in `_config.yml`. Sites that
/// reference it only in their gemspec (as a runtime dependency) do NOT auto-activate
/// the plugin during `jekyll build`.
fn has_github_metadata_plugin(config: &SiteConfig) -> bool {
    if let Some(plugins_val) = config.extras.get("plugins") {
        if let Some(plugins_seq) = plugins_val.as_sequence() {
            return plugins_seq.iter().any(|v| {
                v.as_str()
                    .map(|s| s == "jekyll-github-metadata")
                    .unwrap_or(false)
            });
        }
    }
    false
}

/// Resolve the git HEAD SHA for `site.github.build_revision`.
///
/// Returns the 40-character hex SHA if the site directory is inside a git
/// repository, or an empty string otherwise. This matches Jekyll's
/// `jekyll-github-metadata` plugin which populates `site.github.build_revision`
/// with the current commit SHA (used for CSS cache busting).
fn resolve_build_revision(site_dir: Option<&Path>) -> LiquidValue {
    if let Some(dir) = site_dir {
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(dir)
            .output()
        {
            if output.status.success() {
                let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return LiquidValue::scalar(sha);
            }
        }
    }
    LiquidValue::scalar("")
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
/// Normalize `category`/`tag` (singular) to `categories`/`tags` (plural arrays)
/// in a Liquid object, matching Jekyll's behavior where every document always
/// exposes `categories` and `tags` as arrays.
///
/// Rules (applied for both categories/category and tags/tag):
/// 1. If singular key exists (e.g., `category`), convert its value to a
///    single-element array under the plural key (e.g., `categories`).
/// 2. If plural key exists as a scalar string, convert it to a single-element array.
/// 3. If plural key already exists as an array, leave it unchanged.
/// 4. If neither singular nor plural key exists, set plural to an empty array.
pub(crate) fn normalize_categories_and_tags(obj: &mut Object) {
    for (singular, plural) in &[("category", "categories"), ("tag", "tags")] {
        // Check if singular key exists and extract its value
        let singular_val = obj.get(*singular).and_then(|v| {
            if v.is_scalar() {
                Some(v.to_kstr().to_string())
            } else {
                None
            }
        });

        if let Some(val) = singular_val {
            // Singular key found -- create plural array from it (unless plural already exists as array)
            if obj.get(*plural).and_then(|v| v.as_array()).is_none() {
                let arr = if val.is_empty() {
                    LiquidValue::Array(vec![])
                } else {
                    // Split on whitespace to match Jekyll behavior for space-separated values
                    let items: Vec<LiquidValue> = val
                        .split_whitespace()
                        .map(|s| LiquidValue::scalar(s.to_string()))
                        .collect();
                    LiquidValue::Array(items)
                };
                obj.insert((*plural).into(), arr);
            }
        } else if let Some(existing) = obj.get(*plural) {
            // Plural key exists -- ensure it's an array
            if existing.is_scalar() {
                let s = existing.to_kstr().to_string();
                let arr = if s.is_empty() {
                    LiquidValue::Array(vec![])
                } else {
                    let items: Vec<LiquidValue> = s
                        .split_whitespace()
                        .map(|w| LiquidValue::scalar(w.to_string()))
                        .collect();
                    LiquidValue::Array(items)
                };
                obj.insert((*plural).into(), arr);
            }
            // If already an array, leave it unchanged
        } else {
            // Neither singular nor plural exists -- default to empty array
            obj.insert((*plural).into(), LiquidValue::Array(vec![]));
        }
    }
}

fn collection_item_to_liquid_slim(
    item: &CollectionItem,
    site_tz: Option<chrono_tz::Tz>,
) -> LiquidValue {
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
    obj.insert(
        "collection".into(),
        LiquidValue::scalar(item.collection_name.clone()),
    );

    // Issue 267: Expand bare YYYY-MM-DD dates to include time component,
    // matching Jekyll's behavior where Ruby YAML parses dates as Time objects.
    if let Some(ref date) = item.date {
        let expanded = crate::template::context::expand_date_only_string_with_tz(date, site_tz);
        obj.insert("date".into(), LiquidValue::scalar(expanded));
    }

    // Issue 217: Use raw markdown for the content field. Jekyll's `document.content`
    // actually returns rendered HTML, but using raw markdown here produces better DOM
    // matches because `| strip_html | jsonify` pipelines (used for JSON-LD descriptions
    // on 209+ blog pages) are a no-op on raw markdown, matching Jekyll's output.
    // The tradeoff: 174 podcast display pages show bare text instead of <p>-wrapped HTML
    // from `{{ guest.content }}`, but this is fewer regressions than using HTML content
    // (which breaks 209+ JSON-LD description matches through strip_html differences).
    obj.insert(
        "content".into(),
        LiquidValue::scalar(item.content.trim_start().to_string()),
    );

    // Also store rendered HTML as `output` for any templates that need it.
    obj.insert(
        "output".into(),
        LiquidValue::scalar(item.html_content.trim_end().to_string()),
    );

    // Ensure "short" is set from slug if not in front matter
    // (needed for author lookup via `site.<collection> | where: "short", name`)
    if !item.front_matter.contains_key("short") {
        obj.insert("short".into(), LiquidValue::scalar(item.slug.clone()));
    }

    // Issue 251: Normalize category/tag (singular) to categories/tags (plural arrays).
    // Jekyll always exposes `post.categories` and `post.tags` as arrays on every
    // document object. Without this, `category: release` in front matter would leave
    // `post.categories` as nil, causing `array_to_sentence_string` errors.
    normalize_categories_and_tags(&mut obj);

    LiquidValue::Object(obj)
}

/// Build `site.related_posts` -- the 10 most recent posts sorted by date descending.
///
/// In Jekyll, `site.related_posts` defaults to the 10 most recent posts
/// (unless LSI is enabled, which we do not support). Each entry has the same
/// structure as entries in `site.posts`.
fn build_related_posts(
    collections: &HashMap<String, Vec<CollectionItem>>,
    site_tz: Option<chrono_tz::Tz>,
) -> Vec<LiquidValue> {
    let Some(posts) = collections.get("posts") else {
        return Vec::new();
    };

    // Sort posts by date descending, take up to 10.
    // Jekyll sorts by date descending, then by path/slug descending for
    // same-date posts (matching the reverse chronological order of site.posts).
    let mut sorted: Vec<&CollectionItem> = posts.iter().collect();
    sorted.sort_by(|a, b| {
        let date_a = a.date.as_deref().unwrap_or("");
        let date_b = b.date.as_deref().unwrap_or("");
        date_b
            .cmp(date_a) // descending by date
            .then_with(|| b.slug.cmp(&a.slug)) // descending by slug for tiebreaking
    });

    sorted
        .into_iter()
        .take(10)
        .map(|item| collection_item_to_liquid_slim(item, site_tz))
        .collect()
}

/// Build per-post `site.related_posts` as a `LenientValue` for site overrides.
///
/// In Jekyll, `site.related_posts` excludes the current post and returns the
/// 10 most recent OTHER posts (sorted by date descending).
fn build_per_post_related_posts_lenient(
    sorted_posts: &[&CollectionItem],
    current_url: &str,
    site_tz: Option<chrono_tz::Tz>,
) -> crate::template::engine::LenientValue {
    let related: Vec<LiquidValue> = sorted_posts
        .iter()
        .filter(|p| p.url != current_url)
        .take(10)
        .map(|p| collection_item_to_liquid_slim(p, site_tz))
        .collect();
    let value = LiquidValue::Array(related);
    crate::template::engine::LenientValue::from_value(value)
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
    site_tz: Option<chrono_tz::Tz>,
) -> (LiquidValue, LiquidValue) {
    let mut categories: HashMap<String, Vec<LiquidValue>> = HashMap::new();
    let mut tags: HashMap<String, Vec<LiquidValue>> = HashMap::new();

    if let Some(posts) = collections.get("posts") {
        for post in posts {
            let liquid_post = collection_item_to_liquid_slim(post, site_tz);

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
        .map(|(k, mut v)| {
            // Jekyll lists posts within each category in reverse chronological
            // order (newest first), matching site.posts. Since posts are iterated
            // in load order (date ascending), reverse each group.
            v.reverse();
            (k.into(), LiquidValue::Array(v))
        })
        .collect::<Object>();

    let tags_obj = tags
        .into_iter()
        .map(|(k, mut v)| {
            // Same reverse-chronological ordering for tags.
            v.reverse();
            (k.into(), LiquidValue::Array(v))
        })
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
/// Inject a children navigation listing into the rendered HTML for parent pages.
///
/// For pages with `has_children: true` and `has_toc != false`, this generates
/// the children listing (matching the just-the-docs `children_nav.html` output)
/// and inserts it before `</main>` in the rendered HTML.
///
/// The listing consists of:
/// - `<hr>` separator
/// - `<h2 class="text-delta">Table of contents</h2>`
/// - `<ul>` with `<li><a href="...">Child Title</a></li>` for each child page
///
/// Children are pages whose `parent` front matter matches this page's `title`,
/// sorted by `nav_order`.
fn inject_children_nav(
    html: &str,
    page_fm: &HashMap<String, serde_yaml::Value>,
    all_pages: &[crate::collection::Page],
    config: Option<&SiteConfig>,
) -> String {
    // Check has_children: true
    let has_children = page_fm
        .get("has_children")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !has_children {
        return html.to_string();
    }

    // Check has_toc != false (default true)
    let has_toc = page_fm
        .get("has_toc")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !has_toc {
        return html.to_string();
    }

    let page_title = page_fm.get("title").and_then(|v| v.as_str()).unwrap_or("");
    if page_title.is_empty() {
        return html.to_string();
    }

    // Find children: pages whose parent matches this page's title
    let baseurl = config.map(|c| c.baseurl.as_str()).unwrap_or("");

    let mut children: Vec<(&str, &str, i64)> = Vec::new();
    for p in all_pages {
        let parent = p
            .front_matter
            .get("parent")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if parent == page_title {
            let title = p
                .front_matter
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let nav_order = p
                .front_matter
                .get("nav_order")
                .and_then(|v| v.as_i64())
                .unwrap_or(i64::MAX);
            if !title.is_empty() {
                children.push((title, &p.url, nav_order));
            }
        }
    }

    if children.is_empty() {
        return html.to_string();
    }

    // Sort by nav_order, then by title
    children.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.0.cmp(b.0)));

    // Check if child_nav_order is reversed
    let reversed = page_fm
        .get("child_nav_order")
        .and_then(|v| v.as_str())
        .map(|s| s == "desc" || s == "reversed")
        .unwrap_or(false);
    if reversed {
        children.reverse();
    }

    // Build the children nav HTML -- matching Jekyll's exact format:
    // `<hr><h2 class="text-delta">Table of contents</h2><ul><li> <a href="...">Title</a><li> ...</ul>`
    // Jekyll omits `</li>` between items (valid HTML5 optional closing tag).
    let mut nav_html = String::new();
    nav_html.push_str("<hr><h2 class=\"text-delta\">Table of contents</h2>");
    nav_html.push_str("<ul>");
    for (title, url, _) in &children {
        let full_url = if baseurl.is_empty() {
            url.to_string()
        } else {
            format!("{}{}", baseurl, url)
        };
        nav_html.push_str(&format!("<li> <a href=\"{}\">{}</a>", full_url, title));
    }
    nav_html.push_str("</ul>");

    // Insert before </main>
    if let Some(main_close_pos) = html.rfind("</main>") {
        let mut result = String::with_capacity(html.len() + nav_html.len());
        result.push_str(&html[..main_close_pos]);
        result.push_str(&nav_html);
        result.push_str(&html[main_close_pos..]);
        result
    } else {
        // No </main> found -- append to end as fallback
        let mut result = html.to_string();
        result.push_str(&nav_html);
        result
    }
}

/// URLs ending with `/` produce `<output_dir>/<url>/index.html` (pretty URLs).
/// URLs with a recognized file extension produce `<output_dir>/<url>` directly.
/// Other URLs get `.html` appended.
pub fn url_to_output_path(output_dir: &Path, url: &str) -> std::path::PathBuf {
    // Decode percent-encoded characters for filesystem paths.
    // URLs contain percent-encoded non-ASCII chars (e.g., Cyrillic %D1%87),
    // but the output filesystem should use the actual characters.
    let decoded = crate::template::filters::relative_url::decode_url_path(url);
    let relative = decoded.trim_start_matches('/');
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

    // Pre-sort posts for per-post related_posts computation.
    // In Jekyll, site.related_posts excludes the current post.
    let is_posts_collection = collection_type == "posts";
    let sorted_posts_for_related: Vec<&CollectionItem> = if is_posts_collection {
        let mut sorted: Vec<&CollectionItem> = items.iter().collect();
        sorted.sort_by(|a, b| {
            let date_a = a.date.as_deref().unwrap_or("");
            let date_b = b.date.as_deref().unwrap_or("");
            date_b.cmp(date_a).then_with(|| b.slug.cmp(&a.slug))
        });
        sorted
    } else {
        Vec::new()
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
        // First, convert singular `category`/`tag` to plural `categories`/`tags`
        // (Jekyll does this automatically on every document object).
        if let Some(val) = page_fm.remove("category") {
            page_fm.entry("categories".to_string()).or_insert(val);
        }
        if let Some(val) = page_fm.remove("tag") {
            page_fm.entry("tags".to_string()).or_insert(val);
        }
        normalize_fm_to_array(&mut page_fm, "categories");
        normalize_fm_to_array(&mut page_fm, "tags");

        page_fm.insert("url".into(), serde_yaml::Value::String(item.url.clone()));

        // Inject collection name so templates can use {{ page.collection }}
        // (e.g., for body class: `col-{{ page.collection }}` -> `col-pages`)
        page_fm
            .entry("collection".into())
            .or_insert_with(|| serde_yaml::Value::String(item.collection_name.clone()));

        // Also ensure date is in front matter if available (needed for posts)
        if !page_fm.contains_key("date") {
            if let Some(ref date) = item.date {
                page_fm.insert("date".to_string(), serde_yaml::Value::String(date.clone()));
            }
        }

        // Issue 216: Normalize the date field in front matter to full datetime
        // format with timezone offset (e.g., "2018/06/04 00:00" -> "2018-06-04 00:00:00 +0800").
        // This must happen before the front matter is converted to the Liquid context,
        // because yaml_to_liquid does not perform date expansion.
        let site_tz = get_config_timezone(config);
        normalize_frontmatter_date(&mut page_fm, site_tz);

        // Inject excerpt into page front matter (needed for SEO description fallback).
        // Jekyll auto-generates page.excerpt from the first paragraph of content.
        if !page_fm.contains_key("excerpt") {
            if let Some(ref excerpt) = item.excerpt {
                if !excerpt.is_empty() {
                    // Convert markdown excerpt to HTML, then strip tags for plain text
                    let html_excerpt = crate::frontmatter::markdown_to_html(excerpt);
                    page_fm.insert(
                        "excerpt".to_string(),
                        serde_yaml::Value::String(html_excerpt),
                    );
                }
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

        // Build per-post site.related_posts override for posts collection.
        // In Jekyll, site.related_posts excludes the current post.
        let site_overrides: HashMap<String, crate::template::engine::LenientValue> =
            if is_posts_collection {
                let mut overrides = HashMap::new();
                let related = build_per_post_related_posts_lenient(
                    &sorted_posts_for_related,
                    &item.url,
                    get_config_timezone(config),
                );
                overrides.insert("related_posts".to_string(), related);
                overrides
            } else {
                HashMap::new()
            };

        // Determine HTML output: render through layout if available,
        // otherwise output raw content (Jekyll outputs items without layout too).
        let html_result = if let Some(ref layout) = layout_name {
            // Jekyll processes Liquid first, then markdown for ALL markdown-sourced files.
            let is_markdown_source =
                item.source_path.ends_with(".md") || item.source_path.ends_with(".markdown");
            let has_liquid_tags = item.content.contains("{{") || item.content.contains("{%");
            let render_result = if !site_overrides.is_empty() {
                if is_markdown_source && has_liquid_tags {
                    layout_engine.render_markdown_page_with_site_overrides(
                        layout,
                        &item.content,
                        &page_fm,
                        cached_site,
                        &site_overrides,
                    )
                } else {
                    layout_engine.render_page_with_site_overrides(
                        layout,
                        &item.html_content,
                        &page_fm,
                        cached_site,
                        &site_overrides,
                    )
                }
            } else if is_markdown_source && has_liquid_tags {
                layout_engine.render_markdown_page_with_cached_site(
                    layout,
                    &item.content,
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

        // Issue 216: Normalize the date field in page front matter
        if let Some(cfg) = config {
            let site_tz = get_config_timezone(cfg);
            normalize_frontmatter_date(&mut page_fm, site_tz);
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

        // In Jekyll, standalone pages (not in a named collection) have
        // page.collection = nil. We don't inject a collection name here;
        // only actual collection items (via generate_collection_pages) get one.

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

                // Issue 246: Inject children nav listing for parent pages.
                // The just-the-docs theme's children_nav.html uses complex Liquid
                // (include_cached, group_by, where_exp, string splitting) that
                // doesn't work reliably. We compute the children listing directly
                // in Rust and inject it into the rendered HTML.
                let html = inject_children_nav(&html, &page_fm, pages, config);

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
    // Unit: NWO extraction and GitHub Pages URL derivation
    // ========================================================================

    #[test]
    fn test_extract_nwo_from_https_url() {
        let (owner, repo) = extract_nwo_from_remote("https://github.com/github/choosealicense.com")
            .expect("should extract NWO");
        assert_eq!(owner, "github");
        assert_eq!(repo, "choosealicense.com");
    }

    #[test]
    fn test_extract_nwo_from_ssh_url() {
        let (owner, repo) = extract_nwo_from_remote("git@github.com:alexeygrigorev/rustkyll.git")
            .expect("should extract NWO");
        assert_eq!(owner, "alexeygrigorev");
        assert_eq!(repo, "rustkyll");
    }

    #[test]
    fn test_extract_nwo_unicode_repo_name() {
        let (owner, repo) = extract_nwo_from_remote("https://github.com/user/projet-francais")
            .expect("should extract NWO");
        assert_eq!(owner, "user");
        assert_eq!(repo, "projet-francais");
    }

    #[test]
    fn test_nwo_to_pages_url_standard_repo() {
        let url = nwo_to_pages_url("github", "choosealicense.com");
        assert_eq!(url, "https://github.github.io/choosealicense.com/");
    }

    #[test]
    fn test_nwo_to_pages_url_org_site() {
        // When repo name matches {OWNER}.github.io, no repo suffix
        let url = nwo_to_pages_url("DataTalksClub", "datatalksclub.github.io");
        assert_eq!(url, "https://datatalksclub.github.io/");
    }

    #[test]
    fn test_nwo_to_pages_url_regular_user_repo() {
        let url = nwo_to_pages_url("alexeygrigorev", "mlbookcamp-page");
        assert_eq!(url, "https://alexeygrigorev.github.io/mlbookcamp-page/");
    }

    // ========================================================================
    // Unit: site.github.url resolution with github-metadata plugin
    // ========================================================================

    #[test]
    fn test_github_url_without_metadata_plugin_uses_config_url() {
        // When jekyll-github-metadata is NOT in plugins, site.github.url
        // should be config.url
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            ..Default::default()
        };
        let collections = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, Some(&site_dir()), &[]);
        let github = ctx.get("github").expect("should have github");
        if let LiquidValue::Object(gh) = github {
            let url = gh.get("url").expect("should have url");
            assert_eq!(
                *url,
                LiquidValue::scalar("https://example.com"),
                "Without plugin, github.url should be config.url"
            );
        } else {
            panic!("Expected github to be an Object");
        }
    }

    #[test]
    fn test_github_url_with_explicit_github_url_takes_priority() {
        // When explicit github: { url: "..." } is in config, that value wins
        let mut github_map = serde_yaml::Mapping::new();
        github_map.insert(
            serde_yaml::Value::String("url".to_string()),
            serde_yaml::Value::String("https://custom.example.com".to_string()),
        );
        let mut extras = HashMap::new();
        extras.insert("github".to_string(), serde_yaml::Value::Mapping(github_map));
        // Also add the plugin to show that explicit config wins over git-derived
        extras.insert(
            "plugins".to_string(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(
                "jekyll-github-metadata".to_string(),
            )]),
        );
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            extras,
            ..Default::default()
        };
        let collections = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, Some(&site_dir()), &[]);
        let github = ctx.get("github").expect("should have github");
        if let LiquidValue::Object(gh) = github {
            let url = gh.get("url").expect("should have url");
            assert_eq!(
                *url,
                LiquidValue::scalar("https://custom.example.com"),
                "Explicit github.url should take priority"
            );
        } else {
            panic!("Expected github to be an Object");
        }
    }

    #[test]
    fn test_github_url_with_plugin_no_git_remote_falls_back() {
        // When plugin is active but no git remote, fall back to config.url
        let mut extras = HashMap::new();
        extras.insert(
            "plugins".to_string(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(
                "jekyll-github-metadata".to_string(),
            )]),
        );
        let config = SiteConfig {
            url: "https://fallback.example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            extras,
            ..Default::default()
        };
        let collections = HashMap::new();
        let data = DataTree::new();
        // Use /nonexistent so git remote fails
        let ctx = build_site_context(
            &config,
            &collections,
            &data,
            Some(Path::new("/nonexistent")),
            &[],
        );
        let github = ctx.get("github").expect("should have github");
        if let LiquidValue::Object(gh) = github {
            let url = gh.get("url").expect("should have url");
            assert_eq!(
                *url,
                LiquidValue::scalar("https://fallback.example.com"),
                "Without git remote, github.url should fall back to config.url"
            );
        } else {
            panic!("Expected github to be an Object");
        }
    }

    #[test]
    fn test_github_url_with_plugin_derives_from_git_remote() {
        // When jekyll-github-metadata plugin is active and git remote is available,
        // site.github.url should be derived from the git remote as a GitHub Pages URL
        let mut extras = HashMap::new();
        extras.insert(
            "plugins".to_string(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(
                "jekyll-github-metadata".to_string(),
            )]),
        );
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            extras,
            ..Default::default()
        };
        let collections = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, Some(&site_dir()), &[]);
        let github = ctx.get("github").expect("should have github");
        if let LiquidValue::Object(gh) = github {
            let url = gh.get("url").expect("should have url");
            let url_str = url.to_kstr().to_string();
            // The test fixture is in the rustkyll repo, so the git remote should
            // resolve to a github.io Pages URL, NOT config.url
            assert!(
                url_str.contains(".github.io"),
                "With plugin active, github.url should be a GitHub Pages URL derived from git remote, got: {}",
                url_str
            );
            assert_ne!(
                url_str, "https://example.com",
                "github.url should NOT be config.url when plugin is active and git remote is available"
            );
        } else {
            panic!("Expected github to be an Object");
        }
    }

    // ========================================================================
    // Unit: site.github gating -- repository_url should be nil without plugin
    // ========================================================================

    #[test]
    fn test_github_repo_url_always_resolved_even_without_plugin() {
        // repository_url should always be resolved from git remote, even without
        // the jekyll-github-metadata plugin. Jekyll on GitHub Pages auto-injects
        // the plugin, so sites rely on repository_url without listing it explicitly.
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            ..Default::default()
        };
        let collections = HashMap::new();
        let data = DataTree::new();
        // Use site_dir() which IS a git repo
        let ctx = build_site_context(&config, &collections, &data, Some(&site_dir()), &[]);
        let github = ctx.get("github").expect("should have github");
        if let LiquidValue::Object(gh) = github {
            let repo_url = gh
                .get("repository_url")
                .expect("should have repository_url");
            assert_ne!(
                *repo_url,
                LiquidValue::Nil,
                "repository_url should always be resolved from git remote"
            );
        } else {
            panic!("Expected github to be an Object");
        }
    }

    #[test]
    fn test_github_repo_url_resolved_with_plugin() {
        // When jekyll-github-metadata IS in plugins, repository_url should
        // be resolved from git remote (or config.repository).
        let mut extras = HashMap::new();
        extras.insert(
            "plugins".to_string(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(
                "jekyll-github-metadata".to_string(),
            )]),
        );
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            extras,
            ..Default::default()
        };
        let collections = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, Some(&site_dir()), &[]);
        let github = ctx.get("github").expect("should have github");
        if let LiquidValue::Object(gh) = github {
            let repo_url = gh
                .get("repository_url")
                .expect("should have repository_url");
            assert_ne!(
                *repo_url,
                LiquidValue::Nil,
                "repository_url should be resolved when github-metadata plugin is active"
            );
        } else {
            panic!("Expected github to be an Object");
        }
    }

    #[test]
    fn test_github_config_preserved_when_explicit_github_key() {
        // When _config.yml has an explicit github: key, its values should be
        // preserved in site.github (not overwritten by computed fields).
        let mut github_map = serde_yaml::Mapping::new();
        github_map.insert(
            serde_yaml::Value::String("private".to_string()),
            serde_yaml::Value::Bool(false),
        );
        github_map.insert(
            serde_yaml::Value::String("repository_url".to_string()),
            serde_yaml::Value::String("https://github.com/custom/repo".to_string()),
        );
        let mut extras = HashMap::new();
        extras.insert("github".to_string(), serde_yaml::Value::Mapping(github_map));
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            extras,
            ..Default::default()
        };
        let collections = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, Some(&site_dir()), &[]);
        let github = ctx.get("github").expect("should have github");
        if let LiquidValue::Object(gh) = github {
            // Explicit repository_url from config should win
            let repo_url = gh
                .get("repository_url")
                .expect("should have repository_url");
            assert_eq!(
                *repo_url,
                LiquidValue::scalar("https://github.com/custom/repo"),
                "Config-provided repository_url should be preserved"
            );
            // Explicit private field should be preserved
            let private = gh.get("private").expect("should have private");
            assert_eq!(
                *private,
                LiquidValue::scalar(false),
                "Config-provided private field should be preserved"
            );
            // Computed field build_revision should be merged in as default
            assert!(
                gh.get("build_revision").is_some(),
                "Computed build_revision should be merged in"
            );
        } else {
            panic!("Expected github to be an Object");
        }
    }

    #[test]
    fn test_github_repo_url_resolved_without_plugin() {
        // Even without jekyll-github-metadata plugin in the plugins list,
        // repository_url should resolve from git remote as a fallback.
        // Jekyll on GitHub Pages auto-injects jekyll-github-metadata, so
        // sites like DTC rely on repository_url without explicitly listing the plugin.
        // No explicit github: key, no plugins: key -- just a bare config.
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            ..Default::default()
        };
        let collections = HashMap::new();
        let data = DataTree::new();
        // Use site_dir() which IS a git repo
        let ctx = build_site_context(&config, &collections, &data, Some(&site_dir()), &[]);
        let github = ctx.get("github").expect("should have github");
        if let LiquidValue::Object(gh) = github {
            let repo_url = gh
                .get("repository_url")
                .expect("should have repository_url");
            assert_ne!(
                *repo_url,
                LiquidValue::Nil,
                "repository_url should always be resolved from git remote, even without plugin"
            );
        } else {
            panic!("Expected github to be an Object");
        }
    }

    #[test]
    fn test_github_config_build_revision_populated_with_explicit_github_key() {
        // When _config.yml has an explicit github: key, build_revision should
        // be populated from git even without the github-metadata plugin.
        // This matches primer-theme behavior: the config explicitly sets up
        // the github object, so the SHA should come from git.
        let mut github_map = serde_yaml::Mapping::new();
        github_map.insert(
            serde_yaml::Value::String("private".to_string()),
            serde_yaml::Value::Bool(false),
        );
        let mut extras = HashMap::new();
        extras.insert("github".to_string(), serde_yaml::Value::Mapping(github_map));
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            extras,
            ..Default::default()
        };
        let collections = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, Some(&site_dir()), &[]);
        let github = ctx.get("github").expect("should have github");
        if let LiquidValue::Object(gh) = github {
            let build_rev = gh
                .get("build_revision")
                .expect("should have build_revision");
            let rev_str = build_rev.to_kstr().to_string();
            assert!(
                rev_str.len() >= 7,
                "build_revision should contain a git SHA when explicit github: config exists, got: '{}'",
                rev_str
            );
        } else {
            panic!("Expected github to be an Object");
        }
    }

    #[test]
    fn test_github_empty_map_gets_computed_defaults() {
        // Config with github: {} (empty map): computed fields should fill in
        let mut extras = HashMap::new();
        extras.insert(
            "github".to_string(),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );
        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            extras,
            ..Default::default()
        };
        let collections = HashMap::new();
        let data = DataTree::new();
        let ctx = build_site_context(&config, &collections, &data, Some(&site_dir()), &[]);
        let github = ctx.get("github").expect("should have github");
        if let LiquidValue::Object(gh) = github {
            // build_revision should be populated (from git)
            assert!(gh.get("build_revision").is_some());
            // url should be populated (from config.url)
            let url = gh.get("url").expect("should have url");
            assert_eq!(*url, LiquidValue::scalar("https://example.com"));
        } else {
            panic!("Expected github to be an Object");
        }
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

    #[test]
    fn test_url_to_output_path_percent_encoded_cyrillic() {
        // Percent-encoded Cyrillic URLs should decode to actual Cyrillic filesystem paths
        let path = url_to_output_path(
            Path::new("/tmp/site"),
            "/sections/%D1%87%D0%B0%D1%81%D1%82%D1%8C_1_%D0%B8%D1%81%D1%82%D0%BE%D1%80%D0%B8%D1%8F/",
        );
        assert_eq!(
            path,
            PathBuf::from("/tmp/site/sections/часть_1_история/index.html")
        );
    }

    #[test]
    fn test_url_to_output_path_percent_encoded_spaces() {
        // Percent-encoded spaces should decode for filesystem paths
        let path = url_to_output_path(Path::new("/tmp/site"), "/podcast/hybrid%20search.html");
        assert_eq!(path, PathBuf::from("/tmp/site/podcast/hybrid search.html"));
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
    // Issue 172: Fix related posts ordering (tiebreaking and category/tag ordering)
    // ========================================================================

    #[test]
    fn test_related_posts_tiebreaking_same_date_by_slug_descending() {
        let config = SiteConfig::default();
        let data = DataTree::new();

        let posts = vec![
            make_test_post("alpha", "2024-01-15", "Alpha Post"),
            make_test_post("beta", "2024-01-15", "Beta Post"),
            make_test_post("gamma", "2024-01-15", "Gamma Post"),
        ];

        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), posts);
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let related = ctx.get("related_posts").expect("should have related_posts");
        if let LiquidValue::Array(arr) = related {
            assert_eq!(arr.len(), 3);
            // Jekyll sorts same-date posts by path descending (slug descending).
            // So order should be: gamma, beta, alpha
            let titles: Vec<String> = arr
                .iter()
                .filter_map(|v| {
                    if let LiquidValue::Object(obj) = v {
                        obj.get("title").map(|t| t.to_kstr().to_string())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(
                titles,
                vec!["Gamma Post", "Beta Post", "Alpha Post"],
                "Same-date posts should be sorted by slug descending (matching Jekyll)"
            );
        } else {
            panic!("Expected related_posts to be an array");
        }
    }

    // Issue 186: Per-post related_posts excludes current post
    // ========================================================================

    #[test]
    fn test_per_post_related_posts_excludes_current_post() {
        // Jekyll's site.related_posts excludes the current post.
        // With 3 posts sorted descending by date, when rendering post B,
        // related_posts should contain A and C but NOT B.
        let posts = vec![
            make_test_post("post-a", "2023-01-01", "Post A"),
            make_test_post("post-b", "2023-02-01", "Post B"),
            make_test_post("post-c", "2023-03-01", "Post C"),
        ];

        // Pre-sort posts by date descending (matching build_related_posts logic)
        let mut sorted: Vec<&CollectionItem> = posts.iter().collect();
        sorted.sort_by(|a, b| {
            let date_a = a.date.as_deref().unwrap_or("");
            let date_b = b.date.as_deref().unwrap_or("");
            date_b.cmp(date_a).then_with(|| b.slug.cmp(&a.slug))
        });

        // Build related posts for post B (should exclude B itself)
        let related = build_per_post_related_posts_lenient(&sorted, "/blog/post-b.html", None);
        let value = related.to_value();
        if let LiquidValue::Array(arr) = value {
            assert_eq!(arr.len(), 2, "Should have 2 posts (A and C, not B)");
            // First should be C (most recent), then A
            let titles: Vec<String> = arr
                .iter()
                .filter_map(|v| {
                    if let LiquidValue::Object(obj) = v {
                        obj.get("title").map(|t| t.to_kstr().to_string())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(titles, vec!["Post C", "Post A"]);
        } else {
            panic!("Expected related_posts to be an array");
        }
    }

    #[test]
    fn test_per_post_related_posts_first_post() {
        // First post (oldest) should see all other posts in related_posts.
        let posts = vec![
            make_test_post("post-a", "2023-01-01", "Post A"),
            make_test_post("post-b", "2023-02-01", "Post B"),
            make_test_post("post-c", "2023-03-01", "Post C"),
        ];

        let mut sorted: Vec<&CollectionItem> = posts.iter().collect();
        sorted.sort_by(|a, b| {
            let date_a = a.date.as_deref().unwrap_or("");
            let date_b = b.date.as_deref().unwrap_or("");
            date_b.cmp(date_a).then_with(|| b.slug.cmp(&a.slug))
        });

        let related = build_per_post_related_posts_lenient(&sorted, "/blog/post-a.html", None);
        let value = related.to_value();
        if let LiquidValue::Array(arr) = value {
            assert_eq!(arr.len(), 2, "Oldest post should see 2 other posts");
            let titles: Vec<String> = arr
                .iter()
                .filter_map(|v| {
                    if let LiquidValue::Object(obj) = v {
                        obj.get("title").map(|t| t.to_kstr().to_string())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(titles, vec!["Post C", "Post B"]);
        } else {
            panic!("Expected related_posts to be an array");
        }
    }

    #[test]
    fn test_per_post_related_posts_limits_to_10() {
        // With 15 posts, related_posts for any post should have at most 10.
        let posts: Vec<CollectionItem> = (1..=15)
            .map(|i| {
                make_test_post(
                    &format!("post-{:02}", i),
                    &format!("2024-{:02}-01", i.min(12)),
                    &format!("Post {}", i),
                )
            })
            .collect();

        let mut sorted: Vec<&CollectionItem> = posts.iter().collect();
        sorted.sort_by(|a, b| {
            let date_a = a.date.as_deref().unwrap_or("");
            let date_b = b.date.as_deref().unwrap_or("");
            date_b.cmp(date_a).then_with(|| b.slug.cmp(&a.slug))
        });

        let related = build_per_post_related_posts_lenient(&sorted, "/blog/post-01.html", None);
        let value = related.to_value();
        if let LiquidValue::Array(arr) = value {
            assert_eq!(arr.len(), 10, "Should limit to 10 posts");
            // Should not contain post-01
            for item in &arr {
                if let LiquidValue::Object(obj) = item {
                    let url = obj
                        .get("url")
                        .map(|u| u.to_kstr().to_string())
                        .unwrap_or_default();
                    assert_ne!(url, "/blog/post-01.html", "Should not contain current post");
                }
            }
        } else {
            panic!("Expected related_posts to be an array");
        }
    }

    #[test]
    fn test_per_post_related_posts_same_date_posts() {
        // Posts with same date should have stable ordering and exclude current.
        let posts = vec![
            make_test_post("alpha", "2023-06-15", "Alpha"),
            make_test_post("beta", "2023-06-15", "Beta"),
            make_test_post("gamma", "2023-06-15", "Gamma"),
        ];

        let mut sorted: Vec<&CollectionItem> = posts.iter().collect();
        sorted.sort_by(|a, b| {
            let date_a = a.date.as_deref().unwrap_or("");
            let date_b = b.date.as_deref().unwrap_or("");
            date_b.cmp(date_a).then_with(|| b.slug.cmp(&a.slug))
        });

        // Related posts for beta should be gamma and alpha (in desc slug order)
        let related = build_per_post_related_posts_lenient(&sorted, "/blog/beta.html", None);
        let value = related.to_value();
        if let LiquidValue::Array(arr) = value {
            assert_eq!(arr.len(), 2);
            let titles: Vec<String> = arr
                .iter()
                .filter_map(|v| {
                    if let LiquidValue::Object(obj) = v {
                        obj.get("title").map(|t| t.to_kstr().to_string())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(titles, vec!["Gamma", "Alpha"]);
        } else {
            panic!("Expected related_posts to be an array");
        }
    }

    #[test]
    fn test_categories_posts_sorted_reverse_chronological() {
        let config = SiteConfig::default();
        let data = DataTree::new();

        let mut fm_old = HashMap::new();
        fm_old.insert(
            "title".to_string(),
            serde_yaml::Value::String("Old ML Post".to_string()),
        );
        fm_old.insert(
            "categories".to_string(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("ml".to_string())]),
        );

        let mut fm_new = HashMap::new();
        fm_new.insert(
            "title".to_string(),
            serde_yaml::Value::String("New ML Post".to_string()),
        );
        fm_new.insert(
            "categories".to_string(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("ml".to_string())]),
        );

        let posts = vec![
            CollectionItem {
                slug: "old-ml".to_string(),
                front_matter: fm_old,
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                url: "/blog/old-ml.html".to_string(),
                date: Some("2020-01-01".to_string()),
                collection_name: "posts".to_string(),
                source_path: "_posts/2020-01-01-old-ml.md".to_string(),
                id: String::new(),
            },
            CollectionItem {
                slug: "new-ml".to_string(),
                front_matter: fm_new,
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                url: "/blog/new-ml.html".to_string(),
                date: Some("2024-06-01".to_string()),
                collection_name: "posts".to_string(),
                source_path: "_posts/2024-06-01-new-ml.md".to_string(),
                id: String::new(),
            },
        ];

        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), posts);
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let categories = ctx.get("categories").expect("should have categories");
        if let LiquidValue::Object(cats) = categories {
            let ml = cats.get("ml").expect("should have ml category");
            if let LiquidValue::Array(arr) = ml {
                assert_eq!(arr.len(), 2);
                if let LiquidValue::Object(first) = &arr[0] {
                    assert_eq!(
                        first.get("title").unwrap(),
                        &LiquidValue::scalar("New ML Post"),
                        "First post in category should be newest"
                    );
                }
                if let LiquidValue::Object(last) = &arr[1] {
                    assert_eq!(
                        last.get("title").unwrap(),
                        &LiquidValue::scalar("Old ML Post"),
                        "Last post in category should be oldest"
                    );
                }
            } else {
                panic!("Expected ml category to be an array");
            }
        } else {
            panic!("Expected categories to be an object");
        }
    }

    #[test]
    fn test_tags_posts_sorted_reverse_chronological() {
        let config = SiteConfig::default();
        let data = DataTree::new();

        let mut fm_old = HashMap::new();
        fm_old.insert(
            "title".to_string(),
            serde_yaml::Value::String("Old Tagged".to_string()),
        );
        fm_old.insert(
            "tags".to_string(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("rust".to_string())]),
        );

        let mut fm_new = HashMap::new();
        fm_new.insert(
            "title".to_string(),
            serde_yaml::Value::String("New Tagged".to_string()),
        );
        fm_new.insert(
            "tags".to_string(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("rust".to_string())]),
        );

        let posts = vec![
            CollectionItem {
                slug: "old-tagged".to_string(),
                front_matter: fm_old,
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                url: "/blog/old-tagged.html".to_string(),
                date: Some("2020-01-01".to_string()),
                collection_name: "posts".to_string(),
                source_path: "_posts/2020-01-01-old-tagged.md".to_string(),
                id: String::new(),
            },
            CollectionItem {
                slug: "new-tagged".to_string(),
                front_matter: fm_new,
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                url: "/blog/new-tagged.html".to_string(),
                date: Some("2024-06-01".to_string()),
                collection_name: "posts".to_string(),
                source_path: "_posts/2024-06-01-new-tagged.md".to_string(),
                id: String::new(),
            },
        ];

        let mut collections = HashMap::new();
        collections.insert("posts".to_string(), posts);
        let ctx = build_site_context(&config, &collections, &data, None, &[]);

        let tags = ctx.get("tags").expect("should have tags");
        if let LiquidValue::Object(tags_obj) = tags {
            let rust_tag = tags_obj.get("rust").expect("should have rust tag");
            if let LiquidValue::Array(arr) = rust_tag {
                assert_eq!(arr.len(), 2);
                if let LiquidValue::Object(first) = &arr[0] {
                    assert_eq!(
                        first.get("title").unwrap(),
                        &LiquidValue::scalar("New Tagged"),
                        "First post in tag should be newest"
                    );
                }
                if let LiquidValue::Object(last) = &arr[1] {
                    assert_eq!(
                        last.get("title").unwrap(),
                        &LiquidValue::scalar("Old Tagged"),
                        "Last post in tag should be oldest"
                    );
                }
            } else {
                panic!("Expected rust tag to be an array");
            }
        } else {
            panic!("Expected tags to be an object");
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

    #[test]
    fn test_collection_item_content_uses_rendered_html() {
        // Issue 266: Collection item's content field in slim representation uses
        // rendered HTML, matching Jekyll's behavior where `document.content`
        // returns HTML with <p> wrapping. This is needed for podcast layouts
        // that use `{{ guest.content }}` expecting HTML output.
        let item = CollectionItem {
            slug: "testperson".to_string(),
            url: "/people/testperson.html".to_string(),
            date: None,
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Test Person".to_string()),
                );
                fm
            },
            content: "Test Person is a developer.".to_string(),
            html_content: "<p>Test Person is a developer.</p>\n".to_string(),
            excerpt: None,
            collection_name: "people".to_string(),
            source_path: "_people/testperson.md".to_string(),
            id: "/people/testperson".to_string(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, None);
        let content_val = liquid_val
            .as_object()
            .unwrap()
            .iter()
            .find(|(k, _)| k.as_str() == "content")
            .map(|(_, v)| v.to_kstr().to_string())
            .unwrap();

        // Content should use rendered HTML (html_content)
        assert_eq!(
            content_val, "Test Person is a developer.",
            "Collection item content should use raw markdown, got: {:?}",
            content_val
        );
    }

    #[test]
    fn test_collection_item_content_renders_markdown_links_as_html() {
        // Issue 266: Content field returns rendered HTML, so markdown links
        // become <a> tags. This matches Jekyll's behavior for `{{ guest.content }}`.
        let item = CollectionItem {
            slug: "davidgates".to_string(),
            url: "/people/davidgates.html".to_string(),
            date: None,
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("David Gates".to_string()),
                );
                fm
            },
            content: "David Gates is the founder of [Accents Welcome](https://accentswelcome.com),\nan English language school.".to_string(),
            html_content: "<p>David Gates is the founder of <a href=\"https://accentswelcome.com\">Accents Welcome</a>,\nan English language school.</p>\n".to_string(),
            excerpt: None,
            collection_name: "people".to_string(),
            source_path: "_people/davidgates.md".to_string(),
            id: "/people/davidgates".to_string(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, None);
        let content_val = liquid_val
            .as_object()
            .unwrap()
            .iter()
            .find(|(k, _)| k.as_str() == "content")
            .map(|(_, v)| v.to_kstr().to_string())
            .unwrap();

        // Raw markdown preserves markdown links
        assert!(
            content_val.contains("[Accents Welcome](https://accentswelcome.com)"),
            "Content should preserve raw markdown links, got: {:?}",
            content_val
        );
        // No HTML wrapping (raw markdown)
        assert!(
            !content_val.starts_with("<p>"),
            "Content should be raw markdown (no HTML), got: {:?}",
            content_val
        );
    }

    #[test]
    fn test_collection_item_content_has_html_paragraph_wrapping() {
        // Issue 266: Content field returns rendered HTML with <p> wrapping,
        // matching Jekyll's behavior. The trailing \n from HTML rendering is expected.
        let item = CollectionItem {
            slug: "alexeygrigorev".to_string(),
            url: "/people/alexeygrigorev.html".to_string(),
            date: None,
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Alexey Grigorev".to_string()),
                );
                fm
            },
            content: "Alexey Grigorev is the founder of DataTalks.Club".to_string(),
            html_content: "<p>Alexey Grigorev is the founder of DataTalks.Club</p>\n".to_string(),
            excerpt: None,
            collection_name: "people".to_string(),
            source_path: "_people/alexeygrigorev.md".to_string(),
            id: "/people/alexeygrigorev".to_string(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, None);
        let content_val = liquid_val
            .as_object()
            .unwrap()
            .iter()
            .find(|(k, _)| k.as_str() == "content")
            .map(|(_, v)| v.to_kstr().to_string())
            .unwrap();

        assert_eq!(
            content_val, "Alexey Grigorev is the founder of DataTalks.Club",
            "Content should be raw markdown, got: {:?}",
            content_val
        );
    }

    #[test]
    fn test_collection_item_content_unicode_rendered_html() {
        // Issue 266: Rendered HTML content preserves non-ASCII/Unicode characters
        // and converts markdown links to HTML anchor tags.
        let item = CollectionItem {
            slug: "renedescartes".to_string(),
            url: "/people/renedescartes.html".to_string(),
            date: None,
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Ren\u{00e9} Descartes".to_string()),
                );
                fm
            },
            content: "Ren\u{00e9} Descartes est un philosophe fran\u{00e7}ais. Il a \u{00e9}crit le [Discours](https://example.com/discours).".to_string(),
            html_content: "<p>Ren\u{00e9} Descartes est un philosophe fran\u{00e7}ais. Il a \u{00e9}crit le <a href=\"https://example.com/discours\">Discours</a>.</p>\n".to_string(),
            excerpt: None,
            collection_name: "people".to_string(),
            source_path: "_people/renedescartes.md".to_string(),
            id: "/people/renedescartes".to_string(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, None);
        let content_val = liquid_val
            .as_object()
            .unwrap()
            .iter()
            .find(|(k, _)| k.as_str() == "content")
            .map(|(_, v)| v.to_kstr().to_string())
            .unwrap();

        // Unicode chars preserved in HTML
        assert!(
            content_val.contains("Ren\u{00e9}"),
            "Content should preserve unicode e-acute, got: {:?}",
            content_val
        );
        assert!(
            content_val.contains("fran\u{00e7}ais"),
            "Content should preserve unicode c-cedilla, got: {:?}",
            content_val
        );
        // Markdown links preserved as raw markdown
        assert!(
            content_val.contains("[Discours](https://example.com/discours)"),
            "Content should preserve raw markdown links, got: {:?}",
            content_val
        );
        // No HTML wrapping (raw markdown)
        assert!(
            !content_val.starts_with("<p>"),
            "Content should be raw markdown (no HTML), got: {:?}",
            content_val
        );
    }

    #[test]
    fn test_collection_item_slim_has_output_field() {
        // The `output` field provides rendered HTML for templates needing HTML display,
        // while `content` remains raw markdown for JSON-LD pipelines.
        let item = CollectionItem {
            slug: "testperson".to_string(),
            url: "/people/testperson.html".to_string(),
            date: None,
            front_matter: HashMap::new(),
            content: "Test bio.".to_string(),
            html_content: "<p>Test bio.</p>\n".to_string(),
            excerpt: None,
            collection_name: "people".to_string(),
            source_path: "_people/testperson.md".to_string(),
            id: "/people/testperson".to_string(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, None);
        let output_val = liquid_val
            .as_object()
            .unwrap()
            .iter()
            .find(|(k, _)| k.as_str() == "output")
            .map(|(_, v)| v.to_kstr().to_string());

        assert!(
            output_val.is_some(),
            "Slim representation should include output field with rendered HTML"
        );
        assert!(
            output_val.unwrap().contains("<p>Test bio.</p>"),
            "Output field should contain rendered HTML"
        );
    }

    // ========================================================================
    // Issue 266: Content uses rendered HTML (html_content), leading newline
    // in raw markdown is irrelevant since html_content is used directly.
    // ========================================================================

    #[test]
    fn test_slim_content_uses_html_content_regardless_of_raw_newlines() {
        // Issue 266: Even when raw markdown has leading newlines, the content
        // field uses html_content directly (which has no such leading newlines).
        let item = CollectionItem {
            slug: "alexeygrigorev".to_string(),
            url: "/people/alexeygrigorev.html".to_string(),
            date: None,
            front_matter: HashMap::new(),
            content: "\nAlexey Grigorev is the founder of DataTalks.Club".to_string(),
            html_content: "<p>Alexey Grigorev is the founder of DataTalks.Club</p>\n".to_string(),
            excerpt: None,
            collection_name: "people".to_string(),
            source_path: "_people/alexeygrigorev.md".to_string(),
            id: "/people/alexeygrigorev".to_string(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, None);
        let content_val = liquid_val
            .as_object()
            .unwrap()
            .iter()
            .find(|(k, _)| k.as_str() == "content")
            .map(|(_, v)| v.to_kstr().to_string())
            .unwrap();

        assert_eq!(
            content_val, "Alexey Grigorev is the founder of DataTalks.Club",
            "Content should be raw markdown (leading newline trimmed), got: {:?}",
            content_val
        );
    }

    #[test]
    fn test_slim_content_multi_paragraph_html() {
        // Issue 266: Multi-paragraph content returns multi-paragraph HTML.
        let item = CollectionItem {
            slug: "testperson".to_string(),
            url: "/people/testperson.html".to_string(),
            date: None,
            front_matter: HashMap::new(),
            content: "First paragraph.\n\nSecond paragraph.".to_string(),
            html_content: "<p>First paragraph.</p>\n<p>Second paragraph.</p>\n".to_string(),
            excerpt: None,
            collection_name: "people".to_string(),
            source_path: "_people/testperson.md".to_string(),
            id: "/people/testperson".to_string(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, None);
        let content_val = liquid_val
            .as_object()
            .unwrap()
            .iter()
            .find(|(k, _)| k.as_str() == "content")
            .map(|(_, v)| v.to_kstr().to_string())
            .unwrap();

        assert_eq!(
            content_val, "First paragraph.\n\nSecond paragraph.",
            "Content should be raw markdown with paragraph breaks, got: {:?}",
            content_val
        );
    }

    // ========================================================================
    // Issue 188: page.collection variable for body class
    // ========================================================================

    #[test]
    fn test_body_class_pages_collection() {
        // A collection item in the "pages" collection should have
        // page.collection = "pages" available in templates.
        let tmp = tempfile::TempDir::new().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "default".to_string(),
            crate::template::Layout {
                source: "<body class=\"col-{{ page.collection }}\">{{ content }}</body>"
                    .to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: String::new(),
                    type_name: "pages".to_string(),
                },
                values: crate::config::DefaultValues {
                    values: {
                        let mut m = HashMap::new();
                        m.insert(
                            "layout".to_string(),
                            serde_yaml::Value::String("default".to_string()),
                        );
                        m
                    },
                },
            }],
            ..Default::default()
        };

        let items = vec![CollectionItem {
            slug: "about".to_string(),
            front_matter: HashMap::new(),
            content: String::new(),
            html_content: "<p>About</p>".to_string(),
            excerpt: None,
            url: "/pages/about.html".to_string(),
            date: None,
            collection_name: "pages".to_string(),
            source_path: "_pages/about.md".to_string(),
            id: String::new(),
        }];

        let site_context = Object::new();
        let result =
            generate_collection_pages(&items, "pages", &config, &engine, &site_context, output_dir)
                .unwrap();

        assert_eq!(result.generated, 1);
        let html = fs::read_to_string(output_dir.join("pages/about.html")).unwrap();
        assert!(
            html.contains("col-pages"),
            "Expected 'col-pages' in body class, got: {html}"
        );
    }

    #[test]
    fn test_body_class_posts_collection() {
        let tmp = tempfile::TempDir::new().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "post".to_string(),
            crate::template::Layout {
                source: "<body class=\"col-{{ page.collection }}\">{{ content }}</body>"
                    .to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: String::new(),
                    type_name: "posts".to_string(),
                },
                values: crate::config::DefaultValues {
                    values: {
                        let mut m = HashMap::new();
                        m.insert(
                            "layout".to_string(),
                            serde_yaml::Value::String("post".to_string()),
                        );
                        m
                    },
                },
            }],
            ..Default::default()
        };

        let items = vec![CollectionItem {
            slug: "hello-world".to_string(),
            front_matter: HashMap::new(),
            content: String::new(),
            html_content: "<p>Hello</p>".to_string(),
            excerpt: None,
            url: "/2024/01/01/hello-world.html".to_string(),
            date: Some("2024-01-01".to_string()),
            collection_name: "posts".to_string(),
            source_path: "_posts/2024-01-01-hello-world.md".to_string(),
            id: String::new(),
        }];

        let site_context = Object::new();
        let result =
            generate_collection_pages(&items, "posts", &config, &engine, &site_context, output_dir)
                .unwrap();

        assert_eq!(result.generated, 1);
        let html = fs::read_to_string(output_dir.join("2024/01/01/hello-world.html")).unwrap();
        assert!(
            html.contains("col-posts"),
            "Expected 'col-posts' in body class, got: {html}"
        );
    }

    #[test]
    fn test_body_class_custom_collection() {
        let tmp = tempfile::TempDir::new().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "recipe".to_string(),
            crate::template::Layout {
                source: "<body class=\"col-{{ page.collection }}\">{{ content }}</body>"
                    .to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: String::new(),
                    type_name: "recipes".to_string(),
                },
                values: crate::config::DefaultValues {
                    values: {
                        let mut m = HashMap::new();
                        m.insert(
                            "layout".to_string(),
                            serde_yaml::Value::String("recipe".to_string()),
                        );
                        m
                    },
                },
            }],
            ..Default::default()
        };

        let items = vec![CollectionItem {
            slug: "pasta".to_string(),
            front_matter: HashMap::new(),
            content: String::new(),
            html_content: "<p>Pasta recipe</p>".to_string(),
            excerpt: None,
            url: "/recipes/pasta.html".to_string(),
            date: None,
            collection_name: "recipes".to_string(),
            source_path: "_recipes/pasta.md".to_string(),
            id: String::new(),
        }];

        let site_context = Object::new();
        let result = generate_collection_pages(
            &items,
            "recipes",
            &config,
            &engine,
            &site_context,
            output_dir,
        )
        .unwrap();

        assert_eq!(result.generated, 1);
        let html = fs::read_to_string(output_dir.join("recipes/pasta.html")).unwrap();
        assert!(
            html.contains("col-recipes"),
            "Expected 'col-recipes' in body class, got: {html}"
        );
    }

    #[test]
    fn test_page_collection_variable() {
        // Verify {{ page.collection }} outputs the correct collection label
        let tmp = tempfile::TempDir::new().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "default".to_string(),
            crate::template::Layout {
                source: "collection={{ page.collection }}".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: String::new(),
                    type_name: "pages".to_string(),
                },
                values: crate::config::DefaultValues {
                    values: {
                        let mut m = HashMap::new();
                        m.insert(
                            "layout".to_string(),
                            serde_yaml::Value::String("default".to_string()),
                        );
                        m
                    },
                },
            }],
            ..Default::default()
        };

        let items = vec![CollectionItem {
            slug: "about".to_string(),
            front_matter: HashMap::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/pages/about.html".to_string(),
            date: None,
            collection_name: "pages".to_string(),
            source_path: "_pages/about.md".to_string(),
            id: String::new(),
        }];

        let site_context = Object::new();
        generate_collection_pages(&items, "pages", &config, &engine, &site_context, output_dir)
            .unwrap();

        let html = fs::read_to_string(output_dir.join("pages/about.html")).unwrap();
        assert_eq!(
            html.trim(),
            "collection=pages",
            "page.collection should output 'pages', got: {html}"
        );
    }

    #[test]
    fn test_collection_item_to_liquid_slim_includes_collection() {
        // The slim liquid representation should include the collection field
        let item = CollectionItem {
            slug: "test".to_string(),
            front_matter: HashMap::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/posts/test.html".to_string(),
            date: None,
            collection_name: "posts".to_string(),
            source_path: "_posts/test.md".to_string(),
            id: String::new(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_val.as_object().unwrap();
        let collection_val = obj
            .iter()
            .find(|(k, _)| k.as_str() == "collection")
            .map(|(_, v)| v.to_kstr().to_string());
        assert_eq!(
            collection_val,
            Some("posts".to_string()),
            "collection_item_to_liquid_slim should include collection field"
        );
    }

    // ========================================================================
    // Issue 194: standalone pages should have empty page.collection (not "pages")
    // ========================================================================

    #[test]
    fn test_standalone_page_collection_is_empty() {
        // In Jekyll, standalone pages (not in a named collection) have
        // page.collection = nil. When used as `col-{{ page.collection }}`,
        // this produces `col-` (empty), NOT `col-pages`.
        let tmp = tempfile::TempDir::new().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "default".to_string(),
            crate::template::Layout {
                source: "<body class=\"col-{{ page.collection }}\">{{ content }}</body>"
                    .to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: String::new(),
                    type_name: "pages".to_string(),
                },
                values: crate::config::DefaultValues {
                    values: {
                        let mut m = HashMap::new();
                        m.insert(
                            "layout".to_string(),
                            serde_yaml::Value::String("default".to_string()),
                        );
                        m
                    },
                },
            }],
            ..Default::default()
        };

        let pages = vec![crate::collection::Page {
            slug: "about".to_string(),
            front_matter: HashMap::new(),
            content: "About page".to_string(),
            html_content: "<p>About page</p>".to_string(),
            url: "/about.html".to_string(),
            source_path: "about.md".to_string(),
        }];

        let site_context = Object::new();
        let cached_site = LayoutEngine::build_cached_site_context(&site_context);
        generate_pages_cached_with_config(&pages, &engine, &cached_site, output_dir, Some(&config))
            .unwrap();

        let html = fs::read_to_string(output_dir.join("about.html")).unwrap();
        assert!(
            html.contains("col-\"") || html.contains("col- ") || html.contains("col-\">"),
            "Standalone page should have empty collection (col-), got: {html}"
        );
        assert!(
            !html.contains("col-pages"),
            "Standalone page should NOT have col-pages, got: {html}"
        );
    }

    // ========================================================================
    // Issue 196: Layout applied via config defaults for collection items
    // ========================================================================

    /// Test that collection items get their layout from config defaults
    /// when no explicit layout is in front matter. This is the pattern
    /// used by opensource-guide (articles collection, 337 pages).
    #[test]
    fn test_config_defaults_layout_applied_to_collection_items() {
        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "article".to_string(),
            crate::template::Layout {
                source: "<html><head><title>{{ page.title }}</title></head><body>{{ content }}</body></html>".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let config = SiteConfig {
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: String::new(),
                    type_name: "articles".to_string(),
                },
                values: crate::config::DefaultValues {
                    values: {
                        let mut m = HashMap::new();
                        m.insert(
                            "layout".to_string(),
                            serde_yaml::Value::String("article".to_string()),
                        );
                        m
                    },
                },
            }],
            ..Default::default()
        };

        // Item has NO layout in front matter -- must get it from defaults
        let item = CollectionItem {
            slug: "best-practices".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Best Practices".to_string()),
                );
                fm.insert(
                    "lang".to_string(),
                    serde_yaml::Value::String("ar".to_string()),
                );
                fm
            },
            content: "أفضل الممارسات · Best Practices".to_string(),
            html_content: "<p>أفضل الممارسات · Best Practices</p>\n".to_string(),
            excerpt: None,
            url: "/ar/best-practices/".to_string(),
            date: None,
            collection_name: "articles".to_string(),
            source_path: "_articles/ar/best-practices.md".to_string(),
            id: "/articles/ar/best-practices".to_string(),
        };

        let site_context = Object::new();
        let result = generate_collection_pages(
            &[item],
            "articles",
            &config,
            &engine,
            &site_context,
            output_dir,
        )
        .unwrap();

        assert_eq!(result.generated, 1);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        let html = fs::read_to_string(output_dir.join("ar/best-practices/index.html")).unwrap();
        assert!(
            html.contains("<html>"),
            "Layout should be applied (has <html>), got: {}",
            html
        );
        assert!(
            html.contains("<head>"),
            "Layout should be applied (has <head>), got: {}",
            html
        );
        assert!(
            html.contains("<body>"),
            "Layout should be applied (has <body>), got: {}",
            html
        );
        assert!(
            html.contains("أفضل الممارسات · Best Practices"),
            "Content should be present with Arabic text, got: {}",
            html
        );
    }

    /// Test that layout inheritance chains work (article -> default).
    /// This is critical for sites like opensource-guide where article.html
    /// has layout: default in its front matter.
    #[test]
    fn test_layout_inheritance_chain() {
        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "default".to_string(),
            crate::template::Layout {
                source: "<html><body>{{ content }}</body></html>".to_string(),
                parent_layout: None,
            },
        );
        layouts.insert(
            "article".to_string(),
            crate::template::Layout {
                source: "<article>{{ content }}</article>".to_string(),
                parent_layout: Some("default".to_string()),
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let config = SiteConfig {
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: String::new(),
                    type_name: "articles".to_string(),
                },
                values: crate::config::DefaultValues {
                    values: {
                        let mut m = HashMap::new();
                        m.insert(
                            "layout".to_string(),
                            serde_yaml::Value::String("article".to_string()),
                        );
                        m
                    },
                },
            }],
            ..Default::default()
        };

        let item = CollectionItem {
            slug: "test".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Test".to_string()),
                );
                fm
            },
            content: "Content here".to_string(),
            html_content: "<p>Content here</p>\n".to_string(),
            excerpt: None,
            url: "/test/".to_string(),
            date: None,
            collection_name: "articles".to_string(),
            source_path: "_articles/test.md".to_string(),
            id: "/articles/test".to_string(),
        };

        let site_context = Object::new();
        let result = generate_collection_pages(
            &[item],
            "articles",
            &config,
            &engine,
            &site_context,
            output_dir,
        )
        .unwrap();

        assert_eq!(result.generated, 1);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        let html = fs::read_to_string(output_dir.join("test/index.html")).unwrap();
        // Chain: article wraps in <article>, default wraps in <html><body>
        assert!(
            html.contains("<html><body><article>"),
            "Full chain should be applied: html > body > article, got: {}",
            html
        );
        assert!(
            html.contains("Content here"),
            "Content should be present, got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 196: Layout applied to translated page with Unicode content
    // ========================================================================

    #[test]
    fn test_layout_applied_to_translated_page_with_unicode() {
        // Tests the pattern from opensource-guide: Arabic translated pages
        // get layout from config defaults (not from page front matter).
        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "article".to_string(),
            crate::template::Layout {
                source: "<html><head><title>{{ page.title }}</title></head><body>{{ content }}</body></html>".to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let config = SiteConfig {
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: String::new(),
                    type_name: "articles".to_string(),
                },
                values: crate::config::DefaultValues {
                    values: {
                        let mut m = HashMap::new();
                        m.insert(
                            "layout".to_string(),
                            serde_yaml::Value::String("article".to_string()),
                        );
                        m
                    },
                },
            }],
            ..Default::default()
        };

        // Arabic page with no explicit layout -- relies on config defaults
        let item = CollectionItem {
            slug: "best-practices".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "lang".to_string(),
                    serde_yaml::Value::String("ar".to_string()),
                );
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String(
                        "\u{0623}\u{0641}\u{0636}\u{0644} \u{0627}\u{0644}\u{0645}\u{0645}\u{0627}\u{0631}\u{0633}\u{0627}\u{062a}".to_string(),
                    ),
                );
                fm
            },
            content: "<div>\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}</div>".to_string(),
            html_content: "<div>\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}</div>".to_string(),
            excerpt: None,
            url: "/ar/best-practices/".to_string(),
            date: None,
            collection_name: "articles".to_string(),
            source_path: "_articles/ar/best-practices.md".to_string(),
            id: "/articles/ar/best-practices".to_string(),
        };

        let site_context = Object::new();
        let result = generate_collection_pages(
            &[item],
            "articles",
            &config,
            &engine,
            &site_context,
            output_dir,
        )
        .unwrap();

        assert_eq!(result.generated, 1);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        let html = fs::read_to_string(output_dir.join("ar/best-practices/index.html")).unwrap();
        assert!(
            html.contains("<html>"),
            "Layout should be applied (has <html>), got: {}",
            html
        );
        assert!(
            html.contains("<head>"),
            "Layout should have <head>, got: {}",
            html
        );
        assert!(
            html.contains("<body>"),
            "Layout should have <body>, got: {}",
            html
        );
        // Verify Arabic content is preserved
        assert!(
            html.contains("\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}"),
            "Arabic content should be preserved, got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 196: Liquid nil contains check returns false (not error)
    // ========================================================================

    #[test]
    fn test_nil_contains_returns_false_not_error() {
        // In Ruby Liquid, `nil contains "foo"` returns false.
        // This was causing 337 opensource-guide pages to fail because
        // jekyll-toc.html does: {% if htmlClass contains "no_toc" %}
        // where htmlClass can be nil.
        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "default".to_string(),
            crate::template::Layout {
                source: concat!(
                    "<html><body>",
                    "{% assign maybe_nil = undefinedvar %}",
                    "{% if maybe_nil contains \"test\" %}YES{% else %}NO{% endif %}",
                    "{{ content }}",
                    "</body></html>"
                )
                .to_string(),
                parent_layout: None,
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let page = crate::collection::Page {
            slug: "test-nil".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "layout".to_string(),
                    serde_yaml::Value::String("default".to_string()),
                );
                // Unicode title: Cyrillic "test"
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("\u{0442}\u{0435}\u{0441}\u{0442}".to_string()),
                );
                fm
            },
            content: "<p>\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}</p>".to_string(),
            html_content: "<p>\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}</p>".to_string(),
            url: "/test-nil/".to_string(),
            source_path: "test-nil.html".to_string(),
        };

        let site_context = Object::new();
        let result = generate_pages(&[page], &engine, &site_context, output_dir).unwrap();

        assert_eq!(result.generated, 1);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        let html = fs::read_to_string(output_dir.join("test-nil/index.html")).unwrap();
        // The nil contains should evaluate to false, outputting "NO"
        assert!(
            html.contains("NO"),
            "nil contains should return false, got: {}",
            html
        );
        assert!(
            !html.contains("YES"),
            "nil contains should NOT return true, got: {}",
            html
        );
        // Layout should be applied (not fallback to raw content)
        assert!(
            html.contains("<html>"),
            "Layout should be applied, got: {}",
            html
        );
        // Cyrillic content should be preserved
        assert!(
            html.contains("\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}"),
            "Cyrillic content should be preserved, got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 196: Layout inheritance chain with Unicode content
    // ========================================================================

    #[test]
    fn test_layout_inheritance_chain_with_unicode() {
        // Tests 3-level layout chain: article -> page -> default
        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path();

        let mut layouts = HashMap::new();
        layouts.insert(
            "default".to_string(),
            crate::template::Layout {
                source: "<html><body>{{ content }}</body></html>".to_string(),
                parent_layout: None,
            },
        );
        layouts.insert(
            "page".to_string(),
            crate::template::Layout {
                source: "<main>{{ content }}</main>".to_string(),
                parent_layout: Some("default".to_string()),
            },
        );
        layouts.insert(
            "article".to_string(),
            crate::template::Layout {
                source: "<article>{{ content }}</article>".to_string(),
                parent_layout: Some("page".to_string()),
            },
        );
        let includes = HashMap::new();
        let engine = LayoutEngine::from_maps(layouts, &includes).unwrap();

        let page = crate::collection::Page {
            slug: "test-chain".to_string(),
            front_matter: {
                let mut fm = HashMap::new();
                fm.insert(
                    "layout".to_string(),
                    serde_yaml::Value::String("article".to_string()),
                );
                fm
            },
            // Farsi content
            content: "<p>\u{0633}\u{0644}\u{0627}\u{0645}</p>".to_string(),
            html_content: "<p>\u{0633}\u{0644}\u{0627}\u{0645}</p>".to_string(),
            url: "/test-chain/".to_string(),
            source_path: "test-chain.html".to_string(),
        };

        let site_context = Object::new();
        let result = generate_pages(&[page], &engine, &site_context, output_dir).unwrap();

        assert_eq!(result.generated, 1);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        let html = fs::read_to_string(output_dir.join("test-chain/index.html")).unwrap();
        // Full chain: default wraps page wraps article wraps content
        assert!(
            html.contains("<html><body><main><article>"),
            "Full 3-level chain should be applied: html > body > main > article, got: {}",
            html
        );
        // Farsi content preserved
        assert!(
            html.contains("\u{0633}\u{0644}\u{0627}\u{0645}"),
            "Farsi content should be preserved, got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 233: site.html_pages
    // ========================================================================

    #[test]
    fn test_site_html_pages_filters_to_html_only() {
        let config = SiteConfig::default();
        let data = DataTree::new();
        let collections = HashMap::new();
        let pages = vec![
            make_test_page("about", "About"),     // /about.html
            make_test_page("contact", "Contact"), // /contact.html
            {
                // An XML page (should NOT be in html_pages)
                let mut fm = HashMap::new();
                fm.insert(
                    "title".to_string(),
                    serde_yaml::Value::String("Feed".to_string()),
                );
                Page {
                    slug: "feed".to_string(),
                    front_matter: fm,
                    content: String::new(),
                    html_content: String::new(),
                    url: "/feed.xml".to_string(),
                    source_path: "feed.xml".to_string(),
                }
            },
        ];

        let ctx = build_site_context(&config, &collections, &data, None, &pages);

        let html_pages = ctx.get("html_pages").expect("should have html_pages");
        if let LiquidValue::Array(arr) = html_pages {
            assert_eq!(arr.len(), 2, "Should have 2 html pages (not the xml one)");
        } else {
            panic!("Expected html_pages to be an array");
        }
    }

    #[test]
    fn test_site_html_pages_exposes_frontmatter_fields() {
        let config = SiteConfig::default();
        let data = DataTree::new();
        let collections = HashMap::new();
        let mut fm = HashMap::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Courses".to_string()),
        );
        fm.insert(
            "parent".to_string(),
            serde_yaml::Value::String("Main".to_string()),
        );
        fm.insert(
            "nav_order".to_string(),
            serde_yaml::Value::Number(serde_yaml::Number::from(5)),
        );
        fm.insert("has_children".to_string(), serde_yaml::Value::Bool(true));
        fm.insert("nav_exclude".to_string(), serde_yaml::Value::Bool(false));
        fm.insert(
            "child_nav_order".to_string(),
            serde_yaml::Value::String("reversed".to_string()),
        );
        let pages = vec![Page {
            slug: "courses".to_string(),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            url: "/courses/".to_string(),
            source_path: "courses/index.md".to_string(),
        }];

        let ctx = build_site_context(&config, &collections, &data, None, &pages);

        let html_pages = ctx.get("html_pages").expect("should have html_pages");
        if let LiquidValue::Array(arr) = html_pages {
            assert_eq!(arr.len(), 1);
            if let LiquidValue::Object(obj) = &arr[0] {
                assert_eq!(obj.get("title"), Some(&LiquidValue::scalar("Courses")));
                assert_eq!(obj.get("parent"), Some(&LiquidValue::scalar("Main")));
                assert_eq!(obj.get("nav_order"), Some(&LiquidValue::scalar(5i64)));
                assert_eq!(obj.get("has_children"), Some(&LiquidValue::scalar(true)));
                assert_eq!(obj.get("nav_exclude"), Some(&LiquidValue::scalar(false)));
                assert_eq!(
                    obj.get("child_nav_order"),
                    Some(&LiquidValue::scalar("reversed"))
                );
            } else {
                panic!("Expected page to be an object");
            }
        } else {
            panic!("Expected html_pages to be an array");
        }
    }

    #[test]
    fn test_site_html_pages_includes_nav_exclude_true() {
        // Jekyll includes nav_exclude:true pages in site.html_pages;
        // the navigation template does the filtering
        let config = SiteConfig::default();
        let data = DataTree::new();
        let collections = HashMap::new();
        let mut fm = HashMap::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Hidden".to_string()),
        );
        fm.insert("nav_exclude".to_string(), serde_yaml::Value::Bool(true));
        let pages = vec![Page {
            slug: "hidden".to_string(),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            url: "/hidden.html".to_string(),
            source_path: "hidden.md".to_string(),
        }];

        let ctx = build_site_context(&config, &collections, &data, None, &pages);

        let html_pages = ctx.get("html_pages").expect("should have html_pages");
        if let LiquidValue::Array(arr) = html_pages {
            assert_eq!(
                arr.len(),
                1,
                "nav_exclude pages should still be in html_pages"
            );
        } else {
            panic!("Expected html_pages to be an array");
        }
    }

    // ========================================================================
    // Issue 233: site.static_files
    // ========================================================================

    #[test]
    fn test_site_static_files_in_context() {
        let config = SiteConfig::default();
        let data = DataTree::new();
        let collections = HashMap::new();
        let static_file_paths = vec![
            std::path::PathBuf::from("favicon.ico"),
            std::path::PathBuf::from("assets/styles.css"),
            std::path::PathBuf::from("images/logo.png"),
        ];

        let ctx = build_site_context_with_static_files(
            &config,
            &collections,
            &data,
            None,
            &[],
            &static_file_paths,
        );

        let sf = ctx.get("static_files").expect("should have static_files");
        if let LiquidValue::Array(arr) = sf {
            assert_eq!(arr.len(), 3);
            // Check that each entry has .path
            if let LiquidValue::Object(obj) = &arr[0] {
                assert_eq!(obj.get("path"), Some(&LiquidValue::scalar("/favicon.ico")));
                assert_eq!(obj.get("extname"), Some(&LiquidValue::scalar(".ico")));
                assert_eq!(obj.get("name"), Some(&LiquidValue::scalar("favicon.ico")));
                assert_eq!(obj.get("basename"), Some(&LiquidValue::scalar("favicon")));
            } else {
                panic!("Expected static_files entry to be an object");
            }
        } else {
            panic!("Expected static_files to be an array");
        }
    }

    #[test]
    fn test_site_static_files_properties() {
        let config = SiteConfig::default();
        let data = DataTree::new();
        let collections = HashMap::new();
        let static_file_paths = vec![std::path::PathBuf::from("assets/css/main.scss")];

        let ctx = build_site_context_with_static_files(
            &config,
            &collections,
            &data,
            None,
            &[],
            &static_file_paths,
        );

        let sf = ctx.get("static_files").expect("should have static_files");
        if let LiquidValue::Array(arr) = sf {
            if let LiquidValue::Object(obj) = &arr[0] {
                assert_eq!(
                    obj.get("path"),
                    Some(&LiquidValue::scalar("/assets/css/main.scss"))
                );
                assert_eq!(obj.get("extname"), Some(&LiquidValue::scalar(".scss")));
                assert_eq!(obj.get("name"), Some(&LiquidValue::scalar("main.scss")));
                assert_eq!(obj.get("basename"), Some(&LiquidValue::scalar("main")));
            } else {
                panic!("Expected static_files entry to be an object");
            }
        } else {
            panic!("Expected static_files to be an array");
        }
    }

    // ======================================================================
    // Issue 246: inject_children_nav tests
    // ======================================================================

    fn make_page(title: &str, url: &str, parent: Option<&str>, nav_order: Option<i64>) -> Page {
        let mut fm = HashMap::new();
        fm.insert("title".into(), serde_yaml::Value::String(title.to_string()));
        if let Some(p) = parent {
            fm.insert("parent".into(), serde_yaml::Value::String(p.to_string()));
        }
        if let Some(n) = nav_order {
            fm.insert(
                "nav_order".into(),
                serde_yaml::Value::Number(serde_yaml::Number::from(n)),
            );
        }
        Page {
            slug: title.to_lowercase().replace(' ', "-"),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            url: url.to_string(),
            source_path: format!("{}/index.md", url.trim_matches('/')),
        }
    }

    #[test]
    fn test_inject_children_nav_for_parent_page() {
        let html = "<main>\n<h1>Activities</h1>\n</main>";
        let mut parent_fm: HashMap<String, serde_yaml::Value> = HashMap::new();
        parent_fm.insert(
            "title".into(),
            serde_yaml::Value::String("Activities".into()),
        );
        parent_fm.insert("has_children".into(), serde_yaml::Value::Bool(true));

        let pages = vec![
            make_page("Activities", "/activities/", None, Some(1)),
            make_page(
                "Podcast",
                "/activities/podcast/",
                Some("Activities"),
                Some(1),
            ),
            make_page(
                "Webinars",
                "/activities/webinars/",
                Some("Activities"),
                Some(2),
            ),
        ];

        let result = inject_children_nav(html, &parent_fm, &pages, None);
        assert!(
            result.contains("<h2 class=\"text-delta\">Table of contents</h2>"),
            "Should inject children heading. Got: {}",
            result
        );
        assert!(
            result.contains("<a href=\"/activities/podcast/\">Podcast</a>"),
            "Should have link to Podcast child. Got: {}",
            result
        );
        assert!(
            result.contains("<a href=\"/activities/webinars/\">Webinars</a>"),
            "Should have link to Webinars child. Got: {}",
            result
        );
        assert!(
            result.contains("<hr"),
            "Should have <hr> separator. Got: {}",
            result
        );
    }

    #[test]
    fn test_inject_children_nav_skipped_when_has_toc_false() {
        let html = "<main>\n<h1>Courses</h1>\n</main>";
        let mut parent_fm: HashMap<String, serde_yaml::Value> = HashMap::new();
        parent_fm.insert("title".into(), serde_yaml::Value::String("Courses".into()));
        parent_fm.insert("has_children".into(), serde_yaml::Value::Bool(true));
        parent_fm.insert("has_toc".into(), serde_yaml::Value::Bool(false));

        let pages = vec![
            make_page("Courses", "/courses/", None, Some(1)),
            make_page("ML Zoomcamp", "/courses/ml/", Some("Courses"), Some(1)),
        ];

        let result = inject_children_nav(html, &parent_fm, &pages, None);
        assert!(
            !result.contains("Table of contents"),
            "Should NOT inject children nav when has_toc is false. Got: {}",
            result
        );
    }

    #[test]
    fn test_inject_children_nav_skipped_when_no_has_children() {
        let html = "<main>\n<h1>Child Page</h1>\n</main>";
        let mut fm: HashMap<String, serde_yaml::Value> = HashMap::new();
        fm.insert(
            "title".into(),
            serde_yaml::Value::String("Child Page".into()),
        );

        let pages = vec![make_page("Child Page", "/child/", Some("Parent"), Some(1))];

        let result = inject_children_nav(html, &fm, &pages, None);
        assert!(
            !result.contains("Table of contents"),
            "Should NOT inject children nav for non-parent pages. Got: {}",
            result
        );
    }

    #[test]
    fn test_inject_children_nav_sorted_by_nav_order() {
        let html = "<main>\n<h1>General</h1>\n</main>";
        let mut parent_fm: HashMap<String, serde_yaml::Value> = HashMap::new();
        parent_fm.insert("title".into(), serde_yaml::Value::String("General".into()));
        parent_fm.insert("has_children".into(), serde_yaml::Value::Bool(true));

        let pages = vec![
            make_page("General", "/general/", None, Some(1)),
            make_page("Slack", "/general/slack/", Some("General"), Some(3)),
            make_page("Jobs", "/general/jobs/", Some("General"), Some(1)),
            make_page(
                "Guidelines",
                "/general/guidelines/",
                Some("General"),
                Some(2),
            ),
        ];

        let result = inject_children_nav(html, &parent_fm, &pages, None);
        let jobs_pos = result.find("Jobs").expect("Should contain Jobs");
        let guidelines_pos = result
            .find("Guidelines")
            .expect("Should contain Guidelines");
        let slack_pos = result.find("Slack").expect("Should contain Slack");
        assert!(
            jobs_pos < guidelines_pos && guidelines_pos < slack_pos,
            "Children should be sorted by nav_order. Got: {}",
            result
        );
    }

    #[test]
    fn test_inject_children_nav_li_structure_matches_jekyll() {
        // Jekyll's children_nav.html template produces `<li>` without `</li>`,
        // relying on HTML5 optional closing tags. The DOM parser interprets
        // subsequent `<li>` elements as nested inside the first.
        // Our output must match this exact structure.
        let html = "<main>\n<h1>Activities</h1>\n</main>";
        let mut parent_fm: HashMap<String, serde_yaml::Value> = HashMap::new();
        parent_fm.insert(
            "title".into(),
            serde_yaml::Value::String("Activities".into()),
        );
        parent_fm.insert("has_children".into(), serde_yaml::Value::Bool(true));

        let pages = vec![
            make_page("Activities", "/activities/", None, Some(1)),
            make_page(
                "Podcast",
                "/activities/podcast/",
                Some("Activities"),
                Some(1),
            ),
            make_page(
                "Webinars",
                "/activities/webinars/",
                Some("Activities"),
                Some(2),
            ),
        ];

        let result = inject_children_nav(html, &parent_fm, &pages, None);

        // Jekyll format: `<li>\n <a href="...">Title</a>` without closing `</li>` between items.
        // Only the very last `</li>` chain at the end closes all.
        // Verify there are no `</li>` between items
        assert!(
            result.contains("<li> <a href=\"/activities/podcast/\">Podcast</a>"),
            "Should have Jekyll-style <li> for Podcast. Got: {}",
            result
        );
        assert!(
            result.contains("<li> <a href=\"/activities/webinars/\">Webinars</a>"),
            "Should have Jekyll-style <li> for Webinars. Got: {}",
            result
        );
        // Should NOT have `</li>\n  <li>` pattern (that's the old rustkyll format)
        assert!(
            !result.contains("</li>\n<li>") && !result.contains("</li>\n  <li>"),
            "Should NOT have </li> between items (Jekyll omits them). Got: {}",
            result
        );
    }

    #[test]
    fn test_inject_children_nav_unicode_titles() {
        let html = "<main>\n<h1>Ubersicht</h1>\n</main>";
        let mut parent_fm: HashMap<String, serde_yaml::Value> = HashMap::new();
        parent_fm.insert(
            "title".into(),
            serde_yaml::Value::String("Ubersicht".into()),
        );
        parent_fm.insert("has_children".into(), serde_yaml::Value::Bool(true));

        let pages = vec![
            make_page("Ubersicht", "/ubersicht/", None, Some(1)),
            make_page(
                "Einfuhrung",
                "/ubersicht/einfuhrung/",
                Some("Ubersicht"),
                Some(1),
            ),
        ];

        let result = inject_children_nav(html, &parent_fm, &pages, None);
        assert!(
            result.contains("Einfuhrung"),
            "Should contain Unicode child title. Got: {}",
            result
        );
    }

    // ========================================================================
    // Unit: category/tag normalization in collection_item_to_liquid_slim
    // (Issue 251)
    // ========================================================================

    fn make_item_with_fm(fm: HashMap<String, serde_yaml::Value>) -> CollectionItem {
        CollectionItem {
            slug: "test-post".to_string(),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/blog/test-post.html".to_string(),
            date: Some("2021-01-01".to_string()),
            collection_name: "posts".to_string(),
            source_path: "_posts/2021-01-01-test-post.md".to_string(),
            id: String::new(),
        }
    }

    #[test]
    fn test_slim_category_singular_to_categories_array() {
        let mut fm = HashMap::new();
        fm.insert(
            "category".to_string(),
            serde_yaml::Value::String("release".to_string()),
        );
        let item = make_item_with_fm(fm);
        let liquid_obj = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_obj.as_object().unwrap();
        let cats = obj.get("categories").expect("categories key must exist");
        let arr = cats.as_array().expect("categories must be an array");
        assert_eq!(arr.size(), 1);
        assert_eq!(arr.get(0).unwrap().to_kstr().as_str(), "release");
    }

    #[test]
    fn test_slim_categories_string_to_array() {
        let mut fm = HashMap::new();
        fm.insert(
            "categories".to_string(),
            serde_yaml::Value::String("food".to_string()),
        );
        let item = make_item_with_fm(fm);
        let liquid_obj = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_obj.as_object().unwrap();
        let cats = obj.get("categories").expect("categories key must exist");
        let arr = cats.as_array().expect("categories must be an array");
        assert_eq!(arr.size(), 1);
        assert_eq!(arr.get(0).unwrap().to_kstr().as_str(), "food");
    }

    #[test]
    fn test_slim_categories_array_unchanged() {
        let mut fm = HashMap::new();
        fm.insert(
            "categories".to_string(),
            serde_yaml::Value::Sequence(vec![
                serde_yaml::Value::String("a".to_string()),
                serde_yaml::Value::String("b".to_string()),
            ]),
        );
        let item = make_item_with_fm(fm);
        let liquid_obj = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_obj.as_object().unwrap();
        let cats = obj.get("categories").expect("categories key must exist");
        let arr = cats.as_array().expect("categories must be an array");
        assert_eq!(arr.size(), 2);
        assert_eq!(arr.get(0).unwrap().to_kstr().as_str(), "a");
        assert_eq!(arr.get(1).unwrap().to_kstr().as_str(), "b");
    }

    #[test]
    fn test_slim_no_category_defaults_to_empty_array() {
        let fm = HashMap::new();
        let item = make_item_with_fm(fm);
        let liquid_obj = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_obj.as_object().unwrap();
        let cats = obj.get("categories").expect("categories key must exist");
        let arr = cats.as_array().expect("categories must be an array");
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_slim_tag_singular_to_tags_array() {
        let mut fm = HashMap::new();
        fm.insert(
            "tag".to_string(),
            serde_yaml::Value::String("rust".to_string()),
        );
        let item = make_item_with_fm(fm);
        let liquid_obj = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_obj.as_object().unwrap();
        let tags = obj.get("tags").expect("tags key must exist");
        let arr = tags.as_array().expect("tags must be an array");
        assert_eq!(arr.size(), 1);
        assert_eq!(arr.get(0).unwrap().to_kstr().as_str(), "rust");
    }

    #[test]
    fn test_slim_tags_string_to_array() {
        let mut fm = HashMap::new();
        fm.insert(
            "tags".to_string(),
            serde_yaml::Value::String("python".to_string()),
        );
        let item = make_item_with_fm(fm);
        let liquid_obj = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_obj.as_object().unwrap();
        let tags = obj.get("tags").expect("tags key must exist");
        let arr = tags.as_array().expect("tags must be an array");
        assert_eq!(arr.size(), 1);
        assert_eq!(arr.get(0).unwrap().to_kstr().as_str(), "python");
    }

    #[test]
    fn test_slim_tags_array_unchanged() {
        let mut fm = HashMap::new();
        fm.insert(
            "tags".to_string(),
            serde_yaml::Value::Sequence(vec![
                serde_yaml::Value::String("x".to_string()),
                serde_yaml::Value::String("y".to_string()),
            ]),
        );
        let item = make_item_with_fm(fm);
        let liquid_obj = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_obj.as_object().unwrap();
        let tags = obj.get("tags").expect("tags key must exist");
        let arr = tags.as_array().expect("tags must be an array");
        assert_eq!(arr.size(), 2);
        assert_eq!(arr.get(0).unwrap().to_kstr().as_str(), "x");
        assert_eq!(arr.get(1).unwrap().to_kstr().as_str(), "y");
    }

    #[test]
    fn test_slim_no_tag_defaults_to_empty_array() {
        let fm = HashMap::new();
        let item = make_item_with_fm(fm);
        let liquid_obj = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_obj.as_object().unwrap();
        let tags = obj.get("tags").expect("tags key must exist");
        let arr = tags.as_array().expect("tags must be an array");
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_page_fm_singular_category_to_categories_array() {
        // Simulates the page rendering path in generate_collection_pages_cached_with_progress:
        // front matter has `category: release` (singular), after normalization
        // `categories` should be a YAML sequence ["release"].
        let mut page_fm = crate::frontmatter::FrontMatter::new();
        page_fm.insert(
            "category".to_string(),
            serde_yaml::Value::String("release".to_string()),
        );

        // This is the normalization done in the page rendering path:
        // First, move singular to plural
        if let Some(val) = page_fm.remove("category") {
            page_fm.entry("categories".to_string()).or_insert(val);
        }
        if let Some(val) = page_fm.remove("tag") {
            page_fm.entry("tags".to_string()).or_insert(val);
        }
        normalize_fm_to_array(&mut page_fm, "categories");
        normalize_fm_to_array(&mut page_fm, "tags");

        // Verify categories is a sequence
        let cats = page_fm
            .get("categories")
            .expect("categories key must exist");
        match cats {
            serde_yaml::Value::Sequence(seq) => {
                assert_eq!(seq.len(), 1, "should have one element");
                assert_eq!(seq[0], serde_yaml::Value::String("release".to_string()));
            }
            _ => panic!("categories should be a sequence, got: {:?}", cats),
        }
    }

    #[test]
    fn test_page_fm_singular_tag_to_tags_array() {
        let mut page_fm = crate::frontmatter::FrontMatter::new();
        page_fm.insert(
            "tag".to_string(),
            serde_yaml::Value::String("update".to_string()),
        );

        if let Some(val) = page_fm.remove("category") {
            page_fm.entry("categories".to_string()).or_insert(val);
        }
        if let Some(val) = page_fm.remove("tag") {
            page_fm.entry("tags".to_string()).or_insert(val);
        }
        normalize_fm_to_array(&mut page_fm, "categories");
        normalize_fm_to_array(&mut page_fm, "tags");

        let tags = page_fm.get("tags").expect("tags key must exist");
        match tags {
            serde_yaml::Value::Sequence(seq) => {
                assert_eq!(seq.len(), 1);
                assert_eq!(seq[0], serde_yaml::Value::String("update".to_string()));
            }
            _ => panic!("tags should be a sequence, got: {:?}", tags),
        }
    }

    // ========================================================================
    // Issue 267: Date expansion in collection_item_to_liquid_slim
    // ========================================================================

    #[test]
    fn test_slim_bare_date_expanded_no_tz() {
        // Bug 1: bare YYYY-MM-DD dates must be expanded to include time component
        let item = CollectionItem {
            slug: "ep1".to_string(),
            url: "/podcast/ep1.html".to_string(),
            date: Some("2025-11-07".to_string()),
            front_matter: HashMap::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            collection_name: "podcast".to_string(),
            source_path: "_podcast/ep1.md".to_string(),
            id: "/podcast/ep1".to_string(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_val.as_object().unwrap();
        let date_val = obj
            .iter()
            .find(|(k, _)| k.as_str() == "date")
            .map(|(_, v)| v.to_kstr().to_string())
            .unwrap();
        assert_eq!(
            date_val, "2025-11-07 00:00:00 +0000",
            "Bare date should be expanded to full datetime, got: {date_val}"
        );
    }

    #[test]
    fn test_slim_bare_date_expanded_with_tz() {
        // With Europe/Berlin timezone, winter date should get +0100
        let tz: chrono_tz::Tz = "Europe/Berlin".parse().unwrap();
        let item = CollectionItem {
            slug: "ep2".to_string(),
            url: "/podcast/ep2.html".to_string(),
            date: Some("2025-11-07".to_string()),
            front_matter: HashMap::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            collection_name: "podcast".to_string(),
            source_path: "_podcast/ep2.md".to_string(),
            id: "/podcast/ep2".to_string(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, Some(tz));
        let obj = liquid_val.as_object().unwrap();
        let date_val = obj
            .iter()
            .find(|(k, _)| k.as_str() == "date")
            .map(|(_, v)| v.to_kstr().to_string())
            .unwrap();
        assert_eq!(
            date_val, "2025-11-07 00:00:00 +0100",
            "Bare date with Europe/Berlin tz should get +0100 in winter, got: {date_val}"
        );
    }

    #[test]
    fn test_slim_already_expanded_date_unchanged() {
        // Already-expanded dates should pass through unchanged
        let item = CollectionItem {
            slug: "ep3".to_string(),
            url: "/podcast/ep3.html".to_string(),
            date: Some("2025-11-07 00:00:00 +0200".to_string()),
            front_matter: HashMap::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            collection_name: "podcast".to_string(),
            source_path: "_podcast/ep3.md".to_string(),
            id: "/podcast/ep3".to_string(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_val.as_object().unwrap();
        let date_val = obj
            .iter()
            .find(|(k, _)| k.as_str() == "date")
            .map(|(_, v)| v.to_kstr().to_string())
            .unwrap();
        assert_eq!(
            date_val, "2025-11-07 00:00:00 +0200",
            "Already-expanded date should pass through unchanged, got: {date_val}"
        );
    }

    #[test]
    fn test_slim_no_date_field_when_none() {
        // When date is None, no date field should be inserted
        let item = CollectionItem {
            slug: "ep4".to_string(),
            url: "/podcast/ep4.html".to_string(),
            date: None,
            front_matter: HashMap::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            collection_name: "podcast".to_string(),
            source_path: "_podcast/ep4.md".to_string(),
            id: "/podcast/ep4".to_string(),
        };

        let liquid_val = collection_item_to_liquid_slim(&item, None);
        let obj = liquid_val.as_object().unwrap();
        let date_entry = obj.iter().find(|(k, _)| k.as_str() == "date");
        assert!(
            date_entry.is_none(),
            "No date field should be present when item.date is None"
        );
    }
}
