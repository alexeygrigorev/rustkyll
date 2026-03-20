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
    /// Permalink pattern for category archives (with `:name` placeholder).
    pub category_permalink: String,
    /// Permalink pattern for tag archives (with `:name` placeholder).
    pub tag_permalink: String,
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

        // Parse layouts
        let layouts = mapping
            .get(serde_yaml::Value::String("layouts".to_string()))
            .and_then(|v| v.as_mapping());

        let category_layout = layouts.and_then(|m| {
            m.get(serde_yaml::Value::String("category".to_string()))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

        let tag_layout = layouts.and_then(|m| {
            m.get(serde_yaml::Value::String("tag".to_string()))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

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

        Some(ArchivesConfig {
            enabled,
            category_layout,
            tag_layout,
            category_permalink,
            tag_permalink,
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
                    _ => {} // Ignore unknown types (years, months, days -- descoped)
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

    // Sort posts by date descending (newest first)
    let mut sorted_posts: Vec<&CollectionItem> = posts.to_vec();
    sorted_posts.sort_by(|a, b| {
        let date_a = a.date.as_deref().unwrap_or("");
        let date_b = b.date.as_deref().unwrap_or("");
        date_b.cmp(date_a).then_with(|| a.slug.cmp(&b.slug))
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
}
