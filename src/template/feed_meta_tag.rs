//! `{% feed_meta %}` tag implementing the jekyll-feed plugin's meta link output.
//!
//! Generates `<link>` tags for Atom/RSS feeds based on site configuration:
//! - Main feed link using `site.url` + feed path (default `feed.xml`)
//! - Category-specific feed links when `site.feed.categories` is configured
//! - Title from `site.name` or `site.title`

use std::io::Write;

use liquid_core::model::ValueView;
use liquid_core::{Language, ParseTag, Renderable, Runtime, TagReflection, TagTokenIter};

/// The `{% feed_meta %}` tag parser/reflection.
#[derive(Copy, Clone, Debug, Default)]
pub struct FeedMetaTag;

impl TagReflection for FeedMetaTag {
    fn tag(&self) -> &'static str {
        "feed_meta"
    }

    fn description(&self) -> &'static str {
        "Generate feed <link> tags (jekyll-feed plugin)"
    }
}

impl ParseTag for FeedMetaTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        // Consume any arguments (there shouldn't be any, but be lenient)
        while arguments.expect_next("").is_ok() {}
        Ok(Box::new(FeedMetaRenderable))
    }

    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

#[derive(Debug)]
struct FeedMetaRenderable;

impl std::fmt::Display for FeedMetaRenderable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "feed_meta")
    }
}

/// HTML-escape a string for safe use in attribute values.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Get a nested value from the Liquid runtime context.
fn get_nested_str(runtime: &dyn Runtime, parts: &[&str]) -> Option<String> {
    if parts.is_empty() {
        return None;
    }
    let path: Vec<liquid_core::model::ScalarCow<'_>> = parts
        .iter()
        .map(|p| liquid_core::model::ScalarCow::new(*p))
        .collect();
    let val = runtime.try_get(&path)?;
    let s = val.to_kstr().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Get a nested value as a Liquid Value (for accessing arrays).
fn get_nested_value(runtime: &dyn Runtime, parts: &[&str]) -> Option<Box<dyn ValueView + 'static>> {
    if parts.is_empty() {
        return None;
    }
    let path: Vec<liquid_core::model::ScalarCow<'_>> = parts
        .iter()
        .map(|p| liquid_core::model::ScalarCow::new(*p))
        .collect();
    let val = runtime.try_get(&path)?;
    // Clone the value to own it
    Some(Box::new(val.to_value()))
}

impl Renderable for FeedMetaRenderable {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> liquid_core::Result<()> {
        // Get site URL (may be empty/missing)
        let site_url = get_nested_str(runtime, &["site", "url"]).unwrap_or_default();
        let site_url = site_url.trim_end_matches('/');

        // Get site title: prefer site.name, fall back to site.title
        let title = get_nested_str(runtime, &["site", "name"])
            .or_else(|| get_nested_str(runtime, &["site", "title"]))
            .unwrap_or_default();

        // Get feed path: site.feed.path, default to "feed.xml"
        let feed_path = get_nested_str(runtime, &["site", "feed", "path"])
            .unwrap_or_else(|| "feed.xml".to_string());
        let feed_path = feed_path.trim_start_matches('/');

        // Build the main feed URL
        let feed_url = if site_url.is_empty() {
            format!("/{}", feed_path)
        } else {
            format!("{}/{}", site_url, feed_path)
        };

        // Emit main feed link
        write!(
            writer,
            "<link type=\"application/atom+xml\" rel=\"alternate\" href=\"{}\" title=\"{}\" />",
            html_escape(&feed_url),
            html_escape(&title),
        )
        .map_err(|e| liquid_core::Error::with_msg(format!("feed_meta write error: {}", e)))?;

