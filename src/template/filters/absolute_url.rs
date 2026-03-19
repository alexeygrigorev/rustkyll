use liquid_core::model::ScalarCow;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

use super::relative_url::encode_url_spaces;

/// Prepend the site's url and baseurl to a path, producing an absolute URL.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "absolute_url",
    description = "Prepend the site url and baseurl to a path.",
    parsed(AbsoluteUrlFilter)
)]
pub struct AbsoluteUrl;

#[derive(Debug, Default, Display_filter)]
#[name = "absolute_url"]
struct AbsoluteUrlFilter;

impl Filter for AbsoluteUrlFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let path = input.to_kstr().to_string();

        // If the input already starts with a protocol scheme, return it unchanged.
        // This matches Jekyll's behavior where absolute_url is a no-op for
        // already-absolute URLs (e.g., external nav links).
        if path.starts_with("http://") || path.starts_with("https://") {
            return Ok(Value::scalar(path));
        }

        // Get site.url from context
        let site_url = runtime
            .try_get(&[ScalarCow::new("site"), ScalarCow::new("url")])
            .map(|v| v.to_kstr().to_string())
            .unwrap_or_default();

        // Get site.baseurl from context
        let baseurl = runtime
            .try_get(&[ScalarCow::new("site"), ScalarCow::new("baseurl")])
            .map(|v| v.to_kstr().to_string())
            .unwrap_or_default();

        // Strip trailing slashes from url and baseurl
        let site_url = site_url.trim_end_matches('/');
        let baseurl = baseurl.trim_end_matches('/');

        // Ensure path has a leading slash (unless empty)
        let path = if !path.is_empty() && !path.starts_with('/') {
            format!("/{path}")
        } else {
            path
        };

        // Build the result: url + baseurl + path
        // Avoid double slashes between components
        let result = if site_url.is_empty() && baseurl.is_empty() {
            path
        } else if baseurl.is_empty() {
            if path.is_empty() {
                site_url.to_string()
            } else {
                format!("{site_url}{path}")
            }
        } else if site_url.is_empty() {
            if path.is_empty() {
                baseurl.to_string()
            } else {
                format!("{baseurl}{path}")
            }
        } else if path.is_empty() {
            format!("{site_url}{baseurl}")
        } else {
            format!("{site_url}{baseurl}{path}")
        };

        // Percent-encode spaces in the URL, matching Jekyll behavior
        Ok(Value::scalar(encode_url_spaces(&result)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use liquid::Object;
    use liquid_core::Value;

    use crate::template::TemplateEngine;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    fn make_ctx(site_url: &str) -> Object {
        let mut ctx = Object::new();
        let mut site = Object::new();
        site.insert("url".into(), Value::scalar(site_url.to_string()));
        ctx.insert("site".into(), Value::Object(site));
        ctx
    }

    #[test]
    fn test_no_context_returns_path() {
        let result = liquid_core::call_filter!(AbsoluteUrl, "/about.html").unwrap();
        assert_eq!(result.to_kstr(), "/about.html");
    }

    #[test]
    fn test_spaces_encoded_as_percent20() {
        let result =
            liquid_core::call_filter!(AbsoluteUrl, "/images/podcast/hybrid search.jpg").unwrap();
        assert_eq!(result.to_kstr(), "/images/podcast/hybrid%20search.jpg");
    }

    #[test]
    fn test_absolute_url_skips_https_input() {
        // When input already starts with https://, absolute_url should return it unchanged
        // (matching Jekyll behavior). Currently this produces doubled URLs.
        let eng = engine();
        let ctx = make_ctx("https://mysite.com");
        let out = eng
            .parse_and_render("{{ 'https://example.com/path' | absolute_url }}", &ctx)
            .unwrap();
        assert_eq!(
            out.trim(),
            "https://example.com/path",
            "absolute_url should not prepend site.url to already-absolute https URLs"
        );
    }

    #[test]
    fn test_absolute_url_skips_http_input() {
        // Same for http:// URLs
        let eng = engine();
        let ctx = make_ctx("https://mysite.com");
        let out = eng
            .parse_and_render("{{ 'http://other.com/' | absolute_url }}", &ctx)
            .unwrap();
        assert_eq!(
            out.trim(),
            "http://other.com/",
            "absolute_url should not prepend site.url to already-absolute http URLs"
        );
    }

    #[test]
    fn test_absolute_url_still_prepends_for_relative_paths() {
        // Relative paths should still get site.url prepended (regression test)
        let eng = engine();
        let ctx = make_ctx("https://mysite.com");
        let out = eng
            .parse_and_render("{{ '/relative/path' | absolute_url }}", &ctx)
            .unwrap();
        assert_eq!(
            out.trim(),
            "https://mysite.com/relative/path",
            "absolute_url should prepend site.url to relative paths"
        );
    }
}
