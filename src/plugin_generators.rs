//! Generic plugin generator detection and emulation.
//!
//! This module detects common Jekyll generator plugin patterns from `_plugins/`
//! and emulates them in Rust. Currently supports:
//!
//! - **Author page generators**: Generates `/author/<name>/index.html` and
//!   `/author/<name>/feed.xml` for each author in `_data/authors.yml`
//! - **Tag page generators**: Generates `/tag/<slug>/index.html` and
//!   `/tag/<slug>/feed.xml` for each tag used in posts
//!
//! Both support pagination matching `jekyll-paginate` behavior.
//!
//! Detection is based on scanning Ruby plugin files for known patterns, NOT on
//! hardcoded site names.

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

/// Which plugin generators were detected for a site.
#[derive(Debug, Clone, Default)]
pub struct DetectedGenerators {
    pub author_generator: bool,
    pub tag_generator: bool,
}

/// Configuration for the detected author generator.
#[derive(Debug, Clone)]
pub struct AuthorGeneratorConfig {
    pub index_layout: String,
    pub feed_layout: String,
    pub path_prefix: String,
}

/// Configuration for the detected tag generator.
#[derive(Debug, Clone)]
pub struct TagGeneratorConfig {
    pub index_layout: String,
    pub feed_layout: String,
    pub path_prefix: String,
}

/// Scan the `_plugins/` directory for known generator patterns.
///
/// Looks for Ruby files matching author and tag generator patterns:
/// - Files containing "Author" class names and "/author/" paths
/// - Files containing "Tag" class names and "/tag/" paths
///
/// Returns which generators were detected. Does NOT require layouts to exist
/// (that check is done at generation time).
pub fn detect_generators(source_dir: &Path) -> DetectedGenerators {
    let plugins_dir = source_dir.join("_plugins");
    if !plugins_dir.is_dir() {
        return DetectedGenerators::default();
    }

    let mut detected = DetectedGenerators::default();

    let entries = match fs::read_dir(&plugins_dir) {
        Ok(entries) => entries,
        Err(_) => return detected,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rb") {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if is_author_generator(&content, filename) {
            detected.author_generator = true;
        }
        if is_tag_generator(&content, filename) {
            detected.tag_generator = true;
        }
    }

    detected
}

fn is_author_generator(content: &str, filename: &str) -> bool {
    let filename_lower = filename.to_lowercase();
    let has_author_class = content.contains("AuthorsGenerator")
        || content.contains("AuthorGenerator")
        || content.contains("AutGenerator");
    let has_author_path = content.contains("/author/");
    let filename_hint =
        filename_lower.contains("author") || filename_lower.contains("autgenerator");

    (has_author_class || filename_hint) && has_author_path
}

fn is_tag_generator(content: &str, filename: &str) -> bool {
    let filename_lower = filename.to_lowercase();
    let has_tag_class = content.contains("TagsGenerator") || content.contains("TagGenerator");
    let has_tag_path = content.contains("/tag/");
    let filename_hint = filename_lower.contains("tag") || filename_lower.contains("tagsgenerator");

    (has_tag_class || filename_hint) && has_tag_path
}

/// Extract author names from `_data/authors.yml`.
///
/// Returns the top-level keys of the authors mapping (e.g., "ghost", "hannah").
pub fn extract_author_names(data_tree: &crate::data::DataTree) -> Vec<String> {
    let authors_val = match data_tree.get("authors") {
        Some(v) => v,
        None => return Vec::new(),
    };

    let mapping = match authors_val.as_mapping() {
        Some(m) => m,
        None => return Vec::new(),
    };

    let mut names: Vec<String> = mapping
        .keys()
        .filter_map(|k| k.as_str().map(|s| s.to_string()))
        .collect();
    names.sort();
    names
}

/// Collect posts for a specific author.
///
/// Filters posts where `post.front_matter["author"]` matches the given name.
/// Sorts by date descending (newest first).
pub fn posts_by_author<'a>(posts: &'a [CollectionItem], author: &str) -> Vec<&'a CollectionItem> {
    let mut filtered: Vec<&'a CollectionItem> = posts
        .iter()
        .filter(|p| {
            p.front_matter
                .get("author")
                .and_then(|v| v.as_str())
                .map(|s| s == author)
                .unwrap_or(false)
        })
        .collect();

    filtered.sort_by(|a, b| {
        let date_a = crate::collection::date_sort_key(a.date.as_deref().unwrap_or(""));
        let date_b = crate::collection::date_sort_key(b.date.as_deref().unwrap_or(""));
        date_b.cmp(&date_a).then_with(|| b.slug.cmp(&a.slug))
    });

    filtered
}

