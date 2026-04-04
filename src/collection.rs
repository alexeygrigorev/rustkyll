use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rayon::prelude::*;

use crate::config::SiteConfig;
use crate::frontmatter::{self, FrontMatter};

/// Global permalink style for standalone pages. Used by `{% link %}` tag
/// preprocessing to generate correct URLs (pretty = `/`, default = `.html`).
static PAGE_PERMALINK_STYLE: Mutex<String> = Mutex::new(String::new());

/// Per-collection URL suffix for `{% link %}` tag preprocessing.
/// Maps collection name (e.g., "docs") to its URL suffix ("/" or "").
/// When a collection's permalink pattern ends with `/`, links to its documents
/// should include a trailing slash.
static COLLECTION_PERMALINK_SUFFIXES: Mutex<Option<HashMap<String, &'static str>>> =
    Mutex::new(None);

/// Set the global page permalink style. Must be called before any template
/// preprocessing occurs (typically from `main.rs` after loading config).
pub fn set_page_permalink_style(style: &str) {
    if let Ok(mut guard) = PAGE_PERMALINK_STYLE.lock() {
        *guard = style.to_string();
    }
}

/// Get the URL suffix for the `{% link %}` tag based on the globally configured
/// page permalink style. Returns `"/"` for pretty permalinks, `".html"` otherwise.
pub fn get_link_tag_suffix() -> &'static str {
    let style = PAGE_PERMALINK_STYLE
        .lock()
        .ok()
        .filter(|s| !s.is_empty())
        .map(|s| s.clone())
        .unwrap_or_default();
    page_url_suffix(&style)
}

/// Set the URL suffix for a specific collection's `{% link %}` tag resolution.
/// `suffix` should be `"/"` when the collection permalink ends with `/`,
/// or `""` otherwise (extensionless).
pub fn set_collection_permalink_suffix(collection_name: &str, suffix: &'static str) {
    if let Ok(mut guard) = COLLECTION_PERMALINK_SUFFIXES.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(collection_name.to_string(), suffix);
    }
}

/// Get the URL suffix for a collection's `{% link %}` tag, or `None` if no
/// collection-specific suffix has been configured (falls back to extensionless).
pub fn get_collection_link_suffix(collection_name: &str) -> Option<&'static str> {
    COLLECTION_PERMALINK_SUFFIXES.lock().ok().and_then(|guard| {
        guard
            .as_ref()
            .and_then(|map| map.get(collection_name).copied())
    })
}

/// Clear all collection permalink suffixes (used in tests).
pub fn clear_collection_permalink_suffixes() {
    if let Ok(mut guard) = COLLECTION_PERMALINK_SUFFIXES.lock() {
        *guard = None;
    }
}

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

    /// Content before `<!--more-->` separator, if present (raw markdown).
    pub excerpt: Option<String>,

    /// Pre-rendered HTML version of the excerpt.
    /// Computed once during collection loading to avoid redundant markdown_to_html
    /// calls during page generation (which was a major performance bottleneck).
    pub excerpt_html: Option<String>,

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

/// Pre-render `{% highlight lang %}...{% endhighlight %}` blocks in markdown content
/// before markdown conversion. Jekyll processes Liquid tags before markdown, but
/// rustkyll pre-computes `html_content` before the Liquid engine runs. This function
/// handles just the highlight blocks so they produce proper `<figure>` HTML instead
/// of being mangled by the markdown parser into `<p>` tags.
pub fn pre_render_highlight_blocks(content: &str) -> String {
    // Fast path: no highlight tags present
    if !content.contains("{% highlight") {
        return content.to_string();
    }

    // Build a list of protected byte ranges (raw blocks and fenced code blocks)
    // so we can skip {% highlight %} tags that fall inside them.
    let protected = find_protected_ranges(content);

    let mut result = String::with_capacity(content.len());
    let mut remaining = content;
    let mut offset = 0usize; // tracks byte offset into original content

    while let Some(rel_pos) = remaining.find("{% highlight ") {
        let abs_pos = offset + rel_pos;

        // Check if this {% highlight %} is inside a protected range
        if is_in_protected_range(abs_pos, &protected) {
            // Don't process -- copy up to and past this tag text, advance
            result.push_str(&remaining[..rel_pos + "{% highlight ".len()]);
            remaining = &remaining[rel_pos + "{% highlight ".len()..];
            offset = abs_pos + "{% highlight ".len();
            continue;
        }

        // Push everything before the tag
        result.push_str(&remaining[..rel_pos]);

        let after_open = &remaining[rel_pos + "{% highlight ".len()..];
        let after_open_offset = abs_pos + "{% highlight ".len();

        // Find the closing %} of the opening tag
        if let Some(close_pct) = after_open.find("%}") {
            let tag_args = after_open[..close_pct].trim();
            // Extract language (first word, ignore linenos and other params)
            let lang = tag_args.split_whitespace().next().unwrap_or("text");

            // Extract hl_lines parameter if present (e.g. hl_lines="1 3 5")
            let hl_lines = parse_hl_lines_from_tag_args(tag_args);

            let after_open_tag = &after_open[close_pct + 2..];
            let after_open_tag_offset = after_open_offset + close_pct + 2;

            // Find the matching {% endhighlight %} that is NOT in a protected range
            let mut search_from = 0usize;
            let mut found_end = None;
            while let Some(ep) = after_open_tag[search_from..].find("{% endhighlight %}") {
                let candidate_abs = after_open_tag_offset + search_from + ep;
                if is_in_protected_range(candidate_abs, &protected) {
                    // Skip this one, keep searching
                    search_from += ep + "{% endhighlight %}".len();
                } else {
                    found_end = Some(search_from + ep);
                    break;
                }
            }

            if let Some(end_pos) = found_end {
                let body = &after_open_tag[..end_pos];
                // Strip leading/trailing newline matching Jekyll behavior
                let body = body.strip_prefix('\n').unwrap_or(body);
                let body = body.strip_suffix('\n').unwrap_or(body);

                // Render the highlight block using the same logic as the Liquid tag.
                let escaped_lang = html_escape_highlight(lang);
                let mut figure = format!(
                    "<figure class=\"highlight\"><pre><code class=\"language-{}\" data-lang=\"{}\">",
                    escaped_lang, escaped_lang
                );
                let raw_content =
                    if let Some(highlighted) = crate::syntax::highlight_code(lang, body) {
                        highlighted
                    } else {
                        html_escape_highlight(body)
                    };
                let content =
                    crate::template::highlight_tag::wrap_hl_lines(&raw_content, &hl_lines);
                figure.push_str(&content);
                figure.push_str("</code></pre></figure>");
                // Collapse blank lines so markdown parser treats entire figure as
                // one HTML block (CommonMark type-6 blocks end at blank lines).
                while figure.contains("\r\n\r\n") {
                    figure = figure.replace("\r\n\r\n", "\r\n");
                }
                while figure.contains("\n\n") {
                    figure = figure.replace("\n\n", "\n");
                }
                result.push_str(&figure);

                let new_remaining_rel = end_pos + "{% endhighlight %}".len();
                remaining = &after_open_tag[new_remaining_rel..];
                offset = after_open_tag_offset + new_remaining_rel;
            } else {
                // No matching endhighlight, keep as-is
                result
                    .push_str(&remaining[rel_pos..rel_pos + "{% highlight ".len() + close_pct + 2]);
                remaining = &after_open[close_pct + 2..];
                offset = after_open_offset + close_pct + 2;
            }
        } else {
            // No closing %}, keep as-is
            result.push_str(&remaining[rel_pos..]);
            remaining = "";
            offset = content.len();
        }
    }

    result.push_str(remaining);
    result
}

/// Find byte ranges in `content` that are protected: `{% raw %}...{% endraw %}`
/// blocks and fenced code blocks (``` or ~~~). Returns sorted, non-overlapping
/// `(start, end)` byte ranges.
fn find_protected_ranges(content: &str) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    while pos < len {
        // Check for {% raw %} blocks
        if bytes[pos] == b'{' && content[pos..].starts_with("{%") {
            if let Some(tag_end) = content[pos..].find("%}") {
                let tag_inner = content[pos + 2..pos + tag_end].trim();
                if tag_inner == "raw" {
                    let after_tag = pos + tag_end + 2;
                    if let Some(end_offset) = content[after_tag..].find("{% endraw %}") {
                        let block_end = after_tag + end_offset + "{% endraw %}".len();
                        ranges.push((pos, block_end));
                        pos = block_end;
                        continue;
                    }
                }
            }
        }

        // Check for fenced code blocks (``` or ~~~) at start of line
        if (pos == 0 || bytes[pos - 1] == b'\n')
            && pos + 3 <= len
            && (content[pos..].starts_with("```") || content[pos..].starts_with("~~~"))
        {
            let fence_char = bytes[pos];
            let mut fence_len = 0;
            while pos + fence_len < len && bytes[pos + fence_len] == fence_char {
                fence_len += 1;
            }
            if fence_len >= 3 {
                let block_start = pos;
                let line_end = content[pos + fence_len..]
                    .find('\n')
                    .map(|i| pos + fence_len + i + 1)
                    .unwrap_or(len);
                let mut search_pos = line_end;
                let mut found_end = None;
                while search_pos < len {
                    if bytes[search_pos] == fence_char {
                        let mut close_len = 0;
                        while search_pos + close_len < len
                            && bytes[search_pos + close_len] == fence_char
                        {
                            close_len += 1;
                        }
                        if close_len >= fence_len {
                            let close_line_end = content[search_pos + close_len..]
                                .find('\n')
                                .map(|i| search_pos + close_len + i + 1)
                                .unwrap_or(search_pos + close_len);
                            found_end = Some(close_line_end);
                            break;
                        }
                    }
                    if let Some(nl) = content[search_pos..].find('\n') {
                        search_pos += nl + 1;
                    } else {
                        break;
                    }
                }
                if let Some(block_end) = found_end {
                    ranges.push((block_start, block_end));
                    pos = block_end;
                    continue;
                }
            }
        }

        pos += 1;
    }

    ranges
}

/// Check whether a byte position falls inside any protected range.
fn is_in_protected_range(pos: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| pos >= *start && pos < *end)
}

/// Parse `hl_lines="1 3 5"` from raw tag arguments string.
/// Returns empty vec if no hl_lines parameter is found.
fn parse_hl_lines_from_tag_args(tag_args: &str) -> Vec<usize> {
    // Look for hl_lines="..." pattern in the raw tag arguments
    if let Some(start) = tag_args.find("hl_lines=") {
        let after_eq = &tag_args[start + "hl_lines=".len()..];
        // The value is quoted: hl_lines="1 3 5"
        if let Some(after_eq) = after_eq.strip_prefix('"') {
            if let Some(end_quote) = after_eq.find('"') {
                let value = &after_eq[..end_quote];
                return value
                    .split_whitespace()
                    .filter_map(|s| s.parse::<usize>().ok())
                    .collect();
            }
        }
    }
    Vec::new()
}

