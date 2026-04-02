//! Jekyll-archives plugin support.
//!
//! This module generates individual archive pages for each category and tag
//! found in posts, implementing the `jekyll-archives` plugin behavior.
//!
//! When a site has a `jekyll-archives` config section, this module:
//! - Reads the enabled archive types (categories, tags)
//! - Generates one HTML page per unique category/tag
//! - Each page uses the specified layout and has `page.title`, `page.type`,
//!   `page.posts`, and `page.url` in the template context

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use liquid::model::Value as LiquidValue;
use liquid::Object;

use crate::collection::CollectionItem;
use crate::config::SiteConfig;
use crate::generator::{url_to_output_path, GeneratorError};
use crate::template::context::{normalize_arrays, yaml_to_liquid};
use crate::template::engine::CachedSiteContext;
use crate::template::layout::LayoutEngine;

/// Archive types that can be enabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveType {
    Categories,
    Tags,
    Year,
    Month,
    Day,
}

/// Parsed configuration for the jekyll-archives plugin.
#[derive(Debug, Clone)]
pub struct ArchivesConfig {
    /// Which archive types are enabled.
    pub enabled: Vec<ArchiveType>,
    /// Layout name for category archive pages.
    pub category_layout: Option<String>,
    /// Layout name for tag archive pages.
    pub tag_layout: Option<String>,
    /// Layout name for year archive pages.
    pub year_layout: Option<String>,
    /// Layout name for month archive pages.
    pub month_layout: Option<String>,
    /// Layout name for day archive pages.
    pub day_layout: Option<String>,
    /// Permalink pattern for category archives (with `:name` placeholder).
    pub category_permalink: String,
    /// Permalink pattern for tag archives (with `:name` placeholder).
    pub tag_permalink: String,
    /// Permalink pattern for year archives (with `:year` placeholder).
    pub year_permalink: String,
    /// Permalink pattern for month archives (with `:year/:month` placeholders).
    pub month_permalink: String,
    /// Permalink pattern for day archives (with `:year/:month/:day` placeholders).
    pub day_permalink: String,
}