/// Collect posts for a specific tag.
///
/// Filters posts where the tag appears in `post.front_matter["tags"]`.
/// Sorts by date descending (newest first).
pub fn posts_by_tag<'a>(posts: &'a [CollectionItem], tag: &str) -> Vec<&'a CollectionItem> {
    let mut filtered: Vec<&'a CollectionItem> = posts
        .iter()
        .filter(|p| {
            let post_tags = crate::collection::extract_tags(&p.front_matter);
            post_tags.iter().any(|t| t == tag)
        })
        .collect();

    filtered.sort_by(|a, b| {
        let date_a = crate::collection::date_sort_key(a.date.as_deref().unwrap_or(""));
        let date_b = crate::collection::date_sort_key(b.date.as_deref().unwrap_or(""));
        date_b.cmp(&date_a).then_with(|| b.slug.cmp(&a.slug))
    });

    filtered
}

/// Collect all unique tags from posts (in first-encounter order).
pub fn collect_all_tags(posts: &[CollectionItem]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut tags = Vec::new();
    for post in posts {
        for tag in crate::collection::extract_tags(&post.front_matter) {
            if seen.insert(tag.clone()) {
                tags.push(tag);
            }
        }
    }
    tags
}

/// Slugify a tag name for use in URLs (lowercase, spaces to hyphens).
pub fn slugify_tag(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

/// Build a paginator LiquidValue for author/tag pages.
///
/// Similar to the main pagination's `build_paginator_object` but with
/// custom path handling for author/tag pages.
fn build_group_paginator(
    page_posts: &[&CollectionItem],
    page_num: usize,
    per_page: usize,
    total_posts: usize,
    total_pages: usize,
    base_path: &str,
) -> LiquidValue {
    let mut paginator = Object::new();

    let posts_arr: Vec<LiquidValue> = page_posts
        .iter()
        .map(|item| collection_item_to_liquid_for_generator(item))
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

    if page_num > 1 {
        paginator.insert(
            "previous_page".into(),
            LiquidValue::scalar((page_num - 1) as i64),
        );
        let prev_path = if page_num == 2 {
            format!("{}/", base_path)
        } else {
            format!("{}/page{}/", base_path, page_num - 1)
        };
        paginator.insert("previous_page_path".into(), LiquidValue::scalar(prev_path));
    } else {
        paginator.insert("previous_page".into(), LiquidValue::Nil);
        paginator.insert("previous_page_path".into(), LiquidValue::Nil);
    }

    if page_num < total_pages {
        paginator.insert(
            "next_page".into(),
            LiquidValue::scalar((page_num + 1) as i64),
        );
        let next_path = format!("{}/page{}/", base_path, page_num + 1);
        paginator.insert("next_page_path".into(), LiquidValue::scalar(next_path));
    } else {
        paginator.insert("next_page".into(), LiquidValue::Nil);
        paginator.insert("next_page_path".into(), LiquidValue::Nil);
    }

    LiquidValue::Object(paginator)
}

/// Convert a CollectionItem to a Liquid Value for generator pages.
fn collection_item_to_liquid_for_generator(item: &CollectionItem) -> LiquidValue {
    let mut obj = Object::new();

    for (key, value) in &item.front_matter {
        obj.insert(key.clone().into(), normalize_arrays(yaml_to_liquid(value)));
    }

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

    // Issue #568: Only override excerpt with auto-generated if front matter
    // does not define an explicit `excerpt:` value.
    if !item.front_matter.contains_key("excerpt") {
        if let Some(ref html_excerpt) = item.excerpt_html {
            obj.insert("excerpt".into(), LiquidValue::scalar(html_excerpt.clone()));
        } else if let Some(ref raw_excerpt) = item.excerpt {
            let rendered = crate::frontmatter::markdown_to_html(raw_excerpt);
            obj.insert("excerpt".into(), LiquidValue::scalar(rendered));
        }
    }

    if !item.front_matter.contains_key("short") {
        obj.insert("short".into(), LiquidValue::scalar(item.slug.clone()));
    }

    crate::generator::normalize_categories_and_tags(&mut obj);

    LiquidValue::Object(obj)
}

/// Generate all author and tag plugin-generated pages.
///
/// Returns the total number of pages generated.
#[allow(clippy::too_many_arguments)]
pub fn generate_plugin_pages(
    detected: &DetectedGenerators,
    posts: &[CollectionItem],
    data_tree: &crate::data::DataTree,
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    config: &SiteConfig,
    output_dir: &Path,
) -> Result<usize, GeneratorError> {
    let mut generated = 0;

    let per_page = config
        .extras
        .get("paginate")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(100);

    if detected.author_generator {
        let author_config = AuthorGeneratorConfig {
            index_layout: "author".to_string(),
            feed_layout: "feed".to_string(),
            path_prefix: "author".to_string(),
        };
        generated += generate_author_pages(
            posts,
            data_tree,
            &author_config,
            per_page,
            layout_engine,
            cached_site,
            output_dir,
        )?;
    }

    if detected.tag_generator {
        let tag_config = TagGeneratorConfig {
            index_layout: "tag".to_string(),
            feed_layout: "feed".to_string(),
            path_prefix: "tag".to_string(),
        };
        generated += generate_tag_pages(
            posts,
            &tag_config,
            per_page,
            layout_engine,
            cached_site,
            output_dir,
        )?;
    }

    Ok(generated)
}

/// Generate author index and feed pages for all authors.
#[allow(clippy::too_many_arguments)]
fn generate_author_pages(
    posts: &[CollectionItem],
    data_tree: &crate::data::DataTree,
    config: &AuthorGeneratorConfig,
    per_page: usize,
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    output_dir: &Path,
) -> Result<usize, GeneratorError> {
    let authors = extract_author_names(data_tree);
    let mut generated = 0;

    for author_name in &authors {
        let author_posts = posts_by_author(posts, author_name);
        if author_posts.is_empty() {
            continue;
        }

        let base_path = format!("{}/{}", config.path_prefix, author_name);

        generated += generate_group_feed(
            &author_posts,
            &base_path,
            "author",
            author_name,
            &config.feed_layout,
            layout_engine,
            cached_site,
            output_dir,
        )?;

        generated += generate_group_index_pages(
            &author_posts,
            &base_path,
            "author",
            author_name,
            &config.index_layout,
            per_page,
            layout_engine,
            cached_site,
            output_dir,
        )?;
    }

    Ok(generated)
}

/// Generate tag index and feed pages for all tags.
#[allow(clippy::too_many_arguments)]
fn generate_tag_pages(
    posts: &[CollectionItem],
    config: &TagGeneratorConfig,
    per_page: usize,
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    output_dir: &Path,
) -> Result<usize, GeneratorError> {
    let tags = collect_all_tags(posts);
    let mut generated = 0;

    for tag_name in &tags {
        let tag_posts = posts_by_tag(posts, tag_name);
        if tag_posts.is_empty() {
            continue;
        }

        let slug = slugify_tag(tag_name);
        let base_path = format!("{}/{}", config.path_prefix, slug);

        generated += generate_group_feed(
            &tag_posts,
            &base_path,
            "tag",
            tag_name,
            &config.feed_layout,
            layout_engine,
            cached_site,
            output_dir,
        )?;

        generated += generate_group_index_pages(
            &tag_posts,
            &base_path,
            "tag",
            tag_name,
            &config.index_layout,
            per_page,
            layout_engine,
            cached_site,
            output_dir,
        )?;
    }

    Ok(generated)
}

/// Generate the feed.xml page for an author or tag.
#[allow(clippy::too_many_arguments)]
fn generate_group_feed(
    group_posts: &[&CollectionItem],
    base_path: &str,
    group_type: &str,
    group_value: &str,
    feed_layout: &str,
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    output_dir: &Path,
) -> Result<usize, GeneratorError> {
    let mut page_fm = crate::frontmatter::FrontMatter::new();
    page_fm.insert(
        "layout".to_string(),
        serde_yaml::Value::String(feed_layout.to_string()),
    );
    page_fm.insert(
        "grouptype".to_string(),
        serde_yaml::Value::String(group_type.to_string()),
    );
    page_fm.insert(
        group_type.to_string(),
        serde_yaml::Value::String(group_value.to_string()),
    );
    let page_url = format!("/{}/feed.xml", base_path);
    page_fm.insert(
        "url".to_string(),
        serde_yaml::Value::String(page_url.clone()),
    );

    let feed_posts: Vec<LiquidValue> = group_posts
        .iter()
        .take(10)
        .map(|item| collection_item_to_liquid_for_generator(item))
        .collect();

    let extra_fields = vec![(
        "posts".to_string(),
        normalize_arrays(LiquidValue::Array(feed_posts)),
    )];

    let html = match layout_engine.render_with_extra_page_fields(
        feed_layout,
        "",
        &page_fm,
        &extra_fields,
        cached_site,
    ) {
        Ok(rendered) => rendered,
        Err(e) => {
            eprintln!(
                "Warning: failed to render {} feed for '{}': {}",
                group_type, group_value, e
            );
            return Ok(0);
        }
    };

    let out_path = url_to_output_path(output_dir, &format!("/{}/feed.xml", base_path));
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&out_path, &html).is_ok() {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Generate paginated index pages for an author or tag.
#[allow(clippy::too_many_arguments)]
fn generate_group_index_pages(
    group_posts: &[&CollectionItem],
    base_path: &str,
    group_type: &str,
    group_value: &str,
    index_layout: &str,
    per_page: usize,
    layout_engine: &LayoutEngine,
    cached_site: &CachedSiteContext,
    output_dir: &Path,
) -> Result<usize, GeneratorError> {
    let total_posts = group_posts.len();
    let total_pages_count = if per_page > 0 {
        total_posts.div_ceil(per_page)
    } else {
        1
    };

    let mut generated = 0;

    for page_num in 1..=total_pages_count {
        let start = (page_num - 1) * per_page;
        let end = std::cmp::min(start + per_page, total_posts);
        let page_posts: Vec<&CollectionItem> = group_posts[start..end].to_vec();

        let page_url = if page_num == 1 {
            format!("/{}/", base_path)
        } else {
            format!("/{}/page{}/", base_path, page_num)
        };

        let paginator = build_group_paginator(
            &page_posts,
            page_num,
            per_page,
            total_posts,
            total_pages_count,
            base_path,
        );

        let mut page_fm = crate::frontmatter::FrontMatter::new();
        page_fm.insert(
            "layout".to_string(),
            serde_yaml::Value::String(index_layout.to_string()),
        );
        page_fm.insert(
            "grouptype".to_string(),
            serde_yaml::Value::String(group_type.to_string()),
        );
        page_fm.insert(
            group_type.to_string(),
            serde_yaml::Value::String(group_value.to_string()),
        );
        page_fm.insert(
            "url".to_string(),
            serde_yaml::Value::String(page_url.clone()),
        );

        let html = match layout_engine.render_with_paginator(
            index_layout,
            "",
            &page_fm,
            cached_site,
            &paginator,
        ) {
            Ok(rendered) => rendered,
            Err(e) => {
                eprintln!(
                    "Warning: failed to render {} page {} for '{}': {}",
                    group_type, page_num, group_value, e
                );
                continue;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::CollectionItem;
    use crate::config::SiteConfig;
    use crate::frontmatter::FrontMatter;
    use liquid::model::ValueView;
    use std::path::PathBuf;

    fn make_post(
        title: &str,
        date: &str,
        slug: &str,
        author: &str,
        tags: Vec<&str>,
    ) -> CollectionItem {
        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String(title.to_string()),
        );
        fm.insert(
            "author".to_string(),
            serde_yaml::Value::String(author.to_string()),
        );
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

    fn make_authors_data() -> crate::data::DataTree {
        let yaml = "ghost:\n  name: Ghost\nhannah:\n  name: Hannah\n";
        let mut tree = crate::data::DataTree::new();
        let value = crate::yaml::parse_yaml_lenient(yaml).unwrap();
        tree.insert("authors".to_string(), value);
        tree
    }

    // ========================================================================
    // Detection
    // ========================================================================

    #[test]
    fn test_detect_author_generator_from_jasper2_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let plugins_dir = dir.path().join("_plugins");
        fs::create_dir_all(&plugins_dir).unwrap();
        fs::write(
            plugins_dir.join("jekyll-autgenerator.rb"),
            "module Jekyll\n  class AuthorsGenerator < Generator\n    def generate(site)\n      path = \"/author/#{posts[0]}\"\n    end\n  end\nend\n",
        ).unwrap();

        let detected = detect_generators(dir.path());
        assert!(
            detected.author_generator,
            "Should detect author generator from Jasper2-style plugin"
        );
        assert!(
            !detected.tag_generator,
            "Should NOT detect tag generator when only author plugin exists"
        );
    }

    #[test]
    fn test_detect_tag_generator_from_jasper2_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let plugins_dir = dir.path().join("_plugins");
        fs::create_dir_all(&plugins_dir).unwrap();
        fs::write(
            plugins_dir.join("jekyll-tagsgenerator.rb"),
            "module Jekyll\n  class TagsGenerator < Generator\n    def generate(site)\n      path = \"/tag/\" + posts[0].slugify\n    end\n  end\nend\n",
        ).unwrap();

        let detected = detect_generators(dir.path());
        assert!(
            detected.tag_generator,
            "Should detect tag generator from Jasper2-style plugin"
        );
        assert!(
            !detected.author_generator,
            "Should NOT detect author generator when only tag plugin exists"
        );
    }

    #[test]
    fn test_detect_both_generators() {
        let dir = tempfile::tempdir().unwrap();
        let plugins_dir = dir.path().join("_plugins");
        fs::create_dir_all(&plugins_dir).unwrap();
        fs::write(
            plugins_dir.join("jekyll-autgenerator.rb"),
            "class AuthorsGenerator < Generator\n  path = \"/author/\"\nend\n",
        )
        .unwrap();
        fs::write(
            plugins_dir.join("jekyll-tagsgenerator.rb"),
            "class TagsGenerator < Generator\n  path = \"/tag/\"\nend\n",
        )
        .unwrap();

        let detected = detect_generators(dir.path());
        assert!(detected.author_generator);
        assert!(detected.tag_generator);
    }

    #[test]
    fn test_detect_no_plugins_directory() {
        let dir = tempfile::tempdir().unwrap();
        let detected = detect_generators(dir.path());
        assert!(!detected.author_generator);
        assert!(!detected.tag_generator);
    }

    #[test]
    fn test_detect_no_matching_plugins() {
        let dir = tempfile::tempdir().unwrap();
        let plugins_dir = dir.path().join("_plugins");
        fs::create_dir_all(&plugins_dir).unwrap();
        fs::write(
            plugins_dir.join("other-plugin.rb"),
            "class OtherPlugin < Generator\nend\n",
        )
        .unwrap();

        let detected = detect_generators(dir.path());
        assert!(!detected.author_generator);
        assert!(!detected.tag_generator);
    }

    // ========================================================================
    // Author data extraction
    // ========================================================================

    #[test]
    fn test_extract_author_names_from_data() {
        let data = make_authors_data();
        let names = extract_author_names(&data);
        assert_eq!(names, vec!["ghost", "hannah"]);
    }

    #[test]
    fn test_extract_author_names_no_authors_key() {
        let data = crate::data::DataTree::new();
        let names = extract_author_names(&data);
        assert!(names.is_empty());
    }

    // ========================================================================
    // Post filtering
    // ========================================================================

    #[test]
    fn test_posts_by_author_filters_correctly() {
        let posts = vec![
            make_post("Post 1", "2024-01-03", "p1", "ghost", vec![]),
            make_post("Post 2", "2024-01-02", "p2", "hannah", vec![]),
            make_post("Post 3", "2024-01-01", "p3", "ghost", vec![]),
        ];
        let ghost_posts = posts_by_author(&posts, "ghost");
        assert_eq!(ghost_posts.len(), 2);
        assert_eq!(ghost_posts[0].slug, "p1"); // newest first
        assert_eq!(ghost_posts[1].slug, "p3");
    }

    #[test]
    fn test_posts_by_tag_filters_correctly() {
        let posts = vec![
            make_post("Post 1", "2024-01-03", "p1", "ghost", vec!["rust", "ai"]),
            make_post("Post 2", "2024-01-02", "p2", "hannah", vec!["python"]),
            make_post("Post 3", "2024-01-01", "p3", "ghost", vec!["rust", "web"]),
        ];
        let rust_posts = posts_by_tag(&posts, "rust");
        assert_eq!(rust_posts.len(), 2);
        assert_eq!(rust_posts[0].slug, "p1"); // newest first
        assert_eq!(rust_posts[1].slug, "p3");
    }

    #[test]
    fn test_collect_all_tags() {
        let posts = vec![
            make_post("Post 1", "2024-01-01", "p1", "ghost", vec!["rust", "ai"]),
            make_post(
                "Post 2",
                "2024-01-02",
                "p2",
                "hannah",
                vec!["python", "rust"],
            ),
            make_post("Post 3", "2024-01-03", "p3", "ghost", vec!["web", "ai"]),
        ];
        let tags = collect_all_tags(&posts);
        assert_eq!(tags, vec!["rust", "ai", "python", "web"]);
    }

    #[test]
    fn test_slugify_tag() {
        assert_eq!(slugify_tag("Machine Learning"), "machine-learning");
        assert_eq!(slugify_tag("rust"), "rust");
        assert_eq!(slugify_tag("Web Dev"), "web-dev");
    }

    // ========================================================================
    // Integration: author page generation
    // ========================================================================

    #[test]
    fn test_generate_author_pages_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        fs::write(
            layouts_dir.join("author.html"),
            "---\nlayout: null\n---\n<h1>{{ page.author }}</h1>\n{% for post in paginator.posts %}<p>{{ post.title }}</p>{% endfor %}",
        ).unwrap();
        fs::write(
            layouts_dir.join("feed.xml"),
            "---\nlayout: null\n---\n<?xml version=\"1.0\"?>\n<feed>{% for post in page.posts %}<entry>{{ post.title }}</entry>{% endfor %}</feed>",
        ).unwrap();

        let posts = vec![
            make_post("Post 1", "2024-01-03", "p1", "ghost", vec![]),
            make_post("Post 2", "2024-01-02", "p2", "hannah", vec![]),
            make_post("Post 3", "2024-01-01", "p3", "ghost", vec![]),
        ];
        let data = make_authors_data();

        let author_config = AuthorGeneratorConfig {
            index_layout: "author".to_string(),
            feed_layout: "feed".to_string(),
            path_prefix: "author".to_string(),
        };

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_author_pages(
            &posts,
            &data,
            &author_config,
            100,
            &layout_engine,
            &cached_site,
            output_dir,
        )
        .unwrap();

        assert!(
            count >= 4,
            "Should generate at least 4 pages (2 authors x 2 files), got {}",
            count
        );

        assert!(
            output_dir.join("author/ghost/index.html").exists(),
            "Should create /author/ghost/index.html"
        );
        assert!(
            output_dir.join("author/ghost/feed.xml").exists(),
            "Should create /author/ghost/feed.xml"
        );
        assert!(
            output_dir.join("author/hannah/index.html").exists(),
            "Should create /author/hannah/index.html"
        );
        assert!(
            output_dir.join("author/hannah/feed.xml").exists(),
            "Should create /author/hannah/feed.xml"
        );
    }

    #[test]
    fn test_author_page_has_correct_context() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        fs::write(
            layouts_dir.join("author.html"),
            "---\nlayout: null\n---\nGROUPTYPE={{ page.grouptype }}|AUTHOR={{ page.author }}|POSTS={{ paginator.total_posts }}",
        ).unwrap();
        fs::write(layouts_dir.join("feed.xml"), "---\nlayout: null\n---\nfeed").unwrap();

        let posts = vec![
            make_post("Post 1", "2024-01-03", "p1", "ghost", vec![]),
            make_post("Post 2", "2024-01-01", "p2", "ghost", vec![]),
        ];
        let data = make_authors_data();

        let author_config = AuthorGeneratorConfig {
            index_layout: "author".to_string(),
            feed_layout: "feed".to_string(),
            path_prefix: "author".to_string(),
        };

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        generate_author_pages(
            &posts,
            &data,
            &author_config,
            100,
            &layout_engine,
            &cached_site,
            output_dir,
        )
        .unwrap();

        let content = fs::read_to_string(output_dir.join("author/ghost/index.html")).unwrap();
        assert!(
            content.contains("GROUPTYPE=author"),
            "Should have grouptype=author, got: {}",
            content
        );
        assert!(
            content.contains("AUTHOR=ghost"),
            "Should have author=ghost, got: {}",
            content
        );
        assert!(
            content.contains("POSTS=2"),
            "Should have 2 total posts, got: {}",
            content
        );
    }

    // ========================================================================
    // Integration: tag page generation
    // ========================================================================

    #[test]
    fn test_generate_tag_pages_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        fs::write(
            layouts_dir.join("tag.html"),
            "---\nlayout: null\n---\n<h1>{{ page.tag }}</h1>\n{% for post in paginator.posts %}<p>{{ post.title }}</p>{% endfor %}",
        ).unwrap();
        fs::write(
            layouts_dir.join("feed.xml"),
            "---\nlayout: null\n---\n<?xml version=\"1.0\"?>\n<feed>{% for post in page.posts %}<entry>{{ post.title }}</entry>{% endfor %}</feed>",
        ).unwrap();

        let posts = vec![
            make_post("Post 1", "2024-01-03", "p1", "ghost", vec!["rust", "ai"]),
            make_post("Post 2", "2024-01-02", "p2", "hannah", vec!["python"]),
            make_post("Post 3", "2024-01-01", "p3", "ghost", vec!["rust", "web"]),
        ];

        let tag_config = TagGeneratorConfig {
            index_layout: "tag".to_string(),
            feed_layout: "feed".to_string(),
            path_prefix: "tag".to_string(),
        };

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_tag_pages(
            &posts,
            &tag_config,
            100,
            &layout_engine,
            &cached_site,
            output_dir,
        )
        .unwrap();

        assert!(
            count >= 8,
            "Should generate 8 pages (4 tags x 2 files), got {}",
            count
        );
        assert!(output_dir.join("tag/rust/index.html").exists());
        assert!(output_dir.join("tag/rust/feed.xml").exists());
        assert!(output_dir.join("tag/ai/index.html").exists());
        assert!(output_dir.join("tag/python/index.html").exists());
        assert!(output_dir.join("tag/web/index.html").exists());
    }

    #[test]
    fn test_tag_page_has_correct_context() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        fs::write(
            layouts_dir.join("tag.html"),
            "---\nlayout: null\n---\nGROUPTYPE={{ page.grouptype }}|TAG={{ page.tag }}|POSTS={{ paginator.total_posts }}",
        ).unwrap();
        fs::write(layouts_dir.join("feed.xml"), "---\nlayout: null\n---\nfeed").unwrap();

        let posts = vec![
            make_post("Post 1", "2024-01-03", "p1", "ghost", vec!["rust"]),
            make_post("Post 2", "2024-01-01", "p2", "hannah", vec!["rust"]),
        ];

        let tag_config = TagGeneratorConfig {
            index_layout: "tag".to_string(),
            feed_layout: "feed".to_string(),
            path_prefix: "tag".to_string(),
        };

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        generate_tag_pages(
            &posts,
            &tag_config,
            100,
            &layout_engine,
            &cached_site,
            output_dir,
        )
        .unwrap();

        let content = fs::read_to_string(output_dir.join("tag/rust/index.html")).unwrap();
        assert!(
            content.contains("GROUPTYPE=tag"),
            "Should have grouptype=tag, got: {}",
            content
        );
        assert!(
            content.contains("TAG=rust"),
            "Should have tag=rust, got: {}",
            content
        );
        assert!(
            content.contains("POSTS=2"),
            "Should have 2 total posts, got: {}",
            content
        );
    }

    // ========================================================================
    // Pagination
    // ========================================================================

    #[test]
    fn test_author_pagination_creates_multiple_pages() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        fs::write(
            layouts_dir.join("author.html"),
            "---\nlayout: null\n---\nPAGE={{ paginator.page }}|TOTAL={{ paginator.total_pages }}",
        )
        .unwrap();
        fs::write(layouts_dir.join("feed.xml"), "---\nlayout: null\n---\nfeed").unwrap();

        let mut posts = Vec::new();
        for i in 0..5 {
            posts.push(make_post(
                &format!("Post {}", i + 1),
                &format!("2024-01-0{}", i + 1),
                &format!("p{}", i + 1),
                "ghost",
                vec![],
            ));
        }

        let data = make_authors_data();
        let author_config = AuthorGeneratorConfig {
            index_layout: "author".to_string(),
            feed_layout: "feed".to_string(),
            path_prefix: "author".to_string(),
        };

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_author_pages(
            &posts,
            &data,
            &author_config,
            3,
            &layout_engine,
            &cached_site,
            output_dir,
        )
        .unwrap();

        assert!(
            count >= 3,
            "Should generate at least 3 pages (1 feed + 2 index), got {}",
            count
        );
        assert!(output_dir.join("author/ghost/index.html").exists());
        assert!(output_dir.join("author/ghost/page2/index.html").exists());

        let page1 = fs::read_to_string(output_dir.join("author/ghost/index.html")).unwrap();
        assert!(
            page1.contains("PAGE=1"),
            "Page 1 should have PAGE=1, got: {}",
            page1
        );
        assert!(
            page1.contains("TOTAL=2"),
            "Page 1 should have TOTAL=2, got: {}",
            page1
        );

        let page2 = fs::read_to_string(output_dir.join("author/ghost/page2/index.html")).unwrap();
        assert!(
            page2.contains("PAGE=2"),
            "Page 2 should have PAGE=2, got: {}",
            page2
        );
        assert!(
            page2.contains("TOTAL=2"),
            "Page 2 should have TOTAL=2, got: {}",
            page2
        );
    }

    #[test]
    fn test_paginator_has_navigation_fields() {
        let posts_refs: Vec<&CollectionItem> = Vec::new();
        let paginator = build_group_paginator(&posts_refs, 2, 3, 7, 3, "author/ghost");

        let obj = paginator.as_object().unwrap();
        assert_eq!(
            obj.get("page").unwrap().to_value(),
            LiquidValue::scalar(2i64)
        );
        assert_eq!(
            obj.get("per_page").unwrap().to_value(),
            LiquidValue::scalar(3i64)
        );
        assert_eq!(
            obj.get("total_posts").unwrap().to_value(),
            LiquidValue::scalar(7i64)
        );
        assert_eq!(
            obj.get("total_pages").unwrap().to_value(),
            LiquidValue::scalar(3i64)
        );
        assert_eq!(
            obj.get("previous_page").unwrap().to_value(),
            LiquidValue::scalar(1i64)
        );
        assert_eq!(
            obj.get("previous_page_path").unwrap().to_value(),
            LiquidValue::scalar("author/ghost/")
        );
        assert_eq!(
            obj.get("next_page").unwrap().to_value(),
            LiquidValue::scalar(3i64)
        );
        assert_eq!(
            obj.get("next_page_path").unwrap().to_value(),
            LiquidValue::scalar("author/ghost/page3/")
        );
    }

    #[test]
    fn test_feed_contains_only_first_10_posts() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        fs::write(
            layouts_dir.join("author.html"),
            "---\nlayout: null\n---\nindex",
        )
        .unwrap();
        fs::write(
            layouts_dir.join("feed.xml"),
            "---\nlayout: null\n---\n{% for post in page.posts %}[{{ post.title }}]{% endfor %}",
        )
        .unwrap();

        let mut posts = Vec::new();
        for i in 0..15 {
            posts.push(make_post(
                &format!("Post {:02}", i + 1),
                &format!("2024-01-{:02}", i + 1),
                &format!("p{:02}", i + 1),
                "ghost",
                vec![],
            ));
        }

        let data = make_authors_data();
        let author_config = AuthorGeneratorConfig {
            index_layout: "author".to_string(),
            feed_layout: "feed".to_string(),
            path_prefix: "author".to_string(),
        };

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        generate_author_pages(
            &posts,
            &data,
            &author_config,
            100,
            &layout_engine,
            &cached_site,
            output_dir,
        )
        .unwrap();

        let feed = fs::read_to_string(output_dir.join("author/ghost/feed.xml")).unwrap();
        assert!(
            feed.contains("[Post 15]"),
            "Feed should contain Post 15 (newest), got: {}",
            feed
        );
        assert!(
            feed.contains("[Post 06]"),
            "Feed should contain Post 06 (10th newest), got: {}",
            feed
        );
        assert!(
            !feed.contains("[Post 05]"),
            "Feed should NOT contain Post 05 (only first 10 newest), got: {}",
            feed
        );
    }

    // ========================================================================
    // Edge cases
    // ========================================================================

    #[test]
    fn test_author_with_no_posts_generates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();
        fs::write(layouts_dir.join("author.html"), "---\nlayout: null\n---\n").unwrap();
        fs::write(layouts_dir.join("feed.xml"), "---\nlayout: null\n---\n").unwrap();

        let posts = vec![make_post("Post 1", "2024-01-01", "p1", "ghost", vec![])];

        let data = make_authors_data();
        let author_config = AuthorGeneratorConfig {
            index_layout: "author".to_string(),
            feed_layout: "feed".to_string(),
            path_prefix: "author".to_string(),
        };

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        let count = generate_author_pages(
            &posts,
            &data,
            &author_config,
            100,
            &layout_engine,
            &cached_site,
            output_dir,
        )
        .unwrap();

        assert_eq!(count, 2, "Only ghost has posts (1 index + 1 feed = 2)");
        assert!(output_dir.join("author/ghost/index.html").exists());
        assert!(
            !output_dir.join("author/hannah").exists(),
            "Hannah has no posts, should not generate pages"
        );
    }

    #[test]
    fn test_posts_sorted_by_date_descending() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path();

        let layouts_dir = dir.path().join("_layouts");
        let includes_dir = dir.path().join("_includes");
        fs::create_dir_all(&layouts_dir).unwrap();
        fs::create_dir_all(&includes_dir).unwrap();

        fs::write(
            layouts_dir.join("tag.html"),
            "---\nlayout: null\n---\n{% for post in paginator.posts %}{{ post.title }}|{% endfor %}",
        ).unwrap();
        fs::write(layouts_dir.join("feed.xml"), "---\nlayout: null\n---\nfeed").unwrap();

        let posts = vec![
            make_post("Old Post", "2024-01-01", "old", "ghost", vec!["rust"]),
            make_post("New Post", "2024-01-03", "new", "ghost", vec!["rust"]),
            make_post("Mid Post", "2024-01-02", "mid", "ghost", vec!["rust"]),
        ];

        let tag_config = TagGeneratorConfig {
            index_layout: "tag".to_string(),
            feed_layout: "feed".to_string(),
            path_prefix: "tag".to_string(),
        };

        let layout_engine = LayoutEngine::new(&layouts_dir, &includes_dir).unwrap();
        let site_context = Object::new();
        let cached_site = CachedSiteContext::new(&site_context);

        generate_tag_pages(
            &posts,
            &tag_config,
            100,
            &layout_engine,
            &cached_site,
            output_dir,
        )
        .unwrap();

        let content = fs::read_to_string(output_dir.join("tag/rust/index.html")).unwrap();
        let new_pos = content.find("New Post").expect("Should find New Post");
        let mid_pos = content.find("Mid Post").expect("Should find Mid Post");
        let old_pos = content.find("Old Post").expect("Should find Old Post");
        assert!(
            new_pos < mid_pos && mid_pos < old_pos,
            "Posts should be newest first: new={}, mid={}, old={}",
            new_pos,
            mid_pos,
            old_pos
        );
    }
}