/// HTML-escape for highlight pre-rendering (same logic as highlight_tag.rs).
fn html_escape_highlight(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            _ => result.push(c),
        }
    }
    result
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

    // Jekyll always lowercases categories in URLs (Issue 354)
    let categories_str = ctx
        .categories
        .iter()
        .map(|c| c.to_lowercase())
        .collect::<Vec<_>>()
        .join("/");
    let path_str = ctx.source_path_stem.as_deref().unwrap_or(&ctx.title);

    // Issue 548: Jekyll separates the URL from the output file path.
    // :output_ext in the permalink pattern determines the OUTPUT FILE extension
    // (.html), but item.url strips :output_ext, returning the URL without .html.
    // Example: permalink /:collection/:path:output_ext
    //   -> item.url = /notes/slug (no .html)
    //   -> output file = notes/slug.html (url_to_output_path adds .html)
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
        .replace(":path", path_str)
        .replace(":output_ext", "");

    // Collapse double (or more) slashes to single, preserving leading slash
    while url.contains("//") {
        url = url.replace("//", "/");
    }

    // Issue 557: Jekyll does NOT append a trailing slash to permalink patterns
    // that lack an extension. The URL stays as-is (e.g., /stories/foo) and
    // url_to_output_path converts it to stories/foo.html.
    // Only patterns that explicitly end with / (like "pretty") get trailing slash.

    url
}

/// Check if a URL path ends with a recognized file extension.
///
/// Used by `generate_url_with_context` to determine whether to append a
/// trailing slash for pretty URL generation.
pub fn url_has_extension(url: &str) -> bool {
    // Get the last path segment
    let last_segment = url.rsplit('/').next().unwrap_or(url);
    if let Some(dot_pos) = last_segment.rfind('.') {
        // There's a dot -- check if the extension part is non-empty
        let ext = &last_segment[dot_pos..];
        ext.len() > 1
    } else {
        false
    }
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
                // Jekyll splits space-separated strings into multiple categories.
                // e.g. "classics crime mystery" -> ["classics", "crime", "mystery"]
                if !s.is_empty() {
                    let parts: Vec<String> = s.split_whitespace().map(|p| p.to_string()).collect();
                    return parts;
                }
            }
            _ => {}
        }
    }

    // Fall back to `category` (single string -- never split)
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
                // Jekyll splits space-separated strings into multiple tags.
                // e.g. "formatting audios" -> ["formatting", "audios"]
                if !s.is_empty() {
                    let parts: Vec<String> = s.split_whitespace().map(|p| p.to_string()).collect();
                    return parts;
                }
            }
            _ => {}
        }
    }

    // Fall back to `tag` (single string -- never split)
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
pub fn build_timestamp(site_tz: Option<chrono_tz::Tz>) -> String {
    match site_tz {
        Some(tz) => {
            use chrono::Offset;
            let now = chrono::Utc::now().with_timezone(&tz);
            let offset = now.offset().fix();
            let total_secs = offset.local_minus_utc();
            let sign = if total_secs >= 0 { '+' } else { '-' };
            let abs_secs = total_secs.unsigned_abs();
            let hours = abs_secs / 3600;
            let minutes = (abs_secs % 3600) / 60;
            format!(
                "{} {}{:02}{:02}",
                now.format("%Y-%m-%d %H:%M:%S"),
                sign,
                hours,
                minutes
            )
        }
        None => chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S +0000")
            .to_string(),
    }
}

/// Fill in a default date for collection items that have no date.
///
/// Jekyll gives every collection item a `date` -- when no explicit date is
/// specified in front matter or the filename, it defaults to the build
/// timestamp (`site.time`).  This function replicates that behaviour so
/// that collection iteration (e.g. `site.podcast | map: "date"`) produces
/// a value even for items without an explicit date.
///
/// The same `build_time` string should be used for every item within a
/// single build to match Jekyll's semantics.
///
/// When `set_frontmatter` is true, the backfilled date is also written to
/// `item.front_matter["date"]`, making it visible as `page.date` in
/// templates.  Jekyll only does this for posts; non-post collections have
/// `page.date = nil` unless explicitly set in front matter or filename.
/// (Issue 474)
pub fn backfill_default_dates(
    items: &mut [CollectionItem],
    build_time: &str,
    set_frontmatter: bool,
) {
    for item in items.iter_mut() {
        if item.date.is_none() {
            item.date = Some(build_time.to_string());
            if set_frontmatter {
                // Also add to front matter so that `page.date` is available in templates
                item.front_matter
                    .entry("date".to_string())
                    .or_insert_with(|| serde_yaml::Value::String(build_time.to_string()));
            }
        }
    }
}

