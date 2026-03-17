use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// CGI-escape a string, matching Ruby's `CGI.escape` behavior.
///
/// Encodes spaces as `+` and all non-alphanumeric characters (except `-`, `.`, `_`)
/// as percent-encoded sequences. This matches Jekyll's `cgi_escape` filter.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "cgi_escape",
    description = "CGI-escape a string (spaces become +, matching Ruby CGI.escape).",
    parsed(CgiEscapeFilter)
)]
pub struct CgiEscape;

#[derive(Debug, Default, Display_filter)]
#[name = "cgi_escape"]
struct CgiEscapeFilter;

impl Filter for CgiEscapeFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let text = input.to_kstr();
        Ok(Value::scalar(cgi_escape_string(&text)))
    }
}

/// Encode a string using Ruby's `CGI.escape` rules:
/// - Alphanumerics and `-`, `.`, `_` are left as-is
/// - Space becomes `+`
/// - Everything else is percent-encoded (uppercase hex)
pub(crate) fn cgi_escape_string(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' => {
                result.push(byte as char);
            }
            b' ' => {
                result.push('+');
            }
            _ => {
                result.push('%');
                result.push(to_hex_upper(byte >> 4));
                result.push(to_hex_upper(byte & 0x0F));
            }
        }
    }
    result
}

fn to_hex_upper(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'A' + nibble - 10) as char,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spaces_become_plus() {
        let result = liquid_core::call_filter!(CgiEscape, "foo bar").unwrap();
        assert_eq!(result.to_kstr().as_str(), "foo+bar");
    }

    #[test]
    fn test_at_sign_encoded() {
        let result = liquid_core::call_filter!(CgiEscape, "@Al_Grigor").unwrap();
        assert_eq!(result.to_kstr().as_str(), "%40Al_Grigor");
    }

    #[test]
    fn test_colon_and_slashes_encoded() {
        let result = liquid_core::call_filter!(CgiEscape, "https://example.com").unwrap();
        assert_eq!(result.to_kstr().as_str(), "https%3A%2F%2Fexample.com");
    }

    #[test]
    fn test_twitter_share_text() {
        // Matches the example from the issue
        let input = "Creating an AWS Account by @Al_Grigor https://example.com";
        let result = liquid_core::call_filter!(CgiEscape, input).unwrap();
        assert_eq!(
            result.to_kstr().as_str(),
            "Creating+an+AWS+Account+by+%40Al_Grigor+https%3A%2F%2Fexample.com"
        );
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(CgiEscape, "").unwrap();
        assert_eq!(result.to_kstr().as_str(), "");
    }

    #[test]
    fn test_alphanumeric_passthrough() {
        let result = liquid_core::call_filter!(CgiEscape, "abc123").unwrap();
        assert_eq!(result.to_kstr().as_str(), "abc123");
    }

    #[test]
    fn test_safe_chars_passthrough() {
        let result = liquid_core::call_filter!(CgiEscape, "a-b.c_d").unwrap();
        assert_eq!(result.to_kstr().as_str(), "a-b.c_d");
    }

    #[test]
    fn test_special_chars_encoded() {
        let result = liquid_core::call_filter!(CgiEscape, "a&b=c").unwrap();
        assert_eq!(result.to_kstr().as_str(), "a%26b%3Dc");
    }

    #[test]
    fn test_multibyte_utf8() {
        // Ruby CGI.escape encodes each byte of UTF-8 separately
        let result = liquid_core::call_filter!(CgiEscape, "cafe\u{0301}").unwrap();
        // e followed by combining acute accent (U+0301 = 0xCC 0x81)
        assert_eq!(result.to_kstr().as_str(), "cafe%CC%81");
    }
}
