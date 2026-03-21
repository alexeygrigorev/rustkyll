//! Custom `strip_html` filter matching Jekyll's behavior.
//!
//! Jekyll's `strip_html` uses regex to remove script blocks, style blocks,
//! comments, and HTML tags, then the result has trailing whitespace stripped.
//!
//! This implementation uses a simple state-machine approach (no regex dependency)
//! to match the same behavior, plus strips trailing whitespace from the result
//! to match observed Jekyll output in `strip_html | jsonify` and
//! `strip_html | truncate` pipelines.

use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "strip_html",
    description = "Removes any HTML tags from a string, matching Jekyll behavior.",
    parsed(StripHtmlFilter)
)]
pub struct StripHtml;

#[derive(Debug, Default, Display_filter)]
#[name = "strip_html"]
struct StripHtmlFilter;

/// Remove all content between `<script...>...</script>` (case-insensitive).
fn remove_script_blocks(input: &str) -> String {
    remove_paired_block(input, "script")
}

/// Remove all content between `<style...>...</style>` (case-insensitive).
fn remove_style_blocks(input: &str) -> String {
    remove_paired_block(input, "style")
}

/// Remove all content between `<tag...>...</tag>` (case-insensitive, dotall).
fn remove_paired_block(input: &str, tag: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut remaining = input;

    let open_pattern = format!("<{}", tag);

    while let Some(start) = find_case_insensitive(remaining, &open_pattern) {
        // Verify it's actually the start of a tag (next char is space, >, or end)
        let after_tag = start + open_pattern.len();
        if after_tag < remaining.len() {
            let next_char = remaining.as_bytes()[after_tag];
            if next_char != b' '
                && next_char != b'>'
                && next_char != b'\t'
                && next_char != b'\n'
                && next_char != b'\r'
            {
                result.push_str(&remaining[..after_tag]);
                remaining = &remaining[after_tag..];
                continue;
            }
        }

        result.push_str(&remaining[..start]);

        // Find the closing tag
        let close_pattern = format!("</{}>", tag);
        let search_from = &remaining[start..];
        if let Some(end) = find_case_insensitive(search_from, &close_pattern) {
            remaining = &search_from[end + close_pattern.len()..];
        } else {
            // No closing tag found, remove everything from start
            remaining = "";
        }
    }
    result.push_str(remaining);
    result
}

/// Find case-insensitive substring match position (byte offset).
/// The needle must be ASCII (which is true for HTML tag names).
fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let needle_bytes: Vec<u8> = needle.bytes().map(|b| b.to_ascii_lowercase()).collect();
    let haystack_bytes = haystack.as_bytes();
    let needle_len = needle_bytes.len();
    if needle_len > haystack_bytes.len() {
        return None;
    }
    for i in 0..=(haystack_bytes.len() - needle_len) {
        let mut matched = true;
        for j in 0..needle_len {
            if haystack_bytes[i + j].to_ascii_lowercase() != needle_bytes[j] {
                matched = false;
                break;
            }
        }
        if matched {
            return Some(i);
        }
    }
    None
}

/// Remove HTML comments `<!-- ... -->`.
fn remove_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut remaining = input;
    while let Some(start) = remaining.find("<!--") {
        result.push_str(&remaining[..start]);
        if let Some(end) = remaining[start..].find("-->") {
            remaining = &remaining[start + end + 3..];
        } else {
            remaining = "";
        }
    }
    result.push_str(remaining);
    result
}

/// Remove all HTML tags `<...>`.
fn remove_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' && in_tag {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

