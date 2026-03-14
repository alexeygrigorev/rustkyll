//! Custom `{% seo %}` tag implementing Jekyll SEO Tag plugin functionality.
//!
//! Generates SEO meta tags from page front matter and site configuration,
//! including:
//! - `<title>` tag
//! - `<meta name="description">` tag
//! - Open Graph meta tags
//! - Twitter Card meta tags
//! - JSON-LD structured data
//! - Canonical URL link
//!
//! Supports `{% seo title=false %}` to suppress the `<title>` tag.

use std::io::Write;

use liquid_core::model::ValueView;
use liquid_core::parser::TryMatchToken;
use liquid_core::{Language, ParseTag, Renderable, Runtime, TagReflection, TagTokenIter};

/// The `{% seo %}` tag parser/reflection.
#[derive(Copy, Clone, Debug, Default)]
pub struct SeoTag;

impl TagReflection for SeoTag {
    fn tag(&self) -> &'static str {
        "seo"
    }

    fn description(&self) -> &'static str {
        "Generate SEO meta tags from page and site context"
    }
}

impl ParseTag for SeoTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        let mut suppress_title = false;

        // Parse optional title=false argument.
        // Tokens come as: "title", "=", "false"
        // We consume all tokens and look for the title=false pattern.
        while let Ok(next) = arguments.expect_next("") {
            match next.expect_identifier() {
                TryMatchToken::Matches(id) if id.to_kstr() == "title" => {
                    // Expect "="
                    if let Ok(eq_token) = arguments.expect_next("") {
                        let _ = eq_token.expect_str("=").into_result();
                        // Expect "false" -- could be an identifier or a value
                        if let Ok(val_token) = arguments.expect_next("") {
                            // Try as value (handles false literal)
                            match val_token.expect_value() {
                                TryMatchToken::Matches(expr) => {
                                    // Check if it evaluates to false at the expression level
                                    // For now, check the string representation
                                    let expr_str = format!("{}", expr);
                                    if expr_str == "false" {
                                        suppress_title = true;
                                    }
                                }
                                TryMatchToken::Fails(token) => {
                                    // Try as identifier
                                    match token.expect_identifier() {
                                        TryMatchToken::Matches(val) if val.to_kstr() == "false" => {
                                            suppress_title = true;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Ignore unknown arguments
                }
            }
        }

        Ok(Box::new(SeoRenderable { suppress_title }))
    }

    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

#[derive(Debug)]
struct SeoRenderable {
    suppress_title: bool,
}

/// HTML-escape a string for use in attribute values and text content.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Get a nested value like "site.twitter.username" by traversing objects.
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

impl Renderable for SeoRenderable {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> liquid_core::Result<()> {
        let mut output = String::new();

        // Extract values from context
        let page_title = get_nested_str(runtime, &["page", "title"]);
        let site_title = get_nested_str(runtime, &["site", "title"]);
        let page_description = get_nested_str(runtime, &["page", "description"]);
        let page_excerpt = get_nested_str(runtime, &["page", "excerpt"]);
        let site_description = get_nested_str(runtime, &["site", "description"]);
        let site_url = get_nested_str(runtime, &["site", "url"]);
        let page_url = get_nested_str(runtime, &["page", "url"]);
        let page_image = get_nested_str(runtime, &["page", "image"]);
        let page_date = get_nested_str(runtime, &["page", "date"]);
        let site_locale = get_nested_str(runtime, &["site", "locale"]);
        let twitter_username = get_nested_str(runtime, &["site", "twitter", "username"]);
        let page_author = get_nested_str(runtime, &["page", "author"]);
        let site_author = get_nested_str(runtime, &["site", "author"]);

        // 1. Title
        let title = match (&page_title, &site_title) {
            (Some(pt), Some(st)) => Some(format!("{} - {}", pt, st)),
            (Some(pt), None) => Some(pt.clone()),
            (None, Some(st)) => Some(st.clone()),
            (None, None) => None,
        };

        if !self.suppress_title {
            if let Some(ref t) = title {
                output.push_str(&format!("<title>{}</title>\n", html_escape(t)));
            }
        }

        // 2. Description
        let description = page_description
            .as_deref()
            .or(page_excerpt.as_deref())
            .or(site_description.as_deref());

        if let Some(desc) = description {
            output.push_str(&format!(
                "<meta name=\"description\" content=\"{}\" />\n",
                html_escape(desc)
            ));
        }

        // 3. Canonical URL
        let canonical_url = match (&site_url, &page_url) {
            (Some(base), Some(path)) => {
                let base = base.trim_end_matches('/');
                let path = if path.starts_with('/') {
                    path.clone()
                } else {
                    format!("/{}", path)
                };
                Some(format!("{}{}", base, path))
            }
            (Some(base), None) => Some(base.trim_end_matches('/').to_string()),
            _ => None,
        };

        if let Some(ref url) = canonical_url {
            output.push_str(&format!(
                "<link rel=\"canonical\" href=\"{}\" />\n",
                html_escape(url)
            ));
        }

        // 4-10. Open Graph tags
        if let Some(ref t) = title {
            output.push_str(&format!(
                "<meta property=\"og:title\" content=\"{}\" />\n",
                html_escape(t)
            ));
        }

        if let Some(desc) = description {
            output.push_str(&format!(
                "<meta property=\"og:description\" content=\"{}\" />\n",
                html_escape(desc)
            ));
        }

        if let Some(ref url) = canonical_url {
            output.push_str(&format!(
                "<meta property=\"og:url\" content=\"{}\" />\n",
                html_escape(url)
            ));
        }

        if let Some(ref st) = site_title {
            output.push_str(&format!(
                "<meta property=\"og:site_name\" content=\"{}\" />\n",
                html_escape(st)
            ));
        }

        // og:type - "article" for posts (pages with date), "website" otherwise
        let og_type = if page_date.is_some() {
            "article"
        } else {
            "website"
        };
        output.push_str(&format!(
            "<meta property=\"og:type\" content=\"{}\" />\n",
            og_type
        ));

        // og:image
        if let Some(ref img) = page_image {
            let absolute_img = if img.starts_with("http://") || img.starts_with("https://") {
                img.clone()
            } else if let Some(ref base) = site_url {
                let base = base.trim_end_matches('/');
                let path = if img.starts_with('/') {
                    img.clone()
                } else {
                    format!("/{}", img)
                };
                format!("{}{}", base, path)
            } else {
                img.clone()
            };
            output.push_str(&format!(
                "<meta property=\"og:image\" content=\"{}\" />\n",
                html_escape(&absolute_img)
            ));
        }

        // og:locale
        let locale = site_locale.as_deref().unwrap_or("en_US");
        output.push_str(&format!(
            "<meta property=\"og:locale\" content=\"{}\" />\n",
            html_escape(locale)
        ));

        // 11. Twitter Card
        let card_type = if page_image.is_some() {
            "summary_large_image"
        } else {
            "summary"
        };
        output.push_str(&format!(
            "<meta name=\"twitter:card\" content=\"{}\" />\n",
            card_type
        ));

        // 12. Twitter site
        if let Some(ref username) = twitter_username {
            let handle = if username.starts_with('@') {
                username.clone()
            } else {
                format!("@{}", username)
            };
            output.push_str(&format!(
                "<meta name=\"twitter:site\" content=\"{}\" />\n",
                html_escape(&handle)
            ));
        }

        // 13. JSON-LD structured data
        let schema_type = if page_date.is_some() {
            "BlogPosting"
        } else {
            "WebPage"
        };

        output.push_str("<script type=\"application/ld+json\">\n");
        output.push_str("{\n");
        output.push_str("  \"@context\": \"https://schema.org\",\n");
        output.push_str(&format!("  \"@type\": \"{}\",\n", schema_type));

        if let Some(ref t) = title {
            let escaped = json_escape(t);
            output.push_str(&format!("  \"name\": \"{}\",\n", escaped));
            // headline is max 110 chars per Google guidelines
            let headline = if t.len() > 110 { &t[..110] } else { t };
            output.push_str(&format!("  \"headline\": \"{}\",\n", json_escape(headline)));
        }

        if let Some(desc) = description {
            output.push_str(&format!("  \"description\": \"{}\",\n", json_escape(desc)));
        }

        if let Some(ref url) = canonical_url {
            output.push_str(&format!("  \"url\": \"{}\",\n", json_escape(url)));
        }

        // Author
        let author = page_author.as_deref().or(site_author.as_deref());
        if let Some(author_name) = author {
            output.push_str("  \"author\": {\n");
            output.push_str("    \"@type\": \"Person\",\n");
            output.push_str(&format!("    \"name\": \"{}\"\n", json_escape(author_name)));
            output.push_str("  },\n");
        }

        if let Some(ref date) = page_date {
            output.push_str(&format!(
                "  \"datePublished\": \"{}\",\n",
                json_escape(date)
            ));
        }

        if let Some(ref img) = page_image {
            let absolute_img = if img.starts_with("http://") || img.starts_with("https://") {
                img.clone()
            } else if let Some(ref base) = site_url {
                let base = base.trim_end_matches('/');
                let path = if img.starts_with('/') {
                    img.clone()
                } else {
                    format!("/{}", img)
                };
                format!("{}{}", base, path)
            } else {
                img.clone()
            };
            output.push_str(&format!(
                "  \"image\": \"{}\",\n",
                json_escape(&absolute_img)
            ));
        }

        // Remove trailing comma+newline and replace with just newline
        if output.ends_with(",\n") {
            output.truncate(output.len() - 2);
            output.push('\n');
        }

        output.push_str("}\n");
        output.push_str("</script>\n");

        write!(writer, "{}", output)
            .map_err(|e| liquid_core::Error::with_msg(format!("seo tag write error: {}", e)))?;
        Ok(())
    }
}

/// Escape a string for JSON string values.
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use liquid::Object;
    use liquid_core::Value;

    use crate::template::TemplateEngine;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn make_context(
        page_title: Option<&str>,
        site_title: Option<&str>,
        page_desc: Option<&str>,
        site_desc: Option<&str>,
        site_url: Option<&str>,
        page_url: Option<&str>,
        page_image: Option<&str>,
        page_date: Option<&str>,
        site_locale: Option<&str>,
        twitter_username: Option<&str>,
    ) -> Object {
        let mut ctx = Object::new();
        let mut page = Object::new();
        let mut site = Object::new();

        if let Some(t) = page_title {
            page.insert("title".into(), Value::scalar(t.to_string()));
        }
        if let Some(d) = page_desc {
            page.insert("description".into(), Value::scalar(d.to_string()));
        }
        if let Some(u) = page_url {
            page.insert("url".into(), Value::scalar(u.to_string()));
        }
        if let Some(i) = page_image {
            page.insert("image".into(), Value::scalar(i.to_string()));
        }
        if let Some(d) = page_date {
            page.insert("date".into(), Value::scalar(d.to_string()));
        }

        if let Some(t) = site_title {
            site.insert("title".into(), Value::scalar(t.to_string()));
        }
        if let Some(d) = site_desc {
            site.insert("description".into(), Value::scalar(d.to_string()));
        }
        if let Some(u) = site_url {
            site.insert("url".into(), Value::scalar(u.to_string()));
        }
        if let Some(l) = site_locale {
            site.insert("locale".into(), Value::scalar(l.to_string()));
        }
        if let Some(tu) = twitter_username {
            let mut twitter = Object::new();
            twitter.insert("username".into(), Value::scalar(tu.to_string()));
            site.insert("twitter".into(), Value::Object(twitter));
        }

        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        ctx
    }

    // ========================================================================
    // Title generation
    // ========================================================================

    #[test]
    fn test_title_page_and_site() {
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<title>My Page - My Site</title>"));
    }

    #[test]
    fn test_title_page_only() {
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<title>My Page</title>"));
    }

    #[test]
    fn test_title_site_only() {
        let eng = engine();
        let ctx = make_context(
            None,
            Some("My Site"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<title>My Site</title>"));
    }

    #[test]
    fn test_title_neither() {
        let eng = engine();
        let ctx = make_context(None, None, None, None, None, None, None, None, None, None);
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(!out.contains("<title>"));
    }

    #[test]
    fn test_title_suppressed() {
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo title=false %}", &ctx).unwrap();
        assert!(!out.contains("<title>"));
        // But OG title should still be present
        assert!(out.contains("og:title"));
    }

    // ========================================================================
    // Description meta tag
    // ========================================================================

    #[test]
    fn test_description_from_page() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            Some("A description"),
            Some("Site desc"),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta name=\"description\" content=\"A description\" />"));
    }

    #[test]
    fn test_description_fallback_to_site() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            None,
            Some("Site description"),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta name=\"description\" content=\"Site description\" />"));
    }

    #[test]
    fn test_description_none() {
        let eng = engine();
        let ctx = make_context(None, None, None, None, None, None, None, None, None, None);
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(!out.contains("name=\"description\""));
    }

    #[test]
    fn test_description_with_html_entities() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            Some("Tom & Jerry's \"show\""),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains(
            "<meta name=\"description\" content=\"Tom &amp; Jerry&#39;s &quot;show&quot;\" />"
        ));
    }

    // ========================================================================
    // Open Graph tags
    // ========================================================================

    #[test]
    fn test_og_title() {
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta property=\"og:title\" content=\"My Page - My Site\" />"));
    }

    #[test]
    fn test_og_description() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            Some("Page desc"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta property=\"og:description\" content=\"Page desc\" />"));
    }

    #[test]
    fn test_og_url() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            None,
            None,
            Some("https://example.com"),
            Some("/about"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta property=\"og:url\" content=\"https://example.com/about\" />"));
    }

    #[test]
    fn test_og_site_name() {
        let eng = engine();
        let ctx = make_context(
            None,
            Some("My Site"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta property=\"og:site_name\" content=\"My Site\" />"));
    }

    #[test]
    fn test_og_type_website() {
        let eng = engine();
        let ctx = make_context(None, None, None, None, None, None, None, None, None, None);
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta property=\"og:type\" content=\"website\" />"));
    }

    #[test]
    fn test_og_type_article() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("2024-01-15"),
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta property=\"og:type\" content=\"article\" />"));
    }

    #[test]
    fn test_og_image() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            None,
            None,
            Some("https://example.com"),
            None,
            Some("/img/cover.png"),
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains(
            "<meta property=\"og:image\" content=\"https://example.com/img/cover.png\" />"
        ));
    }

    #[test]
    fn test_og_image_omitted_when_no_image() {
        let eng = engine();
        let ctx = make_context(None, None, None, None, None, None, None, None, None, None);
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(!out.contains("og:image"));
    }

    #[test]
    fn test_og_locale_default() {
        let eng = engine();
        let ctx = make_context(None, None, None, None, None, None, None, None, None, None);
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta property=\"og:locale\" content=\"en_US\" />"));
    }

    #[test]
    fn test_og_locale_custom() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("fr_FR"),
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta property=\"og:locale\" content=\"fr_FR\" />"));
    }

    // ========================================================================
    // Twitter Card tags
    // ========================================================================

    #[test]
    fn test_twitter_card_summary() {
        let eng = engine();
        let ctx = make_context(None, None, None, None, None, None, None, None, None, None);
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta name=\"twitter:card\" content=\"summary\" />"));
    }

    #[test]
    fn test_twitter_card_summary_large_image() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            None,
            None,
            None,
            None,
            Some("/img/big.jpg"),
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta name=\"twitter:card\" content=\"summary_large_image\" />"));
    }

    #[test]
    fn test_twitter_site_with_at() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("@mysite"),
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta name=\"twitter:site\" content=\"@mysite\" />"));
    }

    #[test]
    fn test_twitter_site_without_at() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("mysite"),
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta name=\"twitter:site\" content=\"@mysite\" />"));
    }

    #[test]
    fn test_twitter_site_omitted_when_no_config() {
        let eng = engine();
        let ctx = make_context(None, None, None, None, None, None, None, None, None, None);
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(!out.contains("twitter:site"));
    }

    // ========================================================================
    // JSON-LD structured data
    // ========================================================================

    #[test]
    fn test_json_ld_webpage() {
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("\"@type\": \"WebPage\""));
        assert!(out.contains("application/ld+json"));
    }

    #[test]
    fn test_json_ld_blogposting() {
        let eng = engine();
        let ctx = make_context(
            Some("My Post"),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("2024-01-15"),
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("\"@type\": \"BlogPosting\""));
    }

    #[test]
    fn test_json_ld_contains_name() {
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("\"name\": \"My Page\""));
    }

    #[test]
    fn test_json_ld_contains_url() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            None,
            None,
            Some("https://example.com"),
            Some("/about"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("\"url\": \"https://example.com/about\""));
    }

    // ========================================================================
    // HTML escaping
    // ========================================================================

    #[test]
    fn test_title_with_special_chars() {
        let eng = engine();
        let ctx = make_context(
            Some("Tom & Jerry's <show>"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<title>Tom &amp; Jerry&#39;s &lt;show&gt;</title>"));
    }

    // ========================================================================
    // Integration: full rendering
    // ========================================================================

    #[test]
    fn test_full_context_rendering() {
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            Some("A great page"),
            None,
            Some("https://example.com"),
            Some("/my-page"),
            Some("/img/cover.png"),
            Some("2024-01-15"),
            None,
            Some("mysite"),
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<title>My Page - My Site</title>"));
        assert!(out.contains("name=\"description\""));
        assert!(out.contains("rel=\"canonical\""));
        assert!(out.contains("og:title"));
        assert!(out.contains("og:description"));
        assert!(out.contains("og:url"));
        assert!(out.contains("og:site_name"));
        assert!(out.contains("og:type"));
        assert!(out.contains("og:image"));
        assert!(out.contains("og:locale"));
        assert!(out.contains("twitter:card"));
        assert!(out.contains("twitter:site"));
        assert!(out.contains("application/ld+json"));
        assert!(out.contains("BlogPosting"));
    }

    #[test]
    fn test_minimal_context_rendering() {
        let eng = engine();
        let ctx = make_context(
            None,
            Some("My Site"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<title>My Site</title>"));
        assert!(out.contains("og:type"));
        assert!(out.contains("twitter:card"));
    }

    #[test]
    fn test_empty_context_rendering() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        // Should produce minimal output without errors
        assert!(out.contains("og:type"));
        assert!(out.contains("twitter:card"));
        assert!(!out.contains("<title>"));
        assert!(!out.contains("name=\"description\""));
    }
}
