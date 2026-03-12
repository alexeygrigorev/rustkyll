use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Convert Markdown text to HTML.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "markdownify",
    description = "Convert Markdown text to HTML.",
    parsed(MarkdownifyFilter)
)]
pub struct Markdownify;

#[derive(Debug, Default, Display_filter)]
#[name = "markdownify"]
struct MarkdownifyFilter;

impl Filter for MarkdownifyFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let markdown = input.to_kstr();
        let html = crate::frontmatter::markdown_to_html(&markdown);
        Ok(Value::scalar(html))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bold() {
        let result = liquid_core::call_filter!(Markdownify, "**bold**").unwrap();
        let s = result.to_kstr().to_string();
        assert!(s.contains("<strong>bold</strong>"), "got: {s}");
    }

    #[test]
    fn test_italic() {
        let result = liquid_core::call_filter!(Markdownify, "*italic*").unwrap();
        let s = result.to_kstr().to_string();
        assert!(s.contains("<em>italic</em>"), "got: {s}");
    }

    #[test]
    fn test_link() {
        let result = liquid_core::call_filter!(Markdownify, "[text](url)").unwrap();
        let s = result.to_kstr().to_string();
        assert!(s.contains("<a href=\"url\">text</a>"), "got: {s}");
    }

    #[test]
    fn test_inline_code() {
        let result = liquid_core::call_filter!(Markdownify, "`code`").unwrap();
        let s = result.to_kstr().to_string();
        assert!(s.contains("<code>code</code>"), "got: {s}");
    }

    #[test]
    fn test_paragraph_wrapping() {
        let result = liquid_core::call_filter!(Markdownify, "hello").unwrap();
        assert_eq!(result.to_kstr(), "<p>hello</p>\n");
    }

    #[test]
    fn test_plain_text() {
        let result = liquid_core::call_filter!(Markdownify, "just text").unwrap();
        assert_eq!(result.to_kstr(), "<p>just text</p>\n");
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(Markdownify, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_already_html() {
        let result = liquid_core::call_filter!(Markdownify, "<div>already html</div>").unwrap();
        let s = result.to_kstr().to_string();
        assert!(s.contains("<div>already html</div>"), "got: {s}");
    }
}
