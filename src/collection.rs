use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::config::SiteConfig;
use crate::frontmatter::{self, FrontMatter};

/// Errors that can occur when loading collections.
#[derive(Debug, thiserror::Error)]
pub enum CollectionError {
    #[error("failed to read directory {path}: {source}")]
    ReadDir {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to read file {path}: {source}")]
    ReadFile {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to parse file {path}: {source}")]
    Parse {
        path: String,
        source: frontmatter::ParseError,
    },
}

/// A single item from a Jekyll collection.
#[derive(Debug, Clone)]
pub struct CollectionItem {
    /// Filename stem (e.g. `john-doe` from `john-doe.md`,
    /// `segmentation` from `2020-11-29-segmentation.md` for posts).
    pub slug: String,

    /// Parsed YAML front matter key-value pairs.
    pub front_matter: FrontMatter,

    /// Raw markdown body.
    pub content: String,

    /// Markdown body converted to HTML.
    pub html_content: String,

    /// Content before `<!--more-->` separator, if present.
    pub excerpt: Option<String>,

    /// Generated URL path (e.g. `/people/john-doe.html`).
    pub url: String,

    /// Extracted date for posts (from `YYYY-MM-DD-title.md` filename or front matter).
    pub date: Option<String>,

    /// Which collection this item belongs to (e.g. `people`, `posts`).
    pub collection_name: String,

    /// Relative path to the source file (e.g. `_posts/2020-11-29-segmentation.md`).
    pub source_path: String,

    /// Jekyll-compatible document ID (e.g. `/podcast/my-episode` or `/2020/11/29/title`).
    ///
    /// Unlike `url`, this preserves spaces from the original filename rather than
    /// converting them to hyphens. Jekyll computes `document.id` from the raw
    /// collection-relative path (without extension), not from the permalink URL.
    pub id: String,
}

/// Regex-free post filename parsing. Extracts date and slug from `YYYY-MM-DD-title.md`.
///
/// Returns `(Option<date_string>, slug)`.
pub fn parse_post_filename(filename: &str) -> (Option<String>, String) {
    let stem = filename.strip_suffix(".md").unwrap_or(filename);

    // Check if the stem starts with a date pattern: exactly YYYY-MM-DD-
    if stem.len() >= 11 {
        let maybe_date = &stem[..10];
        let parts: Vec<&str> = maybe_date.split('-').collect();
        if parts.len() == 3
            && parts[0].len() == 4
            && parts[1].len() == 2
            && parts[2].len() == 2
            && parts[0].chars().all(|c| c.is_ascii_digit())
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].chars().all(|c| c.is_ascii_digit())
        {
            // Must have a '-' after the date, followed by the slug
            if stem.len() > 11 && stem.as_bytes()[10] == b'-' {
                let date = maybe_date.to_string();
                let slug = stem[11..].to_string();
                return (Some(date), slug);
            }
        }
    }

    (None, stem.to_string())
}

/// Context for generating URLs with extended permalink variables.
#[derive(Debug, Clone, Default)]
pub struct PermalinkContext {
    /// Collection name (e.g., "posts", "people").
    pub collection: String,
    /// Slug/title extracted from filename.
    pub title: String,
    /// Date string in YYYY-MM-DD format (from filename or front matter).
    pub date: Option<String>,
    /// Categories from front matter (e.g., ["machine-learning", "tutorials"]).
    pub categories: Vec<String>,
    /// Relative source path without extension (e.g., "2021-03-15-my-post").
    pub source_path_stem: Option<String>,
}

/// Expand a named permalink style to its full pattern.
///
/// Jekyll supports named styles like `date`, `pretty`, `ordinal`, `none`.
/// Custom patterns (starting with `/` or containing `:`) are returned unchanged.
pub fn expand_permalink_style(pattern: &str) -> &str {
    match pattern {
        "date" => "/:categories/:year/:month/:day/:title.html",
        "pretty" => "/:categories/:year/:month/:day/:title/",
        "ordinal" => "/:categories/:year/:y_day/:title.html",
        "none" => "/:categories/:title.html",
        _ => pattern,
    }
}

/// Determine the URL suffix for standalone pages based on the site's permalink style.
///
/// This mirrors Jekyll's `Utils.add_permalink_suffix` behavior for pages.
/// Jekyll pages use the template `/:path/:basename` and append a suffix
/// based on the site permalink style:
///
/// - Named style `pretty` -> `/` (trailing slash, pretty URLs)
/// - Named styles `date`, `ordinal`, `none` -> `.html` (output extension)
/// - Custom pattern ending with `/` -> `/`
/// - Custom pattern ending with `:output_ext` -> `.html`
/// - Everything else (e.g. `/blog/:title.html`) -> no suffix (bare basename)
///
/// Index pages always get URL `/<dir>/` regardless of permalink style.
pub fn page_url_suffix(permalink_style: &str) -> &'static str {
    if permalink_style == "pretty" || permalink_style.ends_with('/') {
        "/"
    } else {
        ".html"
    }
}

/// Generate a URL from a permalink pattern by substituting all Jekyll permalink variables.
///
/// Supports `:collection`, `:name`, `:title`, `:slug`, `:year`, `:month`, `:day`,
/// `:short_year`, `:i_month`, `:i_day`, `:categories`, and `:path`.
///
/// Named styles (`date`, `pretty`, `ordinal`, `none`) are expanded first.
/// When `:categories` is empty, double slashes are collapsed to single slashes.
pub fn generate_url(pattern: &str, collection: &str, title: &str) -> String {
    let ctx = PermalinkContext {
        collection: collection.to_string(),
        title: title.to_string(),
        ..Default::default()
    };
    generate_url_with_context(pattern, &ctx)
}

/// Generate a URL from a permalink pattern with full context (date, categories, etc.).
pub fn generate_url_with_context(pattern: &str, ctx: &PermalinkContext) -> String {
    let expanded = expand_permalink_style(pattern);

    // Parse date components
    let (year, month, day) = ctx
        .date
        .as_deref()
        .and_then(parse_date_components)
        .unwrap_or_default();

    let short_year = if year.len() >= 2 {
        &year[year.len() - 2..]
    } else {
        &year
    };

    let i_month = month
        .parse::<u32>()
        .map(|m| m.to_string())
        .unwrap_or_default();
    let i_day = day
        .parse::<u32>()
        .map(|d| d.to_string())
        .unwrap_or_default();

    let categories_str = ctx.categories.join("/");
    let path_str = ctx.source_path_stem.as_deref().unwrap_or(&ctx.title);

    let mut url = expanded
        .replace(":collection", &ctx.collection)
        .replace(":name", &ctx.title)
        .replace(":slug", &ctx.title)
        .replace(":title", &ctx.title)
        .replace(":short_year", short_year)
        .replace(":i_month", &i_month)
        .replace(":i_day", &i_day)
        .replace(":year", &year)
        .replace(":month", &month)
        .replace(":day", &day)
        .replace(":categories", &categories_str)
        .replace(":path", path_str);

    // Collapse double (or more) slashes to single, preserving leading slash
    while url.contains("//") {
        url = url.replace("//", "/");
    }

    url
}

/// Parse a date string (YYYY-MM-DD) into (year, month, day) components.
fn parse_date_components(date: &str) -> Option<(String, String, String)> {
    // Handle both "YYYY-MM-DD" and "YYYY-MM-DD HH:MM:SS ..." formats
    let date_part = date.split_whitespace().next().unwrap_or(date);
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() >= 3 && parts[0].len() == 4 && parts[0].chars().all(|c| c.is_ascii_digit()) {
        Some((
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
        ))
    } else {
        None
    }
}

