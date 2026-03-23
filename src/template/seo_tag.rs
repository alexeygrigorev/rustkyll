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

/// HTML-escape for meta content attributes, matching Jekyll's behavior.
///
/// Jekyll's SEO tag uses `| escape` which maps to Ruby's `CGI.escapeHTML`.
/// This escapes `&`, `<`, `>`, and `"` but NOT `'` (single quotes).
/// However, in practice Jekyll's meta description output does NOT escape
/// double quotes either -- the description goes through `markdownify |
/// strip_html | strip_newlines | truncate` without an explicit `escape`
/// filter in the template. So quotes appear literally in the output.
///
/// We match Jekyll's actual output: only escape `&`, `<`, `>`.
fn html_escape_content(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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
        // Apply SmartyPants to titles (matching Jekyll's `| smartify` in jekyll-seo-tag)
        let page_title = get_nested_str(runtime, &["page", "title"]).map(|t| smartify(&t));

        // just-the-docs theme behavior: when page has nav_order set, strip leading
        // "N. " number prefix from the title (e.g., "1. Your First Actions" -> "Your First Actions").
        // This affects <title>, og:title, twitter:title, and JSON-LD headline.
        let has_nav_order = get_nested_str(runtime, &["page", "nav_order"]).is_some();
        let page_title = if has_nav_order {
            page_title.map(|t| strip_nav_order_prefix(&t))
        } else {
            page_title
        };

        let site_title = get_nested_str(runtime, &["site", "title"]).map(|t| smartify(&t));
        let site_tagline = get_nested_str(runtime, &["site", "tagline"]);
        let page_description = get_nested_str(runtime, &["page", "description"]);
        let page_excerpt = get_nested_str(runtime, &["page", "excerpt"]);
        let site_description = get_nested_str(runtime, &["site", "description"]);
        let page_content = get_nested_str(runtime, &["page", "content"])
            .or_else(|| get_nested_str(runtime, &["content"]));
        let site_url = get_nested_str(runtime, &["site", "url"]);
        let page_url = get_nested_str(runtime, &["page", "url"]);
        let page_image = get_nested_str(runtime, &["page", "image"]);
        let page_date = get_nested_str(runtime, &["page", "date"]);
        let page_lang = get_nested_str(runtime, &["page", "lang"]);
        let site_lang = get_nested_str(runtime, &["site", "lang"]);
        let site_locale = get_nested_str(runtime, &["site", "locale"]);
        let twitter_username = get_nested_str(runtime, &["site", "twitter", "username"]);
        let facebook_publisher = get_nested_str(runtime, &["site", "facebook", "publisher"]);
        let page_author = get_nested_str(runtime, &["page", "author"]);
        let site_author = get_nested_str(runtime, &["site", "author"]);
        let site_logo = get_nested_str(runtime, &["site", "logo"]);

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
        // Priority: page.lang > site.lang > site.locale > "en_US"
        // jekyll-seo-tag uses page.lang || site.lang for og:locale
        let locale = page_lang
            .as_deref()
            .or(site_lang.as_deref())
            .or(site_locale.as_deref())
            .unwrap_or("en_US");
        output.push_str(&format!(
            "<meta property=\"og:locale\" content=\"{}\" />\n",
            html_escape(locale)
        ));

        // 6. Description (both meta name="description" and og:description together)
        // Priority: page.description > page.excerpt > site.description > content snippet
        // jekyll-seo-tag also falls back to page content (stripped HTML, ~200 chars)
        let content_snippet =
            if page_description.is_none() && page_excerpt.is_none() && site_description.is_none() {
                page_content.as_deref().and_then(|c| {
                    let stripped = strip_html_tags(c);
                    let trimmed = stripped.trim().replace('\n', " ");
                    // Collapse multiple spaces
                    let mut prev_space = false;
                    let collapsed: String = trimmed
                        .chars()
                        .filter(|&ch| {
                            if ch == ' ' {
                                if prev_space {
                                    return false;
                                }
                                prev_space = true;
                            } else {
                                prev_space = false;
                            }
                            true
                        })
                        .collect();
                    if collapsed.is_empty() {
                        return None;
                    }
                    // Truncate to ~200 chars on a word boundary
                    if collapsed.len() > 200 {
                        let truncated = &collapsed[..200];
                        if let Some(last_space) = truncated.rfind(' ') {
                            Some(truncated[..last_space].to_string())
                        } else {
                            Some(truncated.to_string())
                        }
                    } else {
                        Some(collapsed)
                    }
                })
            } else {
                None
            };

        let raw_description = page_description
            .as_deref()
            .or(page_excerpt.as_deref())
            .or(site_description.as_deref())
            .or(content_snippet.as_deref());

        // Strip HTML tags from description (Jekyll's SEO tag always does this)
        let stripped_description = raw_description.map(strip_html_tags);
        let description = stripped_description.as_deref();

        if let Some(desc) = description {
            if !desc.is_empty() {
                output.push_str(&format!(
                    "<meta name=\"description\" content=\"{}\" />\n",
                    html_escape_content(desc)
                ));
                output.push_str(&format!(
                    "<meta property=\"og:description\" content=\"{}\" />\n",
                    html_escape_content(desc)
                ));
            }
        }

        // 7. Canonical URL + og:url (together)
        // Jekyll strips trailing `index.html` from canonical URLs:
        //   /index.html -> /
        //   /about/index.html -> /about/
        let canonical_url = match (&site_url, &page_url) {
            (Some(base), Some(path)) => {
                let base = base.trim_end_matches('/');
                let path = if path.starts_with('/') {
                    path.clone()
                } else {
                    format!("/{}", path)
                };
                // Strip trailing index.html (Jekyll behavior)
                let path = if path == "/index.html" {
                    "/".to_string()
                } else if let Some(prefix) = path.strip_suffix("index.html") {
                    prefix.to_string()
                } else {
                    path
                };
                Some(format!("{}{}", base, path))
            }
            (Some(base), None) => Some(base.trim_end_matches('/').to_string()),
            _ => None,
        };

        // Output canonical URL. When site_url is set, use the full absolute URL.
        // When site_url is empty/missing, fall back to just the page path (relative URL),
        // matching jekyll-seo-tag behavior which always outputs canonical/og:url.
        {
            let canonical = if let Some(ref url) = canonical_url {
                Some(url.clone())
            } else {
                // No site_url: use page_url directly as relative canonical
                page_url.as_ref().map(|p| {
                    if p.starts_with('/') {
                        p.clone()
                    } else {
                        format!("/{}", p)
                    }
                })
            };
            if let Some(ref url) = canonical {
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
        if let Some(ref date_str) = page_date {
            output.push_str("<meta property=\"og:type\" content=\"article\" />\n");
            // 10b. article:published_time (only for articles)
            let site_tz = crate::template::filters::get_site_timezone(runtime);
            let formatted_date =
                crate::template::filters::format_date_to_xmlschema(date_str, site_tz);
            output.push_str(&format!(
                "<meta property=\"article:published_time\" content=\"{}\" />\n",
                html_escape(&formatted_date)
            ));
            // 10c. article:publisher (only for articles when site.facebook.publisher is set)
            if let Some(ref publisher) = facebook_publisher {
                output.push_str(&format!(
                    "<meta property=\"article:publisher\" content=\"{}\" />\n",
                    html_escape(publisher)
                ));
            }
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

        // Build compact single-line JSON-LD matching Jekyll's jekyll-seo-tag output.
        // Field order: @context, @type, name, description, headline, url, author,
        // datePublished, image (only present fields are emitted).
        let mut jsonld_fields: Vec<String> = Vec::new();
        jsonld_fields.push("\"@context\":\"https://schema.org\"".to_string());
        jsonld_fields.push(format!("\"@type\":\"{}\"", schema_type));

        // name field: jekyll-seo-tag only includes name for homepage/about pages
        if is_homepage_or_about {
            if let Some(name) = site_title.as_deref() {
                jsonld_fields.push(format!("\"name\":\"{}\"", json_escape(&html_escape(name))));
            }
        }

        // description before headline (matching Jekyll's field order)
        if let Some(desc) = description {
            jsonld_fields.push(format!(
                "\"description\":\"{}\"",
                json_escape(&html_escape(desc))
            ));
        }

        if let Some(ref t) = og_page_title {
            // jekyll-seo-tag uses page_title (page title only, or site title fallback)
            // for the headline field, NOT the full "page | site" combined title.
            // headline is max 110 chars per Google guidelines
            let headline = if t.chars().count() > 110 {
                let end = t.char_indices().nth(110).map(|(i, _)| i).unwrap_or(t.len());
                &t[..end]
            } else {
                t
            };
            jsonld_fields.push(format!(
                "\"headline\":\"{}\"",
                json_escape(&html_escape(headline))
            ));
        }

        // url field: jekyll-seo-tag always includes canonical_url in JSON-LD
        let jsonld_url = canonical_url.as_deref().or(page_url.as_deref());
        if let Some(url) = jsonld_url {
            jsonld_fields.push(format!("\"url\":\"{}\"", json_escape(url)));
        }

        // Author in JSON-LD (compact nested object)
        if let Some(author_name) = author {
            jsonld_fields.push(format!(
                "\"author\":{{\"@type\":\"Person\",\"name\":\"{}\"}}",
                json_escape(author_name)
            ));
        }

        if let Some(ref date) = page_date {
            let site_tz = crate::template::filters::get_site_timezone(runtime);
            let formatted_date = crate::template::filters::format_date_to_xmlschema(date, site_tz);
            jsonld_fields.push(format!(
                "\"datePublished\":\"{}\"",
                json_escape(&formatted_date)
            ));
        }

        // mainEntityOfPage: only for BlogPosting (pages with date)
        if schema_type == "BlogPosting" {
            if let Some(ref url) = canonical_url {
                jsonld_fields.push(format!(
                    "\"mainEntityOfPage\":{{\"@type\":\"WebPage\",\"@id\":\"{}\"}}",
                    json_escape(url)
                ));
            }
        }

        if let Some(ref img) = page_image {
            let absolute_img = absolute_image_url(img, &site_url);
            jsonld_fields.push(format!("\"image\":\"{}\"", json_escape(&absolute_img)));
        }

        // publisher field: when site.logo is configured, include publisher organization
        // matching jekyll-seo-tag behavior
        if let Some(ref logo) = site_logo {
            let absolute_logo = absolute_image_url(logo, &site_url);
            jsonld_fields.push(format!(
                "\"publisher\":{{\"@type\":\"Organization\",\"logo\":{{\"@type\":\"ImageObject\",\"url\":\"{}\"}}}}",
                json_escape(&absolute_logo)
            ));
        }

        // Sort fields after @context and @type alphabetically (matching Jekyll's
        // jekyll-seo-tag output which uses alphabetical key ordering).
        if jsonld_fields.len() > 2 {
            jsonld_fields[2..].sort();
        }

        output.push_str("<script type=\"application/ld+json\">\n");
        output.push('{');
        output.push_str(&jsonld_fields.join(","));
        output.push_str("}\n");
        output.push_str("</script>\n");

        output.push_str("<!-- End Jekyll SEO tag -->\n");

        write!(writer, "{}", output)
            .map_err(|e| liquid_core::Error::with_msg(format!("seo tag write error: {}", e)))?;
        Ok(())
    }
}

/// Strip leading "N. " number prefix from a title string.
/// This matches just-the-docs theme behavior where pages with `nav_order` front matter
/// have their leading number prefix stripped for SEO purposes.
/// Pattern: one or more digits followed by a dot and a space.
fn strip_nav_order_prefix(title: &str) -> String {
    let bytes = title.as_bytes();
    let mut i = 0;
    // Skip leading digits
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Must have at least one digit, followed by ". "
    if i > 0 && title[i..].starts_with(". ") {
        title[i + 2..].to_string()
    } else {
        title.to_string()
    }
}

/// Strip HTML tags from a string, returning only text content.
/// Used to extract plain text from rendered HTML content for description snippets.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

/// Apply SmartyPants-style typography to a string.
///
/// Converts straight quotes and other ASCII typography to Unicode equivalents,
/// matching Jekyll's `| smartify` Liquid filter behavior:
/// - Straight double quotes `"..."` -> `\u{201C}...\u{201D}` (left/right double quotation marks)
/// - Straight apostrophe in contractions (e.g., `it's`) -> `\u{2019}` (right single quotation mark)
/// - `...` -> `\u{2026}` (horizontal ellipsis)
/// - `--` -> `\u{2014}` (em dash)
fn smartify(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        // Handle ellipsis: ... -> \u{2026}
        if ch == '.' && i + 2 < len && chars[i + 1] == '.' && chars[i + 2] == '.' {
            result.push('\u{2026}');
            i += 3;
            continue;
        }

        // Handle em dash: -- -> \u{2014}
        if ch == '-' && i + 1 < len && chars[i + 1] == '-' {
            result.push('\u{2014}');
            i += 2;
            continue;
        }

        // Handle double quotes
        if ch == '"' {
            // Opening quote: at start, after whitespace, or after opening paren/bracket
            let is_opening =
                i == 0 || matches!(chars[i - 1], ' ' | '\t' | '\n' | '\r' | '(' | '[' | '{');
            if is_opening {
                result.push('\u{201c}'); // left double quotation mark
            } else {
                result.push('\u{201d}'); // right double quotation mark
            }
            i += 1;
            continue;
        }

        // Handle single quotes / apostrophes
        if ch == '\'' {
            // Apostrophe in contractions: letter before and letter/s after
            let prev_is_letter = i > 0 && chars[i - 1].is_alphanumeric();
            let next_is_letter = i + 1 < len && chars[i + 1].is_alphabetic();
            if prev_is_letter && next_is_letter {
                result.push('\u{2019}'); // right single quotation mark (apostrophe)
            } else if i == 0 || matches!(chars[i - 1], ' ' | '\t' | '\n' | '\r' | '(' | '[' | '{') {
                result.push('\u{2018}'); // left single quotation mark
            } else {
                result.push('\u{2019}'); // right single quotation mark
            }
            i += 1;
            continue;
        }

        result.push(ch);
        i += 1;
    }

    result
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
        // Issue 301: meta description does NOT escape quotes (matching Jekyll)
        // Only & is escaped. Quotes appear literally.
        assert!(
            out.contains("<meta name=\"description\" content=\"Tom &amp; Jerry's \"show\"\" />")
        );
    }

    // ========================================================================
    // Issue 216: Meta content attributes use double quotes
    // ========================================================================

    #[test]
    fn test_issue216_meta_content_double_quotes_apostrophe() {
        // Meta content attributes use double quotes. Apostrophes are NOT escaped
        // (matching Jekyll's actual behavior per Issue 301).
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            Some("Nathan doesn't write tests"),
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
            out.contains("content=\"Nathan doesn't write tests\""),
            "Meta content should use double quotes with literal apostrophe. Got: {}",
            out
        );
        // Must NOT contain single-quoted attribute
        assert!(
            !out.contains("content='"),
            "Meta content must NOT use single quotes. Got: {}",
            out
        );
    }

    #[test]
    fn test_issue216_meta_content_unicode_with_apostrophe() {
        // Non-ASCII: German text with umlaut and apostrophe
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            Some("B\u{00fc}scher's Buchladen \u{00f6}ffnet um 9 Uhr"),
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
            out.contains("B\u{00fc}scher's Buchladen \u{00f6}ffnet um 9 Uhr"),
            "Unicode should pass through and apostrophe should be literal. Got: {}",
            out
        );
        assert!(
            out.contains("content=\""),
            "Meta content should use double-quoted attribute. Got: {}",
            out
        );
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
        assert!(out.contains("\"@type\":\"WebPage\""));
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
        assert!(out.contains("\"@type\":\"BlogPosting\""));
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
        assert!(out.contains("\"url\":\"https://example.com/about\""));
    }

    // ========================================================================
    // Issue 246: JSON-LD entity encoding
    // ========================================================================

    #[test]
    fn test_json_ld_headline_ampersand_entity_encoded() {
        // Issue 246: Jekyll HTML-entity-encodes ampersands in JSON-LD headline,
        // producing Q&amp;A not Q&A.
        let eng = engine();
        let ctx = make_context(
            Some("Q&A"),
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
        assert!(
            out.contains("\"headline\":\"Q&amp;A\""),
            "JSON-LD headline should HTML-entity-encode ampersand. Got: {}",
            out
        );
    }

    #[test]
    fn test_json_ld_headline_unicode_ampersand_entity_encoded() {
        // Issue 246: Test with Unicode content containing ampersand.
        let eng = engine();
        let ctx = make_context(
            Some("Ubersicht & Mehr"),
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
        assert!(
            out.contains("\"headline\":\"Ubersicht &amp; Mehr\""),
            "JSON-LD headline should HTML-entity-encode ampersand with Unicode content. Got: {}",
            out
        );
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
            out.contains("\"@type\":\"WebSite\""),
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
            out.contains("\"@type\":\"WebSite\""),
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
            out.contains("\"@type\":\"WebSite\""),
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
            out.contains("\"@type\":\"WebPage\""),
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
            out.contains("\"@type\":\"BlogPosting\""),
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
            out.contains("\"url\":\"https://example.com/about\""),
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
            out.contains("\"url\":\"/\""),
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
            out.contains("\"name\":\"My Site\""),
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
        // After smartify, the apostrophe in "Jerry's" becomes U+2019 (right single quotation mark)
        assert!(out.contains("<title>Tom &amp; Jerry\u{2019}s &lt;show&gt;</title>"));
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

    // ========================================================================
    // JSON-LD headline (issue #202)
    // ========================================================================

    #[test]
    fn test_jsonld_headline_is_page_title_not_full_title() {
        // jekyll-seo-tag delegates headline to page_title (page title alone),
        // NOT the full "page_title | site_title" combined title.
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            None,
            None,
            Some("https://example.com"),
            Some("/my-page.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        // headline should be "My Page" (page_title), not "My Page | My Site" (full_title)
        assert!(
            out.contains("\"headline\":\"My Page\""),
            "JSON-LD headline should be page title only, not full title. Got: {}",
            out
        );
        assert!(
            !out.contains("\"headline\":\"My Page | My Site\""),
            "JSON-LD headline must NOT include site title suffix"
        );
    }

    #[test]
    fn test_jsonld_headline_fallback_to_site_title() {
        // When no page title, headline should fall back to site_title
        let eng = engine();
        let ctx = make_context(
            None,
            Some("My Site"),
            None,
            None,
            Some("https://example.com"),
            Some("/"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("\"headline\":\"My Site\""),
            "JSON-LD headline should fall back to site title. Got: {}",
            out
        );
    }

    // ========================================================================
    // Issue 213: JSON-LD compact format and field order
    // ========================================================================

    #[test]
    fn test_jsonld_compact_single_line() {
        // JSON-LD should be compact single-line (no internal newlines)
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
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
        // Find JSON-LD block
        let ld_start = out
            .find("application/ld+json")
            .expect("should have json-ld");
        let after_tag = out[ld_start..].find('\n').unwrap() + ld_start + 1;
        let script_end = out[after_tag..].find('\n').unwrap() + after_tag;
        let json_line = &out[after_tag..script_end];
        // Should start with { and end with }
        assert!(
            json_line.starts_with('{') && json_line.ends_with('}'),
            "JSON-LD should be on a single line: {}",
            json_line
        );
        // Should not contain internal newlines
        assert!(
            !json_line[1..json_line.len() - 1].contains('\n'),
            "JSON-LD should have no internal newlines"
        );
    }

    #[test]
    fn test_jsonld_field_order_description_before_headline() {
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            Some("A description"),
            None,
            None,
            Some("/my-page.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        let desc_pos = out
            .find("\"description\"")
            .expect("should have description in JSON-LD");
        let headline_pos = out
            .find("\"headline\"")
            .expect("should have headline in JSON-LD");
        assert!(
            desc_pos < headline_pos,
            "description should come before headline in JSON-LD"
        );
    }

    #[test]
    fn test_jsonld_compact_special_chars() {
        // JSON-LD with special characters properly escaped in compact format
        let eng = engine();
        let ctx = make_context(
            Some("Tom & Jerry"),
            None,
            Some("Tom & Jerry's \"show\""),
            None,
            None,
            Some("/show.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        // Issue 246: JSON-LD values are now HTML-entity-encoded to match Jekyll.
        // & becomes &amp;, ' becomes &#39;, " becomes &quot; (then JSON-escaped to \")
        assert!(
            out.contains("\"description\":\"Tom &amp; Jerry&#39;s &quot;show&quot;\""),
            "Special chars should be HTML-entity-encoded in JSON-LD. Got: {}",
            out
        );
    }

    #[test]
    fn test_jsonld_compact_unicode() {
        // JSON-LD with Unicode (accented chars, CJK)
        let eng = engine();
        let ctx = make_context(
            Some("Un caf\u{00e9} au lait"),
            None,
            Some("\u{4F60}\u{597D}\u{4E16}\u{754C}"),
            None,
            None,
            Some("/cafe.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("\"headline\":\"Un caf\u{00e9} au lait\""),
            "Unicode headline should be preserved. Got: {}",
            out
        );
        assert!(
            out.contains("\"description\":\"\u{4F60}\u{597D}\u{4E16}\u{754C}\""),
            "CJK description should be preserved. Got: {}",
            out
        );
    }

    #[test]
    fn test_jsonld_homepage_name_position() {
        // For homepage, keys should be in alphabetical order after @context/@type.
        // With alphabetical ordering: description < headline < name < url
        let eng = engine();
        let ctx = make_context(
            Some("My Site"),
            Some("My Site"),
            Some("Site description"),
            None,
            None,
            Some("/"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        // Extract JSON-LD block to avoid matching og:site_name etc.
        let ld_start = out
            .find("application/ld+json")
            .expect("should have json-ld");
        let ld_end = out[ld_start..].find("</script>").unwrap() + ld_start;
        let jsonld = &out[ld_start..ld_end];
        let desc_pos = jsonld
            .find("\"description\"")
            .expect("should have description");
        let name_pos = jsonld.find("\"name\"").expect("homepage should have name");
        assert!(
            desc_pos < name_pos,
            "In alphabetical order, description should come before name. JSON-LD: {}",
            jsonld
        );
    }

    #[test]
    fn test_jsonld_article_date_published_position() {
        // With alphabetical ordering, datePublished should come before url
        let eng = engine();
        let ctx = make_context(
            Some("My Post"),
            Some("My Site"),
            Some("Post description"),
            None,
            Some("https://example.com"),
            Some("/posts/my-post.html"),
            None,
            Some("2024-01-15"),
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        let date_pos = out
            .find("\"datePublished\"")
            .expect("should have datePublished");
        let url_pos = out.find("\"url\"").expect("should have url");
        assert!(
            date_pos < url_pos,
            "datePublished should come before url in alphabetical order"
        );
    }

    #[test]
    fn test_jsonld_script_tag_format() {
        // <script> tag on its own line, JSON on next line, </script> on its own line
        let eng = engine();
        let ctx = make_context(
            Some("Test"),
            None,
            None,
            None,
            None,
            Some("/test.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<script type=\"application/ld+json\">\n{"),
            "Script opening tag should be on its own line, JSON on next. Got: {}",
            out
        );
        assert!(
            out.contains("}\n</script>"),
            "JSON closing brace and script tag should be on separate lines. Got: {}",
            out
        );
    }

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

    // ── Issue 300: Canonical URL should strip trailing index.html ──

    #[test]
    fn test_canonical_url_strips_index_html_for_homepage() {
        // Homepage canonical should be "/" not "/index.html"
        let eng = engine();
        let ctx = make_context(
            Some("Home"),
            Some("My Site"),
            None,
            None,
            Some("https://example.com"),
            Some("/index.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("href=\"https://example.com/\""),
            "Homepage canonical should strip index.html to '/'. Got: {}",
            out
        );
        assert!(
            !out.contains("href=\"https://example.com/index.html\""),
            "Should not contain /index.html in canonical URL. Got: {}",
            out
        );
    }

    #[test]
    fn test_canonical_url_strips_index_html_for_subdir() {
        // /about/index.html -> /about/
        let eng = engine();
        let ctx = make_context(
            Some("About"),
            Some("My Site"),
            None,
            None,
            Some("https://example.com"),
            Some("/about/index.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("href=\"https://example.com/about/\""),
            "Subdir canonical should strip index.html to '/about/'. Got: {}",
            out
        );
    }

    #[test]
    fn test_canonical_url_preserves_non_index_html() {
        // /posts/my-post.html should NOT be stripped
        let eng = engine();
        let ctx = make_context(
            Some("My Post"),
            Some("My Site"),
            None,
            None,
            Some("https://example.com"),
            Some("/posts/my-post.html"),
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("href=\"https://example.com/posts/my-post.html\""),
            "Non-index .html should be preserved. Got: {}",
            out
        );
    }

    // ========================================================================
    // Issue 195: SEO meta tag fixes
    // ========================================================================

    #[test]
    fn test_seo_description_from_page_content() {
        let eng = engine();
        let mut ctx = Object::new();
        let mut page = Object::new();
        let site = Object::new();
        page.insert("title".into(), Value::scalar("My Note".to_string()));
        page.insert(
            "content".into(),
            Value::scalar(
                "<p>Pretty sure my dad is the biggest winner here. Now his friends                  would have heard of the name of the company I work for.</p>"
                    .to_string(),
            ),
        );
        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("name=\"description\""),
            "Description meta tag should be present when page.content exists. Got: {}",
            out
        );
        assert!(
            out.contains("Pretty sure my dad is the biggest winner"),
            "Description should contain text from content. Got: {}",
            out
        );
    }

    #[test]
    fn test_seo_description_from_content_strips_html() {
        let eng = engine();
        let mut ctx = Object::new();
        let mut page = Object::new();
        let site = Object::new();
        page.insert(
            "content".into(),
            Value::scalar(
                "<p>Hello <strong>world</strong> from <a href=\"/\">here</a>.</p>".to_string(),
            ),
        );
        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("Hello world from here."),
            "Description should have HTML stripped. Got: {}",
            out
        );
    }

    #[test]
    fn test_seo_description_from_content_unicode() {
        let eng = engine();
        let mut ctx = Object::new();
        let mut page = Object::new();
        let site = Object::new();
        page.insert(
            "content".into(),
            Value::scalar(
                "<p>\u{1F382} 5 year hubberversary today! \u{1F389} In love with my team.</p>"
                    .to_string(),
            ),
        );
        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("hubberversary"),
            "Content text should be in description. Got: {}",
            out
        );
    }

    #[test]
    fn test_seo_description_truncated_to_snippet() {
        let eng = engine();
        let mut ctx = Object::new();
        let mut page = Object::new();
        let site = Object::new();
        let long_content = format!("<p>{}</p>", "abcde ".repeat(100));
        page.insert("content".into(), Value::scalar(long_content));
        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        if let Some(start) = out.find("name=\"description\" content=\"") {
            let after = &out[start + 27..];
            if let Some(end) = after.find('"') {
                let desc = &after[..end];
                assert!(
                    desc.len() <= 210,
                    "Description should be truncated to ~200 chars, got {} chars",
                    desc.len()
                );
            }
        }
    }

    #[test]
    fn test_seo_description_explicit_overrides_content() {
        let eng = engine();
        let mut ctx = Object::new();
        let mut page = Object::new();
        let site = Object::new();
        page.insert(
            "description".into(),
            Value::scalar("Explicit description".to_string()),
        );
        page.insert(
            "content".into(),
            Value::scalar("<p>Content text</p>".to_string()),
        );
        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("Explicit description"),
            "Explicit description should take priority over content. Got: {}",
            out
        );
        assert!(
            !out.contains("Content text"),
            "Content text should not appear when explicit description exists. Got: {}",
            out
        );
    }

    #[test]
    fn test_og_locale_from_page_lang() {
        let eng = engine();
        let mut ctx = Object::new();
        let mut page = Object::new();
        let site = Object::new();
        page.insert("lang".into(), Value::scalar("ar".to_string()));
        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<meta property=\"og:locale\" content=\"ar\" />"),
            "og:locale should use page.lang value. Got: {}",
            out
        );
    }

    #[test]
    fn test_og_locale_from_site_lang() {
        let eng = engine();
        let mut ctx = Object::new();
        let page = Object::new();
        let mut site = Object::new();
        site.insert("lang".into(), Value::scalar("fr".to_string()));
        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<meta property=\"og:locale\" content=\"fr\" />"),
            "og:locale should use site.lang when no page.lang. Got: {}",
            out
        );
    }

    #[test]
    fn test_og_locale_site_locale_still_works() {
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
            Some("ja_JP"),
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<meta property=\"og:locale\" content=\"ja_JP\" />"),
            "site.locale should still work. Got: {}",
            out
        );
    }

    #[test]
    fn test_og_locale_unicode_lang() {
        let eng = engine();
        let mut ctx = Object::new();
        let mut page = Object::new();
        let site = Object::new();
        page.insert("lang".into(), Value::scalar("zh-Hant".to_string()));
        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("og:locale\" content=\"zh-Hant\""),
            "og:locale should handle multi-part lang tags. Got: {}",
            out
        );
    }

    #[test]
    fn test_article_published_time_emitted_for_articles() {
        let eng = engine();
        let ctx = make_context(
            Some("My Post"),
            Some("My Site"),
            None,
            None,
            None,
            None,
            None,
            Some("2024-06-15"),
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("article:published_time"),
            "article:published_time should be emitted for articles. Got: {}",
            out
        );
    }

    #[test]
    fn test_article_published_time_not_emitted_for_website() {
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
            !out.contains("article:published_time"),
            "article:published_time should not appear for website type. Got: {}",
            out
        );
    }

    #[test]
    fn test_article_published_time_ordering() {
        let eng = engine();
        let ctx = make_context(
            Some("My Post"),
            Some("My Site"),
            None,
            None,
            Some("https://example.com"),
            Some("/post"),
            None,
            Some("2024-06-15"),
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        let pos_og_type = out.find("og:type").expect("og:type");
        let pos_published = out
            .find("article:published_time")
            .expect("article:published_time");
        let pos_twitter = out.find("twitter:card").expect("twitter:card");
        assert!(
            pos_og_type < pos_published,
            "article:published_time should appear after og:type"
        );
        assert!(
            pos_published < pos_twitter,
            "article:published_time should appear before twitter:card"
        );
    }

    // ========================================================================
    // Issue 226: RC2 -- mainEntityOfPage in JSON-LD
    // ========================================================================

    #[test]
    fn test_rc2_blogposting_includes_main_entity_of_page() {
        let eng = engine();
        let ctx = make_context(
            Some("My Post"),
            Some("My Site"),
            None,
            None,
            Some("https://choosealicense.com"),
            Some("/licenses/mit/"),
            None,
            Some("2024-01-15"),
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("\"mainEntityOfPage\":{\"@type\":\"WebPage\",\"@id\":\"https://choosealicense.com/licenses/mit/\"}"),
            "BlogPosting JSON-LD should include mainEntityOfPage with canonical URL. Got: {}",
            out
        );
    }

    #[test]
    fn test_rc2_webpage_no_main_entity_of_page() {
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            None,
            None,
            Some("https://example.com"),
            Some("/about/"),
            None,
            None, // no date = WebPage, not BlogPosting
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
            !jsonld_content.contains("mainEntityOfPage"),
            "WebPage JSON-LD should NOT include mainEntityOfPage. Got: {}",
            jsonld_content
        );
    }

    // ========================================================================
    // Issue 226: RC3 -- Strip HTML from descriptions
    // ========================================================================

    #[test]
    fn test_rc3_description_html_stripped_in_jsonld() {
        let eng = engine();
        let ctx = make_context(
            Some("BSD License"),
            None,
            Some("A variant of the <a href=\"/licenses/bsd-3-clause/\">BSD 3-Clause License</a> that does not grant patent rights."),
            None,
            None,
            None,
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
            jsonld_content.contains(
                "A variant of the BSD 3-Clause License that does not grant patent rights."
            ),
            "JSON-LD description should have HTML stripped. Got: {}",
            jsonld_content
        );
        assert!(
            !jsonld_content.contains("<a href"),
            "JSON-LD description should NOT contain HTML tags. Got: {}",
            jsonld_content
        );
    }

    #[test]
    fn test_rc3_description_html_stripped_in_meta_tags() {
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            Some("A variant of the <a href=\"/licenses/bsd-3-clause/\">BSD 3-Clause License</a> that does not grant patent rights."),
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
            out.contains("content=\"A variant of the BSD 3-Clause License that does not grant patent rights.\""),
            "Meta description should have HTML stripped. Got: {}",
            out
        );
        assert!(
            !out.contains("content=\"A variant of the <a"),
            "Meta description should NOT contain HTML tags. Got: {}",
            out
        );
    }

    // ========================================================================
    // Issue 226: RC4 -- HTML entity preservation
    // ========================================================================

    #[test]
    fn test_rc4_title_smartifies_straight_apostrophe() {
        // Jekyll's SEO tag applies SmartyPants (| smartify) to titles,
        // converting straight apostrophes to right single quotes (U+2019).
        let eng = engine();
        let ctx = make_context(
            Some("What's this about?"),
            Some("Choose a License"),
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
        // The title should contain U+2019 (RIGHT SINGLE QUOTATION MARK), not straight '
        assert!(
            out.contains("<title>What\u{2019}s this about? | Choose a License</title>"),
            "Title should have smartified apostrophe (U+2019). Got: {}",
            out
        );
    }

    #[test]
    fn test_rc4_headline_smartifies_straight_quotes_in_jsonld() {
        // Jekyll's SEO tag applies SmartyPants to the headline in JSON-LD,
        // converting straight double quotes to left/right double quotes.
        let eng = engine();
        let ctx = make_context(
            Some("BSD 2-Clause \"Simplified\" License"),
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
        let jsonld_start = out
            .find("application/ld+json")
            .expect("should have json-ld");
        let jsonld_block = &out[jsonld_start..];
        let script_end = jsonld_block
            .find("</script>")
            .expect("should have closing script tag");
        let jsonld_content = &jsonld_block[..script_end];
        // JSON-LD headline should have smart quotes (U+201C, U+201D)
        assert!(
            jsonld_content.contains("\u{201c}Simplified\u{201d}"),
            "JSON-LD headline should have smartified double quotes. Got: {}",
            jsonld_content
        );
    }

    // ========================================================================
    // Issue 226: RC5 -- Timezone-aware datePublished
    // ========================================================================

    #[test]
    fn test_rc5_date_published_uses_site_timezone() {
        let eng = engine();
        let mut ctx = make_context(
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
        // Add site.timezone
        if let Value::Object(ref mut site) = ctx["site"] {
            site.insert(
                "timezone".into(),
                Value::scalar("Europe/Berlin".to_string()),
            );
        }
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        // Extract datePublished from JSON-LD
        let jsonld_start = out
            .find("application/ld+json")
            .expect("should have json-ld");
        let jsonld_block = &out[jsonld_start..];
        let script_end = jsonld_block
            .find("</script>")
            .expect("should have closing script tag");
        let jsonld_content = &jsonld_block[..script_end];
        // Should NOT be +00:00 when site timezone is Europe/Berlin
        assert!(
            jsonld_content.contains("+01:00") || jsonld_content.contains("+02:00"),
            "datePublished should use Europe/Berlin timezone (CET=+01:00 or CEST=+02:00), not UTC. Got: {}",
            jsonld_content
        );
    }

    #[test]
    fn test_rc5_date_published_matches_article_published_time() {
        let eng = engine();
        let mut ctx = make_context(
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
        if let Value::Object(ref mut site) = ctx["site"] {
            site.insert(
                "timezone".into(),
                Value::scalar("America/New_York".to_string()),
            );
        }
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        // Extract article:published_time value
        let meta_marker = "article:published_time\" content=\"";
        let meta_start = out
            .find(meta_marker)
            .expect("should have article:published_time");
        let after = &out[meta_start + meta_marker.len()..];
        let meta_end = after.find('"').unwrap();
        let meta_value = &after[..meta_end];

        // Extract datePublished from JSON-LD
        let jsonld_marker = "\"datePublished\":\"";
        let jsonld_start = out.find(jsonld_marker).expect("should have datePublished");
        let after = &out[jsonld_start + jsonld_marker.len()..];
        let jsonld_end = after.find('"').unwrap();
        let jsonld_value = &after[..jsonld_end];

        assert_eq!(
            meta_value, jsonld_value,
            "datePublished and article:published_time should be identical"
        );
    }

    // ========================================================================
    // Issue 233: JSON-LD publisher field
    // ========================================================================

    #[test]
    fn test_jsonld_publisher_with_site_logo() {
        let eng = engine();
        let mut ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            None,
            Some("A description"),
            Some("https://example.com"),
            Some("/about"),
            None,
            Some("2024-01-15"),
            None,
            None,
        );
        // Add site.logo
        if let Some(Value::Object(ref mut site)) = ctx.get_mut("site") {
            site.insert("logo".into(), Value::scalar("/images/logo.png".to_string()));
        }
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("\"publisher\":{\"@type\":\"Organization\",\"logo\":{\"@type\":\"ImageObject\",\"url\":\"https://example.com/images/logo.png\"}}"),
            "Should contain publisher with logo in JSON-LD. Got:\n{}",
            out
        );
    }

    #[test]
    fn test_jsonld_no_publisher_without_site_logo() {
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            None,
            Some("A description"),
            Some("https://example.com"),
            Some("/about"),
            None,
            Some("2024-01-15"),
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            !out.contains("\"publisher\""),
            "Should NOT contain publisher when no site.logo. Got:\n{}",
            out
        );
    }

    // ========================================================================
    // Issue 245: SEO title strips leading N. prefix when nav_order is set
    // ========================================================================

    fn make_context_with_nav_order(
        page_title: &str,
        nav_order: Option<i64>,
        site_title: &str,
    ) -> Object {
        let mut ctx = Object::new();
        let mut page = Object::new();
        let mut site = Object::new();

        page.insert("title".into(), Value::scalar(page_title.to_string()));
        if let Some(n) = nav_order {
            page.insert("nav_order".into(), Value::scalar(n));
        }
        page.insert("url".into(), Value::scalar("/test/".to_string()));
        site.insert("title".into(), Value::scalar(site_title.to_string()));
        site.insert(
            "url".into(),
            Value::scalar("https://example.com".to_string()),
        );

        ctx.insert("page".into(), Value::Object(page));
        ctx.insert("site".into(), Value::Object(site));
        ctx
    }

    #[test]
    fn test_seo_title_strips_number_prefix_with_nav_order() {
        // When page has nav_order, leading "N. " should be stripped from title
        // Matching just-the-docs / Jekyll behavior
        let eng = engine();
        let ctx = make_context_with_nav_order("3. My Page Title", Some(2), "My Site");
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<title>My Page Title | My Site</title>"),
            "Title should strip '3. ' prefix when nav_order is set. Got: {}",
            out
        );
    }

    #[test]
    fn test_seo_title_no_strip_without_nav_order() {
        // When page has NO nav_order, leading "N. " should NOT be stripped
        let eng = engine();
        let ctx = make_context_with_nav_order("5. Numbered Title", None, "My Site");
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<title>5. Numbered Title | My Site</title>"),
            "Title should keep '5. ' prefix when nav_order is NOT set. Got: {}",
            out
        );
    }

    #[test]
    fn test_seo_title_no_strip_without_number_prefix() {
        // Regular title with nav_order set should not be modified
        let eng = engine();
        let ctx = make_context_with_nav_order("Regular Title", Some(1), "My Site");
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("<title>Regular Title | My Site</title>"),
            "Title without number prefix should be unchanged. Got: {}",
            out
        );
    }

    #[test]
    fn test_seo_og_title_strips_number_prefix_with_nav_order() {
        // og:title should also have the number prefix stripped
        let eng = engine();
        let ctx = make_context_with_nav_order("3. My Page Title", Some(2), "My Site");
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("og:title\" content=\"My Page Title\""),
            "og:title should strip '3. ' prefix when nav_order is set. Got: {}",
            out
        );
    }

    // ========================================================================
    // Issue 245: JSON-LD key ordering (alphabetical after @context, @type)
    // ========================================================================

    #[test]
    fn test_jsonld_keys_alphabetical_order() {
        // Jekyll's jekyll-seo-tag outputs JSON-LD keys in alphabetical order
        // (after @context and @type). Verify that our output does the same.
        let eng = engine();
        let ctx = make_context(
            Some("My Page"),
            Some("My Site"),
            Some("A test description"),
            None,
            Some("https://example.com"),
            Some("/my-page"),
            Some("/images/test.jpg"),
            Some("2024-01-15"),
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();

        // Extract JSON-LD block content (between the outermost { })
        let jsonld_start = out
            .find("application/ld+json")
            .expect("should have json-ld");
        let jsonld_block = &out[jsonld_start..];
        let brace_start = jsonld_block.find('{').expect("should have opening brace");
        // Find matching closing brace (track nesting)
        let inner = &jsonld_block[brace_start + 1..];
        let mut depth = 1;
        let mut end_pos = 0;
        for (i, ch) in inner.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end_pos = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        let jsonld_inner = &inner[..end_pos];

        // Extract top-level keys by splitting on comma at depth 0
        let mut keys: Vec<String> = Vec::new();
        let mut depth = 0i32;
        let mut field_start = 0;
        for (i, ch) in jsonld_inner.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                ',' if depth == 0 => {
                    let field = jsonld_inner[field_start..i].trim();
                    if let Some(key) = extract_json_key(field) {
                        keys.push(key);
                    }
                    field_start = i + 1;
                }
                _ => {}
            }
        }
        // Last field
        let last_field = jsonld_inner[field_start..].trim();
        if let Some(key) = extract_json_key(last_field) {
            keys.push(key);
        }

        // First two must be @context and @type
        assert_eq!(keys[0], "@context", "First key must be @context");
        assert_eq!(keys[1], "@type", "Second key must be @type");

        // Remaining keys should be in alphabetical order
        let rest = &keys[2..];
        for i in 1..rest.len() {
            assert!(
                rest[i - 1] <= rest[i],
                "JSON-LD keys should be in alphabetical order after @context/@type, \
                 but '{}' comes before '{}'. All keys: {:?}",
                rest[i - 1],
                rest[i],
                keys
            );
        }
    }

    /// Helper to extract a JSON key from a "key":value field string
    fn extract_json_key(field: &str) -> Option<String> {
        let field = field.trim();
        if field.starts_with('"') {
            let end = field[1..].find('"')?;
            Some(field[1..1 + end].to_string())
        } else {
            None
        }
    }

    // ========================================================================
    // Issue 301: Meta description should NOT escape quotes (matching Jekyll)
    // ========================================================================

    #[test]
    fn test_issue301_meta_description_no_quote_escaping() {
        // Jekyll's SEO tag does NOT escape " or ' in meta description content.
        // Only &, <, > should be escaped.
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            Some("A permissive license with an \"advertising clause\" that's useful"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        // Quotes should be literal, not &quot; or &#39;
        assert!(
            out.contains("\"advertising clause\""),
            "Meta description should have literal double quotes, not &quot;. Got: {}",
            out
        );
        assert!(
            out.contains("that's useful"),
            "Meta description should have literal apostrophe, not &#39;. Got: {}",
            out
        );
        // Check only the meta tag lines (not JSON-LD which uses different escaping)
        for line in out.lines() {
            if line.contains("meta") && line.contains("description") {
                assert!(
                    !line.contains("&quot;"),
                    "Meta description line should not contain &quot;: {}",
                    line
                );
                assert!(
                    !line.contains("&#39;"),
                    "Meta description line should not contain &#39;: {}",
                    line
                );
            }
        }
    }

    #[test]
    fn test_issue301_meta_description_ampersand_still_escaped() {
        // & should still be escaped in meta content
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            Some("Tom & Jerry's show"),
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
            out.contains("Tom &amp; Jerry's show"),
            "& should be escaped but quotes should not. Got: {}",
            out
        );
        // Check only meta lines for no &#39;
        for line in out.lines() {
            if line.contains("meta") && line.contains("description") {
                assert!(
                    !line.contains("&#39;"),
                    "Meta description should not escape apostrophe: {}",
                    line
                );
            }
        }
    }

    #[test]
    fn test_issue301_meta_description_unicode_no_escape() {
        // Non-ASCII characters with quotes should not be over-escaped
        let eng = engine();
        let ctx = make_context(
            None,
            None,
            Some("L'universit\u{00e9} de \"Montr\u{00e9}al\" est bien"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        // Check only meta tag lines (JSON-LD uses different escaping rules)
        for line in out.lines() {
            if line.contains("meta") && line.contains("description") {
                assert!(
                    line.contains("L'universit\u{00e9}"),
                    "Unicode and apostrophe should be literal in meta: {}",
                    line
                );
                assert!(
                    !line.contains("&#39;"),
                    "Meta description should not contain &#39; for apostrophes: {}",
                    line
                );
            }
        }
    }

    // ========================================================================
    // Issue 326: article:publisher meta tag
    // ========================================================================

    #[test]
    fn test_article_publisher_meta_tag_present() {
        // When site.facebook.publisher is set and page is an article (has date),
        // the article:publisher meta tag should be output
        let eng = engine();
        let mut ctx = make_context(
            Some("Test Post"),
            Some("Test Site"),
            None,
            None,
            Some("https://example.com"),
            Some("/post/"),
            None,
            Some("2024-01-15"),
            None,
            None,
        );
        // Add site.facebook.publisher
        if let Some(Value::Object(ref mut site)) = ctx.get_mut("site") {
            let mut facebook = Object::new();
            facebook.insert(
                "publisher".into(),
                Value::scalar("https://www.facebook.com/GitHub/"),
            );
            site.insert("facebook".into(), Value::Object(facebook));
        }
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("article:publisher"),
            "Should contain article:publisher meta tag. Got:\n{}",
            out
        );
        assert!(
            out.contains("https://www.facebook.com/GitHub/"),
            "Should contain the publisher URL. Got:\n{}",
            out
        );
    }

    #[test]
    fn test_article_publisher_meta_tag_absent_without_config() {
        // Without site.facebook.publisher, no article:publisher tag
        let eng = engine();
        let ctx = make_context(
            Some("Test Post"),
            Some("Test Site"),
            None,
            None,
            Some("https://example.com"),
            Some("/post/"),
            None,
            Some("2024-01-15"),
            None,
            None,
        );
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            !out.contains("article:publisher"),
            "Should NOT contain article:publisher without config. Got:\n{}",
            out
        );
    }

    #[test]
    fn test_article_publisher_meta_tag_unicode_url() {
        // Publisher URL with non-ASCII characters
        let eng = engine();
        let mut ctx = make_context(
            Some("Beitrag"),
            Some("Meine Seite"),
            None,
            None,
            Some("https://example.com"),
            Some("/beitrag/"),
            None,
            Some("2024-01-15"),
            None,
            None,
        );
        if let Some(Value::Object(ref mut site)) = ctx.get_mut("site") {
            let mut facebook = Object::new();
            facebook.insert(
                "publisher".into(),
                Value::scalar("https://www.facebook.com/M\u{00fc}nchen/"),
            );
            site.insert("facebook".into(), Value::Object(facebook));
        }
        let out = eng.parse_and_render("{% seo %}", &ctx).unwrap();
        assert!(
            out.contains("article:publisher"),
            "Should contain article:publisher. Got:\n{}",
            out
        );
        assert!(
            out.contains("M\u{00fc}nchen"),
            "Publisher URL should contain unicode. Got:\n{}",
            out
        );
    }
}
