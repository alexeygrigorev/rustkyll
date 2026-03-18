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

/// Compute the absolute URL for an image path.
fn absolute_image_url(img: &str, site_url: &Option<String>) -> String {
    if img.starts_with("http://") || img.starts_with("https://") {
        img.to_string()
    } else if let Some(ref base) = site_url {
        let base = base.trim_end_matches('/');
        let path = if img.starts_with('/') {
            img.to_string()
        } else {
            format!("/{}", img)
        };
        format!("{}{}", base, path)
    } else {
        img.to_string()
    }
}

/// Title separator matching jekyll-seo-tag (` | `).
const TITLE_SEPARATOR: &str = " | ";

/// Check if a URL matches jekyll-seo-tag's HOMEPAGE_OR_ABOUT_REGEX:
/// `%r!^/(about/)?(index.html?)?$!`
/// Matches: `/`, `/index.html`, `/index.htm`, `/about/`, `/about/index.html`, `/about/index.htm`
fn is_homepage_or_about_url(url: &str) -> bool {
    let rest = url.strip_prefix('/').unwrap_or(url);
    let rest = rest.strip_prefix("about/").unwrap_or(rest);
    rest.is_empty() || rest == "index.html" || rest == "index.htm"
}

impl Renderable for SeoRenderable {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> liquid_core::Result<()> {
        let mut output = String::new();

        // Extract values from context
        let page_title = get_nested_str(runtime, &["page", "title"]);
        let site_title = get_nested_str(runtime, &["site", "title"]);
        let site_tagline = get_nested_str(runtime, &["site", "tagline"]);
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

        // Compute page_title for og:title (page title alone, falling back to site title)
        let og_page_title = page_title.as_deref().or(site_title.as_deref());

        // Read custom title separator from site config (jekyll-seo-tag reads site.title_separator)
        let title_separator = get_nested_str(runtime, &["site", "title_separator"])
            .map(|s| format!(" {} ", s))
            .unwrap_or_else(|| TITLE_SEPARATOR.to_string());

        // Compute full title matching jekyll-seo-tag logic:
        // - If page_title and site_title differ: "page_title | site_title"
        // - If page_title == site_title: "site_title | site_tagline_or_description"
        // - If only site_title: "site_title | site_tagline_or_description" (or just site_title if none)
        // - If only page_title: just page_title
        let site_tagline_or_description = site_tagline.as_deref().or(site_description.as_deref());
        let full_title: Option<String> = match (&page_title, &site_title) {
            (Some(pt), Some(st)) => {
                if pt != st {
                    Some(format!("{}{}{}", pt, title_separator, st))
                } else if let Some(tagline) = site_tagline_or_description {
                    // page_title == site_title, append tagline/description
                    Some(format!("{}{}{}", st, title_separator, tagline))
                } else {
                    Some(pt.clone())
                }
            }
            (Some(pt), None) => Some(pt.clone()),
            (None, Some(st)) => {
                if let Some(tagline) = site_tagline_or_description {
                    Some(format!("{}{}{}", st, title_separator, tagline))
                } else {
                    Some(st.clone())
                }
            }
            (None, None) => None,
        };

        // --- Begin output in Jekyll SEO tag order ---
        output.push_str("<!-- Begin Jekyll SEO tag v2.8.0 -->\n");

        // 1. <title> tag
        if !self.suppress_title {
            if let Some(ref t) = full_title {
                output.push_str(&format!("<title>{}</title>\n", html_escape(t)));
            }
        }

        // 2. <meta name="generator">
        output.push_str("<meta name=\"generator\" content=\"Jekyll v4.4.1\" />\n");

        // 3. og:title (uses page_title only, not the combined title)
        if let Some(pt) = og_page_title {
            output.push_str(&format!(
                "<meta property=\"og:title\" content=\"{}\" />\n",
                html_escape(pt)
            ));
        }

        // 4. <meta name="author"> (if present)
        let author = page_author.as_deref().or(site_author.as_deref());
        if let Some(author_name) = author {
            output.push_str(&format!(
                "<meta name=\"author\" content=\"{}\" />\n",
                html_escape(author_name)
            ));
        }