/// Extract categories from front matter.
///
/// Supports both `categories: [a, b]` (array) and `category: x` (single string).
pub fn extract_categories(front_matter: &FrontMatter) -> Vec<String> {
    // Try `categories` first (array)
    if let Some(val) = front_matter.get("categories") {
        match val {
            serde_yaml::Value::Sequence(seq) => {
                return seq
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
            serde_yaml::Value::String(s) => {
                // Single string treated as one category
                if !s.is_empty() {
                    return vec![s.clone()];
                }
            }
            _ => {}
        }
    }

    // Fall back to `category` (single string)
    if let Some(val) = front_matter.get("category") {
        if let Some(s) = val.as_str() {
            if !s.is_empty() {
                return vec![s.to_string()];
            }
        }
    }

    Vec::new()
}

/// Extract tags from front matter.
///
/// Supports both `tags: [a, b]` (array) and `tag: x` (single string).
/// When `tags` is present, it takes precedence over `tag`.
pub fn extract_tags(front_matter: &FrontMatter) -> Vec<String> {
    // Try `tags` first (array)
    if let Some(val) = front_matter.get("tags") {
        match val {
            serde_yaml::Value::Sequence(seq) => {
                return seq
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
            serde_yaml::Value::String(s) => {
                // Single string treated as one tag
                if !s.is_empty() {
                    return vec![s.clone()];
                }
            }
            _ => {}
        }
    }

    // Fall back to `tag` (single string)
    if let Some(val) = front_matter.get("tag") {
        if let Some(s) = val.as_str() {
            if !s.is_empty() {
                return vec![s.to_string()];
            }
        }
    }

    Vec::new()
}

/// Extract date from front matter, falling back to filename date.
///
/// Front matter `date` overrides the filename-parsed date.
pub fn extract_date(front_matter: &FrontMatter, filename_date: Option<&str>) -> Option<String> {
    // Front matter date overrides filename date
    if let Some(val) = front_matter.get("date") {
        if let Some(s) = val.as_str() {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }

    filename_date.map(|s| s.to_string())
}

/// Generate a build-time timestamp in Jekyll's default format.
///
/// Jekyll assigns `site.time` (the build timestamp) as the default `date`
/// for collection items that don't have an explicit date in their front matter
/// or filename.  The format is `YYYY-MM-DD HH:MM:SS +0000`.
pub fn build_timestamp() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S +0000")
        .to_string()
}

/// Fill in a default date for collection items that have no date.
///
/// Jekyll gives every collection item a `date` -- when no explicit date is
/// specified in front matter or the filename, it defaults to the build
/// timestamp (`site.time`).  This function replicates that behaviour so
/// that template expressions like `{{ page.date }}` produce a value even
/// for items without an explicit date (e.g. podcast episodes).
///
/// The same `build_time` string should be used for every item within a
/// single build to match Jekyll's semantics.
pub fn backfill_default_dates(items: &mut [CollectionItem], build_time: &str) {
    for item in items.iter_mut() {
        if item.date.is_none() {
            item.date = Some(build_time.to_string());
            // Also add to front matter so that `page.date` is available in templates
            item.front_matter
                .entry("date".to_string())
                .or_insert_with(|| serde_yaml::Value::String(build_time.to_string()));
        }
    }
}

/// Returns true if the filename should be skipped (starts with `_`).
fn should_skip(filename: &str) -> bool {
    filename.starts_with('_')
}

/// Returns true if the front matter has `published: false`.
///
/// Jekyll skips items and pages with `published: false` in their front matter.
/// If the key is absent or has any other value, the item is considered published.
fn is_published_false(front_matter: &FrontMatter) -> bool {
    front_matter
        .get("published")
        .and_then(|v| v.as_bool())
        .is_some_and(|b| !b)
}

/// Sanitize a slug to match Jekyll's behavior.
///
/// - Trims leading and trailing whitespace
/// - Replaces internal spaces with hyphens
/// - Collapses multiple consecutive hyphens into a single hyphen
pub fn sanitize_slug(raw: &str) -> String {
    let trimmed = raw.trim();
    let mut result = String::with_capacity(trimmed.len());
    let mut prev_was_hyphen = false;

    for ch in trimmed.chars() {
        if ch == ' ' || ch == '-' {
            if !prev_was_hyphen {
                result.push('-');
                prev_was_hyphen = true;
            }
        } else {
            result.push(ch);
            prev_was_hyphen = false;
        }
    }

    result
}

/// Check if the config enables CommonMarkGhPages HARDBREAKS option.
///
/// Delegates to `SiteConfig::has_commonmark_hardbreaks()`.
fn has_commonmark_hardbreaks(config: &SiteConfig) -> bool {
    config.has_commonmark_hardbreaks()
}

/// Load all items from a collection directory.
///
/// For posts (`collection_name == "posts"`), the global permalink pattern from
/// `config.permalink` is used. For other collections, the collection's own
/// permalink pattern is used.
///
/// Files whose name starts with `_` are skipped. Non-`.md` files are skipped.
/// If a single file fails to parse, the error is collected but loading continues.
///
/// Returns an empty Vec if the directory does not exist.
///
/// # Errors
///
/// Returns `CollectionError::ReadDir` if the directory exists but cannot be read.
pub fn load_collection(
    collection_name: &str,
    site_dir: &Path,
    config: &SiteConfig,
) -> Result<(Vec<CollectionItem>, Vec<CollectionError>), CollectionError> {
    let dir = site_dir.join(format!("_{}", collection_name));

    if !dir.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let permalink_pattern = if collection_name == "posts" {
        // Check defaults for a post-specific permalink first, then use global
        let defaults = config.defaults_for("posts", "");
        defaults
            .get("permalink")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| config.permalink.clone())
    } else {
        config
            .collection(collection_name)
            .map(|c| c.permalink.clone())
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| "/:collection/:path".to_string())
    };

    // Phase 1: Collect all file paths (fast, sequential directory walk)
    let mut file_paths = Vec::new();
    collect_collection_paths(&dir, &mut file_paths)?;
    file_paths.sort();

    // Issue 216: Determine whether to add kramdown-style inline code classes
    // based on the markdown processor setting in config.
    let add_code_classes = config
        .extras
        .get("markdown")
        .and_then(|v| v.as_str())
        .map(|m| m.eq_ignore_ascii_case("kramdown"))
        .unwrap_or(true); // kramdown is Jekyll's default

    // Issue 223: Check if HARDBREAKS option is enabled in commonmark.options.
    let enable_hardbreaks = has_commonmark_hardbreaks(config);

    // Phase 2: Process files in parallel (read, parse, convert markdown)
    let results: Vec<Result<CollectionItem, CollectionError>> = file_paths
        .par_iter()
        .filter_map(|path| {
            process_collection_file(
                path,
                &dir,
                site_dir,
                collection_name,
                &permalink_pattern,
                add_code_classes,
                enable_hardbreaks,
            )
        })
        .collect();

    let mut items = Vec::with_capacity(results.len());
    let mut errors = Vec::new();
    for result in results {
        match result {
            Ok(item) => items.push(item),
            Err(e) => errors.push(e),
        }
    }

    // Sort collection items by date ascending (oldest first), with slug as
    // tiebreaker. This matches Jekyll's behavior where all collection documents
    // (not just posts) are sorted by date. Without this sort, file path ordering
    // produces incorrect results for filenames with mixed-length numeric prefixes
    // (e.g. 099 sorts before 1000 but 1000 sorts before 100 in string order).
    items.sort_by(|a, b| {
        let date_a = a.date.as_deref().unwrap_or("");
        let date_b = b.date.as_deref().unwrap_or("");
        date_a
            .cmp(date_b)
            .then_with(|| a.source_path.cmp(&b.source_path))
    });

    Ok((items, errors))
}

/// File extensions that can be processed as collection items.
///
/// Jekyll processes any file with front matter in a collection directory.
/// We check `.md` files unconditionally, and also non-`.md` files for
/// front matter presence.
const COLLECTION_EXTENSIONS: &[&str] = &[".md", ".html", ".htm", ".xml", ".json", ".txt"];

/// Recursively collect all file paths in a collection directory.
///
/// This is the first phase of collection loading: it walks the directory tree
/// and collects all candidate file paths. The actual file reading and parsing
/// is done in parallel in the second phase.
fn collect_collection_paths(
    current_dir: &Path,
    file_paths: &mut Vec<PathBuf>,
) -> Result<(), CollectionError> {
    let entries = fs::read_dir(current_dir).map_err(|e| CollectionError::ReadDir {
        path: current_dir.display().to_string(),
        source: e,
    })?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if path.is_dir() {
            if !filename.starts_with('.') && !filename.starts_with('_') {
                collect_collection_paths(&path, file_paths)?;
            }
            continue;
        }

        if !path.is_file() {
            continue;
        }

        if should_skip(&filename) {
            continue;
        }

        // Check file extension
        let has_valid_ext = COLLECTION_EXTENSIONS
            .iter()
            .any(|ext| filename.ends_with(ext));
        if has_valid_ext {
            file_paths.push(path);
        }
    }

    Ok(())
}

/// Process a single collection file: read, parse, and convert to CollectionItem.
///
/// Returns None if the file should be skipped (no front matter for non-markdown,
/// published: false, etc.). Returns Some(Ok(item)) on success or Some(Err(e)) on error.
fn process_collection_file(
    path: &Path,
    _collection_dir: &Path,
    site_dir: &Path,
    collection_name: &str,
    permalink_pattern: &str,
    add_code_classes: bool,
    enable_hardbreaks: bool,
) -> Option<Result<CollectionItem, CollectionError>> {
    let filename = path.file_name()?.to_str()?.to_string();

    let ext = COLLECTION_EXTENSIONS
        .iter()
        .copied()
        .find(|ext| filename.ends_with(ext))?;

    let is_markdown = ext == ".md";

    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            return Some(Err(CollectionError::ReadFile {
                path: path.display().to_string(),
                source: e,
            }));
        }
    };

    // For non-markdown files, only process if they have front matter
    if !is_markdown && !has_front_matter(&raw) {
        return None;
    }

    let doc = match frontmatter::parse_document(&raw) {
        Ok(doc) => doc,
        Err(e) => {
            return Some(Err(CollectionError::Parse {
                path: path.display().to_string(),
                source: e,
            }));
        }
    };

    // Skip items with `published: false` (matching Jekyll behavior)
    if is_published_false(&doc.front_matter) {
        return None;
    }

    let is_posts = collection_name == "posts";
    let stem = filename.strip_suffix(ext).unwrap_or(&filename);

    let (filename_date, slug) = if is_posts {
        let (date, raw_slug) = parse_post_filename(&filename);
        (date, sanitize_slug(&raw_slug))
    } else {
        (None, sanitize_slug(stem))
    };

    // Use front matter date if available, falling back to filename date
    let date = extract_date(&doc.front_matter, filename_date.as_deref());
    let categories = extract_categories(&doc.front_matter);

    let source_path = path
        .strip_prefix(site_dir)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));

    // Build source path stem (path without extension, without leading _collection/)
    let source_path_stem = source_path
        .strip_suffix(ext)
        .unwrap_or(&source_path)
        .strip_prefix(&format!("_{}/", collection_name))
        .unwrap_or(source_path.strip_suffix(ext).unwrap_or(&source_path))
        .to_string();

    // Use front matter `permalink` if present, otherwise use the pattern
    let url = doc
        .front_matter
        .get("permalink")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let ctx = PermalinkContext {
                collection: collection_name.to_string(),
                title: slug.clone(),
                date: date.clone(),
                categories: categories.clone(),
                source_path_stem: Some(source_path_stem.clone()),
            };
            let mut generated = generate_url_with_context(permalink_pattern, &ctx);
            // For non-markdown files (e.g., .json, .xml), Jekyll uses the
            // original file extension as the output extension, not .html.
            // Replace the trailing .html with the source file's extension.
            if !is_markdown && ext != ".html" && ext != ".htm" {
                if let Some(stripped) = generated.strip_suffix(".html") {
                    generated = format!("{}{}", stripped, ext);
                }
            }
            generated
        });

    // Percent-encode non-ASCII characters in URLs, matching Jekyll behavior.
    // Jekyll outputs percent-encoded URLs for Cyrillic and other non-ASCII chars.
    let url = crate::template::filters::relative_url::encode_url_path(&url);

    let html_content = if is_markdown {
        frontmatter::markdown_to_html_with_options(
            &doc.content,
            add_code_classes,
            add_code_classes,
            enable_hardbreaks,
        )
    } else {
        doc.content.clone()
    };

    // Compute Jekyll-compatible document.id.
    // For non-post collections: /<collection>/<raw_stem> (preserves spaces from filename).
    // For posts: /<YYYY>/<MM>/<DD>/<raw_slug> (date-based path).
    let id = if is_posts {
        if let Some(ref d) = date {
            let date_part = &d[..std::cmp::min(10, d.len())];
            let parts: Vec<&str> = date_part.split('-').collect();
            if parts.len() >= 3 {
                let (_, raw_slug) = parse_post_filename(&filename);
                format!("/{}/{}/{}/{}", parts[0], parts[1], parts[2], raw_slug)
            } else {
                let base = url.trim_end_matches(".html").trim_start_matches('/');
                format!("/{}", base)
            }
        } else {
            let base = url.trim_end_matches(".html").trim_start_matches('/');
            format!("/{}", base)
        }
    } else {
        // For non-post collections, id preserves raw filename (including spaces)
        format!("/{}/{}", collection_name, stem)
    };

    Some(Ok(CollectionItem {
        slug,
        front_matter: doc.front_matter,
        content: doc.content,
        html_content,
        excerpt: doc.excerpt,
        url,
        date,
        collection_name: collection_name.to_string(),
        source_path,
        id,
    }))
}

