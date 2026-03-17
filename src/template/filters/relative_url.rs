use liquid_core::model::ScalarCow;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Prepend the site's baseurl to a path.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "relative_url",
    description = "Prepend the site baseurl to a path.",
    parsed(RelativeUrlFilter)
)]
pub struct RelativeUrl;

#[derive(Debug, Default, Display_filter)]
#[name = "relative_url"]
struct RelativeUrlFilter;

impl Filter for RelativeUrlFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let path = input.to_kstr();

        // Try to get site.baseurl from the runtime context
        let baseurl = runtime
            .try_get(&[ScalarCow::new("site"), ScalarCow::new("baseurl")])
            .and_then(|v| {
                let s = v.to_kstr().to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            });

        let result = match baseurl {
            Some(base) => {
                let base = base.trim_end_matches('/');
                if path.starts_with('/') {
                    format!("{base}{path}")
                } else {
                    format!("{base}/{path}")
                }
            }
            None => {
                // No baseurl set: ensure path starts with /
                if path.is_empty() || path.starts_with('/') {
                    path.to_string()
                } else {
                    format!("/{path}")
                }
            }
        };

        // Percent-encode spaces in the URL path, matching Jekyll behavior
        Ok(Value::scalar(encode_url_spaces(&result)))
    }
}

/// Percent-encode non-ASCII characters and spaces in a URL path, matching Jekyll's behavior.
///
/// Jekyll percent-encodes spaces as `%20` and non-ASCII characters (e.g., Cyrillic)
/// as their UTF-8 byte sequences (e.g., `ч` -> `%D1%87`). ASCII-safe URL characters
/// like `/`, `-`, `_`, `.`, `~` are preserved. Already percent-encoded sequences
/// (e.g., `%20`) are left untouched to prevent double-encoding.
pub(crate) fn encode_url_path(url: &str) -> String {
    let mut result = String::with_capacity(url.len());
    let bytes = url.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if b == b'%' && i + 2 < len && is_hex_digit(bytes[i + 1]) && is_hex_digit(bytes[i + 2]) {
            // Already percent-encoded sequence -- pass through
            result.push('%');
            result.push(bytes[i + 1] as char);
            result.push(bytes[i + 2] as char);
            i += 3;
        } else if b == b' ' {
            result.push_str("%20");
            i += 1;
        } else if b > 127 {
            // Non-ASCII byte: percent-encode each byte of the UTF-8 sequence
            // Find how many bytes this UTF-8 character uses
            let char_len = utf8_char_len(b);
            for j in 0..char_len {
                if i + j < len {
                    let byte = bytes[i + j];
                    result.push('%');
                    result.push(HEX_UPPER[(byte >> 4) as usize]);
                    result.push(HEX_UPPER[(byte & 0x0F) as usize]);
                }
            }
            i += char_len;
        } else {
            // ASCII character that's safe in URLs
            result.push(b as char);
            i += 1;
        }
    }

    result
}

/// Backward-compatible alias for `encode_url_path`.
pub(crate) fn encode_url_spaces(url: &str) -> String {
    encode_url_path(url)
}

/// Decode percent-encoded sequences in a URL path back to raw UTF-8.
///
/// Used by `url_to_output_path` to convert percent-encoded URLs to filesystem paths.
/// For example, `%D1%87%D0%B0%D1%81%D1%82%D1%8C` -> `часть`.
pub(crate) fn decode_url_path(url: &str) -> String {
    let mut result = Vec::with_capacity(url.len());
    let bytes = url.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'%'
            && i + 2 < len
            && is_hex_digit(bytes[i + 1])
            && is_hex_digit(bytes[i + 2])
        {
            let high = hex_val(bytes[i + 1]);
            let low = hex_val(bytes[i + 2]);
            result.push((high << 4) | low);
            i += 3;
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }

    String::from_utf8(result).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

const HEX_UPPER: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];