        // 5. og:locale
        let locale = site_locale.as_deref().unwrap_or("en_US");
        output.push_str(&format!(
            "<meta property=\"og:locale\" content=\"{}\" />\n",
            html_escape(locale)
        ));

        // 6. Description (both meta name="description" and og:description together)
        let description = page_description
            .as_deref()
            .or(page_excerpt.as_deref())
            .or(site_description.as_deref());

        if let Some(desc) = description {
            output.push_str(&format!(
                "<meta name=\"description\" content=\"{}\" />\n",
                html_escape(desc)
            ));
            output.push_str(&format!(
                "<meta property=\"og:description\" content=\"{}\" />\n",
                html_escape(desc)
            ));
        }

        // 7. Canonical URL + og:url (together)
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

        if site_url.is_some() {
            if let Some(ref url) = canonical_url {
                output.push_str(&format!(
                    "<link rel=\"canonical\" href=\"{}\" />\n",
                    html_escape(url)
                ));
                output.push_str(&format!(
                    "<meta property=\"og:url\" content=\"{}\" />\n",
                    html_escape(url)
                ));
            }
        }

        // 8. og:site_name
        if let Some(ref st) = site_title {
            output.push_str(&format!(
                "<meta property=\"og:site_name\" content=\"{}\" />\n",
                html_escape(st)
            ));
        }

        // 9. og:image
        if let Some(ref img) = page_image {
            let absolute_img = absolute_image_url(img, &site_url);
            output.push_str(&format!(
                "<meta property=\"og:image\" content=\"{}\" />\n",
                html_escape(&absolute_img)
            ));
        }

        // 10. og:type - "article" for posts (pages with date), "website" otherwise
        if page_date.is_some() {
            output.push_str("<meta property=\"og:type\" content=\"article\" />\n");
        } else {
            output.push_str("<meta property=\"og:type\" content=\"website\" />\n");
        }

        // 11. Twitter Card
        if page_image.is_some() {
            output.push_str("<meta name=\"twitter:card\" content=\"summary_large_image\" />\n");
            let absolute_img = absolute_image_url(page_image.as_deref().unwrap_or(""), &site_url);
            output.push_str(&format!(
                "<meta property=\"twitter:image\" content=\"{}\" />\n",
                html_escape(&absolute_img)
            ));
        } else {
            output.push_str("<meta name=\"twitter:card\" content=\"summary\" />\n");
        }

        // 12. twitter:title
        if let Some(pt) = og_page_title {
            output.push_str(&format!(
                "<meta property=\"twitter:title\" content=\"{}\" />\n",
                html_escape(pt)
            ));
        }

        // 13. Twitter site
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

        // 14. JSON-LD structured data
        // Determine if this is a homepage or about page, matching jekyll-seo-tag's
        // HOMEPAGE_OR_ABOUT_REGEX = %r!^/(about/)?(index.html?)?$!
        let is_homepage_or_about = page_url
            .as_deref()
            .map(is_homepage_or_about_url)
            .unwrap_or(false);

        let schema_type = if page_date.is_some() {
            "BlogPosting"
        } else if is_homepage_or_about {
            "WebSite"
        } else {
            "WebPage"
        };

        output.push_str("<script type=\"application/ld+json\">\n");
        output.push_str("{\n");
        output.push_str("  \"@context\": \"https://schema.org\",\n");
        output.push_str(&format!("  \"@type\": \"{}\",\n", schema_type));

        // name field: jekyll-seo-tag only includes name for homepage/about pages
        if is_homepage_or_about {
            // Use site social name, or site_title
            let jsonld_name = site_title.as_deref();
            if let Some(name) = jsonld_name {
                output.push_str(&format!("  \"name\": \"{}\",\n", json_escape(name)));
            }
        }

        if let Some(ref t) = full_title {
            // headline is max 110 chars per Google guidelines
            let headline = if t.len() > 110 { &t[..110] } else { t.as_str() };
            output.push_str(&format!("  \"headline\": \"{}\",\n", json_escape(headline)));
        }

        if let Some(desc) = description {
            output.push_str(&format!("  \"description\": \"{}\",\n", json_escape(desc)));
        }

