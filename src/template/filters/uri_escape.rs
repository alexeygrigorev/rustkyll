use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// URI-escape a string, matching Jekyll's `uri_escape` behavior.
///
/// Jekyll's `uri_escape` uses Ruby's `URI.escape` which percent-encodes
/// characters that are not valid in a URI component, but preserves
/// URI-safe characters like `/`, `:`, `?`, `#`, `[`, `]`, `@`, `!`,
/// `$`, `&`, `'`, `(`, `)`, `*`, `+`, `,`, `;`, `=`, `-`, `.`, `_`, `~`.
/// Spaces become `%20` (not `+`).
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "uri_escape",
    description = "URI-escape a string (spaces become %20, preserves URI-safe characters).",
    parsed(UriEscapeFilter)
)]
pub struct UriEscape;

#[derive(Debug, Default, Display_filter)]
#[name = "uri_escape"]
struct UriEscapeFilter;

/// Characters that Ruby's URI.escape considers safe (not encoded).
/// This is the union of unreserved + reserved characters from RFC 3986,
/// matching Ruby's URI::UNSAFE regexp inversion.
fn is_uri_safe(c: u8) -> bool {
    matches!(c,
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' |
        b'-' | b'_' | b'.' | b'~' |  // unreserved
        b':' | b'/' | b'?' | b'#' | b'[' | b']' | b'@' |  // gen-delims
        b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='  // sub-delims
    )
}

pub(crate) fn uri_escape_string(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for &byte in input.as_bytes() {
        if is_uri_safe(byte) {
            result.push(byte as char);
        } else {
            result.push_str(&format!("%{:02X}", byte));
        }
    }
    result
}

impl Filter for UriEscapeFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let text = input.to_kstr();
        Ok(Value::scalar(uri_escape_string(&text)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spaces_become_percent20() {
        let result = liquid_core::call_filter!(UriEscape, "foo bar").unwrap();
        assert_eq!(result.to_kstr().as_str(), "foo%20bar");
    }

    #[test]
    fn test_slashes_preserved() {
        let result = liquid_core::call_filter!(UriEscape, "/posts/my-post/").unwrap();
        assert_eq!(result.to_kstr().as_str(), "/posts/my-post/");
    }

    #[test]
    fn test_colon_preserved() {
        let result = liquid_core::call_filter!(UriEscape, "https://example.com").unwrap();
        assert_eq!(result.to_kstr().as_str(), "https://example.com");
    }

    #[test]
    fn test_unicode_encoded() {
        let result = liquid_core::call_filter!(UriEscape, "cafe\u{0301}").unwrap();
        // Each non-ASCII byte is percent-encoded
        assert_eq!(result.to_kstr().as_str(), "cafe%CC%81");
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(UriEscape, "").unwrap();
        assert_eq!(result.to_kstr().as_str(), "");
    }

    #[test]
    fn test_title_with_special_chars() {
        let result =
            liquid_core::call_filter!(UriEscape, "Getting Started - Chirpy Theme").unwrap();
        assert_eq!(
            result.to_kstr().as_str(),
            "Getting%20Started%20-%20Chirpy%20Theme"
        );
    }
}
