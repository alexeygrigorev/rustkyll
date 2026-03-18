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
        // Jekyll adds class="language-plaintext highlighter-rouge" to inline code
        let result = liquid_core::call_filter!(Markdownify, "`code`").unwrap();
        let s = result.to_kstr().to_string();
        assert!(
            s.contains("<code class=\"language-plaintext highlighter-rouge\">code</code>"),
            "Inline code should have language-plaintext highlighter-rouge class. got: {s}"
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

    #[test]
    fn test_markdownify_kramdown_ial_target_blank() {
        // markdownify must process kramdown IAL {:target="_blank"} on links
        let input = r#"[Register](/slack.html){:target="_blank"}"#;
        let result = liquid_core::call_filter!(Markdownify, input).unwrap();
        let s = result.to_kstr().to_string();
        assert!(
            s.contains(r#"target="_blank""#),
            "markdownify should apply IAL target attribute. Got: {s}"
        );
        assert!(
            !s.contains("{:target"),
            "IAL syntax should be consumed. Got: {s}"
        );
    }

    #[test]
    fn test_markdownify_multi_paragraph_output() {
        let input = "First paragraph.\n\nSecond paragraph.";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        // Verify the output format so we know what strip/jsonify will get
        assert!(
            html.starts_with("<p>"),
            "Should start with <p>. Got: {:?}",
            html
        );
    }

    /// Issue 185: markdown_to_html must not modify content inside <script> blocks.
    /// JSON-LD embedded in pages contains HTML tag names like </p> in JSON strings.
    /// The postprocess step must not add newlines or modify content inside <script>.
    #[test]
    fn test_markdown_to_html_preserves_script_block_content() {
        let input = r#"Some text.

<script type="application/ld+json">
{
  "text": "<p>First.</p>\n<p>Second.</p>"
}
</script>
"#;
        let html = crate::frontmatter::markdown_to_html(input);
        // The JSON string inside <script> should be preserved exactly.
        // Specifically, the </p>\n<p> sequence should NOT have extra newlines
        // inserted by add_block_spacing.
        assert!(
            html.contains(r#""<p>First.</p>\n<p>Second.</p>""#),
            "Script block content should be preserved. Got:\n{}",
            html
        );
    }

    /// Issue 185: Full rendering pipeline (dedent + collapse + markdown_to_html)
    /// must preserve content inside <script> blocks.
    #[test]
    fn test_full_pipeline_preserves_script_block() {
        let input = r#"Some text.

<script type="application/ld+json">
{
  "text": "<p>First.</p>\n<p>Second.</p>"
}
</script>
"#;
        // Apply the same pipeline as render_markdown_page_with_site_overrides
        let dedented = crate::frontmatter::dedent_html_lines(input);
        let marked = crate::kramdown::mark_existing_html_headings(&dedented);
        let collapsed = crate::kramdown::collapse_blank_lines_in_html_blocks(&marked);
        let html = crate::frontmatter::markdown_to_html(&collapsed);
        let html = crate::kramdown::remove_heading_markers(&html);

        assert!(
            html.contains(r#""<p>First.</p>\n<p>Second.</p>""#),
            "Full pipeline should preserve script block content. Got:\n{}",
            html
        );
    }

    /// Issue 185: Full pipeline with indented JSON-LD content (as produced by
    /// the FAQ accordion include template with indented lines).
    #[test]
    fn test_full_pipeline_preserves_indented_script_block() {
        // Simulate the actual FAQ accordion include output, where the JSON-LD
        // content has indentation from the template structure
        let input = r#"## FAQ Section

<!-- FAQ Accordion Component -->
<div class="faq-accordion">
  <div class="faq-item">
    <button class="faq-question" type="button">
      <span>What is this?</span>
    </button>
    <div class="faq-answer">
      <div class="faq-answer-content">
        <p>First paragraph.</p>
<p>Second paragraph.</p>

      </div>
    </div>
  </div>
</div>

<!-- FAQ Schema Markup (JSON-LD) -->
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@type": "FAQPage",
  "mainEntity": [


    {
      "@type": "Question",
      "name": "What is this?",
      "acceptedAnswer": {
        "@type": "Answer",
        "text": "<p>First paragraph.</p>\n<p>Second paragraph.</p>"
      }
    }

  ]
}
</script>

<!-- Load accordion JavaScript -->
<script src="/assets/accordion.js"></script>
"#;
        // Apply the same pipeline as render_markdown_page_with_site_overrides
        let dedented = crate::frontmatter::dedent_html_lines(input);
        let marked = crate::kramdown::mark_existing_html_headings(&dedented);
        let collapsed = crate::kramdown::collapse_blank_lines_in_html_blocks(&marked);
        let html = crate::frontmatter::markdown_to_html(&collapsed);
        let html = crate::kramdown::remove_heading_markers(&html);

        // The JSON-LD text value should be preserved exactly
        assert!(
            html.contains(r#""<p>First paragraph.</p>\n<p>Second paragraph.</p>""#),
            "Full pipeline with indentation should preserve script block content. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_markdownify_kramdown_ial_inline_on_link() {
        // Inline IAL immediately after a link should apply the attribute.
        // This is the most common IAL use case in markdownify content
        // (e.g., book Q&A threads).
        let input = r#"Check [this link](https://example.com){:target="_blank"} out"#;
        let result = liquid_core::call_filter!(Markdownify, input).unwrap();
        let s = result.to_kstr().to_string();
        assert!(
            s.contains(r#"target="_blank""#),
            "Inline IAL on link should be applied. Got: {s}"
        );
        assert!(
            !s.contains("{:target"),
            "IAL syntax should be consumed. Got: {s}"
        );
    }

    // --- Issue 218: markdownify filter block spacing and list indentation ---

    #[test]
    fn test_issue218_markdownify_multi_paragraph_block_spacing() {
        let input = "First paragraph.\n\nSecond paragraph.";
        let result = liquid_core::call_filter!(Markdownify, input).unwrap();
        let s = result.to_kstr().to_string();
        assert_eq!(
            s, "<p>First paragraph.</p>\n\n<p>Second paragraph.</p>\n",
            "markdownify should produce double newline between paragraphs. Got: {:?}",
            s
        );
    }

    #[test]
    fn test_issue218_markdownify_ordered_list_indentation() {
        let input = "List:\n\n1. Alpha\n2. Beta\n";
        let result = liquid_core::call_filter!(Markdownify, input).unwrap();
        let s = result.to_kstr().to_string();
        assert!(
            s.contains("  <li>"),
            "markdownify should indent <li> by 2 spaces. Got: {:?}",
            s
        );
    }
}
