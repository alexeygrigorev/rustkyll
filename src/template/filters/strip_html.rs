use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Strip HTML tags from a string, matching Jekyll's behavior.
///
/// Jekyll's `strip_html` filter performs these steps (in order):
/// 1. Remove `<script>...</script>` blocks (multiline)
/// 2. Remove `<style>...</style>` blocks (multiline)
/// 3. Remove HTML comments `<!-- ... -->`
/// 4. Remove all remaining HTML tags via `gsub(/<.*?>/m, '')`
///
/// Trailing newlines are NOT stripped -- Jekyll's strip_html preserves them.
/// Templates that need newline removal use `strip_newlines` separately.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "strip_html",
    description = "Strip HTML tags from a string (Jekyll-compatible).",
    parsed(StripHtmlFilter)
)]
pub struct StripHtml;

#[derive(Debug, Default, Display_filter)]
#[name = "strip_html"]
struct StripHtmlFilter;

impl Filter for StripHtmlFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let text = input.to_kstr();
        let result = strip_html_jekyll(&text);
        Ok(Value::scalar(result))
    }
}

/// Perform Jekyll-compatible HTML stripping.
///
/// Implements the same sequence as Jekyll's Liquid `strip_html` filter:
/// 1. Remove script blocks
/// 2. Remove style blocks
/// 3. Remove HTML comments
/// 4. Remove all HTML tags
///
/// Jekyll's strip_html does NOT strip trailing newlines. Templates that need
/// newlines removed use `strip_newlines` as a separate filter step
/// (e.g., `content | strip_html | strip_newlines`). Templates that use
/// `content | strip_html | jsonify` (like podcast JSON-LD) expect the trailing
/// newline to be preserved and encoded as `\n` in the JSON string.
fn strip_html_jekyll(input: &str) -> String {
    use regex::Regex;

    // Use thread_local to avoid recompiling regexes on every call
    thread_local! {
        static RE_SCRIPT: Regex = Regex::new(r"(?si)<script.*?>.*?</script>").unwrap();
        static RE_STYLE: Regex = Regex::new(r"(?si)<style.*?>.*?</style>").unwrap();
        static RE_COMMENT: Regex = Regex::new(r"(?s)<!--.*?-->").unwrap();
        // Jekyll: gsub(/<.*?>/m, '') -- dot matches newline in multiline mode
        static RE_TAGS: Regex = Regex::new(r"(?s)<.*?>").unwrap();
    }

    let result = RE_SCRIPT.with(|re| re.replace_all(input, "").into_owned());
    let result = RE_STYLE.with(|re| re.replace_all(&result, "").into_owned());
    let result = RE_COMMENT.with(|re| re.replace_all(&result, "").into_owned());
    RE_TAGS.with(|re| re.replace_all(&result, "").into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tags() {
        let result = liquid_core::call_filter!(StripHtml, "<p>Hello <b>World</b></p>").unwrap();
        assert_eq!(result.to_kstr(), "Hello World");
    }

    #[test]
    fn test_trailing_newline_preserved() {
        // Jekyll's strip_html does NOT strip trailing newlines.
        // Podcast templates use `content | strip_html | jsonify` and expect
        // the trailing newline to be encoded as \n in the JSON string.
        let result = liquid_core::call_filter!(StripHtml, "<p>Hello World</p>\n").unwrap();
        assert_eq!(result.to_kstr(), "Hello World\n");
    }

    #[test]
    fn test_multiple_trailing_newlines_preserved() {
        // All trailing newlines are preserved (Jekyll doesn't strip any)
        let result = liquid_core::call_filter!(StripHtml, "<p>Hello</p>\n\n").unwrap();
        assert_eq!(result.to_kstr(), "Hello\n\n");
    }

    #[test]
    fn test_no_trailing_newline() {
        let result = liquid_core::call_filter!(StripHtml, "<p>Hello</p>").unwrap();
        assert_eq!(result.to_kstr(), "Hello");
    }

    #[test]
    fn test_script_blocks_removed() {
        let result =
            liquid_core::call_filter!(StripHtml, "before<script>alert('xss')</script>after")
                .unwrap();
        assert_eq!(result.to_kstr(), "beforeafter");
    }

    #[test]
    fn test_style_blocks_removed() {
        let result =
            liquid_core::call_filter!(StripHtml, "before<style>body{color:red}</style>after")
                .unwrap();
        assert_eq!(result.to_kstr(), "beforeafter");
    }

    #[test]
    fn test_html_comments_removed() {
        let result = liquid_core::call_filter!(StripHtml, "before<!-- comment -->after").unwrap();
        assert_eq!(result.to_kstr(), "beforeafter");
    }

    #[test]
    fn test_multiline_script() {
        let input = "text<script type=\"text/javascript\">\nvar x = 1;\n</script>more";
        let result = liquid_core::call_filter!(StripHtml, input).unwrap();
        assert_eq!(result.to_kstr(), "textmore");
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(StripHtml, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_no_html() {
        let result = liquid_core::call_filter!(StripHtml, "plain text").unwrap();
        assert_eq!(result.to_kstr(), "plain text");
    }

    #[test]
    fn test_nested_tags() {
        let result =
            liquid_core::call_filter!(StripHtml, "<div><p>Hello <a href=\"#\">link</a></p></div>")
                .unwrap();
        assert_eq!(result.to_kstr(), "Hello link");
    }

    #[test]
    fn test_self_closing_tags() {
        let result = liquid_core::call_filter!(StripHtml, "Hello<br/>World<hr/>End").unwrap();
        assert_eq!(result.to_kstr(), "HelloWorldEnd");
    }

    #[test]
    fn test_unicode_content() {
        let result =
            liquid_core::call_filter!(StripHtml, "<p>Привет <b>мир</b>! 你好世界 🌍</p>\n")
                .unwrap();
        assert_eq!(result.to_kstr(), "Привет мир! 你好世界 🌍\n");
    }

    #[test]
    fn test_author_description_pipeline() {
        // Simulates: content | strip_html (then strip_newlines would follow)
        // The trailing newline is preserved by strip_html; strip_newlines removes it.
        let input = "<p>John is a developer at DataTalks.Club</p>\n";
        let result = liquid_core::call_filter!(StripHtml, input).unwrap();
        assert_eq!(result.to_kstr(), "John is a developer at DataTalks.Club\n");
    }

    #[test]
    fn test_multiline_comment() {
        let input = "text<!--\nmultiline\ncomment\n-->more";
        let result = liquid_core::call_filter!(StripHtml, input).unwrap();
        assert_eq!(result.to_kstr(), "textmore");
    }

    #[test]
    fn test_attributes_stripped() {
        let result =
            liquid_core::call_filter!(StripHtml, "<p class=\"intro\" id=\"main\">Hello</p>")
                .unwrap();
        assert_eq!(result.to_kstr(), "Hello");
    }

    // ========================================================================
    // Issue 447: strip_html preserves HTML entities (matching Jekyll)
    // ========================================================================

    #[test]
    fn test_issue447_entities_preserved_after_strip() {
        // Jekyll's strip_html preserves HTML entities like &quot;, &amp; etc.
        // The entities are part of the text content and are not decoded.
        let input = "<p>White Noise</p>\n<details>&quot;I'm scheduled to die.&quot;</details>";
        let result = liquid_core::call_filter!(StripHtml, input).unwrap();
        assert_eq!(
            result.to_kstr(),
            "White Noise\n&quot;I'm scheduled to die.&quot;"
        );
    }
}