/// A standalone page (root-level `.md` file, not part of any collection).
#[derive(Debug, Clone)]
pub struct Page {
    /// Filename stem (e.g. `index` from `index.md`).
    pub slug: String,

    /// Parsed YAML front matter.
    pub front_matter: FrontMatter,

    /// Raw markdown body.
    pub content: String,

    /// Markdown body converted to HTML.
    pub html_content: String,

    /// Generated URL path.
    pub url: String,

    /// Relative path to the source file (e.g. `index.md`).
    pub source_path: String,
}

/// Load standalone pages from the site directory, recursing into subdirectories.
///
/// Loads `.md` files unconditionally, and also non-`.md` files (`.xml`, `.html`,
/// `.htm`, `.json`, `.txt`) that have YAML front matter. This matches Jekyll's
/// behavior of processing any file with front matter through its template engine.
///
/// Skips `README.md` and files whose name starts with `_`.
/// Skips directories that start with `_`, `.`, or are named `node_modules`,
/// or are in the config `exclude` list.
/// If a file fails to parse, the error is collected but loading continues.
///
/// Returns an empty Vec if the directory does not exist.
///
/// # Errors
///
/// Returns `CollectionError::ReadDir` if the directory cannot be read.
pub fn load_pages(
    site_dir: &Path,
    config: &SiteConfig,
) -> Result<(Vec<Page>, Vec<CollectionError>), CollectionError> {
    if !site_dir.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let mut pages = Vec::new();
    let mut errors = Vec::new();

    load_pages_recursive(site_dir, site_dir, config, &mut pages, &mut errors)?;

    // Sort pages by (basename, full URL) to match Jekyll's site.pages order.
    // Jekyll sorts pages by filename first, then by full path for stability.
    pages.sort_by(|a, b| {
        let basename_a = a.url.rsplit('/').next().unwrap_or(&a.url);
        let basename_b = b.url.rsplit('/').next().unwrap_or(&b.url);
        basename_a.cmp(basename_b).then_with(|| a.url.cmp(&b.url))
    });

    Ok((pages, errors))
}

/// Check if a directory name should be skipped during page discovery.
fn should_skip_directory(name: &str, config: &SiteConfig) -> bool {
    // Skip hidden directories and underscore-prefixed directories
    if name.starts_with('_') || name.starts_with('.') {
        return true;
    }
    // Skip node_modules
    if name == "node_modules" {
        return true;
    }
    // Skip directories in the config exclude list
    for excluded in &config.exclude {
        let excluded_name = excluded.trim_end_matches('/');
        if name == excluded_name {
            return true;
        }
    }
    false
}

/// Check if a file extension indicates a processable page type.
///
/// Jekyll processes any file with YAML front matter. We check `.md` files
/// unconditionally, and also check certain other extensions (`.xml`, `.html`,
/// `.htm`, `.json`, `.txt`) for front matter presence.
fn is_processable_extension(name: &str) -> Option<&'static str> {
    [
        ".md", ".xml", ".html", ".htm", ".json", ".txt", ".scss", ".css",
    ]
    .iter()
    .copied()
    .find(|ext| name.ends_with(ext))
}

/// Check if raw file content starts with YAML front matter delimiters.
fn has_front_matter(content: &str) -> bool {
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return false;
    }
    // Must have a closing --- after the opening one
    let after_opening = &trimmed[3..];
    let rest = if let Some(stripped) = after_opening.strip_prefix('\n') {
        stripped
    } else if let Some(stripped) = after_opening.strip_prefix("\r\n") {
        stripped
    } else {
        return false;
    };
    // Look for closing --- on its own line, or at the very start of rest
    // (which happens with empty front matter: "---\n---\n")
    rest.starts_with("---") || rest.contains("\n---")
}

/// Recursively discover and load pages from a directory.
///
/// Loads `.md` files and also non-`.md` files (`.xml`, `.html`, etc.) that
/// have YAML front matter, matching Jekyll's behavior of processing any file
/// with front matter through its template engine.
fn load_pages_recursive(
    current_dir: &Path,
    site_dir: &Path,
    config: &SiteConfig,
    pages: &mut Vec<Page>,
    errors: &mut Vec<CollectionError>,
) -> Result<(), CollectionError> {
    let entries = fs::read_dir(current_dir).map_err(|e| CollectionError::ReadDir {
        path: current_dir.display().to_string(),
        source: e,
    })?;

    let mut entry_paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entry_paths.sort();

    for path in entry_paths {
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        if path.is_dir() {
            if !should_skip_directory(&name, config) {
                load_pages_recursive(&path, site_dir, config, pages, errors)?;
            }
            continue;
        }

        if !path.is_file() {
            continue;
        }

        let ext = match is_processable_extension(&name) {
            Some(ext) => ext,
            None => continue,
        };

        if name == "README.md" || name.starts_with('_') {
            continue;
        }

        let is_markdown = ext == ".md";

        let raw = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(e) => {
                errors.push(CollectionError::ReadFile {
                    path: path.display().to_string(),
                    source: e,
                });
                continue;
            }
        };

        // Jekyll only processes files that have YAML front matter (starting with ---)
        // This applies to all file types including .md files
        if !has_front_matter(&raw) {
            continue;
        }

        let doc = match frontmatter::parse_document(&raw) {
            Ok(doc) => doc,
            Err(e) => {
                errors.push(CollectionError::Parse {
                    path: path.display().to_string(),
                    source: e,
                });
                continue;
            }
        };

        // Skip pages with `published: false` (matching Jekyll behavior)
        if is_published_false(&doc.front_matter) {
            continue;
        }

        // Compute relative path from site_dir for URL generation
        let rel_path = path
            .strip_prefix(site_dir)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));

        let stem = name.strip_suffix(ext).unwrap_or(&name);

        // Use front matter `permalink` if present, otherwise derive from relative path
        let url = doc
            .front_matter
            .get("permalink")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                if is_markdown {
                    let rel_stem = rel_path.strip_suffix(".md").unwrap_or(&rel_path);
                    // Index pages always get directory URL (e.g. "/" or "/subdir/")
                    if stem == "index" {
                        let dir = rel_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
                        if dir.is_empty() {
                            "/".to_string()
                        } else {
                            format!("/{}/", dir)
                        }
                    } else {
                        // Inline the Jekyll Utils.add_permalink_suffix logic here.
                        // See jekyll/lib/jekyll/utils.rb:253-267 for reference.
                        let pl = &config.permalink;
                        let suffix = match pl.as_str() {
                            "pretty" => "/",
                            "date" | "ordinal" | "none" => ".html",
                            s if s.ends_with('/') => "/",
                            s if s.ends_with(":output_ext") => ".html",
                            _ => "",
                        };
                        format!("/{}{}", rel_stem, suffix)
                    }
                } else {
                    // Non-markdown files keep their original extension in the URL
                    // (e.g. podcast.xml -> /podcast.xml)
                    // Exception: .scss files are output as .css (Jekyll compiles SCSS)
                    let url_path = if rel_path.ends_with(".scss") {
                        format!("{}css", rel_path.strip_suffix("scss").unwrap_or(&rel_path))
                    } else {
                        rel_path.clone()
                    };
                    format!("/{}", url_path)
                }
            });

        // Percent-encode non-ASCII characters in URLs, matching Jekyll behavior.
        let url = crate::template::filters::relative_url::encode_url_path(&url);

        // Issue 216: Respect markdown processor setting for inline code classes
        let add_code_classes = config
            .extras
            .get("markdown")
            .and_then(|v| v.as_str())
            .map(|m| m.eq_ignore_ascii_case("kramdown"))
            .unwrap_or(true);

        // Issue 223: Check if HARDBREAKS option is enabled in commonmark.options.
        let enable_hardbreaks = has_commonmark_hardbreaks(config);

        let html_content = if is_markdown {
            frontmatter::markdown_to_html_with_options(
                &doc.content,
                add_code_classes,
                add_code_classes,
                enable_hardbreaks,
            )
        } else {
            // Non-markdown files: content is used as-is (will be rendered
            // through Liquid but not converted from markdown to HTML)
            doc.content.clone()
        };

        pages.push(Page {
            slug: stem.to_string(),
            front_matter: doc.front_matter,
            content: doc.content,
            html_content,
            url,
            source_path: rel_path,
        });
    }

    Ok(())
}

