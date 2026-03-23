use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Remove trailing `/index.html` or `/index.htm` from a URL.
///
/// This matches Jekyll's built-in `strip_index` URL filter defined in
/// `lib/jekyll/filters/url_filters.rb`:
///
/// ```ruby
/// def strip_index(input)
///   return if input.nil? || input.to_s.empty?
///   input.sub(%r!/index\.html?$!, "/")
/// end
/// ```
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "strip_index",
    description = "Remove trailing /index.html from a URL.",
    parsed(StripIndexFilter)
)]
pub struct StripIndex;

#[derive(Debug, Default, Display_filter)]
#[name = "strip_index"]
struct StripIndexFilter;

impl Filter for StripIndexFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let s = input.to_kstr();
        if s.is_empty() {
            return Ok(Value::scalar("".to_string()));
        }
        // Match Jekyll: sub(%r!/index\.html?$!, "/")
        // This strips /index.html or /index.htm at the end of the string
        // Check /index.html first (longer suffix) before /index.htm
        let result = if let Some(prefix) = s.strip_suffix("/index.html") {
            format!("{prefix}/")
        } else if let Some(prefix) = s.strip_suffix("/index.htm") {
            format!("{prefix}/")
        } else {
            s.to_string()
        };
        Ok(Value::scalar(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_index_html() {
        let result =
            liquid_core::call_filter!(StripIndex, "https://example.com/about/index.html").unwrap();
        assert_eq!(result.to_kstr(), "https://example.com/about/");
    }

    #[test]
    fn test_strip_index_htm() {
        let result =
            liquid_core::call_filter!(StripIndex, "https://example.com/about/index.htm").unwrap();
        assert_eq!(result.to_kstr(), "https://example.com/about/");
    }

    #[test]
    fn test_no_index_unchanged() {
        let result = liquid_core::call_filter!(StripIndex, "https://example.com/").unwrap();
        assert_eq!(result.to_kstr(), "https://example.com/");
    }

    #[test]
    fn test_empty_input() {
        let result = liquid_core::call_filter!(StripIndex, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_root_index_html() {
        let result = liquid_core::call_filter!(StripIndex, "/index.html").unwrap();
        assert_eq!(result.to_kstr(), "/");
    }

    #[test]
    fn test_full_url_index_html() {
        let result =
            liquid_core::call_filter!(StripIndex, "https://example.com/index.html").unwrap();
        assert_eq!(result.to_kstr(), "https://example.com/");
    }

    #[test]
    fn test_index_html_not_at_end_unchanged() {
        let result =
            liquid_core::call_filter!(StripIndex, "https://example.com/index.html/page").unwrap();
        assert_eq!(result.to_kstr(), "https://example.com/index.html/page");
    }

    #[test]
    fn test_not_exactly_index_html_unchanged() {
        let result =
            liquid_core::call_filter!(StripIndex, "https://example.com/my-index.html").unwrap();
        assert_eq!(result.to_kstr(), "https://example.com/my-index.html");
    }

    #[test]
    fn test_case_sensitive_uppercase_unchanged() {
        // Jekyll is case-sensitive: INDEX.HTML should not be stripped
        let result =
            liquid_core::call_filter!(StripIndex, "https://example.com/INDEX.HTML").unwrap();
        assert_eq!(result.to_kstr(), "https://example.com/INDEX.HTML");
    }

    #[test]
    fn test_unicode_path() {
        let result =
            liquid_core::call_filter!(StripIndex, "https://example.com/caf\u{00e9}/index.html")
                .unwrap();
        assert_eq!(result.to_kstr(), "https://example.com/caf\u{00e9}/");
    }

    #[test]
    fn test_unicode_path_cyrillic() {
        let result = liquid_core::call_filter!(
            StripIndex,
            "https://example.com/\u{043f}\u{0443}\u{0442}\u{044c}/index.html"
        )
        .unwrap();
        assert_eq!(
            result.to_kstr(),
            "https://example.com/\u{043f}\u{0443}\u{0442}\u{044c}/"
        );
    }
}