        // url field: jekyll-seo-tag always includes canonical_url in JSON-LD
        // When site.url is absent, use page.url directly (Jekyll uses absolute_url which
        // returns page.url when site.url is empty)
        let jsonld_url = canonical_url.as_deref().or(page_url.as_deref());
        if let Some(url) = jsonld_url {
            output.push_str(&format!("  \"url\": \"{}\",\n", json_escape(url)));
        }

        // Author in JSON-LD
        if let Some(author_name) = author {
            output.push_str("  \"author\": {\n");
            output.push_str("    \"@type\": \"Person\",\n");
            output.push_str(&format!("    \"name\": \"{}\"\n", json_escape(author_name)));
            output.push_str("  },\n");
        }

        if let Some(ref date) = page_date {
            // Format date using date_to_xmlschema logic with site timezone,
            // matching Jekyll's jekyll-seo-tag which uses {{ page.date | date_to_xmlschema }}
            let site_tz = crate::template::filters::get_site_timezone(runtime);
            let formatted_date = crate::template::filters::format_date_to_xmlschema(date, site_tz);
            output.push_str(&format!(
                "  \"datePublished\": \"{}\",\n",
                json_escape(&formatted_date)
            ));
        }

        if let Some(ref img) = page_image {
            let absolute_img = absolute_image_url(img, &site_url);
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

        output.push_str("<!-- End Jekyll SEO tag -->\n");

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
        assert!(
            out.contains("<title>My Page | My Site</title>"),
            "Title should use pipe separator, got: {}",
            out
        );
    }

    #[test]
    fn test_title_page_equals_site_title_with_description() {
        // When page_title == site_title, Jekyll appends site tagline or description
        let eng = engine();
        let ctx = make_context(
            Some("My Site"),
            Some("My Site"),
            None,
            Some("A great site"),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<title>My Site | A great site</title>"),
            "When page_title == site_title, should append site description. Got: {}",
            out
        );
    }

