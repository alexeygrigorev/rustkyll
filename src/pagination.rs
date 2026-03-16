//! Pagination support implementing Jekyll's `jekyll-paginate` plugin.
//!
//! This module generates paginated index pages when `paginate` is set in
//! `_config.yml`. It provides the `paginator` variable in templates with
//! all standard Jekyll pagination fields.

use std::fs;
use std::path::Path;

use liquid::model::Value as LiquidValue;
use liquid::Object;

use crate::collection::{CollectionItem, Page};
use crate::config::SiteConfig;
use crate::generator::{url_to_output_path, GeneratorError};
use crate::template::context::{normalize_arrays, yaml_to_liquid};
use crate::template::engine::CachedSiteContext;
use crate::template::layout::{build_render_context_page_only, LayoutEngine};

/// Pagination configuration extracted from `_config.yml`.
#[derive(Debug, Clone)]
pub struct PaginationConfig {
    /// Number of posts per page.
    pub per_page: usize,
    /// URL pattern for paginated pages (e.g., `/blog/page:num/`).
    /// The `:num` placeholder is replaced with the page number.
    pub paginate_path: String,
}

impl PaginationConfig {
    /// Extract pagination configuration from a site config.
    ///
    /// Returns `None` if `paginate` is not set or is not a positive integer.
    pub fn from_config(config: &SiteConfig) -> Option<Self> {
        let per_page = config
            .extras
            .get("paginate")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)?;

        if per_page == 0 {
            return None;
        }

        let paginate_path = config
            .extras
            .get("paginate_path")
            .and_then(|v| v.as_str())
            .unwrap_or("/blog/page:num/")
            .to_string();

        Some(Self {
            per_page,
            paginate_path,
        })
    }
}

/// Build a `paginator` Liquid object for a given page of results.
///
/// The paginator object contains:
/// - `posts`: array of post objects for this page
/// - `per_page`: number of posts per page
/// - `total_posts`: total number of posts across all pages
/// - `total_pages`: total number of pages
/// - `page`: current page number (1-based)
/// - `previous_page`: previous page number (0/nil if on first page)
/// - `next_page`: next page number (0/nil if on last page)
/// - `previous_page_path`: URL path to previous page (nil if on first page)
/// - `next_page_path`: URL path to next page (nil if on last page)
fn build_paginator_object(
    page_posts: &[&CollectionItem],
    page_num: usize,
    per_page: usize,
    total_posts: usize,
    total_pages: usize,
    paginate_path: &str,
) -> LiquidValue {
    let mut paginator = Object::new();

    // posts -- array of post objects for this page
    let posts_arr: Vec<LiquidValue> = page_posts
        .iter()
        .map(|item| collection_item_to_liquid_full(item))
        .collect();
    paginator.insert(
        "posts".into(),
        normalize_arrays(LiquidValue::Array(posts_arr)),
    );

    paginator.insert("per_page".into(), LiquidValue::scalar(per_page as i64));
    paginator.insert(
        "total_posts".into(),
        LiquidValue::scalar(total_posts as i64),
    );
    paginator.insert(
        "total_pages".into(),
        LiquidValue::scalar(total_pages as i64),
    );
    paginator.insert("page".into(), LiquidValue::scalar(page_num as i64));

    // previous_page: page number or 0/nil
    if page_num > 1 {
        paginator.insert(
            "previous_page".into(),
            LiquidValue::scalar((page_num - 1) as i64),
        );
        let prev_path = if page_num == 2 {
            // Page 1 is at the root index, not at paginate_path
            "/".to_string()
        } else {
            paginate_path.replace(":num", &(page_num - 1).to_string())
        };
        paginator.insert("previous_page_path".into(), LiquidValue::scalar(prev_path));
    } else {
        paginator.insert("previous_page".into(), LiquidValue::Nil);
        paginator.insert("previous_page_path".into(), LiquidValue::Nil);
    }

    // next_page: page number or 0/nil
    if page_num < total_pages {
        paginator.insert(
            "next_page".into(),
            LiquidValue::scalar((page_num + 1) as i64),
        );
        let next_path = paginate_path.replace(":num", &(page_num + 1).to_string());
        paginator.insert("next_page_path".into(), LiquidValue::scalar(next_path));
    } else {
        paginator.insert("next_page".into(), LiquidValue::Nil);
        paginator.insert("next_page_path".into(), LiquidValue::Nil);
    }

    LiquidValue::Object(paginator)
}