        // Check for category-specific feeds: site.feed.categories
        if let Some(categories_val) = get_nested_value(runtime, &["site", "feed", "categories"]) {
            if let Some(arr) = categories_val.as_array() {
                for i in 0..arr.size() {
                    if let Some(cat) = arr.get(i) {
                        let cat_name = cat.to_kstr().to_string();
                        if cat_name.is_empty() {
                            continue;
                        }
                        let cat_feed_url = if site_url.is_empty() {
                            format!("/feed/{}.xml", cat_name)
                        } else {
                            format!("{}/feed/{}.xml", site_url, cat_name)
                        };
                        write!(
                            writer,
                            "\n<link type=\"application/atom+xml\" rel=\"alternate\" href=\"{}\" title=\"{}\" />",
                            html_escape(&cat_feed_url),
                            html_escape(&format!("{} | {}", title, cat_name)),
                        )
                        .map_err(|e| {
                            liquid_core::Error::with_msg(format!(
                                "feed_meta write error: {}",
                                e
                            ))
                        })?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::engine::TemplateEngine;
    use liquid::Object;
    use liquid_core::Value;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    fn site_object(entries: Vec<(&str, Value)>) -> Value {
        let mut obj = liquid::Object::new();
        for (k, v) in entries {
            obj.insert(k.to_string().into(), v);
        }
        Value::Object(obj)
    }

    #[test]
    fn test_feed_meta_default_config() {
        // With site.url and site.name, no feed config -> default feed.xml path
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "site".into(),
            site_object(vec![
                ("url", Value::scalar("https://example.com")),
                ("name", Value::scalar("My Site")),
            ]),
        );
        let out = eng.parse_and_render("{% feed_meta %}", &ctx).unwrap();
        assert_eq!(
            out,
            r#"<link type="application/atom+xml" rel="alternate" href="https://example.com/feed.xml" title="My Site" />"#,
        );
    }

    #[test]
    fn test_feed_meta_custom_path() {
        let eng = engine();
        let mut ctx = Object::new();
        let mut feed_obj = liquid::Object::new();
        feed_obj.insert("path".into(), Value::scalar("atom.xml"));
        ctx.insert(
            "site".into(),
            site_object(vec![
                ("url", Value::scalar("https://example.com")),
                ("name", Value::scalar("My Site")),
                ("feed", Value::Object(feed_obj)),
            ]),
        );
        let out = eng.parse_and_render("{% feed_meta %}", &ctx).unwrap();
        assert!(
            out.contains("href=\"https://example.com/atom.xml\""),
            "Should use custom feed path: {}",
            out
        );
    }

    #[test]
    fn test_feed_meta_with_categories() {
        let eng = engine();
        let mut ctx = Object::new();
        let mut feed_obj = liquid::Object::new();
        feed_obj.insert(
            "categories".into(),
            Value::Array(vec![Value::scalar("release")]),
        );
        ctx.insert(
            "site".into(),
            site_object(vec![
                ("url", Value::scalar("https://jekyllrb.com")),
                ("name", Value::scalar("Jekyll")),
                ("feed", Value::Object(feed_obj)),
            ]),
        );
        let out = eng.parse_and_render("{% feed_meta %}", &ctx).unwrap();
        assert!(
            out.contains("href=\"https://jekyllrb.com/feed.xml\""),
            "Should have main feed link: {}",
            out
        );
        assert!(
            out.contains("href=\"https://jekyllrb.com/feed/release.xml\""),
            "Should have category feed link: {}",
            out
        );
        assert!(
            out.contains("title=\"Jekyll | release\""),
            "Category link should have combined title: {}",
            out
        );
    }

    #[test]
    fn test_feed_meta_no_site_url() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "site".into(),
            site_object(vec![("name", Value::scalar("My Site"))]),
        );
        let out = eng.parse_and_render("{% feed_meta %}", &ctx).unwrap();
        assert!(
            out.contains("href=\"/feed.xml\""),
            "Should use relative URL when no site.url: {}",
            out
        );
    }

    #[test]
    fn test_feed_meta_site_title_fallback() {
        // Uses site.title when site.name is not set
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "site".into(),
            site_object(vec![
                ("url", Value::scalar("https://example.com")),
                ("title", Value::scalar("Fallback Title")),
            ]),
        );
        let out = eng.parse_and_render("{% feed_meta %}", &ctx).unwrap();
        assert!(
            out.contains("title=\"Fallback Title\""),
            "Should use site.title as fallback: {}",
            out
        );
    }

    #[test]
    fn test_feed_meta_html_escaping() {
        // Title with special characters should be escaped
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "site".into(),
            site_object(vec![
                ("url", Value::scalar("https://example.com")),
                (
                    "name",
                    Value::scalar("Jekyll \u{2022} Simple, blog-aware, static sites"),
                ),
            ]),
        );
        let out = eng.parse_and_render("{% feed_meta %}", &ctx).unwrap();
        // The bullet character should pass through (it's not an HTML special char)
        assert!(
            out.contains("Jekyll \u{2022} Simple, blog-aware, static sites"),
            "Should preserve Unicode characters: {}",
            out
        );
    }

    #[test]
    fn test_feed_meta_jekyll_docs_expected_output() {
        // Exact expected output for the jekyll-docs site
        let eng = engine();
        let mut ctx = Object::new();
        let mut feed_obj = liquid::Object::new();
        feed_obj.insert(
            "categories".into(),
            Value::Array(vec![Value::scalar("release")]),
        );
        ctx.insert(
            "site".into(),
            site_object(vec![
                ("url", Value::scalar("https://jekyllrb.com")),
                (
                    "name",
                    Value::scalar("Jekyll \u{2022} Simple, blog-aware, static sites"),
                ),
                ("feed", Value::Object(feed_obj)),
            ]),
        );
        let out = eng.parse_and_render("{% feed_meta %}", &ctx).unwrap();
        let expected_main = r#"<link type="application/atom+xml" rel="alternate" href="https://jekyllrb.com/feed.xml" title="Jekyll • Simple, blog-aware, static sites" />"#;
        assert!(
            out.contains(expected_main),
            "Main feed link should match Jekyll output.\nExpected: {}\nGot: {}",
            expected_main,
            out
        );
    }

    #[test]
    fn test_feed_meta_in_head_context() {
        // feed_meta should work within a <head> tag context
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "site".into(),
            site_object(vec![
                ("url", Value::scalar("https://example.com")),
                ("name", Value::scalar("Test")),
            ]),
        );
        let out = eng
            .parse_and_render("<head>{% feed_meta %}<title>Test</title></head>", &ctx)
            .unwrap();
        assert!(
            out.starts_with("<head><link type=\"application/atom+xml\""),
            "Feed meta should appear in head: {}",
            out
        );
        assert!(
            out.ends_with("<title>Test</title></head>"),
            "Rest of head should follow: {}",
            out
        );
    }

    #[test]
    fn test_feed_meta_empty_context() {
        // With no site context at all, should still produce valid output
        let eng = engine();
        let ctx = Object::new();
        let out = eng.parse_and_render("{% feed_meta %}", &ctx).unwrap();
        assert!(
            out.contains("<link type=\"application/atom+xml\""),
            "Should still produce a link tag: {}",
            out
        );
        assert!(
            out.contains("href=\"/feed.xml\""),
            "Should use relative feed.xml: {}",
            out
        );
    }
}