/// A URL collision: multiple source files resolving to the same output URL.
#[derive(Debug, Clone)]
pub struct UrlCollision {
    /// The shared output URL.
    pub url: String,
    /// Source file paths that all map to this URL.
    pub source_paths: Vec<String>,
}

/// Detect URL collisions among a list of (source_path, url) pairs.
///
/// Returns a list of collisions, each containing the shared URL and all source
/// files that map to it. Only URLs with two or more sources are returned.
/// Results are sorted by URL for deterministic output.
pub fn detect_url_collisions(entries: &[(String, String)]) -> Vec<UrlCollision> {
    let mut url_to_sources: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for (source_path, url) in entries {
        url_to_sources
            .entry(url.as_str())
            .or_default()
            .push(source_path.as_str());
    }
    let mut collisions: Vec<UrlCollision> = url_to_sources
        .into_iter()
        .filter(|(_, sources)| sources.len() > 1)
        .map(|(url, sources)| UrlCollision {
            url: url.to_string(),
            source_paths: sources.into_iter().map(|s| s.to_string()).collect(),
        })
        .collect();
    collisions.sort_by(|a, b| a.url.cmp(&b.url));
    // Also sort source paths within each collision for deterministic output
    for collision in &mut collisions {
        collision.source_paths.sort();
    }
    collisions
}

