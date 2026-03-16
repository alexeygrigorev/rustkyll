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
        let html = crate::frontmatter::markdown_to_html_for_filter(&markdown);
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
        assert!(
            s.contains("<code class=\"language-plaintext highlighter-rouge\">code</code>"),
            "got: {s}"
        );
    }

    #[test]
    fn test_paragraph_wrapping() {
        // markdownify filter uses lighter postprocessing: single trailing newline
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

    /// Test the newline_to_br | markdownify pipeline that book pages use.
    /// The input to markdownify already has <br />\n from newline_to_br.
    /// The output must preserve <br /> (XHTML-style) and not add extra
    /// blank lines beyond what pulldown-cmark produces.
    #[test]
    fn test_newline_to_br_then_markdownify_pipeline() {
        // Simulate newline_to_br output: newlines replaced with "<br />\n"
        let input = "First line.<br />\nSecond line.";
        let result = liquid_core::call_filter!(Markdownify, input).unwrap();
        let s = result.to_kstr().to_string();

        // Must preserve <br /> (XHTML-style self-closing tag)
        assert!(
            s.contains("<br />"),
            "Should preserve XHTML-style <br />. Got: {s}"
        );

        // Must wrap in <p> tags
        assert!(s.starts_with("<p>"), "Should start with <p>. Got: {s}");
        assert!(s.contains("</p>"), "Should contain </p>. Got: {s}");

        // Must have single trailing newline (not double)
        assert!(
            s.ends_with("</p>\n"),
            "Should end with </p>\\n (single newline). Got: {:?}",
            s
        );
        assert!(
            !s.ends_with("</p>\n\n"),
            "Should NOT end with double newline. Got: {:?}",
            s
        );
    }

    /// Issue #146: markdownify must not produce <ol start="N"> attributes.
    /// Book archive threads with numbered lists that don't start at 1 were
    /// getting start attributes that Jekyll/kramdown never produces.
    #[test]
    fn test_markdownify_no_ol_start_attribute() {
        // Markdown with a list that starts at a number other than 1
        let input = "2. Second item\n3. Third item\n";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        assert!(
            !html.contains("start="),
            "markdownify should not produce ol start attribute. Got: {html}"
        );
        assert!(
            html.contains("<ol>"),
            "Should have bare <ol> tag. Got: {html}"
        );
    }

    #[test]
    fn test_markdownify_preserves_br_self_closing_slash() {
        // When input contains <br /> from newline_to_br, markdownify must not strip the slash
        let input = "hello<br />\nworld";
        let result = liquid_core::call_filter!(Markdownify, input).unwrap();
        let s = result.to_kstr().to_string();
        assert!(
            s.contains("<br />"),
            "Must preserve <br /> self-closing slash. Got: {s}"
        );
        assert!(
            !s.contains("<br>"),
            "Must NOT have HTML5-style <br> (without slash). Got: {s}"
        );
    }
}
