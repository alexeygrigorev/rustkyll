use std::fs;
use std::path::Path;

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
/// Supports `:collection`, `:title`, `:slug`, `:year`, `:month`, `:day`,
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

/// Returns true if the filename should be skipped (starts with `_` or is not `.md`).
fn should_skip(filename: &str) -> bool {
    filename.starts_with('_') || !filename.ends_with(".md")
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

    let entries = fs::read_dir(&dir).map_err(|e| CollectionError::ReadDir {
        path: dir.display().to_string(),
        source: e,
    })?;

    let permalink_pattern = if collection_name == "posts" {
        config.permalink.clone()
    } else {
        config
            .collection(collection_name)
            .map(|c| c.permalink.clone())
            .unwrap_or_else(|| "/:collection/:title.html".to_string())
    };

    let mut items = Vec::new();
    let mut errors = Vec::new();

    let mut file_paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    file_paths.sort();

    for path in file_paths {
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if should_skip(&filename) {
            continue;
        }

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

        // Skip items with `published: false` (matching Jekyll behavior)
        if is_published_false(&doc.front_matter) {
            continue;
        }

        let is_posts = collection_name == "posts";
        let stem = filename.strip_suffix(".md").unwrap_or(&filename);

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
            .strip_suffix(".md")
            .unwrap_or(&source_path)
            .strip_prefix(&format!("_{}/", collection_name))
            .unwrap_or(source_path.strip_suffix(".md").unwrap_or(&source_path))
            .to_string();

        let ctx = PermalinkContext {
            collection: collection_name.to_string(),
            title: slug.clone(),
            date: date.clone(),
            categories,
            source_path_stem: Some(source_path_stem),
        };
        let url = generate_url_with_context(&permalink_pattern, &ctx);
        let html_content = frontmatter::markdown_to_html(&doc.content);

        items.push(CollectionItem {
            slug,
            front_matter: doc.front_matter,
            content: doc.content,
            html_content,
            excerpt: doc.excerpt,
            url,
            date,
            collection_name: collection_name.to_string(),
            source_path,
        });
    }

    Ok((items, errors))
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
    [".md", ".xml", ".html", ".htm", ".json", ".txt"]
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
    // Look for closing --- on its own line
    rest.contains("\n---")
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

        // For non-markdown files, only process if they have front matter
        // (matching Jekyll behavior: only files starting with --- are processed)
        if !is_markdown && !has_front_matter(&raw) {
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
                    format!("/{}", rel_path)
                }
            });

        let html_content = if is_markdown {
            frontmatter::markdown_to_html(&doc.content)
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
    fn test_should_skip_non_md() {
        assert!(should_skip("file.txt"));
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
        assert_eq!(page_url_suffix("/:title.html"), ".html");
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
        // Default permalink "/:title.html" ends in .html -> no suffix for pages
        assert_eq!(sub_page.url, "/subdir/page");
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
}