/// Convert a `CollectionItem` to a Liquid `Value` for paginator.posts arrays.
///
/// Unlike the slim version used for site context, this includes all front matter
/// fields since paginator.posts needs to be a complete representation.
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

    // Excerpt -- Jekyll provides post.excerpt in paginator.posts
    // Use the CollectionItem's excerpt if present, otherwise generate from content
    if let Some(ref excerpt) = item.excerpt {
        obj.insert("excerpt".into(), LiquidValue::scalar(excerpt.clone()));
    } else if !item.front_matter.contains_key("excerpt") {
        let excerpt = generate_excerpt(&item.html_content);
        obj.insert("excerpt".into(), LiquidValue::scalar(excerpt));
    }

    // Ensure "short" is set from slug if not in front matter
    if !item.front_matter.contains_key("short") {
        obj.insert("short".into(), LiquidValue::scalar(item.slug.clone()));
    }

    LiquidValue::Object(obj)
}

/// Generate a simple excerpt from HTML content.
///
/// Takes the first paragraph (up to the first `</p>` or double newline).
fn generate_excerpt(html: &str) -> String {
    // Look for first <p>...</p> block
    if let Some(start) = html.find("<p>") {
        if let Some(end) = html[start..].find("</p>") {
            return html[start..start + end + 4].to_string();
        }
    }
    // Fallback: first 200 chars
    let truncated: String = html.chars().take(200).collect();
    truncated
}

