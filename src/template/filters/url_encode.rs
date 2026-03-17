use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// URL-encode a string, matching Jekyll's `url_encode` behavior.
///
/// Jekyll's `url_encode` uses Ruby's `CGI.escape` which encodes spaces as `+`.
/// This overrides the Shopify Liquid stdlib `url_encode` which uses `%20` for spaces.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "url_encode",
    description = "URL-encode a string (spaces become +, matching Jekyll/Ruby CGI.escape).",
    parsed(UrlEncodeFilter)
)]
pub struct UrlEncode;

#[derive(Debug, Default, Display_filter)]
#[name = "url_encode"]
struct UrlEncodeFilter;

impl Filter for UrlEncodeFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let text = input.to_kstr();
        // Reuse the same CGI.escape logic
        Ok(Value::scalar(super::cgi_escape::cgi_escape_string(&text)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spaces_become_plus() {
        // Jekyll's url_encode uses + for spaces, not %20
        let result = liquid_core::call_filter!(UrlEncode, "foo bar").unwrap();
        assert_eq!(result.to_kstr().as_str(), "foo+bar");
    }

    #[test]
    fn test_at_sign_encoded() {
        let result = liquid_core::call_filter!(UrlEncode, "foo+1@example.com").unwrap();
        assert_eq!(result.to_kstr().as_str(), "foo%2B1%40example.com");
    }

    #[test]
    fn test_colon_and_slashes_encoded() {
        let result = liquid_core::call_filter!(UrlEncode, "https://example.com/path").unwrap();
        assert_eq!(
            result.to_kstr().as_str(),
            "https%3A%2F%2Fexample.com%2Fpath"
        );
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(UrlEncode, "").unwrap();
        assert_eq!(result.to_kstr().as_str(), "");
    }

    #[test]
    fn test_safe_chars_passthrough() {
        let result = liquid_core::call_filter!(UrlEncode, "abc-123_test.html").unwrap();
        assert_eq!(result.to_kstr().as_str(), "abc-123_test.html");
    }
}