fn is_hex_digit(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

fn hex_val(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

fn utf8_char_len(first_byte: u8) -> usize {
    if first_byte & 0x80 == 0 {
        1
    } else if first_byte & 0xE0 == 0xC0 {
        2
    } else if first_byte & 0xF0 == 0xE0 {
        3
    } else if first_byte & 0xF8 == 0xF0 {
        4
    } else {
        1 // Invalid UTF-8 lead byte, treat as single byte
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_without_leading_slash_no_baseurl() {
        let result = liquid_core::call_filter!(RelativeUrl, "images/photo.jpg").unwrap();
        assert_eq!(result.to_kstr(), "/images/photo.jpg");
    }

    #[test]
    fn test_path_with_leading_slash_no_baseurl() {
        let result = liquid_core::call_filter!(RelativeUrl, "/images/photo.jpg").unwrap();
        assert_eq!(result.to_kstr(), "/images/photo.jpg");
    }

    #[test]
    fn test_empty_string_no_baseurl() {
        let result = liquid_core::call_filter!(RelativeUrl, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_spaces_encoded_as_percent20() {
        let result =
            liquid_core::call_filter!(RelativeUrl, "images/podcast/hybrid search.jpg").unwrap();
        assert_eq!(result.to_kstr(), "/images/podcast/hybrid%20search.jpg");
    }

    #[test]
    fn test_path_with_leading_slash_and_spaces_encoded() {
        let result =
            liquid_core::call_filter!(RelativeUrl, "/images/podcast/hybrid search.jpg").unwrap();
        assert_eq!(result.to_kstr(), "/images/podcast/hybrid%20search.jpg");
    }

    #[test]
    fn test_no_spaces_unchanged() {
        let result =
            liquid_core::call_filter!(RelativeUrl, "images/podcast/no-spaces.jpg").unwrap();
        assert_eq!(result.to_kstr(), "/images/podcast/no-spaces.jpg");
    }

    // ========================================================================
    // encode_url_path: Cyrillic and non-ASCII encoding
    // ========================================================================

    #[test]
    fn test_encode_url_path_cyrillic() {
        // Jekyll encodes Cyrillic characters as percent-encoded UTF-8 bytes
        let url = "/little-book-of-metals-ru/часть_1_история/";
        let encoded = encode_url_path(url);
        assert_eq!(
            encoded,
            "/little-book-of-metals-ru/%D1%87%D0%B0%D1%81%D1%82%D1%8C_1_%D0%B8%D1%81%D1%82%D0%BE%D1%80%D0%B8%D1%8F/"
        );
    }

    #[test]
    fn test_encode_url_path_ascii_unchanged() {
        let url = "/people/alexeygrigorev.html";
        assert_eq!(encode_url_path(url), url);
    }

    #[test]
    fn test_encode_url_path_spaces_and_cyrillic() {
        let url = "/книга/my page/";
        let encoded = encode_url_path(url);
        assert!(encoded.contains("%20"), "spaces should be encoded");
        assert!(!encoded.contains("книга"), "Cyrillic should be encoded");
        assert!(encoded.starts_with('/'));
        assert!(encoded.ends_with('/'));
    }

    #[test]
    fn test_encode_url_path_no_double_encode() {
        // Already encoded sequences should not be double-encoded
        let url = "/path/%D1%87%D0%B0%D1%81%D1%82%D1%8C/";
        let encoded = encode_url_path(url);
        assert_eq!(encoded, url);
    }

    #[test]
    fn test_encode_url_path_preserves_slashes_and_dots() {
        let url = "/blog/2021/пост.html";
        let encoded = encode_url_path(url);
        assert!(encoded.contains('/'));
        assert!(encoded.contains(".html"));
        assert!(!encoded.contains("пост"));
    }

    // ========================================================================
    // decode_url_path: round-trip decoding
    // ========================================================================

    #[test]
    fn test_decode_url_path_cyrillic() {
        let encoded = "/little-book-of-metals-ru/%D1%87%D0%B0%D1%81%D1%82%D1%8C_1_%D0%B8%D1%81%D1%82%D0%BE%D1%80%D0%B8%D1%8F/";
        let decoded = decode_url_path(encoded);
        assert_eq!(decoded, "/little-book-of-metals-ru/часть_1_история/");
    }

    #[test]
    fn test_decode_url_path_spaces() {
        let encoded = "/podcast/hybrid%20search.html";
        let decoded = decode_url_path(encoded);
        assert_eq!(decoded, "/podcast/hybrid search.html");
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = "/книга/часть_1/index.html";
        let encoded = encode_url_path(original);
        let decoded = decode_url_path(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_encode_url_path_mixed_case_hex_passthrough() {
        // Lowercase hex digits in existing encoding should pass through
        let url = "/path/%d1%87/";
        let encoded = encode_url_path(url);
        assert_eq!(encoded, url);
    }
}