impl Filter for StripHtmlFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let input = input.to_kstr().into_string();

        // Match Jekyll's order: script blocks, style blocks, comments, then tags
        let result = remove_script_blocks(&input);
        let result = remove_style_blocks(&result);
        let result = remove_comments(&result);
        let result = remove_tags(&result);

        // Strip one trailing newline to match Jekyll's observed behavior.
        // Jekyll's rendered HTML typically ends with `</tag>\n`. After tag
        // removal, that final `\n` remains. Stripping exactly one trailing
        // newline matches the actual Jekyll output in `strip_html | jsonify`
        // and `strip_html | truncate` pipelines.
        let trimmed = result.strip_suffix('\n').unwrap_or(&result);
        Ok(Value::scalar(trimmed.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_basic() {
        assert_eq!(
            liquid_core::call_filter!(StripHtml, "<p>Hello</p>").unwrap(),
            liquid_core::value!("Hello")
        );
    }

    #[test]
    fn test_strip_html_trailing_newline() {
        assert_eq!(
            liquid_core::call_filter!(StripHtml, "<p>Hello world.</p>\n").unwrap(),
            liquid_core::value!("Hello world.")
        );
    }

    #[test]
    fn test_strip_html_script_removal() {
        assert_eq!(
            liquid_core::call_filter!(
                StripHtml,
                "<script type=\"text/javascript\">alert('Hi!');</script>"
            )
            .unwrap(),
            liquid_core::value!("")
        );
    }

    #[test]
    fn test_strip_html_script_case_insensitive() {
        assert_eq!(
            liquid_core::call_filter!(
                StripHtml,
                "<SCRIPT type=\"text/javascript\">alert('Hi!');</SCRIPT>"
            )
            .unwrap(),
            liquid_core::value!("")
        );
    }

    #[test]
    fn test_strip_html_style_removal() {
        assert_eq!(
            liquid_core::call_filter!(StripHtml, "<style type=\"text/css\">cool style</style>")
                .unwrap(),
            liquid_core::value!("")
        );
    }

    #[test]
    fn test_strip_html_comment_removal() {
        assert_eq!(
            liquid_core::call_filter!(StripHtml, "<!--\n\tcomment\n-->test").unwrap(),
            liquid_core::value!("test")
        );
    }

    #[test]
    fn test_strip_html_list_no_indentation() {
        assert_eq!(
            liquid_core::call_filter!(StripHtml, "<ul>\n<li>A</li>\n<li>B</li>\n</ul>\n").unwrap(),
            liquid_core::value!("\nA\nB\n")
        );
    }

    #[test]
    fn test_strip_html_details_summary() {
        assert_eq!(
            liquid_core::call_filter!(
                StripHtml,
                "<details><summary>CW</summary><p>content</p></details>"
            )
            .unwrap(),
            liquid_core::value!("CWcontent")
        );
    }

    #[test]
    fn test_strip_html_unicode() {
        assert_eq!(
            liquid_core::call_filter!(
                StripHtml,
                "<p>\u{6771}\u{4eac}\u{306e}<a href='#'>\u{30bf}\u{30ef}\u{30fc}</a></p>\n"
            )
            .unwrap(),
            liquid_core::value!("\u{6771}\u{4eac}\u{306e}\u{30bf}\u{30ef}\u{30fc}")
        );
    }

    #[test]
    fn test_strip_html_empty() {
        assert_eq!(
            liquid_core::call_filter!(StripHtml, "").unwrap(),
            liquid_core::value!("")
        );
    }

    #[test]
    fn test_strip_html_preserves_inner_newlines() {
        assert_eq!(
            liquid_core::call_filter!(StripHtml, "<p>One.</p>\n<p>Two.</p>\n").unwrap(),
            liquid_core::value!("One.\nTwo.")
        );
    }

    #[test]
    fn test_strip_html_multiline_tag() {
        assert_eq!(
            liquid_core::call_filter!(StripHtml, "<p\nclass='loooong'>test</p>").unwrap(),
            liquid_core::value!("test")
        );
    }

    #[test]
    fn test_strip_html_with_attributes() {
        assert_eq!(
            liquid_core::call_filter!(StripHtml, "<p id='xxx'>test</p>").unwrap(),
            liquid_core::value!("test")
        );
    }
}