/// Format a URL collision as a warning message matching Jekyll's format.
///
/// Produces: `Conflict: The URL '<url>' is the destination for the following pages: <file1>, <file2>`
pub fn format_collision_warning(collision: &UrlCollision) -> String {
    format!(
        "Conflict: The URL '{}' is the destination for the following pages: {}",
        collision.url,
        collision.source_paths.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn site_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    fn test_config() -> SiteConfig {
        SiteConfig::from_file(&site_dir().join("_config.yml")).unwrap()
    }

    // ========================================================================
    // Unit: Post filename parsing
    // ========================================================================

    #[test]
    fn test_parse_post_filename_standard() {
        let (date, slug) = parse_post_filename("2020-11-29-segmentation.md");
        assert_eq!(date, Some("2020-11-29".to_string()));
        assert_eq!(slug, "segmentation");
    }

    #[test]
    fn test_parse_post_filename_with_hyphens_in_slug() {
        let (date, slug) = parse_post_filename("2021-01-01-ml-deployment-lambda.md");
        assert_eq!(date, Some("2021-01-01".to_string()));
        assert_eq!(slug, "ml-deployment-lambda");
    }

    #[test]
    fn test_parse_post_filename_no_date() {
        let (date, slug) = parse_post_filename("non-date-filename.md");
        assert_eq!(date, None);
        assert_eq!(slug, "non-date-filename");
    }

    // ========================================================================
    // Unit: URL generation
    // ========================================================================

    #[test]
    fn test_generate_url_collection_pattern() {
        let url = generate_url("/:collection/:title.html", "people", "alexeygrigorev");
        assert_eq!(url, "/people/alexeygrigorev.html");
    }

    #[test]
    fn test_generate_url_books_pattern() {
        let url = generate_url("/:collection/:title.html", "books", "20201214-ml-bookcamp");
        assert_eq!(url, "/books/20201214-ml-bookcamp.html");
    }

    #[test]
    fn test_generate_url_blog_pattern() {
        let url = generate_url("/blog/:title.html", "posts", "segmentation");
        assert_eq!(url, "/blog/segmentation.html");
    }

    #[test]
    fn test_generate_url_cyrillic_title() {
        // Cyrillic titles should produce URLs with raw Cyrillic in generate_url
        // (percent-encoding is applied separately when storing item.url)
        let url = generate_url(
            "/:collection/:title/",
            "little-book-of-metals-ru",
            "часть_1_история",
        );
        assert_eq!(url, "/little-book-of-metals-ru/часть_1_история/");
    }

    // ========================================================================
    // Unit: Named permalink style expansion
    // ========================================================================

    #[test]
    fn test_expand_style_date() {
        assert_eq!(
            expand_permalink_style("date"),
            "/:categories/:year/:month/:day/:title.html"
        );
    }

    #[test]
    fn test_expand_style_pretty() {
        assert_eq!(
            expand_permalink_style("pretty"),
            "/:categories/:year/:month/:day/:title/"
        );
    }

    #[test]
    fn test_expand_style_none() {
        assert_eq!(expand_permalink_style("none"), "/:categories/:title.html");
    }

    #[test]
    fn test_expand_style_custom_pattern_unchanged() {
        assert_eq!(
            expand_permalink_style("/blog/:title.html"),
            "/blog/:title.html"
        );
    }

    // ========================================================================
    // Unit: Date variable substitution
    // ========================================================================

    #[test]
    fn test_date_variables_from_filename() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2021-03-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/:year/:month/:day/:title.html", &ctx);
        assert_eq!(url, "/2021/03/15/my-post.html");
    }

    #[test]
    fn test_short_year_and_unpadded_date() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2021-03-05".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/:short_year/:i_month/:i_day/:title/", &ctx);
        assert_eq!(url, "/21/3/5/my-post/");
    }

    #[test]
    fn test_front_matter_date_overrides_filename() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "date".to_string(),
            serde_yaml::Value::String("2022-06-01".to_string()),
        );
        let date = extract_date(&fm, Some("2021-03-15"));
        assert_eq!(date, Some("2022-06-01".to_string()));
    }

    #[test]
    fn test_filename_date_used_when_no_front_matter_date() {
        let fm = FrontMatter::new();
        let date = extract_date(&fm, Some("2021-03-15"));
        assert_eq!(date, Some("2021-03-15".to_string()));
    }

    #[test]
    fn test_no_date_produces_empty_strings() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: None,
            ..Default::default()
        };
        let url = generate_url_with_context("/:year/:month/:day/:title.html", &ctx);
        assert_eq!(url, "/my-post.html");
    }

    // ========================================================================
    // Unit: Category substitution
    // ========================================================================

    #[test]
    fn test_categories_joined_with_slash() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "hello".to_string(),
            date: Some("2021-03-15".to_string()),
            categories: vec!["tech".to_string()],
            ..Default::default()
        };
        let url = generate_url_with_context("/:categories/:year/:month/:day/:title.html", &ctx);
        assert_eq!(url, "/tech/2021/03/15/hello.html");
    }

    #[test]
    fn test_multiple_categories() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "hello".to_string(),
            categories: vec!["machine-learning".to_string(), "tutorials".to_string()],
            ..Default::default()
        };
        let url = generate_url_with_context("/:categories/:title/", &ctx);
        assert_eq!(url, "/machine-learning/tutorials/hello/");
    }

    #[test]
    fn test_single_category_from_front_matter() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "category".to_string(),
            serde_yaml::Value::String("blog".to_string()),
        );
        let cats = extract_categories(&fm);
        assert_eq!(cats, vec!["blog"]);
    }

    #[test]
    fn test_categories_array_from_front_matter() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "categories".to_string(),
            serde_yaml::Value::Sequence(vec![
                serde_yaml::Value::String("machine-learning".to_string()),
                serde_yaml::Value::String("tutorials".to_string()),
            ]),
        );
        let cats = extract_categories(&fm);
        assert_eq!(cats, vec!["machine-learning", "tutorials"]);
    }

    #[test]
    fn test_no_categories_returns_empty() {
        let fm = FrontMatter::new();
        let cats = extract_categories(&fm);
        assert!(cats.is_empty());
    }

    // ========================================================================
    // Unit: Double-slash collapsing
    // ========================================================================

    #[test]
    fn test_empty_categories_no_double_slash() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            categories: vec![],
            ..Default::default()
        };
        let url = generate_url_with_context("/:categories/:title.html", &ctx);
        assert_eq!(url, "/my-post.html");
    }

    #[test]
    fn test_empty_categories_date_pattern_no_double_slash() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2021-03-15".to_string()),
            categories: vec![],
            ..Default::default()
        };
        let url = generate_url_with_context("/:categories/:year/:month/:day/:title/", &ctx);
        assert_eq!(url, "/2021/03/15/my-post/");
    }

    // ========================================================================
    // Unit: Slug and path substitution
    // ========================================================================

    #[test]
    fn test_slug_alias_for_title() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            ..Default::default()
        };
        let url = generate_url_with_context("/:slug.html", &ctx);
        assert_eq!(url, "/my-post.html");
    }

    #[test]
    fn test_path_substitution() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            source_path_stem: Some("2021-03-15-my-post".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/:path/", &ctx);
        assert_eq!(url, "/2021-03-15-my-post/");
    }

    // ========================================================================
    // Unit: :name placeholder (Issue 254)
    // ========================================================================

    #[test]
    fn test_name_placeholder_basic() {
        let ctx = PermalinkContext {
            collection: "introduction".to_string(),
            title: "getting-started".to_string(),
            ..Default::default()
        };
        let url = generate_url_with_context("/:name/", &ctx);
        assert_eq!(url, "/getting-started/");
    }

    #[test]
    fn test_name_placeholder_with_collection() {
        let ctx = PermalinkContext {
            collection: "introduction".to_string(),
            title: "overview".to_string(),
            ..Default::default()
        };
        let url = generate_url_with_context("/:collection/:name/", &ctx);
        assert_eq!(url, "/introduction/overview/");
    }

    #[test]
    fn test_name_placeholder_with_html_suffix() {
        let ctx = PermalinkContext {
            collection: "pages".to_string(),
            title: "my-page".to_string(),
            ..Default::default()
        };
        let url = generate_url_with_context("/:name.html", &ctx);
        assert_eq!(url, "/my-page.html");
    }

    #[test]
    fn test_name_placeholder_equivalence_with_title() {
        let ctx = PermalinkContext {
            collection: "docs".to_string(),
            title: "some-document".to_string(),
            date: Some("2024-06-15".to_string()),
            ..Default::default()
        };
        let name_url = generate_url_with_context("/:name/", &ctx);
        let title_url = generate_url_with_context("/:title/", &ctx);
        assert_eq!(name_url, title_url);
    }

    #[test]
    fn test_name_placeholder_with_date() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2024-01-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/:year/:name/", &ctx);
        assert_eq!(url, "/2024/my-post/");
    }

    // ========================================================================
    // Unit: External site patterns
    // ========================================================================

    #[test]
    fn test_beautiful_jekyll_pattern() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2021-01-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/:year-:month-:day-:title/", &ctx);
        assert_eq!(url, "/2021-01-15-my-post/");
    }

    #[test]
    fn test_categories_pattern_with_categories() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "hello".to_string(),
            categories: vec!["updates".to_string()],
            ..Default::default()
        };
        let url = generate_url_with_context("/:categories/:title/", &ctx);
        assert_eq!(url, "/updates/hello/");
    }

    // ========================================================================
    // Unit: DTC site existing pattern
    // ========================================================================

    #[test]
    fn test_dtc_blog_pattern_still_works() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "segmentation".to_string(),
            date: Some("2020-11-29".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/blog/:title.html", &ctx);
        assert_eq!(url, "/blog/segmentation.html");
    }

    // ========================================================================
    // Unit: Named style with full context
    // ========================================================================

    #[test]
    fn test_named_style_date_with_context() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "hello".to_string(),
            date: Some("2021-03-15".to_string()),
            categories: vec!["tech".to_string()],
            ..Default::default()
        };
        let url = generate_url_with_context("date", &ctx);
        assert_eq!(url, "/tech/2021/03/15/hello.html");
    }

    #[test]
    fn test_named_style_pretty_no_categories() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "hello".to_string(),
            date: Some("2021-03-15".to_string()),
            categories: vec![],
            ..Default::default()
        };
        let url = generate_url_with_context("pretty", &ctx);
        assert_eq!(url, "/2021/03/15/hello/");
    }

    #[test]
    fn test_named_style_none_with_categories() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "hello".to_string(),
            categories: vec!["blog".to_string()],
            ..Default::default()
        };
        let url = generate_url_with_context("none", &ctx);
        assert_eq!(url, "/blog/hello.html");
    }

    // ========================================================================
    // Unit: Date with timestamp format
    // ========================================================================

    #[test]
    fn test_date_with_timestamp_format() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2022-06-01 12:00:00 +0000".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/:year/:month/:day/:title/", &ctx);
        assert_eq!(url, "/2022/06/01/my-post/");
    }

    // ========================================================================
    // Unit: Skip underscore-prefixed files
    // ========================================================================

    #[test]
    fn test_should_skip_underscore() {
        assert!(should_skip("_template.md"));
    }

    #[test]
    fn test_should_not_skip_non_md() {
        // should_skip now only checks for underscore prefix;
        // extension filtering is handled by COLLECTION_EXTENSIONS
        assert!(!should_skip("file.txt"));
    }

    #[test]
    fn test_should_not_skip_regular_md() {
        assert!(!should_skip("alexeygrigorev.md"));
    }

    // ========================================================================
    // Integration: Load real _people/ collection
    // ========================================================================

    #[test]
    fn test_load_people_collection_count() {
        let config = test_config();
        let (items, errors) = load_collection("people", &site_dir(), &config).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert!(
            items.len() >= 2,
            "Expected 2+ people items, got {}",
            items.len()
        );
    }

    #[test]
    fn test_load_people_known_item() {
        let config = test_config();
        let (items, _) = load_collection("people", &site_dir(), &config).unwrap();
        let alexey = items.iter().find(|i| i.slug == "alexeygrigorev");
        assert!(alexey.is_some(), "Expected to find alexeygrigorev");
        let alexey = alexey.unwrap();
        assert_eq!(
            alexey.front_matter.get("title").and_then(|v| v.as_str()),
            Some("Alexey Grigorev")
        );
        assert!(alexey.front_matter.contains_key("short"));
        assert!(alexey.front_matter.contains_key("picture"));
        assert_eq!(alexey.url, "/people/alexeygrigorev.html");
        assert_eq!(alexey.collection_name, "people");
    }

    // ========================================================================
    // Integration: Load real _books/ collection
    // ========================================================================

    #[test]
    fn test_load_books_collection_count() {
        let config = test_config();
        let (items, errors) = load_collection("books", &site_dir(), &config).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert!(
            items.len() >= 2,
            "Expected 2+ books items, got {}",
            items.len()
        );
    }

    #[test]
    fn test_load_books_known_item() {
        let config = test_config();
        let (items, _) = load_collection("books", &site_dir(), &config).unwrap();
        let book = items.iter().find(|i| i.slug == "20201214-ml-bookcamp");
        assert!(book.is_some(), "Expected to find 20201214-ml-bookcamp");
        let book = book.unwrap();
        assert_eq!(
            book.front_matter.get("title").and_then(|v| v.as_str()),
            Some("Machine Learning Bookcamp")
        );
        assert_eq!(book.url, "/books/20201214-ml-bookcamp.html");
    }

    // ========================================================================
    // Integration: Load real _podcast/ collection
    // ========================================================================

    #[test]
    fn test_load_podcast_collection_count() {
        let config = test_config();
        let (items, errors) = load_collection("podcast", &site_dir(), &config).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert!(
            items.len() >= 2,
            "Expected 2+ podcast items, got {}",
            items.len()
        );
    }

    #[test]
    fn test_load_podcast_skips_underscore_files() {
        let config = test_config();
        let (items, _) = load_collection("podcast", &site_dir(), &config).unwrap();
        // None of the loaded items should have a slug starting with '_'
        for item in &items {
            assert!(
                !item.slug.starts_with('_'),
                "Found underscore-prefixed item: {}",
                item.slug
            );
        }
    }

    // ========================================================================
    // Integration: Load real _posts/ directory
    // ========================================================================

    #[test]
    fn test_load_posts_count() {
        let config = test_config();
        let (items, errors) = load_collection("posts", &site_dir(), &config).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert_eq!(items.len(), 3, "Expected 3 posts, got {}", items.len());
    }

    #[test]
    fn test_load_posts_known_item() {
        let config = test_config();
        let (items, _) = load_collection("posts", &site_dir(), &config).unwrap();
        let post = items.iter().find(|i| i.slug == "segmentation");
        assert!(post.is_some(), "Expected to find segmentation post");
        let post = post.unwrap();
        assert_eq!(post.date, Some("2020-11-29".to_string()));
        assert_eq!(post.url, "/blog/segmentation.html");
        assert_eq!(post.collection_name, "posts");
    }

    // ========================================================================
    // Integration: Load real _courses/, _conferences/, _tools/
    // ========================================================================

    #[test]
    fn test_load_courses_count() {
        let config = test_config();
        let (items, _) = load_collection("courses", &site_dir(), &config).unwrap();
        assert!(
            !items.is_empty(),
            "Expected 1+ courses, got {}",
            items.len()
        );
    }

    #[test]
    fn test_load_conferences_count() {
        let config = test_config();
        let (items, _) = load_collection("conferences", &site_dir(), &config).unwrap();
        assert!(
            items.len() >= 2,
            "Expected 2+ conferences, got {}",
            items.len()
        );
    }

    #[test]
    fn test_load_tools_count() {
        let config = test_config();
        let (items, _) = load_collection("tools", &site_dir(), &config).unwrap();
        assert!(items.len() >= 2, "Expected 2+ tools, got {}", items.len());
    }

    // ========================================================================
    // Integration: Load standalone pages
    // ========================================================================

    #[test]
    fn test_load_pages_count() {
        let config = test_config();
        let (pages, errors) = load_pages(&site_dir(), &config).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert_eq!(
            pages.len(),
            10,
            "Expected 10 standalone pages, got {}",
            pages.len()
        );
    }

    #[test]
    fn test_load_pages_index() {
        let config = test_config();
        let (pages, _) = load_pages(&site_dir(), &config).unwrap();
        let index = pages.iter().find(|p| p.slug == "index");
        assert!(index.is_some(), "Expected to find index page");
        let index = index.unwrap();
        assert_eq!(
            index.front_matter.get("title").and_then(|v| v.as_str()),
            Some("Welcome to DataTalks.Club")
        );
    }

    #[test]
    fn test_load_pages_excludes_readme() {
        let config = test_config();
        let (pages, _) = load_pages(&site_dir(), &config).unwrap();
        let readme = pages.iter().find(|p| p.slug == "README");
        assert!(readme.is_none(), "README.md should be excluded");
    }

    // ========================================================================
    // Edge cases
    // ========================================================================

    #[test]
    fn test_nonexistent_collection_returns_empty() {
        let config = test_config();
        let (items, errors) = load_collection("nonexistent", &site_dir(), &config).unwrap();
        assert!(items.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nonexistent_pages_dir_returns_empty() {
        let config = SiteConfig::default();
        let (pages, errors) = load_pages(Path::new("/nonexistent/dir"), &config).unwrap();
        assert!(pages.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_collection_items_have_nonempty_fields() {
        let config = test_config();
        let (items, _) = load_collection("people", &site_dir(), &config).unwrap();
        for item in &items {
            assert!(!item.slug.is_empty(), "Slug should not be empty");
            assert!(!item.url.is_empty(), "URL should not be empty");
            assert_eq!(item.collection_name, "people");
        }
    }

    #[test]
    fn test_file_with_no_front_matter() {
        // Create a temp dir with a .md file that has no front matter
        let dir = tempfile::TempDir::new().unwrap();
        let collection_dir = dir.path().join("_test");
        fs::create_dir(&collection_dir).unwrap();
        fs::write(collection_dir.join("nofront.md"), "Just plain content.").unwrap();

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            ..Default::default()
        };

        let (items, errors) = load_collection("test", dir.path(), &config).unwrap();
        assert!(errors.is_empty());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].slug, "nofront");
        assert!(items[0].front_matter.is_empty());
        assert_eq!(items[0].content, "Just plain content.");
    }

    #[test]
    fn test_file_with_front_matter_only() {
        let dir = tempfile::TempDir::new().unwrap();
        let collection_dir = dir.path().join("_test");
        fs::create_dir(&collection_dir).unwrap();
        fs::write(
            collection_dir.join("frontonly.md"),
            "---\ntitle: Just Front Matter\n---\n",
        )
        .unwrap();

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            ..Default::default()
        };

        let (items, errors) = load_collection("test", dir.path(), &config).unwrap();
        assert!(errors.is_empty());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].slug, "frontonly");
        assert_eq!(
            items[0].front_matter.get("title").and_then(|v| v.as_str()),
            Some("Just Front Matter")
        );
    }

    // ========================================================================
    // Unit: extract_tags
    // ========================================================================

    #[test]
    fn test_extract_tags_array() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "tags".to_string(),
            serde_yaml::Value::Sequence(vec![
                serde_yaml::Value::String("machine-learning".to_string()),
                serde_yaml::Value::String("tutorial".to_string()),
            ]),
        );
        let tags = extract_tags(&fm);
        assert_eq!(tags, vec!["machine-learning", "tutorial"]);
    }

    #[test]
    fn test_extract_tags_single_tag_fallback() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "tag".to_string(),
            serde_yaml::Value::String("python".to_string()),
        );
        let tags = extract_tags(&fm);
        assert_eq!(tags, vec!["python"]);
    }

    #[test]
    fn test_extract_tags_string_instead_of_array() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "tags".to_string(),
            serde_yaml::Value::String("single-tag".to_string()),
        );
        let tags = extract_tags(&fm);
        assert_eq!(tags, vec!["single-tag"]);
    }

    #[test]
    fn test_extract_tags_none() {
        let fm = FrontMatter::new();
        let tags = extract_tags(&fm);
        assert!(tags.is_empty());
    }

    #[test]
    fn test_extract_tags_precedence_over_tag() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "tags".to_string(),
            serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("a".to_string())]),
        );
        fm.insert(
            "tag".to_string(),
            serde_yaml::Value::String("b".to_string()),
        );
        let tags = extract_tags(&fm);
        assert_eq!(tags, vec!["a"]);
    }

    #[test]
    fn test_extract_tags_empty_array() {
        let mut fm = FrontMatter::new();
        fm.insert("tags".to_string(), serde_yaml::Value::Sequence(vec![]));
        let tags = extract_tags(&fm);
        assert!(tags.is_empty());
    }

    // ========================================================================
    // Unit: page_url_suffix (Jekyll Utils.add_permalink_suffix for pages)
    // ========================================================================

    #[test]
    fn test_page_url_suffix_pretty() {
        assert_eq!(page_url_suffix("pretty"), "/");
    }

    #[test]
    fn test_page_url_suffix_date() {
        assert_eq!(page_url_suffix("date"), ".html");
    }

    #[test]
    fn test_page_url_suffix_ordinal() {
        assert_eq!(page_url_suffix("ordinal"), ".html");
    }

    #[test]
    fn test_page_url_suffix_none() {
        assert_eq!(page_url_suffix("none"), ".html");
    }

    #[test]
    fn test_page_url_suffix_custom_ending_slash() {
        // e.g. permalink: /:title/
        assert_eq!(page_url_suffix("/:title/"), "/");
    }

    #[test]
    fn test_page_url_suffix_custom_ending_output_ext() {
        // e.g. permalink: /:title:output_ext
        assert_eq!(page_url_suffix("/:title:output_ext"), ".html");
    }

    #[test]
    fn test_page_url_suffix_custom_ending_html() {
        // Jekyll pages always get .html extension regardless of permalink pattern
        assert_eq!(page_url_suffix("/blog/:title.html"), ".html");
    }

    #[test]
    fn test_page_url_suffix_collection_pattern() {
        assert_eq!(page_url_suffix("/:collection/:title.html"), ".html");
    }

    #[test]
    fn test_page_url_suffix_default_permalink() {
        // Default permalink is "date" which maps to .html suffix for pages
        assert_eq!(page_url_suffix("date"), ".html");
    }

    // ========================================================================
    // Unit: Standalone page URL generation (via load_pages)
    // ========================================================================

    #[test]
    fn test_page_url_no_permalink_fixture_config() {
        // Fixture has permalink: "/blog/:title.html" which ends with .html (not :output_ext).
        // Jekyll add_permalink_suffix adds no suffix for such patterns -> URL is "/events".
        let config = test_config();
        let (pages, _) = load_pages(&site_dir(), &config).unwrap();
        let events = pages.iter().find(|p| p.slug == "events");
        assert!(events.is_some(), "Should find events page in fixtures");
        assert_eq!(events.unwrap().url, "/events.html");
    }

    #[test]
    fn test_page_url_with_explicit_permalink() {
        // Create a temp dir with a page that has an explicit permalink
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("about.md"),
            "---\ntitle: About\npermalink: /about/\n---\nContent",
        )
        .unwrap();
        let config = SiteConfig {
            permalink: "/blog/:title.html".to_string(),
            ..SiteConfig::default()
        };
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/about/");
    }

    #[test]
    fn test_page_url_index_always_directory() {
        // index.md always gets "/" regardless of permalink style
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("index.md"),
            "---\ntitle: Home\n---\nWelcome",
        )
        .unwrap();
        let config = SiteConfig {
            permalink: "/blog/:title.html".to_string(),
            ..SiteConfig::default()
        };
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/");
    }

    #[test]
    fn test_page_url_pretty_permalink_gets_trailing_slash() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("about.md"),
            "---\ntitle: About\n---\nContent",
        )
        .unwrap();
        let config = SiteConfig {
            permalink: "pretty".to_string(),
            ..SiteConfig::default()
        };
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/about/");
    }

    #[test]
    fn test_page_url_date_permalink_gets_html() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("about.md"),
            "---\ntitle: About\n---\nContent",
        )
        .unwrap();
        let config = SiteConfig {
            permalink: "date".to_string(),
            ..SiteConfig::default()
        };
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/about.html");
    }

    #[test]
    fn test_page_url_custom_slash_permalink_gets_trailing_slash() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("contact.md"),
            "---\ntitle: Contact\n---\nContent",
        )
        .unwrap();
        let config = SiteConfig {
            permalink: "/:title/".to_string(),
            ..SiteConfig::default()
        };
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/contact/");
    }

    #[test]
    fn test_page_url_custom_html_no_suffix() {
        // permalink: /blog/:title.html -> no suffix per Jekyll add_permalink_suffix
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("articles.md"),
            "---\ntitle: Articles\n---\nContent",
        )
        .unwrap();
        let config = SiteConfig {
            permalink: "/blog/:title.html".to_string(),
            ..SiteConfig::default()
        };
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/articles");
    }

    #[test]
    fn test_page_url_subdir_index() {
        // Subdirectory index.md should get /subdir/
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("slack");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("index.md"), "---\ntitle: Slack\n---\nJoin").unwrap();
        let config = SiteConfig {
            permalink: "/blog/:title.html".to_string(),
            ..SiteConfig::default()
        };
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/slack/");
    }

    #[test]
    fn test_page_url_subdir_non_index() {
        // Subdirectory non-index page
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("slack");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(
            subdir.join("guidelines.md"),
            "---\ntitle: Guidelines\n---\nRules",
        )
        .unwrap();
        let config = SiteConfig {
            permalink: "/blog/:title.html".to_string(),
            ..SiteConfig::default()
        };
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        // Custom pattern ending in .html -> no suffix
        assert_eq!(pages[0].url, "/slack/guidelines");
    }

    // ========================================================================
    // Cyrillic / non-ASCII URL percent-encoding (issue #175)
    // ========================================================================

    #[test]
    fn test_page_url_cyrillic_subdir_percent_encoded() {
        // Pages in subdirectories with Cyrillic names get percent-encoded URLs
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("часть_1");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("index.md"), "---\ntitle: Part 1\n---\nContent").unwrap();
        let config = SiteConfig {
            permalink: "pretty".to_string(),
            ..SiteConfig::default()
        };
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        // Cyrillic directory name should be percent-encoded in the URL
        assert_eq!(pages[0].url, "/%D1%87%D0%B0%D1%81%D1%82%D1%8C_1/");
    }

    #[test]
    fn test_page_url_cyrillic_filename_percent_encoded() {
        // Pages with Cyrillic filenames get percent-encoded URLs
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("история.md"),
            "---\ntitle: History\n---\nContent",
        )
        .unwrap();
        let config = SiteConfig {
            permalink: "pretty".to_string(),
            ..SiteConfig::default()
        };
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/%D0%B8%D1%81%D1%82%D0%BE%D1%80%D0%B8%D1%8F/");
    }

    #[test]
    fn test_collection_item_cyrillic_url_percent_encoded() {
        // Collection items with Cyrillic slugs get percent-encoded URLs
        let dir = tempfile::tempdir().unwrap();
        let coll_dir = dir.path().join("_sections");
        std::fs::create_dir(&coll_dir).unwrap();
        std::fs::write(
            coll_dir.join("часть_1_история.md"),
            "---\ntitle: Part 1 History\n---\nContent",
        )
        .unwrap();

        let config = SiteConfig {
            collections: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    "sections".to_string(),
                    crate::config::CollectionConfig {
                        output: true,
                        permalink: "/:collection/:title/".to_string(),
                    },
                );
                m
            },
            ..SiteConfig::default()
        };

        let (items, _) = load_collection("sections", dir.path(), &config).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].url,
            "/sections/%D1%87%D0%B0%D1%81%D1%82%D1%8C_1_%D0%B8%D1%81%D1%82%D0%BE%D1%80%D0%B8%D1%8F/"
        );
    }

    #[test]
    fn test_sanitize_slug_leading_space() {
        assert_eq!(sanitize_slug(" aashishnair"), "aashishnair");
    }
    #[test]
    fn test_sanitize_slug_internal_space() {
        assert_eq!(
            sanitize_slug("production-ml-search-vector-search-embeddings-hybrid search"),
            "production-ml-search-vector-search-embeddings-hybrid-search"
        );
    }
    #[test]
    fn test_sanitize_slug_trailing_space() {
        assert_eq!(sanitize_slug("foo "), "foo");
    }
    #[test]
    fn test_sanitize_slug_normal_unchanged() {
        assert_eq!(sanitize_slug("johndoe"), "johndoe");
    }
    #[test]
    fn test_sanitize_slug_multiple_consecutive_spaces() {
        assert_eq!(sanitize_slug("a   b"), "a-b");
    }
    #[test]
    fn test_sanitize_slug_space_and_hyphen_collapsed() {
        assert_eq!(sanitize_slug("a - b"), "a-b");
    }
    // ========================================================================
    // Unit: Document ID preserves spaces (Issue 149)
    // ========================================================================

    /// Jekyll's document.id preserves spaces from the original filename for
    /// non-post collection items (e.g. `_podcast/hybrid search.md` produces
    /// id `/podcast/hybrid search`). The slug and URL have hyphens, but id
    /// keeps the raw filename stem.
    #[test]
    fn test_collection_item_id_preserves_spaces_in_filename() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let site = tmp.path();

        // Create _config.yml with a podcast collection
        std::fs::write(
            site.join("_config.yml"),
            "collections:\n  podcast:\n    output: true\n    permalink: /:collection/:title.html\n",
        )
        .unwrap();

        // Create a podcast item with a space in the filename
        let podcast_dir = site.join("_podcast");
        std::fs::create_dir_all(&podcast_dir).unwrap();
        std::fs::write(
            podcast_dir.join("hybrid search.md"),
            "---\ntitle: Hybrid Search\nseason: 1\nepisode: 1\n---\nContent\n",
        )
        .unwrap();

        let config = crate::config::SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, errors) = load_collection("podcast", site, &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        assert_eq!(items.len(), 1);

        let item = &items[0];
        // The slug should have hyphens (sanitized)
        assert_eq!(item.slug, "hybrid-search");
        // The URL should have hyphens
        assert_eq!(item.url, "/podcast/hybrid-search.html");
        // But the id should preserve the space from the raw filename
        assert_eq!(item.id, "/podcast/hybrid search");
    }

    /// For post collections, the id uses the date-based path format
    /// (e.g. `/2024/01/15/my-post`).
    #[test]
    fn test_post_item_id_uses_date_path() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let site = tmp.path();

        std::fs::write(site.join("_config.yml"), "permalink: /blog/:title.html\n").unwrap();

        let posts_dir = site.join("_posts");
        std::fs::create_dir_all(&posts_dir).unwrap();
        std::fs::write(
            posts_dir.join("2024-01-15-my-post.md"),
            "---\ntitle: My Post\n---\nContent\n",
        )
        .unwrap();

        let config = crate::config::SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, errors) = load_collection("posts", site, &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        assert_eq!(items.len(), 1);

        let item = &items[0];
        assert_eq!(item.id, "/2024/01/15/my-post");
    }

    #[test]
    fn test_load_pages_discovers_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("index.md"),
            "---\ntitle: Home\nlayout: page\n---\nHome",
        )
        .unwrap();
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(
            subdir.join("page.md"),
            "---\ntitle: Sub Page\nlayout: page\n---\nSub content",
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, errors) = load_pages(dir.path(), &config).unwrap();
        assert!(errors.is_empty());
        assert_eq!(pages.len(), 2);
        let sub_page = pages.iter().find(|p| p.slug == "page").unwrap();
        // Default permalink "date" -> .html suffix for pages
        assert_eq!(sub_page.url, "/subdir/page.html");
    }
    #[test]
    fn test_load_pages_skips_underscore_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("top.md"), "---\ntitle: Top\n---\nTop").unwrap();
        let hidden = dir.path().join("_hidden");
        std::fs::create_dir(&hidden).unwrap();
        std::fs::write(hidden.join("secret.md"), "---\ntitle: Secret\n---\nS").unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
    }
    #[test]
    fn test_load_pages_skips_excluded_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("top.md"), "---\ntitle: Top\n---\nTop").unwrap();
        let excluded = dir.path().join("scripts");
        std::fs::create_dir(&excluded).unwrap();
        std::fs::write(excluded.join("h.md"), "---\ntitle: H\n---\nH").unwrap();
        let config = SiteConfig {
            exclude: vec!["scripts/".to_string()],
            ..SiteConfig::default()
        };
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
    }
    #[test]
    fn test_published_false_skips_collection_item() {
        let dir = tempfile::tempdir().unwrap();
        let coll_dir = dir.path().join("_tools");
        std::fs::create_dir(&coll_dir).unwrap();
        std::fs::write(coll_dir.join("visible.md"), "---\ntitle: Visible\n---\nC").unwrap();
        std::fs::write(
            coll_dir.join("hidden.md"),
            "---\ntitle: Hidden\npublished: false\n---\nC",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("_config.yml"),
            "collections:\n  tools:\n    output: true\n    permalink: /:collection/:title.html\n",
        )
        .unwrap();
        let config = SiteConfig::from_file(&dir.path().join("_config.yml")).unwrap();
        let (items, errors) = load_collection("tools", dir.path(), &config).unwrap();
        assert!(errors.is_empty());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].slug, "visible");
    }
    #[test]
    fn test_published_false_skips_page() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("visible.md"), "---\ntitle: V\n---\nC").unwrap();
        std::fs::write(
            dir.path().join("hidden.md"),
            "---\ntitle: H\npublished: false\n---\nC",
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].slug, "visible");
    }
    #[test]
    fn test_published_true_not_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("v.md"),
            "---\ntitle: V\npublished: true\n---\nC",
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
    }

    // ========================================================================
    // Unit: Non-markdown files with front matter
    // ========================================================================

    #[test]
    fn test_load_pages_includes_xml_with_front_matter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.md"), "---\ntitle: Home\n---\nHi").unwrap();
        std::fs::write(
            dir.path().join("podcast.xml"),
            "---\nlayout: null\n---\n<rss>{{ site.title }}</rss>",
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 2);
        let xml_page = pages.iter().find(|p| p.slug == "podcast").unwrap();
        assert_eq!(xml_page.url, "/podcast.xml");
        assert_eq!(xml_page.source_path, "podcast.xml");
    }

    #[test]
    fn test_load_pages_skips_xml_without_front_matter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.md"), "---\ntitle: Home\n---\nHi").unwrap();
        std::fs::write(
            dir.path().join("data.xml"),
            "<data>plain xml without front matter</data>",
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].slug, "index");
    }

    #[test]
    fn test_load_pages_html_with_front_matter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("custom.html"),
            "---\nlayout: default\ntitle: Custom\n---\n<h1>Hello</h1>",
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].slug, "custom");
        assert_eq!(pages[0].url, "/custom.html");
    }

    #[test]
    fn test_load_pages_xml_content_not_markdown_converted() {
        let dir = tempfile::tempdir().unwrap();
        let xml_content = "<rss>{{ site.title }}</rss>";
        std::fs::write(
            dir.path().join("feed.xml"),
            format!("---\nlayout: null\n---\n{}", xml_content),
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        let page = &pages[0];
        // Non-markdown files should not have markdown-converted content
        assert_eq!(page.content, xml_content);
        assert_eq!(page.html_content, xml_content);
    }

    #[test]
    fn test_has_front_matter_true() {
        assert!(has_front_matter("---\ntitle: Test\n---\ncontent"));
    }

    #[test]
    fn test_has_front_matter_false_no_delimiters() {
        assert!(!has_front_matter("<xml>no front matter</xml>"));
    }

    #[test]
    fn test_has_front_matter_false_only_opening() {
        assert!(!has_front_matter(
            "---\ntitle: Test\ncontent without closing"
        ));
    }

    #[test]
    fn test_has_front_matter_with_bom() {
        assert!(has_front_matter("\u{feff}---\ntitle: Test\n---\ncontent"));
    }

    // -- Tests for build_timestamp / backfill_default_dates (issue 104) --

    #[test]
    fn test_build_timestamp_format() {
        let ts = build_timestamp();
        // Must match Jekyll's format: "YYYY-MM-DD HH:MM:SS +0000"
        assert!(
            ts.ends_with(" +0000"),
            "build_timestamp should end with ' +0000', got: {ts}"
        );
        // Length: "2026-03-15 12:30:45 +0000" = 25 chars
        assert_eq!(ts.len(), 25, "Unexpected timestamp length: {ts}");
        // Verify date portion is valid
        let date_part = &ts[..10];
        assert_eq!(
            date_part.matches('-').count(),
            2,
            "Date portion should have 2 dashes: {date_part}"
        );
    }

    #[test]
    fn test_backfill_default_dates_fills_missing() {
        let mut items = vec![CollectionItem {
            slug: "test-episode".to_string(),
            front_matter: FrontMatter::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/podcast/test-episode.html".to_string(),
            date: None,
            collection_name: "podcast".to_string(),
            source_path: "test-episode.md".to_string(),
            id: "/podcast/test-episode".to_string(),
        }];

        let build_time = "2026-03-15 10:30:00 +0000";
        backfill_default_dates(&mut items, build_time);

        assert_eq!(items[0].date.as_deref(), Some(build_time));
        // Also check front matter
        let fm_date = items[0]
            .front_matter
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(fm_date, build_time);
    }

    #[test]
    fn test_backfill_default_dates_preserves_existing() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "date".into(),
            serde_yaml::Value::String("2024-01-15".to_string()),
        );
        let mut items = vec![CollectionItem {
            slug: "my-post".to_string(),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/posts/my-post.html".to_string(),
            date: Some("2024-01-15".to_string()),
            collection_name: "posts".to_string(),
            source_path: "2024-01-15-my-post.md".to_string(),
            id: "/posts/my-post".to_string(),
        }];

        let build_time = "2026-03-15 10:30:00 +0000";
        backfill_default_dates(&mut items, build_time);

        // Should keep the original date
        assert_eq!(items[0].date.as_deref(), Some("2024-01-15"));
        let fm_date = items[0]
            .front_matter
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(fm_date, "2024-01-15");
    }

    #[test]
    fn test_backfill_default_dates_mixed_items() {
        let mut fm_with_date = FrontMatter::new();
        fm_with_date.insert(
            "date".into(),
            serde_yaml::Value::String("2023-06-01".to_string()),
        );

        let mut items = vec![
            CollectionItem {
                slug: "has-date".to_string(),
                front_matter: fm_with_date,
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                url: "/posts/has-date.html".to_string(),
                date: Some("2023-06-01".to_string()),
                collection_name: "posts".to_string(),
                source_path: "has-date.md".to_string(),
                id: "/posts/has-date".to_string(),
            },
            CollectionItem {
                slug: "no-date".to_string(),
                front_matter: FrontMatter::new(),
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                url: "/podcast/no-date.html".to_string(),
                date: None,
                collection_name: "podcast".to_string(),
                source_path: "no-date.md".to_string(),
                id: "/podcast/no-date".to_string(),
            },
        ];

        let build_time = "2026-03-15 10:30:00 +0000";
        backfill_default_dates(&mut items, build_time);

        assert_eq!(items[0].date.as_deref(), Some("2023-06-01"));
        assert_eq!(items[1].date.as_deref(), Some(build_time));
    }

    // ========================================================================
    // Unit: Page sort order (Issue 121)
    // ========================================================================

    /// Jekyll sorts site.pages by (basename, full URL). This test verifies
    /// that load_pages produces pages in that order by testing the sort logic
    /// directly on a Vec<Page>.
    #[test]
    fn test_pages_sorted_by_basename_then_url() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let site = tmp.path();

        // Create _config.yml
        std::fs::write(site.join("_config.yml"), "title: test\n").unwrap();

        // Create pages in subdirectories to test cross-directory sort order.
        // Jekyll sorts by filename first, so page-1.html < page-10.html < page-2.html
        // (string sort, not numeric), and pages from different dirs interleave by filename.
        let dir_a = site.join("docs").join("alpha");
        let dir_b = site.join("docs").join("beta");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();

        // page-1.md in alpha, page-10.md in beta, page-2.md in alpha
        std::fs::write(
            dir_a.join("page-1.md"),
            "---\ntitle: Alpha Page 1\n---\nContent",
        )
        .unwrap();
        std::fs::write(
            dir_b.join("page-10.md"),
            "---\ntitle: Beta Page 10\n---\nContent",
        )
        .unwrap();
        std::fs::write(
            dir_a.join("page-2.md"),
            "---\ntitle: Alpha Page 2\n---\nContent",
        )
        .unwrap();
        // Same filename in different dirs: beta/page-1.md should come after alpha/page-1.md
        std::fs::write(
            dir_b.join("page-1.md"),
            "---\ntitle: Beta Page 1\n---\nContent",
        )
        .unwrap();

        let config = SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (pages, errors) = load_pages(site, &config).unwrap();
        assert!(errors.is_empty(), "Should have no errors: {:?}", errors);

        let urls: Vec<&str> = pages.iter().map(|p| p.url.as_str()).collect();

        // Expected order: sort by basename first, then full URL
        // page-1.html (alpha) < page-1.html (beta) [same basename, alpha < beta]
        // page-10.html (beta) [next basename]
        // page-2.html (alpha) [next basename]
        assert_eq!(
            urls,
            vec![
                "/docs/alpha/page-1.html",
                "/docs/beta/page-1.html",
                "/docs/beta/page-10.html",
                "/docs/alpha/page-2.html",
            ],
            "Pages should be sorted by (basename, full URL) to match Jekyll"
        );
    }

    // ========================================================================
    // Issue 209: Default collection permalink has no .html extension
    // ========================================================================

    #[test]
    fn test_default_collection_permalink_no_html() {
        // Jekyll's default permalink for non-post collections is /:collection/:path
        // (no .html extension). Sites that explicitly set permalink in collection
        // config keep their setting; only the fallback default changes.
        let dir = tempfile::tempdir().unwrap();
        let site = dir.path();

        // Create a collection with NO explicit permalink in config
        std::fs::write(
            site.join("_config.yml"),
            "collections:\n  pages:\n    output: true\n",
        )
        .unwrap();
        std::fs::create_dir_all(site.join("_pages")).unwrap();
        std::fs::write(
            site.join("_pages/banners.md"),
            "---\ntitle: \"Баннеры и флаги\"\n---\nSome content about banners",
        )
        .unwrap();

        let config = SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, _) = load_collection("pages", site, &config).unwrap();
        assert_eq!(items.len(), 1);
        // Should be /pages/banners (no .html), matching Jekyll's /:collection/:path default
        assert_eq!(items[0].url, "/pages/banners");
    }

    #[test]
    fn test_explicit_collection_permalink_html_preserved() {
        // When a collection explicitly sets permalink: /:collection/:title.html,
        // that setting must be preserved (no regression for DTC site).
        let dir = tempfile::tempdir().unwrap();
        let site = dir.path();

        std::fs::write(
            site.join("_config.yml"),
            "collections:\n  books:\n    output: true\n    permalink: /:collection/:title.html\n",
        )
        .unwrap();
        std::fs::create_dir_all(site.join("_books")).unwrap();
        std::fs::write(
            site.join("_books/ml-bookcamp.md"),
            "---\ntitle: ML Bookcamp\n---\nContent",
        )
        .unwrap();

        let config = SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, _) = load_collection("books", site, &config).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].url, "/books/ml-bookcamp.html");
    }

    #[test]
    fn test_generate_url_collection_path_pattern() {
        // The /:collection/:path pattern should produce URLs without .html
        let url = generate_url("/:collection/:path", "notes", "2018-06-04-aa");
        assert_eq!(url, "/notes/2018-06-04-aa");
    }

    #[test]
    fn test_generate_url_collection_path_unicode() {
        // Unicode path stems should work with the :path pattern
        let ctx = PermalinkContext {
            collection: "pages".to_string(),
            title: "uber-uns".to_string(),
            source_path_stem: Some("über-uns".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/:collection/:path", &ctx);
        assert_eq!(url, "/pages/über-uns");
    }

    // ========================================================================
    // Unit: URL collision detection (Issue 225)
    // ========================================================================

    #[test]
    fn test_detect_url_collisions_finds_duplicate() {
        let entries = vec![
            (
                "_posts/2025-04-15-hello-world.md".to_string(),
                "/blog/hello-world.html".to_string(),
            ),
            (
                "_posts/2025-04-29-hello-world.md".to_string(),
                "/blog/hello-world.html".to_string(),
            ),
        ];
        let collisions = detect_url_collisions(&entries);
        assert_eq!(collisions.len(), 1);
        assert_eq!(collisions[0].url, "/blog/hello-world.html");
        assert_eq!(collisions[0].source_paths.len(), 2);
        assert!(collisions[0]
            .source_paths
            .contains(&"_posts/2025-04-15-hello-world.md".to_string()));
        assert!(collisions[0]
            .source_paths
            .contains(&"_posts/2025-04-29-hello-world.md".to_string()));
    }

    #[test]
    fn test_detect_url_collisions_no_false_positives() {
        let entries = vec![
            (
                "_posts/2025-04-15-hello.md".to_string(),
                "/blog/hello.html".to_string(),
            ),
            (
                "_posts/2025-04-29-world.md".to_string(),
                "/blog/world.html".to_string(),
            ),
            (
                "_posts/2025-05-01-foo.md".to_string(),
                "/blog/foo.html".to_string(),
            ),
        ];
        let collisions = detect_url_collisions(&entries);
        assert!(collisions.is_empty());
    }

    #[test]
    fn test_detect_url_collisions_three_way() {
        let entries = vec![
            ("_posts/a.md".to_string(), "/blog/same.html".to_string()),
            ("_posts/b.md".to_string(), "/blog/same.html".to_string()),
            ("_posts/c.md".to_string(), "/blog/same.html".to_string()),
        ];
        let collisions = detect_url_collisions(&entries);
        assert_eq!(collisions.len(), 1);
        assert_eq!(collisions[0].source_paths.len(), 3);
    }

    #[test]
    fn test_detect_url_collisions_multiple_independent() {
        let entries = vec![
            ("_posts/a1.md".to_string(), "/blog/alpha.html".to_string()),
            ("_posts/a2.md".to_string(), "/blog/alpha.html".to_string()),
            ("_posts/b1.md".to_string(), "/blog/beta.html".to_string()),
            ("_posts/b2.md".to_string(), "/blog/beta.html".to_string()),
            ("_posts/c1.md".to_string(), "/blog/gamma.html".to_string()),
        ];
        let collisions = detect_url_collisions(&entries);
        assert_eq!(collisions.len(), 2);
        let urls: Vec<&str> = collisions.iter().map(|c| c.url.as_str()).collect();
        assert!(urls.contains(&"/blog/alpha.html"));
        assert!(urls.contains(&"/blog/beta.html"));
    }

    #[test]
    fn test_detect_url_collisions_unicode_urls() {
        let entries = vec![
            ("_posts/a.md".to_string(), "/blog/über-uns.html".to_string()),
            ("_posts/b.md".to_string(), "/blog/über-uns.html".to_string()),
        ];
        let collisions = detect_url_collisions(&entries);
        assert_eq!(collisions.len(), 1);
        assert_eq!(collisions[0].url, "/blog/über-uns.html");
    }
}