    #[test]
    fn test_title_page_equals_site_title_no_description() {
        // When page_title == site_title and no tagline/description, just use the title
        let eng = engine();
        let ctx = make_context(
            Some("My Site"),
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
        assert!(
            out.contains("<title>My Site</title>"),
            "When page_title == site_title and no description, just title. Got: {}",
            out
        );
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
    fn test_og_title_uses_page_title_only() {
        // og:title should use page_title only, not the combined title
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
        assert!(
            out.contains("<meta property=\"og:title\" content=\"My Page\" />"),
            "og:title should be page title only, got: {}",
            out
        );
    }

    #[test]
    fn test_og_title_falls_back_to_site_title() {
        // When no page title, og:title falls back to site_title
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
        assert!(
            out.contains("<meta property=\"og:title\" content=\"My Site\" />"),
            "og:title should fall back to site title, got: {}",
            out
        );
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
    fn test_json_ld_no_name_for_non_homepage() {
        // jekyll-seo-tag only includes name for homepage/about pages
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            None,
            None,
            None,
            None,
            Some("/my-page.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        let jsonld_start = out
            .find("application/ld+json")
            .expect("should have json-ld");
        let jsonld_block = &out[jsonld_start..];
        let script_end = jsonld_block
            .find("</script>")
            .expect("should have closing script tag");
        let jsonld_content = &jsonld_block[..script_end];
        assert!(
            !jsonld_content.contains("\"name\""),
            "Non-homepage JSON-LD should not contain name field"
        );
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
    // JSON-LD @type: WebSite for homepage
    // ========================================================================

    #[test]
    fn test_jsonld_type_homepage_is_website() {
        let eng = engine();
        let ctx = make_context(
            Some("My Site"),
            Some("My Site"),
            None,
            None,
            None,
            Some("/"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("\"@type\": \"WebSite\""),
            "Homepage (url='/') should have @type WebSite, got: {}",
            out
        );
    }

    #[test]
    fn test_jsonld_type_index_html_is_website() {
        let eng = engine();
        let ctx = make_context(
            Some("My Site"),
            Some("My Site"),
            None,
            None,
            None,
            Some("/index.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("\"@type\": \"WebSite\""),
            "Index page (url='/index.html') should have @type WebSite, got: {}",
            out
        );
    }

    #[test]
    fn test_jsonld_type_about_is_website() {
        let eng = engine();
        let ctx = make_context(
            Some("About"),
            Some("My Site"),
            None,
            None,
            None,
            Some("/about/"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("\"@type\": \"WebSite\""),
            "About page (url='/about/') should have @type WebSite, got: {}",
            out
        );
    }

    #[test]
    fn test_jsonld_type_subpage_is_webpage() {
        let eng = engine();
        let ctx = make_context(
            Some("Other Page"),
            Some("My Site"),
            None,
            None,
            None,
            Some("/about.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("\"@type\": \"WebPage\""),
            "Subpage (url='/about.html') should have @type WebPage, got: {}",
            out
        );
    }

    #[test]
    fn test_jsonld_type_post_with_date_is_blogposting() {
        let eng = engine();
        let ctx = make_context(
            Some("My Post"),
            Some("My Site"),
            None,
            None,
            None,
            Some("/posts/my-post.html"),
            None,
            Some("2024-01-15"),
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("\"@type\": \"BlogPosting\""),
            "Post with date should have @type BlogPosting, got: {}",
            out
        );
    }

    // ========================================================================
    // JSON-LD url field inclusion
    // ========================================================================

    #[test]
    fn test_jsonld_includes_url_field() {
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
        assert!(
            out.contains("\"url\": \"https://example.com/about\""),
            "JSON-LD should include url field with absolute URL, got: {}",
            out
        );
    }

    #[test]
    fn test_jsonld_url_without_site_url() {
        // When no site.url, Jekyll still includes url from page.url
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            None,
            None,
            None,
            Some("/"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("\"url\": \"/\""),
            "JSON-LD should include url field even without site.url, got: {}",
            out
        );
    }

    // ========================================================================
    // JSON-LD name field logic
    // ========================================================================

    #[test]
    fn test_jsonld_website_includes_name() {
        // Homepage (WebSite type) should include name with site title
        let eng = engine();
        let ctx = make_context(
            Some("My Site"),
            Some("My Site"),
            None,
            None,
            None,
            Some("/"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("\"name\": \"My Site\""),
            "Homepage JSON-LD should include name field, got: {}",
            out
        );
    }

    #[test]
    fn test_jsonld_webpage_no_name() {
        // Subpage (WebPage type) should NOT include name
        let eng = engine();
        let ctx = make_context(
            Some("Other Page"),
            Some("My Site"),
            None,
            None,
            None,
            Some("/other.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        // Extract JSON-LD block
        let jsonld_start = out
            .find("application/ld+json")
            .expect("should have json-ld");
        let jsonld_block = &out[jsonld_start..];
        let script_end = jsonld_block
            .find("</script>")
            .expect("should have closing script tag");
        let jsonld_content = &jsonld_block[..script_end];
        assert!(
            !jsonld_content.contains("\"name\""),
            "Subpage JSON-LD should NOT include name field, got: {}",
            jsonld_content
        );
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
        assert!(out.contains("<title>My Page | My Site</title>"));
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
        assert!(out.contains("twitter:title"));
        assert!(out.contains("twitter:image"));
        assert!(out.contains("application/ld+json"));
        assert!(out.contains("BlogPosting"));
        assert!(out.contains("<!-- Begin Jekyll SEO tag"));
        assert!(out.contains("<!-- End Jekyll SEO tag -->"));
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

    // ========================================================================
    // Canonical URL construction (issue #69)
    // ========================================================================

    #[test]
    fn test_canonical_url_no_extension() {
        // Page URL /articles (no .html) -> canonical = site_url + /articles
        let eng = engine();
        let ctx = make_context(
            Some("Articles"),
            Some("My Site"),
            None,
            None,
            Some("https://example.com"),
            Some("/articles"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("href=\"https://example.com/articles\""),
            "Canonical should be https://example.com/articles, got: {}",
            out
        );
    }

    #[test]
    fn test_canonical_url_with_trailing_slash() {
        let eng = engine();
        let ctx = make_context(
            Some("Articles"),
            Some("My Site"),
            None,
            None,
            Some("https://example.com"),
            Some("/articles/"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("href=\"https://example.com/articles/\""),
            "Canonical should preserve trailing slash"
        );
    }

    // ========================================================================
    // Meta tag ordering (issue #173)
    // ========================================================================

    #[test]
    fn test_meta_tag_order_matches_jekyll() {
        // Verify meta tags appear in the same order as jekyll-seo-tag
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            Some("A description"),
            None,
            Some("https://example.com"),
            Some("/page"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();

        // Find positions of key elements
        let pos_begin = out
            .find("<!-- Begin Jekyll SEO tag")
            .expect("begin comment");
        let pos_title = out.find("<title>").expect("title");
        let pos_generator = out.find("name=\"generator\"").expect("generator");
        let pos_og_title = out.find("og:title").expect("og:title");
        let pos_og_locale = out.find("og:locale").expect("og:locale");
        let pos_description = out.find("name=\"description\"").expect("description");
        let pos_og_description = out.find("og:description").expect("og:description");
        let pos_canonical = out.find("rel=\"canonical\"").expect("canonical");
        let pos_og_url = out.find("og:url").expect("og:url");
        let pos_og_site_name = out.find("og:site_name").expect("og:site_name");
        let pos_og_type = out.find("og:type").expect("og:type");
        let pos_twitter_card = out.find("twitter:card").expect("twitter:card");
        let pos_json_ld = out.find("application/ld+json").expect("json-ld");
        let pos_end = out.find("<!-- End Jekyll SEO tag").expect("end comment");

        // Assert ordering matches Jekyll template
        assert!(pos_begin < pos_title, "begin comment before title");
        assert!(pos_title < pos_generator, "title before generator");
        assert!(pos_generator < pos_og_title, "generator before og:title");
        assert!(pos_og_title < pos_og_locale, "og:title before og:locale");
        assert!(
            pos_og_locale < pos_description,
            "og:locale before description"
        );
        assert!(
            pos_description < pos_og_description,
            "description before og:description"
        );
        assert!(
            pos_og_description < pos_canonical,
            "og:description before canonical"
        );
        assert!(pos_canonical < pos_og_url, "canonical before og:url");
        assert!(pos_og_url < pos_og_site_name, "og:url before og:site_name");
        assert!(
            pos_og_site_name < pos_og_type,
            "og:site_name before og:type"
        );
        assert!(
            pos_og_type < pos_twitter_card,
            "og:type before twitter:card"
        );
        assert!(
            pos_twitter_card < pos_json_ld,
            "twitter:card before json-ld"
        );
        assert!(pos_json_ld < pos_end, "json-ld before end comment");
    }

    #[test]
    fn test_generator_meta_tag() {
        let eng = engine();
        let ctx = make_context(None, None, None, None, None, None, None, None, None, None);
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<meta name=\"generator\" content=\"Jekyll v4.4.1\" />"),
            "Should have generator meta tag, got: {}",
            out
        );
    }

    #[test]
    fn test_twitter_title_present() {
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
        assert!(
            out.contains("<meta property=\"twitter:title\" content=\"My Page\" />"),
            "Should have twitter:title with page title, got: {}",
            out
        );
    }

    #[test]
    fn test_twitter_image_present_with_image() {
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
        assert!(
            out.contains(
                "<meta property=\"twitter:image\" content=\"https://example.com/img/cover.png\" />"
            ),
            "Should have twitter:image, got: {}",
            out
        );
    }

    #[test]
    fn test_begin_end_comments() {
        let eng = engine();
        let ctx = make_context(None, None, None, None, None, None, None, None, None, None);
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.starts_with("<!-- Begin Jekyll SEO tag v2.8.0 -->\n"));
        assert!(out.ends_with("<!-- End Jekyll SEO tag -->\n"));
    }

    #[test]
    fn test_description_and_og_description_emitted_together() {
        // Both name="description" and og:description should be present
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            Some("My desc"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(out.contains("<meta name=\"description\" content=\"My desc\" />"));
        assert!(out.contains("<meta property=\"og:description\" content=\"My desc\" />"));
    }

    #[test]
    fn test_architect_theme_like_output() {
        // Simulate architect theme: site.title = "Architect theme",
        // site.description = "Architect is a theme for GitHub Pages."
        // page.title = "Architect theme" (same as site title on index)
        let eng = engine();
        let ctx = make_context(
            Some("Architect theme"),
            Some("Architect theme"),
            None,
            Some("Architect is a theme for GitHub Pages."),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<title>Architect theme | Architect is a theme for GitHub Pages.</title>"),
            "Title should be 'site_title | site_description' when page==site. Got: {}",
            out
        );
        assert!(
            out.contains("<meta property=\"og:title\" content=\"Architect theme\" />"),
            "og:title should be page title only. Got: {}",
            out
        );
    }

    // ========================================================================
    // Title tag description suffix (issue #192)
    // ========================================================================

    #[test]
    fn test_title_tag_with_description_suffix() {
        // No page title, site title + description => "My Site | A cool site"
        let eng = engine();
        let ctx = make_context(
            None,
            Some("My Site"),
            None,
            Some("A cool site"),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<title>My Site | A cool site</title>"),
            "Title should be 'site_title | site_description' when no page title. Got: {}",
            out
        );
    }

    #[test]
    fn test_title_tag_homepage_format() {
        // Homepage: no page title, only site title + description
        let eng = engine();
        let ctx = make_context(
            None,
            Some("My Site"),
            None,
            Some("A cool site"),
            Some("https://example.com"),
            Some("/"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<title>My Site | A cool site</title>"),
            "Homepage title should be 'site_title | description'. Got: {}",
            out
        );
    }

    #[test]
    fn test_title_tag_with_tagline() {
        // Site with tagline -- tagline preferred over description
        let eng = engine();
        let mut ctx = Object::new();
        let page = Object::new();
        let mut site = Object::new();
        site.insert("title".into(), Value::scalar("My Site".to_string()));
        site.insert("tagline".into(), Value::scalar("My Tagline".to_string()));
        site.insert(
            "description".into(),
            Value::scalar("My Description".to_string()),
        );
        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<title>My Site | My Tagline</title>"),
            "Tagline should be preferred over description in suffix. Got: {}",
            out
        );
    }

    #[test]
    fn test_title_tag_custom_separator() {
        // Site with custom title_separator
        let eng = engine();
        let mut ctx = Object::new();
        let page = Object::new();
        let mut site = Object::new();
        site.insert("title".into(), Value::scalar("My Site".to_string()));
        site.insert(
            "description".into(),
            Value::scalar("A cool site".to_string()),
        );
        site.insert("title_separator".into(), Value::scalar("-".to_string()));
        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<title>My Site - A cool site</title>"),
            "Should use custom separator. Got: {}",
            out
        );
    }

    #[test]
    fn test_title_tag_no_description() {
        // Site with no description or tagline -- no suffix
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
        assert!(
            out.contains("<title>My Site</title>"),
            "No suffix when no description/tagline. Got: {}",
            out
        );
    }

    #[test]
    fn test_title_tag_page_with_different_titles() {
        // page_title != site_title: "About | My Site" (uses site title, not description)
        let eng = engine();
        let ctx = make_context(
            Some("About"),
            Some("My Site"),
            None,
            Some("A cool site"),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<title>About | My Site</title>"),
            "When page != site title, suffix should be site title not description. Got: {}",
            out
        );
    }

    // ========================================================================
    // Canonical URL construction (issue #69)
    // ========================================================================

    #[test]
    fn test_canonical_url_no_double_slashes() {
        let eng = engine();
        let ctx = make_context(
            Some("Home"),
            Some("My Site"),
            None,
            None,
            Some("https://example.com/"),
            Some("/"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        // site.url has trailing slash, page.url is "/" -- should not produce "//"
        assert!(
            out.contains("href=\"https://example.com/\""),
            "Should not have double slashes in canonical URL"
        );
        assert!(
            !out.contains("href=\"https://example.com//\""),
            "Must not have double slashes"
        );
    }
}
