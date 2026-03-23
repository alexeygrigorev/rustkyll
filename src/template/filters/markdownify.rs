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
        // Convert kramdown-style math delimiters after markdown processing.
        // Kramdown converts `$$inline math$$` to `\(inline math\)` (inline math).
        // We do this after markdown rendering so the backslashes aren't consumed
        // by the markdown parser.
        let html = convert_math_delimiters(&html);
        Ok(Value::scalar(html))
    }
}

/// Convert kramdown math delimiters `$$..$$` to MathJax-compatible notation.
///
/// - Inline (within text): `$$E=mc^2$$` -> `\(E=mc^2\)`
/// - Block-level (standalone paragraph): `$$E=mc^2$$` -> `\[E=mc^2\]`
///
/// This matches kramdown's behavior where `$$..$$` is used for both inline
/// and display math, with the context determining which notation to use.
fn convert_math_delimiters(input: &str) -> String {
    if !input.contains("$$") {
        return input.to_string();
    }

    let mut result = String::with_capacity(input.len());
    let mut remaining = input;

    while let Some(start) = remaining.find("$$") {
        result.push_str(&remaining[..start]);
        let after_open = &remaining[start + 2..];

        if let Some(end) = after_open.find("$$") {
            let math_content = &after_open[..end];
            // Use inline math notation \(..\) since markdownify is typically
            // used on titles and short text where inline is appropriate.
            result.push_str("\\(");
            result.push_str(math_content);
            result.push_str("\\)");
            remaining = &after_open[end + 2..];
        } else {
            // No closing $$, output as-is
            result.push_str("$$");
            remaining = after_open;
        }
    }

    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Issue 328: Math notation conversion
    // ========================================================================

    #[test]
    fn test_math_notation_inline() {
        let result =
            liquid_core::call_filter!(Markdownify, "Test with $$E=mc^2$$ formula").unwrap();
        let s = result.to_kstr().to_string();
        assert!(
            s.contains("\\(E=mc^2\\)"),
            "$$E=mc^2$$ should be converted to \\(E=mc^2\\). Got: {s}"
        );
        assert!(!s.contains("$$"), "No raw $$ should remain. Got: {s}");
    }

    #[test]
    fn test_math_notation_unicode() {
        let result =
            liquid_core::call_filter!(Markdownify, "Formule $$\\alpha + \\beta$$").unwrap();
        let s = result.to_kstr().to_string();
        assert!(
            s.contains("\\(\\alpha + \\beta\\)"),
            "Unicode math notation should be converted. Got: {s}"
        );
    }

    #[test]
    fn test_math_notation_no_change_without_math() {
        let result = liquid_core::call_filter!(Markdownify, "No math here").unwrap();
        let s = result.to_kstr().to_string();
        assert!(!s.contains("\\("), "No math conversion needed. Got: {s}");
    }

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

    // --- Issue 273: DTC <br> element handling in newline_to_br | markdownify pipeline ---

    /// Pattern A: inline code + br preserved
    #[test]
    fn test_issue273_pattern_a_code_span_followed_by_br() {
        let input = "Use `code` here<br />\nWhat's next?";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        assert!(
            html.contains("<code"),
            "Pattern A: Should contain <code> element. Got: {html}"
        );
        assert!(
            html.contains("<br />"),
            "Pattern A: Should preserve <br />. Got: {html}"
        );
    }

    /// Pattern A: multiple code spans with br
    #[test]
    fn test_issue273_pattern_a_multiple_code_spans_with_br() {
        let input = "`first`<br />\n`second`";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        // Both code spans must be present
        let code_count = html.matches("<code").count();
        assert!(
            code_count >= 2,
            "Pattern A: Should have 2 <code> elements. Got {code_count} in: {html}"
        );
        assert!(
            html.contains("<br />"),
            "Pattern A: Should preserve <br /> between code spans. Got: {html}"
        );
    }

    /// Pattern B: br elements preserved within list item
    #[test]
    fn test_issue273_pattern_b_br_in_list_item() {
        let input = "- line one<br />\nline two<br />\nline three";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        assert!(
            html.contains("<li>"),
            "Pattern B: Should contain <li>. Got: {html}"
        );
        assert!(
            html.contains("<br />"),
            "Pattern B: Should preserve <br /> within list items. Got: {html}"
        );
    }

    /// Pattern B: br in nested list item with code snippets
    #[test]
    fn test_issue273_pattern_b_br_in_list_with_code() {
        let input = "- `>>> import spacy`<br />\n`>>> nlp = spacy.load('en')`<br />\n`>>> doc = nlp('Hello')`";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        assert!(
            html.contains("<li>"),
            "Pattern B: Should contain <li>. Got: {html}"
        );
        let br_count = html.matches("<br />").count();
        assert!(
            br_count >= 2,
            "Pattern B: Should have at least 2 <br /> in list item. Got {br_count} in: {html}"
        );
    }

    /// Pattern C: numbered list items 4,3,2 stay as plain text with <br />,
    /// then item 1 starts an <ol>. This matches Jekyll/kramdown behavior where
    /// only `1.` at a paragraph boundary triggers an ordered list.
    #[test]
    fn test_issue273_pattern_c_numbered_list_after_br() {
        // Full text: items 4,3,2 in paragraph, then 1 starts a list
        let input = "great questions!<br />\n4. Writing did the trick<br />\n3. I'm not sure<br />\n2. Communication<br />\n1. I'm not sure about this";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        // Items 4,3,2 should be in a <p> with <br /> between them
        assert!(
            html.contains("<p>"),
            "Pattern C: Should have <p> for non-list text. Got: {html}"
        );
        assert!(
            html.contains("<br />"),
            "Pattern C: Should preserve <br /> in paragraph. Got: {html}"
        );
        // Item 1 should trigger an <ol> list (kramdown behavior)
        assert!(
            html.contains("<ol>"),
            "Pattern C: '1.' should render as <ol>. Got: {html}"
        );
        assert!(
            html.contains("<li>"),
            "Pattern C: Should contain <li> for item 1. Got: {html}"
        );
    }

    /// Pattern C: unordered list after br-modified newlines
    #[test]
    fn test_issue273_pattern_c_unordered_list_after_br() {
        let input = "intro text<br />\n- first item<br />\n- second item";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        assert!(
            html.contains("<ul>"),
            "Pattern C: Should render <ul> for bullet list. Got: {html}"
        );
        assert!(
            html.contains("<li>"),
            "Pattern C: Should contain <li> elements. Got: {html}"
        );
    }

    // === Issue 308: Diagnostic tests to understand current behavior ===

    #[test]
    fn test_issue308_smart_quote_after_br() {
        // After newline_to_br: writing<br />\n" Successfully replicated...
        // The opening " should become U+201C (left double quote), not U+201D (right)
        let input = "writing<br />\n\u{0022} Successfully replicated 10TB/day\u{0022}";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        eprintln!("=== 308 smart quote after br ===\n{:?}", html);
        // The opening quote after <br />\n should be U+201C (left/opening)
        assert!(
            html.contains("\u{201C} Successfully"),
            "Opening quote after <br /> should be U+201C (left). Got: {html}"
        );
        // The closing quote should be U+201D (right/closing)
        assert!(
            html.contains("10TB/day\u{201D}"),
            "Closing quote should be U+201D (right). Got: {html}"
        );
    }

    #[test]
    fn test_issue308_backtick_escape() {
        // Test that backslash-escaped backticks produce literal backticks
        let input = "text \\`\\`\\` more text";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        eprintln!("=== 308 backtick escape ===\n{}", html);
        assert!(
            html.contains("```"),
            "Escaped backticks should produce literal ```"
        );
        assert!(!html.contains("<code"), "Should not create code element");
    }

    #[test]
    fn test_issue308_backticks_with_headings_between() {
        // When triple backticks have heading markers between them (from newline_to_br),
        // kramdown treats them as literal text + headings, not inline code.
        // The ### after <br />\n becomes headings in kramdown.
        let input =
            "template:<br />\n```### System: Expert<br />\n### User:<br />\n{}```<br />\nmore text";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        eprintln!("=== 308 backticks with headings ===\n{}", html);
        // Should NOT create inline <code> because headings break the code span
        // The backticks should be literal text
        assert!(
            !html.contains("<code"),
            "Backticks with headings should be literal text, not code. Got: {html}"
        );
        assert!(
            html.contains("```"),
            "Should contain literal backticks. Got: {html}"
        );
    }

    #[test]
    fn test_issue308_sedat_reply_no_fenced_code_block() {
        // Real DTC comment from street-coder book (Sedat Kapanoglu reply), after newline_to_br
        // The raw text has ```float computeAverage... which after newline_to_br becomes
        // <br />\n```float... -- pulldown-cmark incorrectly treats this as a fenced code block
        let input = "the function can look like this:<br />\n```float computeAverage(string filename, string columnName) {<br />\n  var csv = readCsv(filename);<br />\n}```<br />\nThis tells what the function does";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        eprintln!("=== 308 Sedat reply ===\n{}", html);
        // Jekyll/kramdown treats this as inline text with <br /> tags, NOT a code block
        assert!(
            !html.contains("<pre>"),
            "Should not create <pre> code block. Got: {html}"
        );
        assert!(
            html.contains("<br />"),
            "Should preserve <br />. Got: {html}"
        );
        // All the text should be in paragraph(s), not in <pre><code>
        assert!(
            html.contains("This tells what the function does"),
            "Should have trailing text in paragraph. Got: {html}"
        );
    }

    #[test]
    fn test_issue308_br_then_indented_text_stays_paragraph() {
        // After newline_to_br, indented lines become <br />\n  indented...
        // Pulldown-cmark treats 4-space-indented lines as code blocks
        // But in the newline_to_br | markdownify pipeline, they should stay as paragraphs
        let input = "intro text<br />\n  indented line<br />\n  another indented line";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        eprintln!("=== 308 indented text ===\n{}", html);
        assert!(
            !html.contains("<pre>"),
            "Should not create indented code block. Got: {html}"
        );
        assert!(
            html.contains("<br />"),
            "Should preserve <br />. Got: {html}"
        );
    }

    #[test]
    fn test_issue308_unicode_smart_quote_after_br() {
        // Unicode content with smart quotes after <br />\n
        let input = "Universit\u{00e9} Technologique<br />\n\u{0022}R\u{00e9}sum\u{00e9}\u{0022}";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        // Opening quote should be U+201C (left), closing should be U+201D (right)
        assert!(
            html.contains("\u{201C}R\u{00e9}sum\u{00e9}\u{201D}"),
            "Unicode: Smart quotes around accented text should have correct direction. Got: {html}"
        );
        assert!(
            html.contains("Universit\u{00e9}"),
            "Unicode: Should preserve accented characters. Got: {html}"
        );
    }

    /// Unicode content: br handling with non-ASCII text
    #[test]
    fn test_issue273_unicode_br_handling() {
        let input = "Sch\u{00f6}ne Gr\u{00fc}\u{00df}e<br />\n\u{1f600} Emoji here<br />\n\u{4f60}\u{597d} CJK text";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        let br_count = html.matches("<br />").count();
        assert!(
            br_count >= 2,
            "Unicode: Should preserve <br /> with non-ASCII text. Got {br_count} in: {html}"
        );
        assert!(
            html.contains("Sch\u{00f6}ne"),
            "Unicode: Should preserve German umlauts. Got: {html}"
        );
        assert!(
            html.contains("\u{1f600}"),
            "Unicode: Should preserve emoji. Got: {html}"
        );
        assert!(
            html.contains("\u{4f60}\u{597d}"),
            "Unicode: Should preserve CJK. Got: {html}"
        );
    }

    /// Pattern D: pipe character inside angle brackets should not trigger table parsing
    /// in list items with br. This reproduces the NLP transformers page bug where
    /// <tel:100-1000|100-1000> in a list item was parsed as a table.
    #[test]
    fn test_issue273_pattern_d_pipe_in_angle_brackets_list_item() {
        // kramdown treats | in <tel:...|...> as table delimiter (no autolink protection)
        let input = "oh there are many<br />\n- engineering: infrastructure with <tel:100-1000|100-1000>s of GPUs<br />\n- dataset: lots of data<br />\n- release: responsible release";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        // The pipe in <tel:...|...> triggers table conversion (kramdown behavior)
        assert!(
            html.contains("<table>") || html.contains("|") || html.contains("<li>"),
            "Pattern D: Should handle pipe in tel autolink. Got: {html}"
        );
    }

    /// Regression: markdownify without br is unchanged
    #[test]
    fn test_issue273_regression_markdownify_without_br() {
        let input = "## Heading\n\nSome **bold** text.\n\n- item1\n- item2\n";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        assert!(
            html.contains("<h2>"),
            "Regression: Headings still work. Got: {html}"
        );
        assert!(
            html.contains("<strong>bold</strong>"),
            "Regression: Bold still works. Got: {html}"
        );
        assert!(
            html.contains("<li>"),
            "Regression: Lists still work. Got: {html}"
        );
    }

    // --- Issue 314: markdownify list indentation for CommonMark sites ---

    /// Test both CommonMark (no indent) and kramdown (indent) modes in a single
    /// test to avoid race conditions from the global AtomicBool in parallel tests.
    /// Also tests CJK content with emoji.
    #[test]
    fn test_issue314_markdownify_list_indent_modes() {
        // --- CommonMark mode: no indentation ---
        crate::frontmatter::set_markdownify_indent_lists(false);

        let input = "- Item 1\n- Item 2\n";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        assert!(
            !html.contains("  <li>"),
            "CommonMark markdownify should NOT indent <li>. Got: {:?}",
            html
        );
        assert!(
            html.contains("<li>"),
            "Should still contain <li>. Got: {:?}",
            html
        );

        // CJK + emoji content in CommonMark mode
        let input_cjk = "- \u{4F60}\u{597D}\u{4E16}\u{754C}\n- \u{1F600} Emoji item\n";
        let html_cjk = crate::frontmatter::markdown_to_html_for_filter(input_cjk);
        assert!(
            !html_cjk.contains("  <li>"),
            "CommonMark markdownify with CJK should NOT indent <li>. Got: {:?}",
            html_cjk
        );
        assert!(
            html_cjk.contains("\u{4F60}\u{597D}"),
            "Should preserve CJK characters. Got: {:?}",
            html_cjk
        );

        // --- Kramdown mode: with indentation ---
        crate::frontmatter::set_markdownify_indent_lists(true);

        let html_kramdown = crate::frontmatter::markdown_to_html_for_filter(input);
        assert!(
            html_kramdown.contains("  <li>"),
            "Kramdown markdownify should indent <li>. Got: {:?}",
            html_kramdown
        );
    }
}