/// Filter out future-dated posts when `future` is not enabled.
///
/// Jekyll defaults to `future: false`, meaning posts with dates after the
/// current build time are excluded from the site. When `allow_future` is
/// true (from `future: true` in `_config.yml`), no filtering is done.
///
/// Only the date portion (YYYY-MM-DD) is compared, matching Jekyll's behavior.
pub fn filter_future_posts(items: &mut Vec<CollectionItem>, allow_future: bool) {
    if allow_future {
        return;
    }
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    items.retain(|item| {
        if let Some(ref date) = item.date {
            // Compare only the date portion (first 10 chars: YYYY-MM-DD)
            let date_part = if date.len() >= 10 {
                &date[..10]
            } else {
                date.as_str()
            };
            date_part <= today.as_str()
        } else {
            // Items without dates are always included
            true
        }
    });
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

/// Extract a float from a serde_yaml::Value for numeric sorting.
fn yaml_value_to_f64(val: &serde_yaml::Value) -> Option<f64> {
    match val {
        serde_yaml::Value::Number(n) => n.as_f64(),
        serde_yaml::Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// Convert a serde_yaml::Value to a string for comparison.
fn yaml_value_to_string(val: &serde_yaml::Value) -> String {
    match val {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => String::new(),
        _ => format!("{:?}", val),
    }
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
        // Check collection config first, then defaults, then fallback
        config
            .collection(collection_name)
            .map(|c| c.permalink.clone())
            .filter(|p| !p.is_empty())
            .or_else(|| {
                // Check defaults for this collection type
                let defaults = config.defaults_for(collection_name, "");
                defaults
                    .get("permalink")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "/:collection/:path:output_ext".to_string())
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

    // Issue 294: Check if autolink extension is enabled in commonmark.extensions.
    let enable_autolink = config.has_commonmark_autolink();

    // Issue 358: Extract site-level excerpt_separator from config.
    let site_excerpt_separator = config
        .extras
        .get("excerpt_separator")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

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
                enable_autolink,
                site_excerpt_separator.as_deref(),
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

    // Check if this collection has a custom sort_by field configured.
    let sort_by_field = config
        .collection(collection_name)
        .and_then(|c| c.sort_by.as_deref())
        .map(|s| s.to_string());

    if let Some(ref field) = sort_by_field {
        // Sort by the specified front matter field. Items without the field
        // sort to the end, matching Jekyll's behavior for sort_by.
        items.sort_by(|a, b| {
            let val_a = a.front_matter.get(field);
            let val_b = b.front_matter.get(field);
            match (val_a, val_b) {
                (Some(va), Some(vb)) => {
                    // Try numeric comparison first, then string
                    let num_a = yaml_value_to_f64(va);
                    let num_b = yaml_value_to_f64(vb);
                    match (num_a, num_b) {
                        (Some(na), Some(nb)) => {
                            na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        _ => {
                            let sa = yaml_value_to_string(va);
                            let sb = yaml_value_to_string(vb);
                            sa.cmp(&sb)
                        }
                    }
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.source_path.cmp(&b.source_path),
            }
        });
    } else if collection_name == "posts" {
        // Posts are ordered by date ascending (oldest first) before site.posts
        // gets reversed to newest-first in the site context.
        items.sort_by(|a, b| {
            let date_a = a.date.as_deref().unwrap_or("");
            let date_b = b.date.as_deref().unwrap_or("");
            date_a
                .cmp(date_b)
                .then_with(|| a.source_path.cmp(&b.source_path))
        });
    } else {
        // For non-post collections, preserve source-path order unless the site
        // explicitly configures `sort_by`. Jekyll exposes collections in their
        // natural document order, and Liquid `sort` relies on that input order
        // being stable for equal-key values.
    }

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
#[allow(clippy::too_many_arguments)]
fn process_collection_file(
    path: &Path,
    _collection_dir: &Path,
    site_dir: &Path,
    collection_name: &str,
    permalink_pattern: &str,
    add_code_classes: bool,
    enable_hardbreaks: bool,
    enable_autolink: bool,
    site_excerpt_separator: Option<&str>,
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

    let doc = match frontmatter::parse_document_with_separator(&raw, site_excerpt_separator) {
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

    // Use front matter `permalink` if present, otherwise use the pattern.
    // Ensure permalink always starts with `/` to match Jekyll's behavior.
    let url = doc
        .front_matter
        .get("permalink")
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.starts_with('/') {
                s.to_string()
            } else {
                format!("/{}", s)
            }
        })
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
            // Replace the trailing .html with the source file's extension,
            // or append it if :output_ext was stripped (leaving no extension).
            if !is_markdown && ext != ".html" && ext != ".htm" {
                if let Some(stripped) = generated.strip_suffix(".html") {
                    generated = format!("{}{}", stripped, ext);
                } else if permalink_pattern.contains(":output_ext") {
                    // :output_ext was replaced with "" in generate_url_with_context,
                    // so the URL has no extension. Append the source file's extension
                    // so url_to_output_path produces the correct output file.
                    generated = format!("{}{}", generated, ext);
                }
            }
            generated
        });

    // Percent-encode non-ASCII characters in URLs, matching Jekyll behavior.
    // Jekyll outputs percent-encoded URLs for Cyrillic and other non-ASCII chars.
    let url = crate::template::filters::relative_url::encode_url_path(&url);

    let has_non_highlight_liquid = contains_non_highlight_liquid(&doc.content);

    let html_content = if is_markdown && has_non_highlight_liquid {
        // Markdown documents with Liquid are rendered again during page
        // generation with the full site context. Avoid paying the markdown
        // conversion cost twice up front.
        String::new()
    } else if is_markdown {
        // Pre-render {% highlight %}...{% endhighlight %} blocks before markdown
        // conversion, matching Jekyll's Liquid-before-markdown order of operations.
        let preprocessed = pre_render_highlight_blocks(&doc.content);
        frontmatter::markdown_to_html_with_options(
            &preprocessed,
            add_code_classes,
            add_code_classes,
            enable_hardbreaks,
            enable_autolink,
        )
    } else {
        doc.content.clone()
    };

    // Compute Jekyll-compatible document.id.
    // Jekyll: Document#id = File.join(File.dirname(url), slug)
    // For non-post collections: /<collection>/<raw_stem> (preserves spaces from filename).
    // For posts: dirname(url) + raw_slug (respects permalink pattern).
    let id = if is_posts {
        let (_, raw_slug) = parse_post_filename(&filename);
        // Get the directory portion of the resolved URL, matching Jekyll's
        // File.dirname(url). For `/blog/my-post.html` -> `/blog`.
        // For `/stories/my-post/` -> `/stories` (strip trailing slash first).
        // Ruby's File.dirname strips trailing slashes before computing dirname.
        let url_for_dirname = url.trim_end_matches('/');
        let url_dir = if let Some(pos) = url_for_dirname.rfind('/') {
            if pos == 0 {
                "/".to_string()
            } else {
                url_for_dirname[..pos].to_string()
            }
        } else {
            "/".to_string()
        };
        if url_dir == "/" {
            format!("/{}", raw_slug)
        } else {
            format!("{}/{}", url_dir, raw_slug)
        }
    } else {
        // For non-post collections, id preserves raw filename (including spaces)
        format!("/{}/{}", collection_name, stem)
    };

    let excerpt_html = doc.excerpt.as_ref().and_then(|e| {
        if e.is_empty() {
            None
        } else {
            // If the excerpt contains Liquid tags (e.g. {% highlight %}),
            // process them through the Liquid engine first, then markdown.
            // This ensures syntax-highlighted code blocks render properly
            // in post excerpts on index/listing pages (issue 300).
            let processed = if e.contains("{%") || e.contains("{{") {
                if let Ok(engine) = crate::template::TemplateEngine::new() {
                    let ctx = liquid::Object::new();
                    engine
                        .parse_and_render(e, &ctx)
                        .unwrap_or_else(|_| e.clone())
                } else {
                    e.clone()
                }
            } else {
                e.clone()
            };
            Some(crate::frontmatter::markdown_to_html(&processed))
        }
    });

    Some(Ok(CollectionItem {
        slug,
        front_matter: doc.front_matter,
        content: doc.content,
        html_content,
        excerpt: doc.excerpt,
        excerpt_html,
        url,
        date,
        collection_name: collection_name.to_string(),
        source_path,
        id,
    }))
}

fn contains_non_highlight_liquid(content: &str) -> bool {
    if content.contains("{{") {
        return true;
    }

    let mut remaining = content;
    while let Some(pos) = remaining.find("{%") {
        let after = &remaining[pos + 2..];
        let trimmed = after.trim_start_matches([' ', '\t', '\n', '\r', '-']);
        if !(trimmed.starts_with("highlight") || trimmed.starts_with("endhighlight")) {
            return true;
        }
        remaining = &after["".len()..];
    }

    false
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

    let mut file_paths = Vec::new();
    collect_page_paths(site_dir, site_dir, config, &mut file_paths)?;

    let add_code_classes = config
        .extras
        .get("markdown")
        .and_then(|v| v.as_str())
        .map(|m| m.eq_ignore_ascii_case("kramdown"))
        .unwrap_or(true);
    let enable_hardbreaks = has_commonmark_hardbreaks(config);
    let enable_autolink = config.has_commonmark_autolink();

    let processed: Vec<_> = file_paths
        .par_iter()
        .filter_map(|path| {
            process_page_file(
                path,
                site_dir,
                config,
                add_code_classes,
                enable_hardbreaks,
                enable_autolink,
            )
        })
        .collect();

    let mut pages = Vec::new();
    let mut errors = Vec::new();
    for result in processed {
        match result {
            Ok(page) => pages.push(page),
            Err(err) => errors.push(err),
        }
    }

    // Sort pages by filename (page.name) to match Jekyll's site.pages order.
    // Jekyll sorts pages by filename rather than by full path. This means
    // `main.scss` (from `assets/css/`) sorts between `cv.md` and `markdown.md`
    // (both from `_pages/`), matching Jekyll's interleaved ordering.
    pages.sort_by(|a, b| {
        let name_a = a.source_path.rsplit('/').next().unwrap_or(&a.source_path);
        let name_b = b.source_path.rsplit('/').next().unwrap_or(&b.source_path);
        name_a
            .cmp(name_b)
            .then_with(|| a.source_path.cmp(&b.source_path))
    });

    Ok((pages, errors))
}

/// Check if a directory name should be skipped during page discovery.
fn should_skip_directory(name: &str, config: &SiteConfig) -> bool {
    // Check include list first -- force-included directories are never skipped.
    // This matches Jekyll's behavior where `include:` overrides the default
    // underscore/dot directory exclusion.
    for included in &config.include {
        let included_name = included.trim_end_matches('/');
        if name == included_name {
            return false;
        }
    }
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

/// Recursively collect page file paths before parallel processing.
fn collect_page_paths(
    current_dir: &Path,
    site_dir: &Path,
    config: &SiteConfig,
    file_paths: &mut Vec<PathBuf>,
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
                collect_page_paths(&path, site_dir, config, file_paths)?;
            }
            continue;
        }

        if !path.is_file() {
            continue;
        }

        let _ext = match is_processable_extension(&name) {
            Some(ext) => ext,
            None => continue,
        };

        // Skip underscore-prefixed files.
        // README.md in subdirectories is kept (Jekyll's jekyll-readme-index
        // converts them to index.html). Root-level README.md is still skipped
        // because it's typically the project README, not a site page.
        let is_readme = name == "README.md";
        let is_root_level = path.parent() == Some(site_dir);
        if name.starts_with('_') || (is_readme && is_root_level) {
            continue;
        }

        file_paths.push(path);
    }

    Ok(())
}

fn process_page_file(
    path: &Path,
    site_dir: &Path,
    config: &SiteConfig,
    add_code_classes: bool,
    enable_hardbreaks: bool,
    enable_autolink: bool,
) -> Option<Result<Page, CollectionError>> {
    let name = path.file_name().and_then(|n| n.to_str())?.to_string();
    let ext = is_processable_extension(&name)?;
    let is_markdown = ext == ".md";
    let is_readme = name == "README.md";

    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            return Some(Err(CollectionError::ReadFile {
                path: path.display().to_string(),
                source: e,
            }));
        }
    };

    // Jekyll only processes files that have YAML front matter (starting with ---)
    // This applies to all file types including .md files.
    // Exception: README.md files are processed even without front matter
    // if config defaults target that path specifically (matching Jekyll's
    // behavior when README.md is explicitly configured via defaults in
    // _config.yml). A catch-all default (type: pages, path: "") does NOT
    // cause README.md to be discovered -- only path-specific defaults do.
    if !has_front_matter(&raw) {
        if is_readme {
            let rel = path.strip_prefix(site_dir).unwrap_or(path);
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let has_path_specific_default = config
                .defaults
                .iter()
                .any(|d| !d.scope.path.is_empty() && rel_str.starts_with(&d.scope.path));
            if !has_path_specific_default {
                return None;
            }
        } else {
            return None;
        }
    }

    let doc = if has_front_matter(&raw) {
        match frontmatter::parse_document(&raw) {
            Ok(doc) => doc,
            Err(e) => {
                return Some(Err(CollectionError::Parse {
                    path: path.display().to_string(),
                    source: e,
                }));
            }
        }
    } else {
        frontmatter::Document {
            front_matter: FrontMatter::new(),
            content: raw.clone(),
            excerpt: None,
        }
    };

    if is_published_false(&doc.front_matter) {
        return None;
    }

    let rel_path = path
        .strip_prefix(site_dir)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));

    let stem = name.strip_suffix(ext).unwrap_or(&name);

    let url = doc
        .front_matter
        .get("permalink")
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.starts_with('/') {
                s.to_string()
            } else {
                format!("/{}", s)
            }
        })
        .unwrap_or_else(|| {
            if is_markdown {
                let rel_stem = rel_path.strip_suffix(".md").unwrap_or(&rel_path);
                if stem == "index" || stem == "README" {
                    let dir = rel_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
                    if dir.is_empty() {
                        "/".to_string()
                    } else {
                        format!("/{}/", dir)
                    }
                } else {
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
            } else if stem == "index" && ext == ".html" {
                let dir = rel_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
                if dir.is_empty() {
                    "/".to_string()
                } else {
                    format!("/{}/", dir)
                }
            } else if rel_path.ends_with(".scss") {
                format!("/{}css", rel_path.strip_suffix("scss").unwrap_or(&rel_path))
            } else {
                format!("/{}", rel_path)
            }
        });

    let url = crate::template::filters::relative_url::encode_url_path(&url);

    let html_content = if is_markdown {
        let preprocessed = pre_render_highlight_blocks(&doc.content);
        frontmatter::markdown_to_html_with_options(
            &preprocessed,
            add_code_classes,
            add_code_classes,
            enable_hardbreaks,
            enable_autolink,
        )
    } else {
        doc.content.clone()
    };

    Some(Ok(Page {
        slug: stem.to_string(),
        front_matter: doc.front_matter,
        content: doc.content,
        html_content,
        url,
        source_path: rel_path,
    }))
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
    // Issue 347: Pretty permalink URL generation (no .html extension)
    // ========================================================================

    #[test]
    fn test_permalink_title_no_ext_no_trailing_slash_347() {
        // Issue 557: permalink: /:title -> URL should be /my-post (NO trailing slash)
        // Jekyll does not auto-append trailing slash to patterns without extension
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2024-01-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/:title", &ctx);
        assert_eq!(url, "/my-post");
    }

    #[test]
    fn test_permalink_categories_title_no_ext_no_trailing_slash_347() {
        // Issue 557: permalink: /:categories/:title -> URL should be /tech/intro (NO trailing slash)
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "intro".to_string(),
            date: Some("2024-01-15".to_string()),
            categories: vec!["tech".to_string()],
            ..Default::default()
        };
        let url = generate_url_with_context("/:categories/:title", &ctx);
        assert_eq!(url, "/tech/intro");
    }

    #[test]
    fn test_permalink_title_html_no_trailing_slash() {
        // permalink: /:title.html -> URL should be /my-post.html (no trailing slash)
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2024-01-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/:title.html", &ctx);
        assert_eq!(url, "/my-post.html");
    }

    #[test]
    fn test_permalink_blog_title_html_no_trailing_slash() {
        // permalink: /blog/:title.html -> URL should be /blog/my-post.html (DTC pattern)
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2024-01-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/blog/:title.html", &ctx);
        assert_eq!(url, "/blog/my-post.html");
    }

    #[test]
    fn test_permalink_pretty_named_style_trailing_slash() {
        // permalink: pretty -> URL should end with /
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2024-01-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("pretty", &ctx);
        assert_eq!(url, "/2024/01/15/my-post/");
    }

    #[test]
    fn test_permalink_year_month_title_no_ext_no_trailing_slash_347() {
        // Issue 557: permalink: /:year/:month/:title -> URL should be /2024/01/my-post (NO trailing slash)
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2024-01-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/:year/:month/:title", &ctx);
        assert_eq!(url, "/2024/01/my-post");
    }

    // ========================================================================
    // Issue 557: Permalink no-extension should NOT get trailing slash
    // ========================================================================

    #[test]
    fn test_permalink_stories_title_no_trailing_slash() {
        // permalink: /stories/:title -> URL should be /stories/foo (NO trailing slash)
        // Jekyll outputs stories/foo.html, NOT stories/foo/index.html
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "foo".to_string(),
            date: Some("2024-01-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/stories/:title", &ctx);
        assert_eq!(url, "/stories/foo");
    }

    #[test]
    fn test_permalink_stories_title_with_trailing_slash_preserved() {
        // permalink: /stories/:title/ -> URL should be /stories/foo/ (trailing slash preserved)
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "foo".to_string(),
            date: Some("2024-01-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/stories/:title/", &ctx);
        assert_eq!(url, "/stories/foo/");
    }

    #[test]
    fn test_permalink_blog_title_html_still_works() {
        // permalink: /blog/:title.html -> URL should be /blog/foo.html (extension preserved)
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "foo".to_string(),
            date: Some("2024-01-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/blog/:title.html", &ctx);
        assert_eq!(url, "/blog/foo.html");
    }

    #[test]
    fn test_permalink_unicode_title_no_trailing_slash() {
        // Non-ASCII: permalink with Unicode title, no extension
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "\u{043F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}".to_string(),
            date: Some("2024-01-15".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/stories/:title", &ctx);
        assert_eq!(
            url,
            "/stories/\u{043F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}"
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
    fn test_load_pages_excludes_root_readme() {
        // Root-level README.md is the project README, not a site page.
        // Subdirectory README.md files ARE included (see test_load_pages_includes_readme_*).
        let config = test_config();
        let (pages, _) = load_pages(&site_dir(), &config).unwrap();
        let readme = pages.iter().find(|p| p.slug == "README");
        assert!(readme.is_none(), "Root-level README.md should be excluded");
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
    // Unit: Collection item URL with non-markdown extensions (Issue 562)
    // ========================================================================

    #[test]
    fn test_json_collection_item_preserves_extension() {
        // Issue 562: A .json file in a collection with permalink /:collection/:path:output_ext
        // should produce a URL ending in .json, not .html
        let dir = tempfile::tempdir().unwrap();
        let coll_dir = dir.path().join("_pages");
        std::fs::create_dir_all(&coll_dir).unwrap();
        std::fs::write(
            coll_dir.join("acitivitypub.json"),
            "---\n---\n{\"@context\": \"https://www.w3.org/ns/activitystreams\"}",
        )
        .unwrap();

        let mut config = SiteConfig::default();
        config.collections.insert(
            "pages".to_string(),
            crate::config::CollectionConfig {
                output: true,
                permalink: String::new(), // empty -> default /:collection/:path:output_ext
                sort_by: None,
            },
        );

        let (items, _) = load_collection("pages", dir.path(), &config).unwrap();
        assert!(
            !items.is_empty(),
            "Should find at least one collection item"
        );
        let item = items.iter().find(|i| i.slug == "acitivitypub");
        assert!(item.is_some(), "Should find acitivitypub item");
        let url = &item.unwrap().url;
        assert!(
            url.ends_with(".json"),
            "URL should end with .json, got: {}",
            url
        );
        assert!(
            !url.ends_with(".html"),
            "URL should NOT end with .html, got: {}",
            url
        );
    }

    #[test]
    fn test_xml_collection_item_preserves_extension() {
        // Non-markdown, non-html extension (.xml) should also be preserved
        let dir = tempfile::tempdir().unwrap();
        let coll_dir = dir.path().join("_feeds");
        std::fs::create_dir_all(&coll_dir).unwrap();
        std::fs::write(coll_dir.join("custom.xml"), "---\n---\n<feed></feed>").unwrap();

        let mut config = SiteConfig::default();
        config.collections.insert(
            "feeds".to_string(),
            crate::config::CollectionConfig {
                output: true,
                permalink: String::new(),
                sort_by: None,
            },
        );

        let (items, _) = load_collection("feeds", dir.path(), &config).unwrap();
        let item = items.iter().find(|i| i.slug == "custom");
        assert!(item.is_some(), "Should find custom item");
        let url = &item.unwrap().url;
        assert!(
            url.ends_with(".xml"),
            "URL should end with .xml, got: {}",
            url
        );
    }

    #[test]
    fn test_html_collection_item_keeps_html_extension() {
        // .html files should keep .html as usual (no change needed)
        let dir = tempfile::tempdir().unwrap();
        let coll_dir = dir.path().join("_pages");
        std::fs::create_dir_all(&coll_dir).unwrap();
        std::fs::write(
            coll_dir.join("about.html"),
            "---\ntitle: About\n---\n<p>About page</p>",
        )
        .unwrap();

        let mut config = SiteConfig::default();
        config.collections.insert(
            "pages".to_string(),
            crate::config::CollectionConfig {
                output: true,
                permalink: String::new(),
                sort_by: None,
            },
        );

        let (items, _) = load_collection("pages", dir.path(), &config).unwrap();
        let item = items.iter().find(|i| i.slug == "about");
        assert!(item.is_some(), "Should find about item");
        // HTML items with :output_ext -> url has no extension (url_to_output_path adds .html)
        // This is the expected behavior for .html files
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
                        sort_by: None,
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

    /// For post collections, the id uses dirname(url) + slug, matching
    /// Jekyll's `Document#id = File.join(File.dirname(url), slug)`.
    /// With permalink `/blog/:title.html`, url = `/blog/my-post.html`,
    /// dirname = `/blog`, so id = `/blog/my-post`.
    #[test]
    fn test_post_item_id_uses_url_dirname_and_slug() {
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
        // dirname of /blog/my-post.html is /blog, slug is my-post
        assert_eq!(item.id, "/blog/my-post");
    }

    /// With permalink `/stories/:title`, url = `/stories/my-post`,
    /// dirname = `/stories`, so id = `/stories/my-post`.
    #[test]
    fn test_post_item_id_uses_permalink_based_path() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let site = tmp.path();

        std::fs::write(site.join("_config.yml"), "permalink: /stories/:title\n").unwrap();

        let posts_dir = site.join("_posts");
        std::fs::create_dir_all(&posts_dir).unwrap();
        std::fs::write(
            posts_dir.join("2013-10-14-canadian-web-experience-toolkit.md"),
            "---\ntitle: Canadian Web Experience Toolkit\n---\nContent\n",
        )
        .unwrap();

        let config = crate::config::SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, errors) = load_collection("posts", site, &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        assert_eq!(items.len(), 1);

        let item = &items[0];
        assert_eq!(item.id, "/stories/canadian-web-experience-toolkit");
    }

    /// With permalink `/:year/:month/:title`, url = `/2024/01/my-post`,
    /// dirname = `/2024/01`, so id = `/2024/01/my-post`.
    #[test]
    fn test_post_item_id_year_month_title_permalink() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let site = tmp.path();

        std::fs::write(
            site.join("_config.yml"),
            "permalink: /:year/:month/:title\n",
        )
        .unwrap();

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
        assert_eq!(item.id, "/2024/01/my-post");
    }

    /// Default permalink produces date-based id (unchanged behavior).
    #[test]
    fn test_post_item_id_default_permalink() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let site = tmp.path();

        std::fs::write(site.join("_config.yml"), "\n").unwrap();

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
        // Default permalink: /:categories/:year/:month/:day/:title:output_ext
        // url = /2024/01/15/my-post.html, dirname = /2024/01/15, slug = my-post
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
        let ts = build_timestamp(None);
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
            excerpt_html: None,
            url: "/podcast/test-episode.html".to_string(),
            date: None,
            collection_name: "podcast".to_string(),
            source_path: "test-episode.md".to_string(),
            id: "/podcast/test-episode".to_string(),
        }];

        let build_time = "2026-03-15 10:30:00 +0000";
        backfill_default_dates(&mut items, build_time, true);

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
            excerpt_html: None,
            url: "/posts/my-post.html".to_string(),
            date: Some("2024-01-15".to_string()),
            collection_name: "posts".to_string(),
            source_path: "2024-01-15-my-post.md".to_string(),
            id: "/posts/my-post".to_string(),
        }];

        let build_time = "2026-03-15 10:30:00 +0000";
        backfill_default_dates(&mut items, build_time, true);

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
                excerpt_html: None,
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
                excerpt_html: None,
                url: "/podcast/no-date.html".to_string(),
                date: None,
                collection_name: "podcast".to_string(),
                source_path: "no-date.md".to_string(),
                id: "/podcast/no-date".to_string(),
            },
        ];

        let build_time = "2026-03-15 10:30:00 +0000";
        backfill_default_dates(&mut items, build_time, true);

        assert_eq!(items[0].date.as_deref(), Some("2023-06-01"));
        assert_eq!(items[1].date.as_deref(), Some(build_time));
    }

    // ========================================================================
    // Issue 474: non-post collections get item.date but NOT front_matter date
    // ========================================================================

    #[test]
    fn test_backfill_non_post_sets_item_date_but_not_frontmatter() {
        // Non-post collections (portfolio, talks, etc.) should get item.date
        // backfilled (for `site.collection | map: "date"` to work) but NOT
        // front_matter["date"] (so `page.date` remains nil in templates).
        let mut items = vec![CollectionItem {
            slug: "portfolio-1".to_string(),
            front_matter: FrontMatter::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            excerpt_html: None,
            url: "/portfolio/portfolio-1.html".to_string(),
            date: None,
            collection_name: "portfolio".to_string(),
            source_path: "portfolio-1.md".to_string(),
            id: "/portfolio/portfolio-1".to_string(),
        }];

        let build_time = "2026-03-15 10:30:00 +0000";
        // set_frontmatter=false for non-post collections
        backfill_default_dates(&mut items, build_time, false);

        // item.date should be set (for map: "date" in cross-page iteration)
        assert_eq!(
            items[0].date.as_deref(),
            Some(build_time),
            "item.date should be backfilled for non-post collections"
        );

        // front_matter should NOT have "date" (page.date remains nil)
        assert!(
            !items[0].front_matter.contains_key("date"),
            "Non-post items should not have 'date' in front matter (page.date = nil)"
        );
    }

    #[test]
    fn test_backfill_posts_sets_both_item_date_and_frontmatter() {
        // Posts should get both item.date and front_matter["date"] backfilled
        let mut items = vec![CollectionItem {
            slug: "my-post".to_string(),
            front_matter: FrontMatter::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            excerpt_html: None,
            url: "/posts/my-post.html".to_string(),
            date: None,
            collection_name: "posts".to_string(),
            source_path: "my-post.md".to_string(),
            id: "/posts/my-post".to_string(),
        }];

        let build_time = "2026-03-15 10:30:00 +0000";
        // set_frontmatter=true for posts
        backfill_default_dates(&mut items, build_time, true);

        // item.date should be set
        assert_eq!(items[0].date.as_deref(), Some(build_time));

        // front_matter should ALSO have "date" (page.date is truthy)
        let fm_date = items[0]
            .front_matter
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(fm_date, build_time);
    }

    #[test]
    fn test_backfill_simulates_main_loop_posts_vs_portfolio() {
        // Simulate the main.rs loop: posts get set_frontmatter=true,
        // other collections get set_frontmatter=false
        use std::collections::HashMap;

        let build_time = "2026-03-15 10:30:00 +0000";

        let mut collections: HashMap<String, Vec<CollectionItem>> = HashMap::new();

        collections.insert(
            "posts".to_string(),
            vec![CollectionItem {
                slug: "my-post".to_string(),
                front_matter: FrontMatter::new(),
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                excerpt_html: None,
                url: "/posts/my-post.html".to_string(),
                date: None,
                collection_name: "posts".to_string(),
                source_path: "my-post.md".to_string(),
                id: "/posts/my-post".to_string(),
            }],
        );

        collections.insert(
            "portfolio".to_string(),
            vec![CollectionItem {
                slug: "portfolio-1".to_string(),
                front_matter: FrontMatter::new(),
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                excerpt_html: None,
                url: "/portfolio/portfolio-1.html".to_string(),
                date: None,
                collection_name: "portfolio".to_string(),
                source_path: "portfolio-1.md".to_string(),
                id: "/portfolio/portfolio-1".to_string(),
            }],
        );

        // Apply backfill like main.rs does
        for (name, items) in collections.iter_mut() {
            let is_posts = name == "posts";
            backfill_default_dates(items, build_time, is_posts);
        }

        // Posts: both item.date and front_matter["date"] are set
        assert_eq!(collections["posts"][0].date.as_deref(), Some(build_time));
        assert!(collections["posts"][0].front_matter.contains_key("date"));

        // Portfolio: item.date is set but front_matter["date"] is NOT
        assert_eq!(
            collections["portfolio"][0].date.as_deref(),
            Some(build_time)
        );
        assert!(
            !collections["portfolio"][0]
                .front_matter
                .contains_key("date"),
            "Portfolio page.date should remain nil (no front_matter date)"
        );
    }

    // ========================================================================
    // Issue 267: build_timestamp with timezone support
    // ========================================================================

    #[test]
    fn test_build_timestamp_with_no_tz() {
        let ts = build_timestamp(None);
        assert!(
            ts.ends_with(" +0000"),
            "build_timestamp(None) should end with ' +0000', got: {ts}"
        );
        assert_eq!(ts.len(), 25, "Unexpected timestamp length: {ts}");
    }

    #[test]
    fn test_build_timestamp_with_berlin_tz() {
        let tz: chrono_tz::Tz = "Europe/Berlin".parse().unwrap();
        let ts = build_timestamp(Some(tz));
        // Europe/Berlin is +0100 (CET) or +0200 (CEST), never +0000
        assert!(
            !ts.ends_with("+0000"),
            "build_timestamp with Europe/Berlin should not use +0000, got: {ts}"
        );
        assert_eq!(ts.len(), 25, "Unexpected timestamp length: {ts}");
        // Verify it ends with a timezone offset pattern
        let offset = &ts[20..];
        assert!(
            offset.starts_with('+') || offset.starts_with('-'),
            "Timezone offset should start with + or -, got: {offset}"
        );
    }

    #[test]
    fn test_build_timestamp_with_utc_tz() {
        let tz: chrono_tz::Tz = "UTC".parse().unwrap();
        let ts = build_timestamp(Some(tz));
        assert!(
            ts.ends_with(" +0000"),
            "build_timestamp with UTC should end with ' +0000', got: {ts}"
        );
    }

    // ========================================================================
    // Unit: Page sort order (Issue 121)
    // ========================================================================

    /// Jekyll sorts site.pages by filename (page.name), with full path as
    /// tie-breaker. This means files from different directories interleave
    /// by their basename.
    #[test]
    fn test_pages_sorted_by_filename_then_path() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let site = tmp.path();

        // Create _config.yml
        std::fs::write(site.join("_config.yml"), "title: test\n").unwrap();

        // Create pages in subdirectories to test cross-directory sort order.
        // Sort is by filename first, then full path as tie-breaker.
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

        // Expected order: sort by filename first, then full path as tie-breaker
        // page-1.md (alpha) < page-1.md (beta) [same name, alpha path < beta path]
        // page-10.md (beta) < page-2.md (alpha) [different names, "page-10" < "page-2"]
        assert_eq!(
            urls,
            vec![
                "/docs/alpha/page-1.html",
                "/docs/beta/page-1.html",
                "/docs/beta/page-10.html",
                "/docs/alpha/page-2.html",
            ],
            "Pages should be sorted by (filename, full path) to match Jekyll"
        );
    }

    // ========================================================================
    // Issue 209: Default collection permalink has no .html extension
    // ========================================================================

    #[test]
    fn test_default_collection_permalink_no_html() {
        // Jekyll's default permalink for non-post collections is /:collection/:path:output_ext
        // which produces .html URLs. Sites that explicitly set permalink in collection
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
        // Jekyll strips :output_ext from item.url, so no .html
        // Output file will still be /pages/banners.html via url_to_output_path
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
        // Issue 557: /:collection/:path produces URL without trailing slash
        // url_to_output_path adds .html for the output file
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
    // Issue 548: Default collection permalink uses :output_ext
    // ========================================================================

    #[test]
    fn test_default_collection_permalink_uses_output_ext() {
        // Jekyll's actual default collection permalink is /:collection/:path:output_ext
        // which produces .html URLs, not pretty URLs with trailing slash.
        let dir = tempfile::tempdir().unwrap();
        let site = dir.path();

        // Create a collection with NO explicit permalink in config
        std::fs::write(
            site.join("_config.yml"),
            "collections:\n  notes:\n    output: true\n",
        )
        .unwrap();
        std::fs::create_dir_all(site.join("_notes")).unwrap();
        std::fs::write(
            site.join("_notes/2026-03-12-cc.md"),
            "---\ntitle: \"Test Note\"\n---\nSome content",
        )
        .unwrap();

        let config = SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, _) = load_collection("notes", site, &config).unwrap();
        assert_eq!(items.len(), 1);
        // Issue 548 fix: item.url should NOT include .html -- Jekyll strips :output_ext from URL
        // The output file should be notes/2026-03-12-cc.html, but the URL is /notes/2026-03-12-cc
        assert_eq!(items[0].url, "/notes/2026-03-12-cc");
    }

    #[test]
    fn test_default_collection_url_vs_output_path_diverge() {
        // Jekyll separates item.url from the output file path:
        // - item.url = /notes/slug (no .html)
        // - output file = notes/slug.html
        // This test verifies both are correct.
        let dir = tempfile::tempdir().unwrap();
        let site = dir.path();

        std::fs::write(
            site.join("_config.yml"),
            "collections:\n  notes:\n    output: true\n",
        )
        .unwrap();
        std::fs::create_dir_all(site.join("_notes")).unwrap();
        std::fs::write(
            site.join("_notes/2026-03-12-cc.md"),
            "---\ntitle: \"Test Note\"\n---\nSome content",
        )
        .unwrap();

        let config = SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, _) = load_collection("notes", site, &config).unwrap();
        assert_eq!(items.len(), 1);

        // URL should NOT have .html
        assert_eq!(items[0].url, "/notes/2026-03-12-cc");

        // But output path should be .html (not index.html in a directory)
        let out_dir = std::path::Path::new("/tmp/test_site");
        let out_path = crate::generator::url_to_output_path(out_dir, &items[0].url);
        assert_eq!(out_path, out_dir.join("notes/2026-03-12-cc.html"));
    }

    #[test]
    fn test_default_collection_permalink_unicode_output_ext() {
        // Unicode filenames should also get .html extension with default permalink
        let dir = tempfile::tempdir().unwrap();
        let site = dir.path();

        std::fs::write(
            site.join("_config.yml"),
            "collections:\n  pages:\n    output: true\n",
        )
        .unwrap();
        std::fs::create_dir_all(site.join("_pages")).unwrap();
        std::fs::write(
            site.join("_pages/über-uns.md"),
            "---\ntitle: \"Über Uns\"\n---\nInhalt",
        )
        .unwrap();

        let config = SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, _) = load_collection("pages", site, &config).unwrap();
        assert_eq!(items.len(), 1);
        // Jekyll strips :output_ext from URL, so no .html
        assert_eq!(items[0].url, "/pages/%C3%BCber-uns");
    }

    #[test]
    fn test_generate_url_collection_path_output_ext() {
        // generate_url replaces :output_ext with empty string for the URL
        let url = generate_url("/:collection/:path:output_ext", "notes", "2018-06-04-aa");
        assert_eq!(url, "/notes/2018-06-04-aa");
    }

    #[test]
    fn test_generate_url_output_ext_unicode() {
        // Unicode with :output_ext pattern
        let ctx = PermalinkContext {
            collection: "notes".to_string(),
            title: "заметка".to_string(),
            source_path_stem: Some("заметка".to_string()),
            ..Default::default()
        };
        let url = generate_url_with_context("/:collection/:path:output_ext", &ctx);
        // :output_ext is stripped from URL
        assert_eq!(url, "/notes/заметка");
    }

    #[test]
    fn test_explicit_permalink_not_affected_by_output_ext_default() {
        // Collections with explicit permalink should NOT be affected
        let dir = tempfile::tempdir().unwrap();
        let site = dir.path();

        std::fs::write(
            site.join("_config.yml"),
            "collections:\n  books:\n    output: true\n    permalink: /:collection/:title/\n",
        )
        .unwrap();
        std::fs::create_dir_all(site.join("_books")).unwrap();
        std::fs::write(
            site.join("_books/my-book.md"),
            "---\ntitle: My Book\n---\nContent",
        )
        .unwrap();

        let config = SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, _) = load_collection("books", site, &config).unwrap();
        assert_eq!(items.len(), 1);
        // Explicit trailing-slash permalink should still produce pretty URL
        assert_eq!(items[0].url, "/books/my-book/");
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

    // ========================================================================
    // README.md pages: Jekyll includes README.md in site.pages
    // ========================================================================

    #[test]
    fn test_load_pages_includes_readme_in_subdirectory() {
        // Jekyll (via jekyll-readme-index) includes README.md files as pages,
        // converting them to index.html in their directory. Templates like
        // the metals-ru book rely on finding README.md pages in site.pages.
        // README.md without front matter is only included if config defaults target it.
        let dir = tempfile::TempDir::new().unwrap();
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("README.md"), "# Hello from subdirectory").unwrap();

        let config = SiteConfig {
            permalink: "pretty".to_string(),
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: "subdir/README.md".to_string(),
                    type_name: String::new(),
                },
                values: crate::config::DefaultValues {
                    values: std::collections::HashMap::from([(
                        "layout".to_string(),
                        serde_yaml::Value::String("default".to_string()),
                    )]),
                },
            }],
            ..Default::default()
        };

        let (pages, errors) = load_pages(dir.path(), &config).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);

        let readme_page = pages.iter().find(|p| p.source_path.contains("README.md"));
        assert!(
            readme_page.is_some(),
            "README.md in subdirectory should be included in site.pages"
        );

        let readme_page = readme_page.unwrap();
        // README.md should get a directory index URL (like index.md)
        assert_eq!(
            readme_page.url, "/subdir/",
            "README.md should produce directory index URL"
        );
        // source_path should contain README.md for template matching
        assert!(
            readme_page.source_path.contains("README.md"),
            "source_path should contain README.md, got: {}",
            readme_page.source_path
        );
    }

    #[test]
    fn test_load_pages_readme_without_front_matter() {
        // README.md files without front matter are included only if
        // config defaults target that path (matching Jekyll behavior).
        let dir = tempfile::TempDir::new().unwrap();
        let sub = dir.path().join("часть_1");
        fs::create_dir(&sub).unwrap();
        fs::write(
            sub.join("README.md"),
            "# Часть 1: Введение\n\nСодержание части.",
        )
        .unwrap();

        let config = SiteConfig {
            permalink: "pretty".to_string(),
            defaults: vec![crate::config::DefaultConfig {
                scope: crate::config::DefaultScope {
                    path: "часть_1/README.md".to_string(),
                    type_name: String::new(),
                },
                values: crate::config::DefaultValues {
                    values: std::collections::HashMap::from([(
                        "layout".to_string(),
                        serde_yaml::Value::String("part".to_string()),
                    )]),
                },
            }],
            ..Default::default()
        };

        let (pages, errors) = load_pages(dir.path(), &config).unwrap();
        assert!(errors.is_empty());

        let readme = pages.iter().find(|p| p.source_path.contains("README.md"));
        assert!(
            readme.is_some(),
            "README.md without front matter should still be included"
        );
        let readme = readme.unwrap();
        assert!(
            readme.html_content.contains("Часть 1"),
            "README.md content should be converted to HTML"
        );
    }

    #[test]
    fn test_load_pages_readme_with_front_matter() {
        // README.md with explicit front matter should also work
        let dir = tempfile::TempDir::new().unwrap();
        let sub = dir.path().join("docs");
        fs::create_dir(&sub).unwrap();
        fs::write(
            sub.join("README.md"),
            "---\ntitle: Documentation\nlayout: default\n---\n# Docs\nWelcome.",
        )
        .unwrap();

        let config = SiteConfig {
            permalink: "pretty".to_string(),
            ..Default::default()
        };

        let (pages, errors) = load_pages(dir.path(), &config).unwrap();
        assert!(errors.is_empty());

        let readme = pages.iter().find(|p| p.source_path.contains("README.md"));
        assert!(
            readme.is_some(),
            "README.md with front matter should be a page"
        );
        let readme = readme.unwrap();
        assert_eq!(
            readme.front_matter.get("title").and_then(|v| v.as_str()),
            Some("Documentation")
        );
        assert_eq!(readme.url, "/docs/");
    }

    #[test]
    fn test_load_pages_root_readme_excluded() {
        // Root-level README.md should NOT be included (it's the project README,
        // not a site page). Only subdirectory README.md files are included.
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("README.md"), "# Project Readme").unwrap();
        // Also add an index.md so there's at least one page
        fs::write(
            dir.path().join("index.md"),
            "---\ntitle: Home\n---\nWelcome",
        )
        .unwrap();

        let config = SiteConfig {
            permalink: "pretty".to_string(),
            ..Default::default()
        };

        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        let root_readme = pages.iter().find(|p| p.source_path == "README.md");
        assert!(
            root_readme.is_none(),
            "Root-level README.md should be excluded from pages"
        );
    }

    #[test]
    fn test_collection_item_source_path_includes_collection_dir_prefix() {
        // page.path for collection items must include the collection directory
        // prefix (e.g., "_licenses/mit.txt"), matching Jekyll's behavior.
        // This is needed for github_edit_link to generate correct URLs.
        let dir = tempfile::tempdir().unwrap();
        let licenses_dir = dir.path().join("_licenses");
        std::fs::create_dir(&licenses_dir).unwrap();
        std::fs::write(
            licenses_dir.join("mit.txt"),
            "---\ntitle: MIT License\npermalink: /licenses/mit/\n---\nMIT License text",
        )
        .unwrap();

        let config = SiteConfig::default();
        let (items, errors) = load_collection("licenses", dir.path(), &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        assert_eq!(items.len(), 1);

        let item = &items[0];
        assert_eq!(
            item.source_path, "_licenses/mit.txt",
            "source_path should include _licenses/ prefix, got: {}",
            item.source_path
        );
    }

    #[test]
    fn test_collection_item_source_path_unicode_filename() {
        let dir = tempfile::tempdir().unwrap();
        let coll_dir = dir.path().join("_docs");
        std::fs::create_dir(&coll_dir).unwrap();
        std::fs::write(
            coll_dir.join("\u{00fc}ber.md"),
            "---\ntitle: \u{00dc}ber Uns\n---\nContent",
        )
        .unwrap();

        let config = SiteConfig::default();
        let (items, errors) = load_collection("docs", dir.path(), &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        assert_eq!(items.len(), 1);

        let item = &items[0];
        assert_eq!(
            item.source_path, "_docs/\u{00fc}ber.md",
            "source_path should include collection prefix and preserve Unicode, got: {}",
            item.source_path
        );
    }

    // ── Issue 300: Excerpt with {% highlight %} Liquid tags ──

    #[test]
    fn test_excerpt_highlight_tag_rendered_in_excerpt_html() {
        // A post whose first paragraph contains {% highlight js %}...{% endhighlight %}
        // should have the highlight tag rendered in excerpt_html, not literal text.
        let dir = tempfile::tempdir().unwrap();
        let posts_dir = dir.path().join("_posts");
        std::fs::create_dir_all(&posts_dir).unwrap();
        std::fs::write(
            posts_dir.join("2020-04-02-example.md"),
            "---\ntitle: Example\n---\n{% highlight js %}var x = 1;{% endhighlight %}\n\nMore content here.\n",
        ).unwrap();

        let config = SiteConfig::default();
        let (items, errors) = load_collection("posts", dir.path(), &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        assert_eq!(items.len(), 1);

        let item = &items[0];
        let excerpt_html = item
            .excerpt_html
            .as_ref()
            .expect("excerpt_html should exist");
        assert!(
            excerpt_html.contains("<figure class=\"highlight\">"),
            "Excerpt should contain rendered highlight tag, not literal text. Got: {}",
            excerpt_html
        );
        assert!(
            !excerpt_html.contains("{% highlight"),
            "Excerpt should NOT contain literal {{% highlight %}} text. Got: {}",
            excerpt_html
        );
    }

    #[test]
    fn test_excerpt_highlight_beyond_cutoff_not_partial() {
        // When the highlight block is in the second paragraph (beyond excerpt cutoff),
        // the excerpt should not contain a partial/broken highlight tag.
        let dir = tempfile::tempdir().unwrap();
        let posts_dir = dir.path().join("_posts");
        std::fs::create_dir_all(&posts_dir).unwrap();
        std::fs::write(
            posts_dir.join("2020-04-02-example.md"),
            "---\ntitle: Example\n---\nFirst paragraph text.\n\n{% highlight js %}var x = 1;{% endhighlight %}\n",
        ).unwrap();

        let config = SiteConfig::default();
        let (items, errors) = load_collection("posts", dir.path(), &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        assert_eq!(items.len(), 1);

        let item = &items[0];
        let excerpt_html = item
            .excerpt_html
            .as_ref()
            .expect("excerpt_html should exist");
        // The excerpt is only the first paragraph, so it should not contain highlight content
        assert!(
            !excerpt_html.contains("{% highlight"),
            "Excerpt beyond cutoff should not contain partial highlight. Got: {}",
            excerpt_html
        );
        assert!(
            excerpt_html.contains("First paragraph text"),
            "Excerpt should contain first paragraph. Got: {}",
            excerpt_html
        );
    }

    #[test]
    fn test_excerpt_highlight_unicode_content() {
        // Non-ASCII: highlight block with Unicode variable names
        let dir = tempfile::tempdir().unwrap();
        let posts_dir = dir.path().join("_posts");
        std::fs::create_dir_all(&posts_dir).unwrap();
        std::fs::write(
            posts_dir.join("2020-04-02-unicode.md"),
            "---\ntitle: Unicode\n---\n{% highlight python %}x = \"\u{4f60}\u{597d}\"{% endhighlight %}\n\nMore.\n",
        ).unwrap();

        let config = SiteConfig::default();
        let (items, errors) = load_collection("posts", dir.path(), &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        assert_eq!(items.len(), 1);

        let item = &items[0];
        let excerpt_html = item
            .excerpt_html
            .as_ref()
            .expect("excerpt_html should exist");
        assert!(
            excerpt_html.contains("<figure class=\"highlight\">"),
            "Unicode highlight excerpt should render. Got: {}",
            excerpt_html
        );
        assert!(
            excerpt_html.contains("\u{4f60}\u{597d}"),
            "Unicode content should be preserved in excerpt. Got: {}",
            excerpt_html
        );
    }

    // ========================================================================
    // Issue 326: include config for underscore directories
    // ========================================================================

    #[test]
    fn test_include_config_allows_underscore_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("top.md"), "---\ntitle: Top\n---\nTop").unwrap();
        let pages_dir = dir.path().join("_pages");
        std::fs::create_dir(&pages_dir).unwrap();
        std::fs::write(
            pages_dir.join("about.md"),
            "---\ntitle: About\npermalink: /about/\n---\nAbout page",
        )
        .unwrap();

        // Without include: _pages should be skipped
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert!(
            !pages.iter().any(|p| p.slug == "about"),
            "_pages should be skipped without include config"
        );

        // With include: [_pages], the directory should be processed
        let mut config_with_include = SiteConfig::default();
        config_with_include.include = vec!["_pages".to_string()];
        let (pages, _) = load_pages(dir.path(), &config_with_include).unwrap();
        assert!(
            pages.iter().any(|p| p.slug == "about"),
            "_pages should be included when listed in config.include"
        );
    }

    #[test]
    fn test_include_config_with_unicode_directory_name() {
        let dir = tempfile::tempdir().unwrap();
        let pages_dir = dir.path().join("_\u{043f}\u{0430}\u{0433}\u{0435}"); // _паге in Cyrillic
        std::fs::create_dir(&pages_dir).unwrap();
        std::fs::write(
            pages_dir.join("test.md"),
            "---\ntitle: \u{0422}\u{0435}\u{0441}\u{0442}\n---\nContent",
        )
        .unwrap();

        let mut config = SiteConfig::default();
        config.include = vec!["_\u{043f}\u{0430}\u{0433}\u{0435}".to_string()];
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert!(
            pages.iter().any(|p| p.slug == "test"),
            "Unicode underscore directory should be included"
        );
    }

    // ========================================================================
    // Issue 326: collection sort_by
    // ========================================================================

    #[test]
    fn test_collection_sort_by_numeric_field() {
        let dir = tempfile::tempdir().unwrap();
        let tabs_dir = dir.path().join("_tabs");
        std::fs::create_dir(&tabs_dir).unwrap();
        std::fs::write(
            tabs_dir.join("about.md"),
            "---\ntitle: About\norder: 4\n---\nAbout",
        )
        .unwrap();
        std::fs::write(
            tabs_dir.join("categories.md"),
            "---\ntitle: Categories\norder: 1\n---\nCategories",
        )
        .unwrap();
        std::fs::write(
            tabs_dir.join("tags.md"),
            "---\ntitle: Tags\norder: 2\n---\nTags",
        )
        .unwrap();

        let mut config = SiteConfig::default();
        config.collections.insert(
            "tabs".to_string(),
            crate::config::CollectionConfig {
                output: true,
                permalink: "/:title/".to_string(),
                sort_by: Some("order".to_string()),
            },
        );

        let (items, _) = load_collection("tabs", dir.path(), &config).unwrap();
        assert_eq!(items.len(), 3);
        // Should be sorted by order: 1, 2, 4
        assert_eq!(items[0].slug, "categories");
        assert_eq!(items[1].slug, "tags");
        assert_eq!(items[2].slug, "about");
    }

    // ========================================================================
    // Issue 326: permalink leading slash normalization
    // ========================================================================

    #[test]
    fn test_page_permalink_without_leading_slash_gets_normalized() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("test.md"),
            "---\ntitle: Test\npermalink: test.html\n---\nContent",
        )
        .unwrap();

        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        let page = pages.iter().find(|p| p.slug == "test").unwrap();
        assert!(
            page.url.starts_with('/'),
            "Page URL should start with /. Got: {}",
            page.url
        );
        assert_eq!(page.url, "/test.html");
    }

    #[test]
    fn test_collection_permalink_without_leading_slash_gets_normalized() {
        let dir = tempfile::tempdir().unwrap();
        let coll_dir = dir.path().join("_docs");
        std::fs::create_dir(&coll_dir).unwrap();
        std::fs::write(
            coll_dir.join("intro.md"),
            "---\ntitle: Intro\npermalink: intro.html\n---\nIntro",
        )
        .unwrap();

        let mut config = SiteConfig::default();
        config.collections.insert(
            "docs".to_string(),
            crate::config::CollectionConfig {
                output: true,
                permalink: String::new(),
                sort_by: None,
            },
        );

        let (items, _) = load_collection("docs", dir.path(), &config).unwrap();
        assert_eq!(items.len(), 1);
        assert!(
            items[0].url.starts_with('/'),
            "Collection item URL should start with /. Got: {}",
            items[0].url
        );
        assert_eq!(items[0].url, "/intro.html");
    }

    // ========================================================================
    // Issue 326: collection permalink from defaults
    // ========================================================================

    #[test]
    fn test_collection_permalink_from_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let tabs_dir = dir.path().join("_tabs");
        std::fs::create_dir(&tabs_dir).unwrap();
        std::fs::write(
            tabs_dir.join("about.md"),
            "---\ntitle: About\n---\nAbout content",
        )
        .unwrap();

        let mut config = SiteConfig::default();
        config.collections.insert(
            "tabs".to_string(),
            crate::config::CollectionConfig {
                output: true,
                permalink: String::new(), // no permalink in collection config
                sort_by: None,
            },
        );
        // Set permalink in defaults for tabs type
        config.defaults.push(crate::config::DefaultConfig {
            scope: crate::config::DefaultScope {
                path: String::new(),
                type_name: "tabs".to_string(),
            },
            values: crate::config::DefaultValues {
                values: {
                    let mut m = std::collections::HashMap::new();
                    m.insert(
                        "permalink".to_string(),
                        serde_yaml::Value::String("/:title/".to_string()),
                    );
                    m
                },
            },
        });

        let (items, _) = load_collection("tabs", dir.path(), &config).unwrap();
        assert_eq!(items.len(), 1);
        // Should use /:title/ pattern from defaults, not /:collection/:path
        assert_eq!(items[0].url, "/about/");
    }

    // ========================================================================
    // Issue 300: Non-markdown index.html gets directory URL (like index.md)
    // ========================================================================

    #[test]
    fn test_page_url_html_index_root_gets_directory_url() {
        // Root index.html should get "/" not "/index.html"
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("index.html"),
            "---\nlayout: default\ntitle: Home\n---\n<h1>Welcome</h1>",
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/");
    }

    #[test]
    fn test_page_url_html_index_subdir_gets_directory_url() {
        // Subdirectory index.html should get "/subdir/" not "/subdir/index.html"
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("blog");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(
            subdir.join("index.html"),
            "---\nlayout: default\ntitle: Blog\n---\n<h1>Blog</h1>",
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/blog/");
    }

    #[test]
    fn test_page_url_html_index_unicode_subdir() {
        // Non-ASCII subdir index.html should get percent-encoded directory URL
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("über-uns");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(
            subdir.join("index.html"),
            "---\ntitle: About Us\n---\n<p>Über uns</p>",
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        // URL should be percent-encoded directory path
        assert!(
            pages[0].url.ends_with('/'),
            "Expected directory URL ending with /, got: {}",
            pages[0].url
        );
        assert!(
            !pages[0].url.contains("index.html"),
            "Should not contain index.html, got: {}",
            pages[0].url
        );
    }

    #[test]
    fn test_page_url_html_non_index_preserved() {
        // Non-index HTML files should keep their original URL
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("about.html"),
            "---\ntitle: About\n---\n<p>About page</p>",
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/about.html");
    }

    #[test]
    fn test_page_url_html_index_with_permalink_override() {
        // index.html with explicit permalink should use that permalink
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("index.html"),
            "---\npermalink: /custom/\ntitle: Home\n---\n<h1>Welcome</h1>",
        )
        .unwrap();
        let config = SiteConfig::default();
        let (pages, _) = load_pages(dir.path(), &config).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/custom/");
    }

    #[test]
    fn test_load_podcast_preserves_source_path_order_without_sort_by() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let site = tmp.path();

        std::fs::write(
            site.join("_config.yml"),
            "collections:\n  podcast:\n    output: true\n    permalink: /:collection/:title.html\n",
        )
        .unwrap();

        let podcast_dir = site.join("_podcast");
        std::fs::create_dir_all(&podcast_dir).unwrap();
        std::fs::write(
            podcast_dir.join("data-translator-role-and-data-strategy.md"),
            "---\ntitle: Translator\nseason: 3\nepisode: 4\n---\nContent\n",
        )
        .unwrap();
        std::fs::write(
            podcast_dir.join("data-science-interview-and-cv-guide.md"),
            "---\ntitle: Interview\ndate: 2025-11-07\nseason: 3\nepisode: 4\n---\nContent\n",
        )
        .unwrap();

        let config = crate::config::SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, errors) = load_collection("podcast", site, &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        let slugs: Vec<_> = items.iter().map(|item| item.slug.as_str()).collect();
        assert_eq!(
            slugs,
            vec![
                "data-science-interview-and-cv-guide",
                "data-translator-role-and-data-strategy"
            ]
        );
    }

    #[test]
    fn test_pre_render_highlight_blocks_basic() {
        let input = "Some text\n\n{% highlight js %}\nvar x = 1;\n{% endhighlight %}\n\nMore text";
        let result = pre_render_highlight_blocks(input);
        assert!(
            result.contains("<figure class=\"highlight\">"),
            "Expected <figure class=\"highlight\"> in output, got: {result}"
        );
        assert!(
            !result.contains("{% highlight"),
            "Liquid highlight tag should be replaced, got: {result}"
        );
        assert!(
            !result.contains("{% endhighlight"),
            "Liquid endhighlight tag should be replaced, got: {result}"
        );
        assert!(
            result.contains("Some text"),
            "Surrounding text should be preserved"
        );
        assert!(
            result.contains("More text"),
            "Surrounding text should be preserved"
        );
    }

    #[test]
    fn test_pre_render_highlight_blocks_no_tags() {
        let input = "No highlight tags here";
        let result = pre_render_highlight_blocks(input);
        assert_eq!(
            result, input,
            "Content without highlight tags should be unchanged"
        );
    }

    #[test]
    fn test_pre_render_highlight_blocks_linenos() {
        let input = "{% highlight ruby linenos %}\nputs 'hi'\n{% endhighlight %}";
        let result = pre_render_highlight_blocks(input);
        assert!(
            result.contains("<figure class=\"highlight\">"),
            "Should handle linenos parameter, got: {result}"
        );
        assert!(
            result.contains("language-ruby"),
            "Should use ruby language, got: {result}"
        );
    }

    #[test]
    fn test_pre_render_highlight_no_blank_lines_in_figure() {
        // Code with blank lines should produce figure output with no blank lines
        let input = "Before\n\n{% highlight js %}\n// comment 1\n\n// comment 2\nvar x;\n\nadder();\n{% endhighlight %}\n\nAfter";
        let result = pre_render_highlight_blocks(input);
        // Find the figure block
        let fig_start = result.find("<figure").expect("must have figure");
        let fig_end = result[fig_start..]
            .find("</figure>")
            .expect("must have /figure");
        let figure_block = &result[fig_start..fig_start + fig_end + "</figure>".len()];
        assert!(
            !figure_block.contains("\n\n"),
            "Figure block should not contain blank lines, got: {}",
            figure_block
        );
    }

    #[test]
    fn test_pre_render_then_markdown_no_p_in_figure() {
        // The preprocessed content goes through markdown; verify no <p> inside figure
        // Use the exact lanyon content: multiple blank lines inside the highlight block
        let input = "Cum sociis natoque penatibus et magnis dis `code element` montes, nascetur ridiculus mus.\n\n{% highlight js %}\n// Example can be run directly in your JavaScript console\n\n// Create a function that takes two arguments and returns the sum of those arguments\nvar adder = new Function(\"a\", \"b\", \"return a + b\");\n\n// Call the function\nadder(2, 6);\n// > 8\n{% endhighlight %}\n\nAenean lacinia bibendum nulla sed consectetur.";
        let preprocessed = pre_render_highlight_blocks(input);
        // Verify preprocessed content has no blank lines in figure block
        if let Some(fig_start) = preprocessed.find("<figure") {
            if let Some(fig_end) = preprocessed[fig_start..].find("</figure>") {
                let block = &preprocessed[fig_start..fig_start + fig_end + "</figure>".len()];
                assert!(
                    !block.contains("\n\n"),
                    "preprocessed figure has blank LF lines"
                );
                assert!(
                    !block.contains("\r\n\r\n"),
                    "preprocessed figure has blank CRLF lines"
                );
            }
        }
        // Use the same options as collection loading (add_code_classes=true for kramdown)
        let html = crate::frontmatter::markdown_to_html_with_options(
            &preprocessed,
            true,
            true,
            false,
            false,
        );
        let fig_start = html.find("<figure").expect("must have figure");
        let fig_end = html[fig_start..]
            .find("</figure>")
            .expect("must have /figure");
        let figure_block = &html[fig_start..fig_start + fig_end + "</figure>".len()];
        assert!(
            !figure_block.contains("<p>"),
            "No <p> tags should appear inside <figure> after markdown conversion, got: {}",
            figure_block
        );
    }

    #[test]
    fn test_pre_render_highlight_blocks_unicode() {
        let input = "{% highlight python %}\nx = \"cafe\\u0301\"\n{% endhighlight %}";
        let result = pre_render_highlight_blocks(input);
        assert!(
            result.contains("<figure class=\"highlight\">"),
            "Should handle unicode content, got: {result}"
        );
    }

    #[test]
    fn test_pre_render_highlight_blocks_hl_lines() {
        let input =
            "{% highlight plaintext hl_lines=\"2\" %}\nline one\nline two\nline three\n{% endhighlight %}";
        let result = pre_render_highlight_blocks(input);
        assert!(
            result.contains("<span class=\"hll\">line two\n</span>"),
            "Line 2 should be wrapped in hll span, got: {result}"
        );
        assert!(
            !result.contains("<span class=\"hll\">line one"),
            "Line 1 should NOT be wrapped, got: {result}"
        );
        assert!(
            !result.contains("<span class=\"hll\">line three"),
            "Line 3 should NOT be wrapped, got: {result}"
        );
    }

    #[test]
    fn test_pre_render_highlight_blocks_inside_raw() {
        // {% highlight %} inside {% raw %}...{% endraw %} should NOT be processed
        let input = "{% raw %}{% highlight ruby linenos %}\ndef foo\n  puts 'foo'\nend\n{% endhighlight %}{% endraw %}";
        let result = pre_render_highlight_blocks(input);
        assert!(
            !result.contains("<figure"),
            "highlight inside raw should not be processed, got: {result}"
        );
        assert!(
            result.contains("{% highlight ruby linenos %}"),
            "highlight tag should be kept as literal text, got: {result}"
        );
    }

    #[test]
    fn test_pre_render_highlight_blocks_inside_fenced_code() {
        // {% highlight %} inside fenced code blocks should NOT be processed
        let input = "```\n{% raw %}{% highlight ruby linenos %}\ndef foo\n  puts 'foo'\nend\n{% endhighlight %}{% endraw %}\n```";
        let result = pre_render_highlight_blocks(input);
        assert!(
            !result.contains("<figure"),
            "highlight inside fenced code block should not be processed, got: {result}"
        );
        assert!(
            result.contains("{% highlight ruby linenos %}"),
            "highlight tag inside code block should be literal, got: {result}"
        );
    }

    #[test]
    fn test_pre_render_highlight_wrapping_raw_highlight() {
        // Outer {% highlight yaml %} wraps inner {% raw %}{% highlight %}{% endraw %}
        // The outer should be processed; the inner should be literal text in output
        let input = "{% highlight yaml %}\n{% raw %}{% highlight some_language %}\nSome code\n{% endhighlight %}{% endraw %}\n{% endhighlight %}";
        let result = pre_render_highlight_blocks(input);
        assert!(
            result.contains("<figure class=\"highlight\">"),
            "Outer highlight should be processed, got: {result}"
        );
        // The inner {% highlight some_language %} should appear as text in the output
        // (it's inside {% raw %} inside the outer highlight's body)
        assert!(
            result.contains("highlight some_language"),
            "Inner highlight text should appear literally, got: {result}"
        );
    }

    #[test]
    fn test_pre_render_highlight_blocks_mixed_protected_unprotected() {
        // Mix of: unprotected highlight, then fenced code with highlight, then another unprotected
        let input = "{% highlight js %}\nvar x = 1;\n{% endhighlight %}\n\n```\n{% highlight ruby %}\ndef foo\nend\n{% endhighlight %}\n```\n\n{% highlight python %}\nx = 1\n{% endhighlight %}";
        let result = pre_render_highlight_blocks(input);
        // Count figure tags - should be exactly 2 (js and python, not ruby)
        let figure_count = result.matches("<figure class=\"highlight\">").count();
        assert_eq!(
            figure_count, 2,
            "Should have 2 figures (js + python), not ruby in fenced code. Got: {result}"
        );
    }

    #[test]
    fn test_pre_render_highlight_blocks_raw_with_unicode() {
        // Unicode content inside raw+highlight should be preserved literally
        let input =
            "{% raw %}{% highlight python %}\nx = \"caf\u{e9}\"\n{% endhighlight %}{% endraw %}";
        let result = pre_render_highlight_blocks(input);
        assert!(
            !result.contains("<figure"),
            "highlight inside raw should not be processed (unicode), got: {result}"
        );
        assert!(
            result.contains("caf\u{e9}"),
            "Unicode should be preserved, got: {result}"
        );
    }

    #[test]
    fn test_collection_item_html_content_processes_highlight() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let site = tmp.path();

        std::fs::write(
            site.join("_config.yml"),
            "collections:\n  posts:\n    output: true\n",
        )
        .unwrap();

        let posts_dir = site.join("_posts");
        std::fs::create_dir_all(&posts_dir).unwrap();
        std::fs::write(
            posts_dir.join("2020-04-02-example.md"),
            "---\nlayout: post\ntitle: Example\n---\n\nSome text\n\n{% highlight js %}\nvar x = 1;\n{% endhighlight %}\n\nMore text\n",
        )
        .unwrap();

        let config = crate::config::SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, errors) = load_collection("posts", site, &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        assert_eq!(items.len(), 1);

        let item = &items[0];
        assert!(
            item.html_content.contains("<figure class=\"highlight\">"),
            "html_content should have processed highlight tags, got: {}",
            item.html_content
        );
        assert!(
            !item.html_content.contains("{% highlight"),
            "html_content should not contain raw Liquid tags, got: {}",
            item.html_content
        );
        // The <figure> block must not have <p> tags inside (markdown parser artifact)
        let fig_start = item.html_content.find("<figure").expect("must have figure");
        let fig_end = item.html_content[fig_start..]
            .find("</figure>")
            .expect("must have /figure");
        let figure_block = &item.html_content[fig_start..fig_start + fig_end + "</figure>".len()];
        assert!(
            !figure_block.contains("<p>"),
            "No <p> tags should appear inside <figure> block, got: {}",
            figure_block
        );
    }

    #[test]
    fn test_collection_item_html_content_highlight_with_blank_lines() {
        // Test with code that has blank lines (like the lanyon example)
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let site = tmp.path();

        std::fs::write(
            site.join("_config.yml"),
            "collections:\n  posts:\n    output: true\n",
        )
        .unwrap();

        let posts_dir = site.join("_posts");
        std::fs::create_dir_all(&posts_dir).unwrap();
        std::fs::write(
            posts_dir.join("2020-04-02-example.md"),
            "---\nlayout: post\ntitle: Example\n---\n\nBefore code\n\n{% highlight js %}\n// Comment 1\n\n// Comment 2\nvar x = 1;\n\nadder(2, 6);\n{% endhighlight %}\n\nAfter code\n",
        )
        .unwrap();

        let config = crate::config::SiteConfig::from_file(&site.join("_config.yml")).unwrap();
        let (items, errors) = load_collection("posts", site, &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        assert_eq!(items.len(), 1);

        let item = &items[0];
        assert!(
            item.html_content.contains("<figure class=\"highlight\">"),
            "html_content should have processed highlight tags, got: {}",
            item.html_content
        );
        // No <p> inside the figure block
        let fig_start = item.html_content.find("<figure").expect("must have figure");
        let fig_end = item.html_content[fig_start..]
            .find("</figure>")
            .expect("must have /figure");
        let figure_block = &item.html_content[fig_start..fig_start + fig_end + "</figure>".len()];
        assert!(
            !figure_block.contains("<p>"),
            "No <p> tags should appear inside <figure> block with blank lines, got: {}",
            figure_block
        );
    }

    #[test]
    fn test_highlight_tag_processed_before_markdown() {
        // Self-contained fixture: a post with {% highlight %} tags
        // should have them converted to <figure> HTML before markdown runs
        let dir = tempfile::tempdir().unwrap();
        let posts_dir = dir.path().join("_posts");
        std::fs::create_dir_all(&posts_dir).unwrap();
        std::fs::write(
            posts_dir.join("2024-01-01-example.md"),
            "---\ntitle: Example\n---\n\nSome text.\n\n{% highlight ruby %}\ndef foo\n  42\nend\n{% endhighlight %}\n\nMore text.\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("_config.yml"), "title: Test\n").unwrap();
        let config = crate::config::SiteConfig::from_file(&dir.path().join("_config.yml")).unwrap();
        let (items, errors) = load_collection("posts", dir.path(), &config).unwrap();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert!(
            item.html_content.contains("<figure class=\"highlight\">"),
            "highlight tags should be processed into <figure>. Got: {}",
            item.html_content
        );
        assert!(
            !item.html_content.contains("{% highlight"),
            "Raw Liquid highlight tags should not remain. Got: {}",
            item.html_content
        );
        // No <p> inside <figure> (markdown should not wrap figure internals)
        if let Some(fig_start) = item.html_content.find("<figure") {
            if let Some(fig_end) = item.html_content[fig_start..].find("</figure>") {
                let figure_block =
                    &item.html_content[fig_start..fig_start + fig_end + "</figure>".len()];
                assert!(
                    !figure_block.contains("<p>"),
                    "No <p> tags inside <figure>. Got: {}",
                    figure_block
                );
            }
        }
    }

    // ========================================================================
    // Unit: Category URL lowercasing (Issue 354)
    // ========================================================================

    #[test]
    fn test_category_url_lowercased_edge_case() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2009-05-15".to_string()),
            categories: vec!["Edge Case".to_string()],
            ..Default::default()
        };
        let url = generate_url_with_context("/:categories/:year/:month/:day/:title.html", &ctx);
        assert_eq!(
            url, "/edge case/2009/05/15/my-post.html",
            "Jekyll lowercases categories in URLs"
        );
    }

    #[test]
    fn test_category_url_lowercased_markup() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "my-post".to_string(),
            date: Some("2013-08-17".to_string()),
            categories: vec!["Markup".to_string()],
            ..Default::default()
        };
        let url = generate_url_with_context("/:categories/:year/:month/:day/:title.html", &ctx);
        assert_eq!(
            url, "/markup/2013/08/17/my-post.html",
            "Jekyll lowercases categories in URLs"
        );
    }

    #[test]
    fn test_multiple_categories_all_lowercased() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "hello".to_string(),
            categories: vec!["Foo".to_string(), "Bar Baz".to_string()],
            ..Default::default()
        };
        let url = generate_url_with_context("/:categories/:title/", &ctx);
        assert_eq!(
            url, "/foo/bar baz/hello/",
            "All categories must be lowercased in URLs"
        );
    }

    #[test]
    fn test_already_lowercase_categories_unchanged() {
        let ctx = PermalinkContext {
            collection: "posts".to_string(),
            title: "hello".to_string(),
            categories: vec!["tech".to_string()],
            ..Default::default()
        };
        let url = generate_url_with_context("/:categories/:title/", &ctx);
        assert_eq!(url, "/tech/hello/");
    }

    // ========================================================================
    // Unit: Future-date post filtering (Issue 354)
    // ========================================================================

    #[test]
    fn test_filter_future_posts_excludes_future() {
        let mut items = vec![
            CollectionItem {
                slug: "past-post".to_string(),
                date: Some("2020-01-01".to_string()),
                url: "/2020/01/01/past-post.html".to_string(),
                front_matter: FrontMatter::new(),
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                excerpt_html: None,
                source_path: String::new(),
                collection_name: "posts".to_string(),
                id: "/posts/past-post".to_string(),
            },
            CollectionItem {
                slug: "future-post".to_string(),
                date: Some("9999-12-31".to_string()),
                url: "/9999/12/31/future-post.html".to_string(),
                front_matter: FrontMatter::new(),
                content: String::new(),
                html_content: String::new(),
                excerpt: None,
                excerpt_html: None,
                source_path: String::new(),
                collection_name: "posts".to_string(),
                id: "/posts/future-post".to_string(),
            },
        ];
        filter_future_posts(&mut items, false);
        assert_eq!(items.len(), 1, "Future post should be excluded");
        assert_eq!(items[0].slug, "past-post");
    }

    #[test]
    fn test_filter_future_posts_includes_when_future_true() {
        let mut items = vec![CollectionItem {
            slug: "future-post".to_string(),
            date: Some("9999-12-31".to_string()),
            url: "/9999/12/31/future-post.html".to_string(),
            front_matter: FrontMatter::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            excerpt_html: None,
            source_path: String::new(),
            collection_name: "posts".to_string(),
            id: "/posts/future-post".to_string(),
        }];
        filter_future_posts(&mut items, true);
        assert_eq!(
            items.len(),
            1,
            "Future post should be included when future=true"
        );
    }

    #[test]
    fn test_filter_future_posts_keeps_past_posts() {
        let mut items = vec![CollectionItem {
            slug: "yesterday".to_string(),
            date: Some("2020-06-15".to_string()),
            url: "/2020/06/15/yesterday.html".to_string(),
            front_matter: FrontMatter::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            excerpt_html: None,
            source_path: String::new(),
            collection_name: "posts".to_string(),
            id: "/posts/yesterday".to_string(),
        }];
        filter_future_posts(&mut items, false);
        assert_eq!(items.len(), 1, "Past posts should always be included");
    }

    #[test]
    fn test_readme_default_scope_matching_with_backslash_paths() {
        // Regression test for Windows CI: on Windows, rel.to_string_lossy()
        // produces backslashes (e.g., "subdir\README.md"), but config default
        // scope paths use forward slashes ("subdir/README.md"). The
        // starts_with check must normalize backslashes to forward slashes.
        //
        // This test verifies the normalization logic directly, independent of
        // the OS path separator, by checking that a backslash-containing
        // string matches against a forward-slash scope path after
        // normalization.
        let rel_str_windows = r"subdir\README.md";
        let normalized = rel_str_windows.replace('\\', "/");
        let scope_path = "subdir/README.md";
        assert!(
            normalized.starts_with(scope_path),
            "Backslash path '{}' should match scope '{}' after normalization, got '{}'",
            rel_str_windows,
            scope_path,
            normalized,
        );

        // Also test Unicode paths (e.g., Cyrillic directory names)
        let rel_str_unicode_win = r"часть_1\README.md";
        let normalized_unicode = rel_str_unicode_win.replace('\\', "/");
        let scope_path_unicode = "часть_1/README.md";
        assert!(
            normalized_unicode.starts_with(scope_path_unicode),
            "Unicode backslash path should match after normalization"
        );
    }

    #[test]
    fn test_extract_tags_space_separated_string() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "tags".to_string(),
            serde_yaml::Value::String("formatting audios".to_string()),
        );
        let tags = extract_tags(&fm);
        assert_eq!(tags, vec!["formatting", "audios"]);
    }

    #[test]
    fn test_extract_categories_space_separated_string() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "categories".to_string(),
            serde_yaml::Value::String("classics crime mystery".to_string()),
        );
        let cats = extract_categories(&fm);
        assert_eq!(cats, vec!["classics", "crime", "mystery"]);
    }

    #[test]
    fn test_extract_categories_single_string_no_spaces() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "categories".to_string(),
            serde_yaml::Value::String("sample-posts".to_string()),
        );
        let cats = extract_categories(&fm);
        assert_eq!(cats, vec!["sample-posts"]);
    }

    #[test]
    fn test_extract_tags_unicode_space_separated() {
        let mut fm = FrontMatter::new();
        fm.insert(
            "tags".to_string(),
            serde_yaml::Value::String("programacao dados".to_string()),
        );
        let tags = extract_tags(&fm);
        assert_eq!(tags, vec!["programacao", "dados"]);
    }
}