impl ArchivesConfig {
    /// Extract archives configuration from a site config.
    ///
    /// Returns `None` if `jekyll-archives` is not present in config or
    /// if no archive types are enabled.
    pub fn from_config(config: &SiteConfig) -> Option<Self> {
        let archives_val = config.extras.get("jekyll-archives")?;
        let mapping = archives_val.as_mapping()?;

        // Parse enabled types
        let enabled = if let Some(enabled_val) =
            mapping.get(serde_yaml::Value::String("enabled".to_string()))
        {
            parse_enabled(enabled_val)
        } else {
            Vec::new()
        };

        if enabled.is_empty() {
            return None;
        }

        // Parse layouts. jekyll-archives supports two formats:
        //   1. `layout: archive` -- a single layout name applied to all archive types
        //   2. `layouts: { category: archive, tag: tag_archive }` -- per-type layout names
        // Check the singular `layout` key first, then the plural `layouts` key.
        let single_layout = mapping
            .get(serde_yaml::Value::String("layout".to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let layouts = mapping
            .get(serde_yaml::Value::String("layouts".to_string()))
            .and_then(|v| v.as_mapping());

        let category_layout = layouts
            .and_then(|m| {
                m.get(serde_yaml::Value::String("category".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| single_layout.clone());

        let tag_layout = layouts
            .and_then(|m| {
                m.get(serde_yaml::Value::String("tag".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| single_layout.clone());

        let year_layout = layouts
            .and_then(|m| {
                m.get(serde_yaml::Value::String("year".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| single_layout.clone());

        let month_layout = layouts
            .and_then(|m| {
                m.get(serde_yaml::Value::String("month".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| single_layout.clone());

        let day_layout = layouts
            .and_then(|m| {
                m.get(serde_yaml::Value::String("day".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| single_layout.clone());

        // Parse permalinks
        let permalinks = mapping
            .get(serde_yaml::Value::String("permalinks".to_string()))
            .and_then(|v| v.as_mapping());

        let category_permalink = permalinks
            .and_then(|m| {
                m.get(serde_yaml::Value::String("category".to_string()))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("/categories/:name/")
            .to_string();

        let tag_permalink = permalinks
            .and_then(|m| {
                m.get(serde_yaml::Value::String("tag".to_string()))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("/tags/:name/")
            .to_string();

        let year_permalink = permalinks
            .and_then(|m| {
                m.get(serde_yaml::Value::String("year".to_string()))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("/:year/")
            .to_string();

        let month_permalink = permalinks
            .and_then(|m| {
                m.get(serde_yaml::Value::String("month".to_string()))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("/:year/:month/")
            .to_string();

        let day_permalink = permalinks
            .and_then(|m| {
                m.get(serde_yaml::Value::String("day".to_string()))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("/:year/:month/:day/")
            .to_string();

        Some(ArchivesConfig {
            enabled,
            category_layout,
            tag_layout,
            year_layout,
            month_layout,
            day_layout,
            category_permalink,
            tag_permalink,
            year_permalink,
            month_permalink,
            day_permalink,
        })
    }

    /// Check if categories are enabled.
    pub fn categories_enabled(&self) -> bool {
        self.enabled.contains(&ArchiveType::Categories)
    }

    /// Check if tags are enabled.
    pub fn tags_enabled(&self) -> bool {
        self.enabled.contains(&ArchiveType::Tags)
    }

    /// Check if year archives are enabled.
    pub fn year_enabled(&self) -> bool {
        self.enabled.contains(&ArchiveType::Year)
    }

    /// Check if month archives are enabled.
    pub fn month_enabled(&self) -> bool {
        self.enabled.contains(&ArchiveType::Month)
    }

    /// Check if day archives are enabled.
    pub fn day_enabled(&self) -> bool {
        self.enabled.contains(&ArchiveType::Day)
    }
}

/// Parse the `enabled` array from the jekyll-archives config.
fn parse_enabled(value: &serde_yaml::Value) -> Vec<ArchiveType> {
    let mut result = Vec::new();
    if let Some(seq) = value.as_sequence() {
        for item in seq {
            if let Some(s) = item.as_str() {
                match s {
                    "categories" => result.push(ArchiveType::Categories),
                    "tags" => result.push(ArchiveType::Tags),
                    "year" => result.push(ArchiveType::Year),
                    "month" => result.push(ArchiveType::Month),
                    "day" => result.push(ArchiveType::Day),
                    _ => {} // Ignore unknown types
                }
            }
        }
    }
    result
}

/// Slugify a name for use in archive URLs.
///
/// Jekyll's archive slug behavior:
/// - Lowercase the name
/// - Replace spaces with hyphens
/// - Keep URL-safe characters
pub fn slugify(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

/// Resolve a permalink pattern by replacing `:name` with the slugified name.
pub fn resolve_permalink(pattern: &str, name: &str) -> String {
    pattern.replace(":name", &slugify(name))
}

/// Resolve a date-based permalink pattern by replacing `:year`, `:month`, `:day`.
pub fn resolve_date_permalink(pattern: &str, year: &str, month: &str, day: &str) -> String {
    pattern
        .replace(":year", year)
        .replace(":month", month)
        .replace(":day", day)
}

/// Extract (year, month, day) strings from a date string.
/// Expects format like "2024-03-15" or "2024-03-15 12:00:00 +0000".
/// Returns None if the date string can't be parsed.
fn parse_date_components(date: &str) -> Option<(String, String, String)> {
    let trimmed = date.trim();
    if trimmed.len() < 10 {
        return None;
    }
    let date_part = &trimmed[..10];
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() >= 3 && parts[0].len() == 4 && parts[1].len() == 2 && parts[2].len() == 2 {
        Some((
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
        ))
    } else {
        None
    }
}

/// Convert a `CollectionItem` to a Liquid `Value` for archive page `page.posts` arrays.
///
/// This is the same full representation used in pagination, including all front
/// matter fields since archive page templates need complete post objects.
fn collection_item_to_liquid_full(item: &CollectionItem) -> LiquidValue {
    let mut obj = Object::new();

    // Copy all front matter fields
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

    // Excerpt
    if let Some(ref excerpt) = item.excerpt {
        obj.insert("excerpt".into(), LiquidValue::scalar(excerpt.clone()));
    }

    // Ensure "short" is set from slug if not in front matter
    if !item.front_matter.contains_key("short") {
        obj.insert("short".into(), LiquidValue::scalar(item.slug.clone()));
    }

    // Normalize category/tag to categories/tags arrays
    crate::generator::normalize_categories_and_tags(&mut obj);

    LiquidValue::Object(obj)
}

/// Generate all archive pages for a site.
///
/// Returns the number of archive pages generated.
pub fn generate_archive_pages(
    posts: &[CollectionItem],
    archives_config: &ArchivesConfig,
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    config: &SiteConfig,
    output_dir: &Path,
) -> Result<usize, GeneratorError> {
    let mut generated = 0;

    // Collect categories and tags from posts
    let mut categories: HashMap<String, Vec<&CollectionItem>> = HashMap::new();
    let mut tags: HashMap<String, Vec<&CollectionItem>> = HashMap::new();
    // Date-based groupings: key is year, year-month, or year-month-day
    let mut years: HashMap<String, Vec<&CollectionItem>> = HashMap::new();
    let mut months: HashMap<String, Vec<&CollectionItem>> = HashMap::new();
    let mut days: HashMap<String, Vec<&CollectionItem>> = HashMap::new();

    let need_year = archives_config.year_enabled();
    let need_month = archives_config.month_enabled();
    let need_day = archives_config.day_enabled();

    for post in posts {
        if archives_config.categories_enabled() {
            let post_categories = crate::collection::extract_categories(&post.front_matter);
            for cat in post_categories {
                categories.entry(cat).or_default().push(post);
            }
        }

        if archives_config.tags_enabled() {
            let post_tags = crate::collection::extract_tags(&post.front_matter);
            for tag in post_tags {
                tags.entry(tag).or_default().push(post);
            }
        }

        // Date-based grouping (only for posts with a valid date)
        if need_year || need_month || need_day {
            if let Some(ref date_str) = post.date {
                if let Some((year, month, day)) = parse_date_components(date_str) {
                    if need_year {
                        years.entry(year.clone()).or_default().push(post);
                    }
                    if need_month {
                        let key = format!("{}-{}", year, month);
                        months.entry(key).or_default().push(post);
                    }
                    if need_day {
                        let key = format!("{}-{}-{}", year, month, day);
                        days.entry(key).or_default().push(post);
                    }
                }
            }
        }
    }

    // Generate category archive pages
    if archives_config.categories_enabled() {
        for (cat_name, cat_posts) in &categories {
            let count = generate_single_archive_page(
                cat_name,
                "category",
                cat_posts,
                archives_config.category_layout.as_deref(),
                &archives_config.category_permalink,
                layout_engine,
                cached_site,
                config,
                output_dir,
            )?;
            generated += count;
        }
    }

    // Generate tag archive pages
    if archives_config.tags_enabled() {
        for (tag_name, tag_posts) in &tags {
            let count = generate_single_archive_page(
                tag_name,
                "tag",
                tag_posts,
                archives_config.tag_layout.as_deref(),
                &archives_config.tag_permalink,
                layout_engine,
                cached_site,
                config,
                output_dir,
            )?;
            generated += count;
        }
    }

    // Generate year archive pages
    if need_year {
        for (year_key, year_posts) in &years {
            let url = resolve_date_permalink(&archives_config.year_permalink, year_key, "01", "01");
            let count = generate_single_date_archive_page(
                year_key, // title is the year string
                "year",
                &url,
                year_posts,
                archives_config.year_layout.as_deref(),
                layout_engine,
                cached_site,
                config,
                output_dir,
            )?;
            generated += count;
        }
    }

    // Generate month archive pages
    if need_month {
        for (month_key, month_posts) in &months {
            // month_key is "YYYY-MM"
            let parts: Vec<&str> = month_key.split('-').collect();
            let (year, month) = (parts[0], parts[1]);
            let url = resolve_date_permalink(&archives_config.month_permalink, year, month, "01");
            let count = generate_single_date_archive_page(
                month_key, // title is "YYYY-MM"
                "month",
                &url,
                month_posts,
                archives_config.month_layout.as_deref(),
                layout_engine,
                cached_site,
                config,
                output_dir,
            )?;
            generated += count;
        }
    }

    // Generate day archive pages
    if need_day {
        for (day_key, day_posts) in &days {
            // day_key is "YYYY-MM-DD"
            let parts: Vec<&str> = day_key.split('-').collect();
            let (year, month, day) = (parts[0], parts[1], parts[2]);
            let url = resolve_date_permalink(&archives_config.day_permalink, year, month, day);
            let count = generate_single_date_archive_page(
                day_key, // title is "YYYY-MM-DD"
                "day",
                &url,
                day_posts,
                archives_config.day_layout.as_deref(),
                layout_engine,
                cached_site,
                config,
                output_dir,
            )?;
            generated += count;
        }
    }

    Ok(generated)
}

/// Generate a single archive page for one category or tag.
#[allow(clippy::too_many_arguments)]
fn generate_single_archive_page(
    name: &str,
    archive_type: &str,
    posts: &[&CollectionItem],
    layout_name: Option<&str>,
    permalink_pattern: &str,
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    _config: &SiteConfig,
    output_dir: &Path,
) -> Result<usize, GeneratorError> {
    let url = resolve_permalink(permalink_pattern, name);

    // Sort posts by date descending (newest first), matching Jekyll's behavior.
    // Within the same date, posts are ordered by slug descending (reverse
    // alphabetical), matching Jekyll's reverse-chronological ordering where
    // filenames with later slugs come first.
    let mut sorted_posts: Vec<&CollectionItem> = posts.to_vec();
    sorted_posts.sort_by(|a, b| {
        let date_a = a.date.as_deref().unwrap_or("");
        let date_b = b.date.as_deref().unwrap_or("");
        date_b.cmp(date_a).then_with(|| b.slug.cmp(&a.slug))
    });

    // Build the posts array for this archive page
    let posts_arr: Vec<LiquidValue> = sorted_posts
        .iter()
        .map(|item| collection_item_to_liquid_full(item))
        .collect();

    // Build the page front matter
    let mut page_fm = crate::frontmatter::FrontMatter::new();
    page_fm.insert(
        "title".to_string(),
        serde_yaml::Value::String(name.to_string()),
    );
    page_fm.insert(
        "type".to_string(),
        serde_yaml::Value::String(archive_type.to_string()),
    );
    page_fm.insert("url".to_string(), serde_yaml::Value::String(url.clone()));
    if let Some(layout) = layout_name {
        page_fm.insert(
            "layout".to_string(),
            serde_yaml::Value::String(layout.to_string()),
        );
    }

    // Build extra page fields (page.posts) that need to be Liquid values
    let extra_page_fields = vec![(
        "posts".to_string(),
        normalize_arrays(LiquidValue::Array(posts_arr)),
    )];

    // Render through the layout if specified
    let html = if let Some(layout) = layout_name {
        match layout_engine.render_with_extra_page_fields(
            layout,
            "",
            &page_fm,
            &extra_page_fields,
            cached_site,
        ) {
            Ok(rendered) => rendered,
            Err(e) => {
                eprintln!(
                    "Warning: failed to render archive page for {} '{}': {}",
                    archive_type, name, e
                );
                // Fall back to empty content
                String::new()
            }
        }
    } else {
        // No layout specified, just generate a basic page
        format!(
            "<h1>{}</h1>\n<ul>\n{}</ul>\n",
            name,
            sorted_posts
                .iter()
                .map(|p| format!(
                    "  <li><a href=\"{}\">{}</a></li>\n",
                    p.url,
                    p.front_matter
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&p.slug)
                ))
                .collect::<String>()
        )
    };

    let out_path = url_to_output_path(output_dir, &url);
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&out_path, &html).is_ok() {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Generate a single date-based archive page (year, month, or day).
///
/// Similar to `generate_single_archive_page` but takes a pre-resolved URL
/// instead of a permalink pattern with `:name`.
#[allow(clippy::too_many_arguments)]
fn generate_single_date_archive_page(
    title: &str,
    archive_type: &str,
    url: &str,
    posts: &[&CollectionItem],
    layout_name: Option<&str>,
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    _config: &SiteConfig,
    output_dir: &Path,
) -> Result<usize, GeneratorError> {
    // Sort posts by date descending (newest first)
    let mut sorted_posts: Vec<&CollectionItem> = posts.to_vec();
    sorted_posts.sort_by(|a, b| {
        let date_a = a.date.as_deref().unwrap_or("");
        let date_b = b.date.as_deref().unwrap_or("");
        date_b.cmp(date_a).then_with(|| b.slug.cmp(&a.slug))
    });

    let posts_arr: Vec<LiquidValue> = sorted_posts
        .iter()
        .map(|item| collection_item_to_liquid_full(item))
        .collect();

    let mut page_fm = crate::frontmatter::FrontMatter::new();
    page_fm.insert(
        "title".to_string(),
        serde_yaml::Value::String(title.to_string()),
    );
    page_fm.insert(
        "type".to_string(),
        serde_yaml::Value::String(archive_type.to_string()),
    );
    page_fm.insert(
        "url".to_string(),
        serde_yaml::Value::String(url.to_string()),
    );
    if let Some(layout) = layout_name {
        page_fm.insert(
            "layout".to_string(),
            serde_yaml::Value::String(layout.to_string()),
        );
    }

    let extra_page_fields = vec![(
        "posts".to_string(),
        normalize_arrays(LiquidValue::Array(posts_arr)),
    )];

    let html = if let Some(layout) = layout_name {
        match layout_engine.render_with_extra_page_fields(
            layout,
            "",
            &page_fm,
            &extra_page_fields,
            cached_site,
        ) {
            Ok(rendered) => rendered,
            Err(e) => {
                eprintln!(
                    "Warning: failed to render date archive page for {} '{}': {}",
                    archive_type, title, e
                );
                String::new()
            }
        }
    } else {
        format!(
            "<h1>{}</h1>\n<ul>\n{}</ul>\n",
            title,
            sorted_posts
                .iter()
                .map(|p| format!(
                    "  <li><a href=\"{}\">{}</a></li>\n",
                    p.url,
                    p.front_matter
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&p.slug)
                ))
                .collect::<String>()
        )
    };

    let out_path = url_to_output_path(output_dir, url);
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&out_path, &html).is_ok() {
        Ok(1)
    } else {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::CollectionItem;
    use crate::config::SiteConfig;
    use crate::frontmatter::FrontMatter;

    fn make_post(
        title: &str,
        date: &str,
        slug: &str,
        categories: Vec<&str>,
        tags: Vec<&str>,
    ) -> CollectionItem {
        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String(title.to_string()),
        );
        if !categories.is_empty() {
            let cats: Vec<serde_yaml::Value> = categories
                .iter()
                .map(|c| serde_yaml::Value::String(c.to_string()))
                .collect();
            fm.insert("categories".to_string(), serde_yaml::Value::Sequence(cats));
        }
        if !tags.is_empty() {
            let tag_vals: Vec<serde_yaml::Value> = tags
                .iter()
                .map(|t| serde_yaml::Value::String(t.to_string()))
                .collect();
            fm.insert("tags".to_string(), serde_yaml::Value::Sequence(tag_vals));
        }
        CollectionItem {
            slug: slug.to_string(),
            front_matter: fm,
            content: format!("Content of {}", title),
            html_content: format!("<p>Content of {}</p>", title),
            excerpt: None,
            excerpt_html: None,
            url: format!("/blog/{}.html", slug),
            source_path: format!("_posts/{}-{}.md", date, slug),
            date: Some(date.to_string()),
            collection_name: "posts".to_string(),
            id: format!("/posts/{}", slug),
        }
    }

    // ========================================================================
    // Config parsing
    // ========================================================================

    #[test]
    fn test_config_parsing_full() {
        let yaml = r#"
jekyll-archives:
  enabled:
    - categories
    - tags
  layouts:
    category: category
    tag: tag
  permalinks:
    tag: /tags/:name/
    category: /categories/:name/
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let archives = ArchivesConfig::from_config(&config).unwrap();

        assert!(archives.categories_enabled());
        assert!(archives.tags_enabled());
        assert_eq!(archives.category_layout, Some("category".to_string()));
        assert_eq!(archives.tag_layout, Some("tag".to_string()));
        assert_eq!(archives.category_permalink, "/categories/:name/");
        assert_eq!(archives.tag_permalink, "/tags/:name/");
    }

    #[test]
    fn test_config_parsing_no_archives() {
        let yaml = "url: https://example.com\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(ArchivesConfig::from_config(&config).is_none());
    }

    #[test]
    fn test_config_parsing_empty_enabled() {
        let yaml = r#"
jekyll-archives:
  enabled: []
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(ArchivesConfig::from_config(&config).is_none());
    }

    #[test]
    fn test_config_parsing_only_categories() {
        let yaml = r#"
jekyll-archives:
  enabled:
    - categories
  layouts:
    category: archive
  permalinks:
    category: /cat/:name/
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let archives = ArchivesConfig::from_config(&config).unwrap();

        assert!(archives.categories_enabled());
        assert!(!archives.tags_enabled());
        assert_eq!(archives.category_layout, Some("archive".to_string()));
        assert_eq!(archives.category_permalink, "/cat/:name/");
    }

    #[test]
    fn test_config_parsing_singular_layout_key() {
        // Mediumish-style config: `layout: archive` (singular, applies to all types)
        let yaml = r#"
jekyll-archives:
  enabled:
    - categories
  layout: archive
  permalinks:
    category: '/category/:name/'
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let archives = ArchivesConfig::from_config(&config).unwrap();

        assert!(archives.categories_enabled());
        assert_eq!(
            archives.category_layout,
            Some("archive".to_string()),
            "Singular `layout` key should be used for category archives"
        );
    }

    // ========================================================================
    // Slug generation
    // ========================================================================

    #[test]
    fn test_slugify_machine_learning() {
        assert_eq!(slugify("Machine Learning"), "machine-learning");
    }

    #[test]
    fn test_slugify_cpp() {
        assert_eq!(slugify("C++"), "c++");
    }

    #[test]
    fn test_slugify_already_lowercase() {
        assert_eq!(slugify("rust"), "rust");
    }

    #[test]
    fn test_slugify_unicode() {
        // Non-ASCII characters should pass through gracefully
        assert_eq!(slugify("Programacao"), "programacao");
    }

    #[test]
    fn test_slugify_mixed_case_spaces() {
        assert_eq!(slugify("Web Development"), "web-development");
    }

    // ========================================================================
    // Permalink resolution
    // ========================================================================

    #[test]
    fn test_permalink_category() {
        assert_eq!(
            resolve_permalink("/categories/:name/", "Web Development"),
            "/categories/web-development/"
        );
    }

    #[test]
    fn test_permalink_tag() {
        assert_eq!(resolve_permalink("/tags/:name/", "rust"), "/tags/rust/");
    }

    // ========================================================================
    // Integration: Archive page generation
    // ========================================================================

    #[test]
    fn test_generate_archive_pages_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let posts = vec![
            make_post(
                "Post A",
                "2024-01-03",
                "post-a",
                vec!["ML"],
                vec!["rust", "ai"],
            ),
            make_post(
                "Post B",
                "2024-01-02",
                "post-b",
                vec!["ML", "Web"],
                vec!["rust"],
            ),
            make_post(
                "Post C",
                "2024-01-01",
                "post-c",
                vec!["Web"],
                vec!["python"],
            ),
        ];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Categories, ArchiveType::Tags],
            category_layout: None,
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        // 2 categories (ML, Web) + 3 tags (rust, ai, python) = 5 pages
        assert_eq!(count, 5);

        // Verify category pages exist
        assert!(output_dir.join("categories/ml/index.html").exists());
        assert!(output_dir.join("categories/web/index.html").exists());

        // Verify tag pages exist
        assert!(output_dir.join("tags/rust/index.html").exists());
        assert!(output_dir.join("tags/ai/index.html").exists());
        assert!(output_dir.join("tags/python/index.html").exists());
    }

    #[test]
    fn test_archive_page_contains_post_listings() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let posts = vec![
            make_post("First Post", "2024-01-02", "first-post", vec!["ML"], vec![]),
            make_post(
                "Second Post",
                "2024-01-01",
                "second-post",
                vec!["ML"],
                vec![],
            ),
        ];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Categories],
            category_layout: None,
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        let content = fs::read_to_string(output_dir.join("categories/ml/index.html")).unwrap();
        assert!(
            content.contains("ML"),
            "Archive page should contain the category name"
        );
        assert!(
            content.contains("First Post"),
            "Archive page should list first post"
        );
        assert!(
            content.contains("Second Post"),
            "Archive page should list second post"
        );
    }

    #[test]
    fn test_archive_posts_reverse_chronological() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let posts = vec![
            make_post("Old Post", "2024-01-01", "old-post", vec!["Dev"], vec![]),
            make_post("New Post", "2024-01-03", "new-post", vec!["Dev"], vec![]),
            make_post("Mid Post", "2024-01-02", "mid-post", vec!["Dev"], vec![]),
        ];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Categories],
            category_layout: None,
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        let content = fs::read_to_string(output_dir.join("categories/dev/index.html")).unwrap();
        // Verify newest post appears before oldest post in the HTML
        let new_pos = content.find("New Post").expect("Should contain New Post");
        let mid_pos = content.find("Mid Post").expect("Should contain Mid Post");
        let old_pos = content.find("Old Post").expect("Should contain Old Post");
        assert!(
            new_pos < mid_pos && mid_pos < old_pos,
            "Posts should be in reverse chronological order: new_pos={}, mid_pos={}, old_pos={}",
            new_pos,
            mid_pos,
            old_pos
        );
    }

    #[test]
    fn test_no_archive_pages_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let _posts = vec![make_post(
            "Post A",
            "2024-01-01",
            "post-a",
            vec!["ML"],
            vec!["rust"],
        )];

        // No archives config means from_config returns None
        let yaml = "url: https://example.com\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(ArchivesConfig::from_config(&config).is_none());

        // Verify no files are generated when there's no config
        // (the caller should check for None and not call generate_archive_pages)
        assert!(!output_dir.join("categories").exists());
        assert!(!output_dir.join("tags").exists());

        // Also verify empty enabled produces None
        let yaml2 = "jekyll-archives:\n  enabled: []\n";
        let config2 = SiteConfig::from_yaml_str(yaml2).unwrap();
        assert!(ArchivesConfig::from_config(&config2).is_none());
    }

    #[test]
    fn test_archive_with_layout() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        // Create a simple layout
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();
        fs::write(
            layouts_dir.join("tag.html"),
            "<html><body><h1>{{ page.title }}</h1><p>Type: {{ page.type }}</p>{{ content }}</body></html>",
        )
        .unwrap();

        let posts = vec![make_post(
            "Post A",
            "2024-01-01",
            "post-a",
            vec![],
            vec!["rust"],
        )];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Tags],
            category_layout: None,
            tag_layout: Some("tag".to_string()),
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        assert_eq!(count, 1);
        let content = fs::read_to_string(output_dir.join("tags/rust/index.html")).unwrap();
        assert!(
            content.contains("<h1>rust</h1>"),
            "Layout should render page.title: {}",
            content
        );
        assert!(
            content.contains("Type: tag"),
            "Layout should render page.type: {}",
            content
        );
    }

    #[test]
    fn test_archive_page_has_correct_page_posts() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        // Create a layout that renders page.posts
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();
        fs::write(
            layouts_dir.join("category.html"),
            "{% for post in page.posts %}<div>{{ post.title }}</div>{% endfor %}",
        )
        .unwrap();

        let posts = vec![
            make_post(
                "Relevant Post",
                "2024-01-02",
                "relevant",
                vec!["ML"],
                vec![],
            ),
            make_post("Other Post", "2024-01-01", "other", vec!["Web"], vec![]),
        ];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Categories],
            category_layout: Some("category".to_string()),
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        // ML category should only have "Relevant Post"
        let ml_content = fs::read_to_string(output_dir.join("categories/ml/index.html")).unwrap();
        assert!(
            ml_content.contains("Relevant Post"),
            "ML archive should contain Relevant Post: {}",
            ml_content
        );
        assert!(
            !ml_content.contains("Other Post"),
            "ML archive should NOT contain Other Post: {}",
            ml_content
        );

        // Web category should only have "Other Post"
        let web_content = fs::read_to_string(output_dir.join("categories/web/index.html")).unwrap();
        assert!(
            web_content.contains("Other Post"),
            "Web archive should contain Other Post: {}",
            web_content
        );
        assert!(
            !web_content.contains("Relevant Post"),
            "Web archive should NOT contain Relevant Post: {}",
            web_content
        );
    }

    // ========================================================================
    // Date-based archives: Config parsing
    // ========================================================================

    #[test]
    fn test_config_parsing_year_enabled() {
        let yaml = r#"
jekyll-archives:
  enabled:
    - year
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let archives = ArchivesConfig::from_config(&config).unwrap();
        assert!(archives.year_enabled());
        assert!(!archives.month_enabled());
        assert!(!archives.day_enabled());
    }

    #[test]
    fn test_config_parsing_all_five_types() {
        let yaml = r#"
jekyll-archives:
  enabled:
    - year
    - month
    - day
    - tags
    - categories
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let archives = ArchivesConfig::from_config(&config).unwrap();
        assert!(archives.year_enabled());
        assert!(archives.month_enabled());
        assert!(archives.day_enabled());
        assert!(archives.tags_enabled());
        assert!(archives.categories_enabled());
    }

    #[test]
    fn test_config_parsing_tags_only_no_date_types() {
        let yaml = r#"
jekyll-archives:
  enabled:
    - tags
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let archives = ArchivesConfig::from_config(&config).unwrap();
        assert!(!archives.year_enabled());
        assert!(!archives.month_enabled());
        assert!(!archives.day_enabled());
        assert!(archives.tags_enabled());
    }

    #[test]
    fn test_config_parsing_date_permalinks() {
        let yaml = r#"
jekyll-archives:
  enabled:
    - year
    - month
    - day
  permalinks:
    year: "/blog/:year/"
    month: "/blog/:year/:month/"
    day: "/blog/:year/:month/:day/"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let archives = ArchivesConfig::from_config(&config).unwrap();
        assert_eq!(archives.year_permalink, "/blog/:year/");
        assert_eq!(archives.month_permalink, "/blog/:year/:month/");
        assert_eq!(archives.day_permalink, "/blog/:year/:month/:day/");
    }

    #[test]
    fn test_config_parsing_date_permalinks_defaults() {
        let yaml = r#"
jekyll-archives:
  enabled:
    - year
    - month
    - day
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let archives = ArchivesConfig::from_config(&config).unwrap();
        assert_eq!(archives.year_permalink, "/:year/");
        assert_eq!(archives.month_permalink, "/:year/:month/");
        assert_eq!(archives.day_permalink, "/:year/:month/:day/");
    }

    #[test]
    fn test_config_parsing_date_layouts() {
        let yaml = r#"
jekyll-archives:
  enabled:
    - year
    - month
    - day
  layouts:
    year: year-archive
    month: month-archive
    day: day-archive
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let archives = ArchivesConfig::from_config(&config).unwrap();
        assert_eq!(archives.year_layout, Some("year-archive".to_string()));
        assert_eq!(archives.month_layout, Some("month-archive".to_string()));
        assert_eq!(archives.day_layout, Some("day-archive".to_string()));
    }

    #[test]
    fn test_config_parsing_date_layout_fallback_to_singular() {
        let yaml = r#"
jekyll-archives:
  enabled:
    - year
  layout: archive
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let archives = ArchivesConfig::from_config(&config).unwrap();
        assert_eq!(archives.year_layout, Some("archive".to_string()));
    }

    // ========================================================================
    // Date-based archives: Permalink resolution
    // ========================================================================

    #[test]
    fn test_resolve_date_permalink_year() {
        assert_eq!(
            resolve_date_permalink("/:year/", "2024", "03", "15"),
            "/2024/"
        );
    }

    #[test]
    fn test_resolve_date_permalink_month() {
        assert_eq!(
            resolve_date_permalink("/:year/:month/", "2024", "03", "15"),
            "/2024/03/"
        );
    }

    #[test]
    fn test_resolve_date_permalink_day() {
        assert_eq!(
            resolve_date_permalink("/:year/:month/:day/", "2024", "03", "15"),
            "/2024/03/15/"
        );
    }

    #[test]
    fn test_resolve_date_permalink_blog_prefix() {
        assert_eq!(
            resolve_date_permalink("/blog/:year/", "2024", "01", "01"),
            "/blog/2024/"
        );
    }

    // ========================================================================
    // Date-based archives: Generation
    // ========================================================================

    #[test]
    fn test_year_archive_generation() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let posts = vec![
            make_post("Post 2024a", "2024-03-15", "post-2024a", vec![], vec![]),
            make_post("Post 2024b", "2024-01-10", "post-2024b", vec![], vec![]),
            make_post("Post 2023", "2023-06-01", "post-2023", vec![], vec![]),
        ];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Year],
            category_layout: None,
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        // 2 unique years: 2023, 2024
        assert_eq!(count, 2);
        assert!(output_dir.join("2024/index.html").exists());
        assert!(output_dir.join("2023/index.html").exists());

        // 2024 page should contain both 2024 posts
        let content_2024 = fs::read_to_string(output_dir.join("2024/index.html")).unwrap();
        assert!(content_2024.contains("Post 2024a"));
        assert!(content_2024.contains("Post 2024b"));
        assert!(!content_2024.contains("Post 2023"));

        // 2023 page should contain only the 2023 post
        let content_2023 = fs::read_to_string(output_dir.join("2023/index.html")).unwrap();
        assert!(content_2023.contains("Post 2023"));
        assert!(!content_2023.contains("Post 2024a"));
    }

    #[test]
    fn test_year_archive_reverse_chronological() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let posts = vec![
            make_post("Old Post", "2024-01-01", "old-post", vec![], vec![]),
            make_post("New Post", "2024-06-15", "new-post", vec![], vec![]),
            make_post("Mid Post", "2024-03-10", "mid-post", vec![], vec![]),
        ];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Year],
            category_layout: None,
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        let content = fs::read_to_string(output_dir.join("2024/index.html")).unwrap();
        let new_pos = content.find("New Post").expect("Should contain New Post");
        let mid_pos = content.find("Mid Post").expect("Should contain Mid Post");
        let old_pos = content.find("Old Post").expect("Should contain Old Post");
        assert!(
            new_pos < mid_pos && mid_pos < old_pos,
            "Posts should be in reverse chronological order"
        );
    }

    #[test]
    fn test_month_archive_generation() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let posts = vec![
            make_post("March Post", "2024-03-15", "march-post", vec![], vec![]),
            make_post("Jan Post A", "2024-01-20", "jan-post-a", vec![], vec![]),
            make_post("Jan Post B", "2024-01-05", "jan-post-b", vec![], vec![]),
        ];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Month],
            category_layout: None,
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        // 2 unique months: 2024-01 and 2024-03
        assert_eq!(count, 2);
        assert!(output_dir.join("2024/01/index.html").exists());
        assert!(output_dir.join("2024/03/index.html").exists());

        // January page should have both January posts, not March
        let jan_content = fs::read_to_string(output_dir.join("2024/01/index.html")).unwrap();
        assert!(jan_content.contains("Jan Post A"));
        assert!(jan_content.contains("Jan Post B"));
        assert!(!jan_content.contains("March Post"));
    }

    #[test]
    fn test_day_archive_generation() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let posts = vec![
            make_post("Day1 Post", "2024-03-15", "day1-post", vec![], vec![]),
            make_post("Day2 Post", "2024-03-16", "day2-post", vec![], vec![]),
        ];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Day],
            category_layout: None,
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        assert_eq!(count, 2);
        assert!(output_dir.join("2024/03/15/index.html").exists());
        assert!(output_dir.join("2024/03/16/index.html").exists());
    }

    #[test]
    fn test_combined_date_and_tag_archives() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let posts = vec![
            make_post("Post A", "2024-01-15", "post-a", vec![], vec!["rust"]),
            make_post(
                "Post B",
                "2023-06-01",
                "post-b",
                vec![],
                vec!["rust", "python"],
            ),
        ];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Year, ArchiveType::Tags],
            category_layout: None,
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        // 2 years (2023, 2024) + 2 tags (rust, python) = 4 pages
        assert_eq!(count, 4);
        assert!(output_dir.join("2024/index.html").exists());
        assert!(output_dir.join("2023/index.html").exists());
        assert!(output_dir.join("tags/rust/index.html").exists());
        assert!(output_dir.join("tags/python/index.html").exists());
    }

    #[test]
    fn test_year_archive_with_layout() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();
        fs::write(
            layouts_dir.join("year-archive.html"),
            "<html><h1>{{ page.title }}</h1><p>Type: {{ page.type }}</p>{% for post in page.posts %}<div>{{ post.title }}</div>{% endfor %}</html>",
        ).unwrap();

        let posts = vec![make_post(
            "Alpha Post",
            "2024-05-01",
            "alpha-post",
            vec![],
            vec![],
        )];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Year],
            category_layout: None,
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: Some("year-archive".to_string()),
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        assert_eq!(count, 1);
        let content = fs::read_to_string(output_dir.join("2024/index.html")).unwrap();
        assert!(
            content.contains("<h1>2024</h1>"),
            "page.title should be the year: {}",
            content
        );
        assert!(
            content.contains("Type: year"),
            "page.type should be 'year': {}",
            content
        );
        assert!(
            content.contains("Alpha Post"),
            "page.posts should contain the post: {}",
            content
        );
    }

    #[test]
    fn test_post_without_date_excluded_from_date_archives() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let mut no_date_post = make_post("No Date", "2024-01-01", "no-date", vec![], vec![]);
        no_date_post.date = None;

        let posts = vec![
            make_post("Dated Post", "2024-01-15", "dated-post", vec![], vec![]),
            no_date_post,
        ];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Year],
            category_layout: None,
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        // Only 1 year archive (2024) from the dated post
        assert_eq!(count, 1);
        let content = fs::read_to_string(output_dir.join("2024/index.html")).unwrap();
        assert!(content.contains("Dated Post"));
        assert!(!content.contains("No Date"));
    }

    #[test]
    fn test_date_archive_unicode_post_titles() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let posts = vec![
            make_post(
                "Programmierung auf Deutsch: Ubung macht den Meister",
                "2024-01-15",
                "german-post",
                vec![],
                vec![],
            ),
            make_post(
                "Programacao em Portugues",
                "2024-02-10",
                "portuguese-post",
                vec![],
                vec![],
            ),
        ];

        let archives_config = ArchivesConfig {
            enabled: vec![ArchiveType::Year],
            category_layout: None,
            tag_layout: None,
            category_permalink: "/categories/:name/".to_string(),
            tag_permalink: "/tags/:name/".to_string(),
            year_layout: None,
            month_layout: None,
            day_layout: None,
            year_permalink: "/:year/".to_string(),
            month_permalink: "/:year/:month/".to_string(),
            day_permalink: "/:year/:month/:day/".to_string(),
        };

        let config = SiteConfig::default();
        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_archive_pages(
            &posts,
            &archives_config,
            &layout_engine,
            &cached_site,
            &config,
            output_dir,
        )
        .unwrap();

        assert_eq!(count, 1);
        let content = fs::read_to_string(output_dir.join("2024/index.html")).unwrap();
        assert!(content.contains("Programmierung auf Deutsch"));
        assert!(content.contains("Portugues"));
    }
}