/// Generate all pagination pages for a site.
///
/// This implements Jekyll's `jekyll-paginate` behavior:
/// - Page 1 is the index.html (re-rendered with a `paginator` variable)
/// - Pages 2+ are generated at `paginate_path` locations
///
/// The index page template is found from the loaded pages (looking for
/// `index.html` or `index.md` at the root).
///
/// Returns the number of pagination pages generated (including page 1 re-render).
pub fn generate_pagination_pages(
    posts: &[CollectionItem],
    pagination: &PaginationConfig,
    index_page: &Page,
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    config: &SiteConfig,
    output_dir: &Path,
) -> Result<usize, GeneratorError> {
    if pagination.per_page == 0 || posts.is_empty() {
        return Ok(0);
    }

    // Sort posts by date descending (newest first) like Jekyll
    let mut sorted_posts: Vec<&CollectionItem> = posts.iter().collect();
    sorted_posts.sort_by(|a, b| {
        let date_a = a.date.as_deref().unwrap_or("");
        let date_b = b.date.as_deref().unwrap_or("");
        date_b.cmp(date_a).then_with(|| a.slug.cmp(&b.slug))
    });

    let total_posts = sorted_posts.len();
    let total_pages = total_posts.div_ceil(pagination.per_page);

    let mut generated = 0;

    // Generate all pages (including page 1 which overwrites index.html)
    for page_num in 1..=total_pages {
        let start = (page_num - 1) * pagination.per_page;
        let end = std::cmp::min(start + pagination.per_page, total_posts);
        let page_posts: Vec<&CollectionItem> = sorted_posts[start..end].to_vec();

        let paginator = build_paginator_object(
            &page_posts,
            page_num,
            pagination.per_page,
            total_posts,
            total_pages,
            &pagination.paginate_path,
        );

        // Determine the output URL for this page
        let page_url = if page_num == 1 {
            index_page.url.clone()
        } else {
            pagination
                .paginate_path
                .replace(":num", &page_num.to_string())
        };

        // Build page front matter with url set correctly
        let mut page_fm = index_page.front_matter.clone();
        // Apply config defaults for pages
        let defaults = config.defaults_for_page(&index_page.source_path);
        for (key, value) in defaults {
            page_fm.entry(key).or_insert(value);
        }
        page_fm.insert("url".into(), serde_yaml::Value::String(page_url.clone()));

        // page.name -- the source filename
        let page_name = std::path::Path::new(&index_page.source_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        page_fm
            .entry("name".into())
            .or_insert_with(|| serde_yaml::Value::String(page_name));
        page_fm
            .entry("path".into())
            .or_insert_with(|| serde_yaml::Value::String(index_page.source_path.clone()));

        // Resolve layout from front matter
        let layout_name = page_fm
            .get("layout")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        // Build the render context with paginator added
        let is_markdown = index_page.source_path.ends_with(".md");
        let raw_content = &index_page.content;

        let render_result = render_with_paginator(
            layout_engine,
            cached_site,
            raw_content,
            &page_fm,
            &paginator,
            layout_name.as_deref(),
            is_markdown,
        );

        let html = match render_result {
            Ok(html) => html,
            Err(e) => {
                eprintln!(
                    "Warning: failed to render pagination page {}, writing fallback: {}",
                    page_num, e
                );
                // Fall back to raw content (with markdown conversion if needed),
                // matching the normal page generator's fallback behavior.
                if is_markdown {
                    crate::frontmatter::markdown_to_html(raw_content)
                } else {
                    raw_content.to_string()
                }
            }
        };

        let out_path = url_to_output_path(output_dir, &page_url);
        if let Some(parent) = out_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::write(&out_path, &html).is_ok() {
            generated += 1;
        }
    }

    Ok(generated)
}

/// Render a page with a `paginator` variable in the context.
///
/// This is similar to the standard page rendering pipeline but adds
/// the `paginator` object to the template context so that templates
/// can access `paginator.posts`, `paginator.total_pages`, etc.
fn render_with_paginator(
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    raw_content: &str,
    page_front_matter: &crate::frontmatter::FrontMatter,
    paginator: &LiquidValue,
    layout_name: Option<&str>,
    is_markdown: bool,
) -> Result<String, crate::template::TemplateError> {
    // Build the base context with page and content
    let mut ctx = build_render_context_page_only("", page_front_matter);

    // Add the paginator to the context -- this will be accessible as
    // `paginator.*` in templates because LenientObject falls through
    // to inner.get() for unknown keys.
    ctx.insert("paginator".into(), paginator.clone());

    // Step 1: Render the page content (resolving Liquid tags including paginator)
    let rendered_content = if raw_content.contains("{{") || raw_content.contains("{%") {
        layout_engine
            .engine()
            .parse_and_render_with_cached_site(raw_content, &ctx, cached_site)?
    } else {
        raw_content.to_string()
    };

    // Step 2: Convert markdown to HTML if needed
    let html_content = if is_markdown {
        let dedented = crate::frontmatter::dedent_html_lines(&rendered_content);
        crate::frontmatter::markdown_to_html(&dedented)
    } else {
        rendered_content
    };

    // Step 3: Wrap in layout if specified
    if let Some(layout) = layout_name {
        // Rebuild context with rendered content for layout wrapping
        let mut layout_ctx = build_render_context_page_only(&html_content, page_front_matter);
        layout_ctx.insert("paginator".into(), paginator.clone());

        // Use the layout engine's render_with_cached_site which handles layout chaining
        // But we need to pass paginator through the chain. We'll use a custom approach:
        // render the layout template directly with our augmented context.
        layout_engine.render_with_paginator(
            layout,
            &html_content,
            page_front_matter,
            cached_site,
            paginator,
        )
    } else {
        Ok(html_content)
    }
}

/// Find the index page from loaded pages that should be used for pagination.
///
/// Jekyll's `jekyll-paginate` only works on the main `index.html` or `index.md`
/// file at the site root. Returns the index page if found.
pub fn find_index_page(pages: &[Page]) -> Option<&Page> {
    pages.iter().find(|p| {
        let path = &p.source_path;
        path == "index.html" || path == "index.md" || path == "index.htm"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::CollectionItem;
    use crate::frontmatter::FrontMatter;
    use liquid::model::ValueView;

    fn make_post(title: &str, date: &str, slug: &str) -> CollectionItem {
        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String(title.to_string()),
        );
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
        }
    }

    // ========================================================================
    // PaginationConfig
    // ========================================================================

    #[test]
    fn test_pagination_config_from_config_with_paginate() {
        let yaml = "paginate: 5\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let pagination = PaginationConfig::from_config(&config);
        assert!(pagination.is_some());
        let pagination = pagination.unwrap();
        assert_eq!(pagination.per_page, 5);
        assert_eq!(pagination.paginate_path, "/blog/page:num/");
    }

    #[test]
    fn test_pagination_config_from_config_with_custom_path() {
        let yaml = "paginate: 10\npaginate_path: /page:num/\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let pagination = PaginationConfig::from_config(&config).unwrap();
        assert_eq!(pagination.per_page, 10);
        assert_eq!(pagination.paginate_path, "/page:num/");
    }

    #[test]
    fn test_pagination_config_none_without_paginate() {
        let yaml = "url: https://example.com\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(PaginationConfig::from_config(&config).is_none());
    }

    #[test]
    fn test_pagination_config_none_with_zero() {
        let yaml = "paginate: 0\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(PaginationConfig::from_config(&config).is_none());
    }

    // ========================================================================
    // build_paginator_object
    // ========================================================================

    #[test]
    fn test_paginator_object_first_page() {
        let posts = vec![
            make_post("Post 1", "2024-01-03", "post-1"),
            make_post("Post 2", "2024-01-02", "post-2"),
        ];
        let post_refs: Vec<&CollectionItem> = posts.iter().collect();

        let paginator = build_paginator_object(&post_refs, 1, 2, 5, 3, "/blog/page:num/");

        let obj = paginator.as_object().unwrap();
        assert_eq!(
            obj.get("page").unwrap().to_value(),
            LiquidValue::scalar(1i64)
        );
        assert_eq!(
            obj.get("per_page").unwrap().to_value(),
            LiquidValue::scalar(2i64)
        );
        assert_eq!(
            obj.get("total_posts").unwrap().to_value(),
            LiquidValue::scalar(5i64)
        );
        assert_eq!(
            obj.get("total_pages").unwrap().to_value(),
            LiquidValue::scalar(3i64)
        );
        // First page: no previous
        assert!(obj.get("previous_page").unwrap().is_nil());
        assert!(obj.get("previous_page_path").unwrap().is_nil());
        // First page: has next
        assert_eq!(
            obj.get("next_page").unwrap().to_value(),
            LiquidValue::scalar(2i64)
        );
        assert_eq!(
            obj.get("next_page_path").unwrap().to_value(),
            LiquidValue::scalar("/blog/page2/")
        );
    }

    #[test]
    fn test_paginator_object_middle_page() {
        let posts = vec![make_post("Post 3", "2024-01-01", "post-3")];
        let post_refs: Vec<&CollectionItem> = posts.iter().collect();

        let paginator = build_paginator_object(&post_refs, 2, 2, 5, 3, "/blog/page:num/");

        let obj = paginator.as_object().unwrap();
        assert_eq!(
            obj.get("page").unwrap().to_value(),
            LiquidValue::scalar(2i64)
        );
        // Page 2's previous is page 1 (root /)
        assert_eq!(
            obj.get("previous_page").unwrap().to_value(),
            LiquidValue::scalar(1i64)
        );
        assert_eq!(
            obj.get("previous_page_path").unwrap().to_value(),
            LiquidValue::scalar("/")
        );
        // Page 2 has next page 3
        assert_eq!(
            obj.get("next_page").unwrap().to_value(),
            LiquidValue::scalar(3i64)
        );
        assert_eq!(
            obj.get("next_page_path").unwrap().to_value(),
            LiquidValue::scalar("/blog/page3/")
        );
    }

    #[test]
    fn test_paginator_object_last_page() {
        let posts = vec![make_post("Post 5", "2023-12-30", "post-5")];
        let post_refs: Vec<&CollectionItem> = posts.iter().collect();

        let paginator = build_paginator_object(&post_refs, 3, 2, 5, 3, "/blog/page:num/");

        let obj = paginator.as_object().unwrap();
        assert_eq!(
            obj.get("page").unwrap().to_value(),
            LiquidValue::scalar(3i64)
        );
        // Last page: has previous
        assert_eq!(
            obj.get("previous_page").unwrap().to_value(),
            LiquidValue::scalar(2i64)
        );
        assert_eq!(
            obj.get("previous_page_path").unwrap().to_value(),
            LiquidValue::scalar("/blog/page2/")
        );
        // Last page: no next
        assert!(obj.get("next_page").unwrap().is_nil());
        assert!(obj.get("next_page_path").unwrap().is_nil());
    }

    #[test]
    fn test_paginator_posts_array_has_correct_count() {
        let posts = vec![
            make_post("Post 1", "2024-01-03", "post-1"),
            make_post("Post 2", "2024-01-02", "post-2"),
        ];
        let post_refs: Vec<&CollectionItem> = posts.iter().collect();

        let paginator = build_paginator_object(&post_refs, 1, 2, 5, 3, "/blog/page:num/");

        let obj = paginator.as_object().unwrap();
        let posts_val = obj.get("posts").unwrap();
        let arr = posts_val.as_array().unwrap();
        assert_eq!(arr.size(), 2);
    }

    #[test]
    fn test_paginator_posts_have_title_and_url() {
        let posts = vec![make_post("My Title", "2024-01-01", "my-title")];
        let post_refs: Vec<&CollectionItem> = posts.iter().collect();

        let paginator = build_paginator_object(&post_refs, 1, 5, 1, 1, "/page:num/");

        let obj = paginator.as_object().unwrap();
        let posts_val = obj.get("posts").unwrap();
        let arr = posts_val.as_array().unwrap();
        let first = arr.values().next().unwrap();
        let first_obj = first.as_object().unwrap();
        assert_eq!(
            first_obj.get("title").unwrap().to_kstr().as_str(),
            "My Title"
        );
        assert_eq!(
            first_obj.get("url").unwrap().to_kstr().as_str(),
            "/blog/my-title.html"
        );
    }

    // ========================================================================
    // find_index_page
    // ========================================================================

    #[test]
    fn test_find_index_page_html() {
        let pages = vec![
            Page {
                slug: "about".to_string(),
                front_matter: FrontMatter::new(),
                content: String::new(),
                html_content: String::new(),
                url: "/about/".to_string(),
                source_path: "about.md".to_string(),
            },
            Page {
                slug: "index".to_string(),
                front_matter: FrontMatter::new(),
                content: String::new(),
                html_content: String::new(),
                url: "/".to_string(),
                source_path: "index.html".to_string(),
            },
        ];
        let idx = find_index_page(&pages);
        assert!(idx.is_some());
        assert_eq!(idx.unwrap().source_path, "index.html");
    }

    #[test]
    fn test_find_index_page_md() {
        let pages = vec![Page {
            slug: "index".to_string(),
            front_matter: FrontMatter::new(),
            content: String::new(),
            html_content: String::new(),
            url: "/".to_string(),
            source_path: "index.md".to_string(),
        }];
        let idx = find_index_page(&pages);
        assert!(idx.is_some());
        assert_eq!(idx.unwrap().source_path, "index.md");
    }

    #[test]
    fn test_find_index_page_none() {
        let pages = vec![Page {
            slug: "about".to_string(),
            front_matter: FrontMatter::new(),
            content: String::new(),
            html_content: String::new(),
            url: "/about/".to_string(),
            source_path: "about.md".to_string(),
        }];
        assert!(find_index_page(&pages).is_none());
    }

    // ========================================================================
    // generate_excerpt
    // ========================================================================

    #[test]
    fn test_excerpt_from_paragraph() {
        let html = "<p>Hello world</p><p>Second paragraph</p>";
        assert_eq!(generate_excerpt(html), "<p>Hello world</p>");
    }

    #[test]
    fn test_excerpt_no_paragraph() {
        let html = "Just some text without paragraphs";
        assert_eq!(generate_excerpt(html), "Just some text without paragraphs");
    }

    // ========================================================================
    // PaginationConfig edge cases
    // ========================================================================

    #[test]
    fn test_pagination_config_string_paginate_ignored() {
        let yaml = "paginate: \"five\"\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(PaginationConfig::from_config(&config).is_none());
    }

    #[test]
    fn test_pagination_total_pages_calculation() {
        // 7 posts with per_page=3 should give 3 pages (3, 3, 1)
        assert_eq!(7usize.div_ceil(3), 3);
        // 6 posts with per_page=3 should give 2 pages (3, 3)
        assert_eq!(6usize.div_ceil(3), 2);
        // 1 post with per_page=5 should give 1 page
        assert_eq!(1usize.div_ceil(5), 1);
    }
}
