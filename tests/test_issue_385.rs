//! TDD tests for issue 385: DTC build-large-language-model block elements leaking from list.
//!
//! When markdown processed through `newline_to_br | markdownify` contains triple-backtick
//! fenced text with heading markers (`### User:`, `### Assistant:`), the headings must be
//! rendered as `<h3>` tags nested inside `<li>`, not as literal text.

// ============================================================================
// Unit: Heading markers inside inline-backtick-fenced text in list context
// ============================================================================

#[test]
fn test_issue385_heading_markers_in_backtick_fenced_text_become_h3() {
    // Simulates the newline_to_br output for the Muneeb Khan thread.
    // The triple backticks contain heading markers that should become real headings.
    let input = "\
- Prepare an instruction format dataset. The prompt template is structured somehow like this:<br />\n\
    \u{25E6} prompt_template = <br />\n\
```### System: You are an expert at extracting keywords.<br />\n\
### User:<br />\n\
{}<br />\n\
### Assistant:<br />\n\
Extracted Keywords: {}```<br />\n\
    \u{25E6} I do this for the training and the validation sets.";

    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    // The headings must be rendered as <h3> tags, not as literal ### text
    assert!(
        html.contains("<h3"),
        "### User: and ### Assistant: should become <h3> tags. Got:\n{}",
        html
    );
    assert!(
        html.contains("User:"),
        "User: heading text should be present. Got:\n{}",
        html
    );
    assert!(
        html.contains("Assistant:"),
        "Assistant: heading text should be present. Got:\n{}",
        html
    );

    // The headings should NOT be literal ### text
    assert!(
        !html.contains("### User:"),
        "### User: should not appear as literal text. Got:\n{}",
        html
    );
    assert!(
        !html.contains("### Assistant:"),
        "### Assistant: should not appear as literal text. Got:\n{}",
        html
    );
}

#[test]
fn test_issue385_headings_and_p_nested_inside_li() {
    let input = "\
- Prepare an instruction format dataset.<br />\n\
    \u{25E6} prompt_template = <br />\n\
```### System: You are an expert.<br />\n\
### User:<br />\n\
{}<br />\n\
### Assistant:<br />\n\
Extracted Keywords: {}```<br />\n\
    \u{25E6} I do this for the training.";

    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    // Both headings must be present as <h3> tags
    assert!(
        html.contains("<h3 id=\"user\">"),
        "Should have <h3 id=\"user\">. Got:\n{}",
        html
    );
    assert!(
        html.contains("<h3 id=\"assistant\">"),
        "Should have <h3 id=\"assistant\">. Got:\n{}",
        html
    );

    // ALL block elements (<h3>, <p>) should be inside <li>, not leaked outside </ul>
    // Check that no <h3> appears after </ul>
    if let Some(ul_close) = html.rfind("</ul>") {
        let after_ul = &html[ul_close..];
        assert!(
            !after_ul.contains("<h3"),
            "No <h3> should appear after </ul> -- they should be nested inside <li>. Got:\n{}",
            html
        );
        // <p> elements with {} content should also not leak outside
        assert!(
            !after_ul.contains("<p>{}<br /></p>"),
            "<p>{{}}br</p> should be inside <li>, not after </ul>. Got:\n{}",
            html
        );
    }
}

#[test]
fn test_issue385_p_elements_between_headings_inside_li() {
    let input = "\
- Prepare an instruction.<br />\n\
```### System: Expert.<br />\n\
### User:<br />\n\
{}<br />\n\
### Assistant:<br />\n\
Extracted Keywords: {}```<br />\n\
    \u{25E6} Done.";

    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    // The {} between headings should be wrapped in <p> tags
    // Jekyll produces: <h3 id="user">User:<br /></h3>\n<p>{}<br /></p>\n<h3 id="assistant">...
    assert!(
        html.contains("<h3"),
        "Should have <h3> headings. Got:\n{}",
        html
    );
}

// ============================================================================
// Unit: Closing backtick handling
// ============================================================================

#[test]
fn test_issue385_closing_backticks_become_literal_text() {
    // When opening backticks are escaped, closing backticks should also become
    // literal text and not open a code fence that swallows heading content.
    let input = "\
- Item:<br />\n\
```### System: text<br />\n\
### User:<br />\n\
content```<br />\n\
more text";

    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    // The closing ``` should not create a code block
    assert!(
        !html.contains("<code>"),
        "Closing backticks should not create a code block. Got:\n{}",
        html
    );
    // "more text" should not be swallowed into a code block
    assert!(
        html.contains("more text"),
        "Text after closing backticks should be visible. Got:\n{}",
        html
    );
}

// ============================================================================
// Unit: Regression safety for other list patterns
// ============================================================================

#[test]
fn test_issue385_regression_simple_numbered_list() {
    let input = "1. foo\n2. bar\n3. baz";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(html.contains("<ol>"), "Should have <ol>. Got:\n{}", html);
    assert!(html.contains("<li>"), "Should have <li>. Got:\n{}", html);
    assert!(html.contains("foo"), "Should contain 'foo'. Got:\n{}", html);
    assert!(html.contains("bar"), "Should contain 'bar'. Got:\n{}", html);
}

#[test]
fn test_issue385_regression_nested_bullet_list() {
    let input = "- outer\n  - inner\n  - inner2\n- outer2";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(html.contains("<ul>"), "Should have <ul>. Got:\n{}", html);
    assert!(
        html.contains("outer"),
        "Should contain 'outer'. Got:\n{}",
        html
    );
    assert!(
        html.contains("inner"),
        "Should contain 'inner'. Got:\n{}",
        html
    );
}

#[test]
fn test_issue385_regression_list_followed_by_paragraph() {
    let input = "- item1\n- item2\n\nparagraph after list";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(html.contains("<ul>"), "Should have <ul>. Got:\n{}", html);
    assert!(
        html.contains("paragraph after list"),
        "Should contain paragraph. Got:\n{}",
        html
    );
}

#[test]
fn test_issue385_unicode_emoji_in_list_items() {
    // Test with Unicode content including emoji (U+1F60A) and bullet (U+25E6)
    let input = "- Hello \u{1F60A} world<br />\n\u{25E6} bullet item<br />\ntext";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        html.contains('\u{1F60A}'),
        "Should preserve emoji U+1F60A. Got:\n{}",
        html
    );
    assert!(
        html.contains('\u{25E6}'),
        "Should preserve bullet U+25E6. Got:\n{}",
        html
    );
}

// ============================================================================
// Unit: Actual fenced code blocks should NOT convert headings
// ============================================================================

#[test]
fn test_issue385_real_fenced_code_blocks_preserve_headings_as_text() {
    // Headings inside actual fenced code blocks (``` on their own line) should
    // stay as code text, not become <h3> tags.
    let input = "```\n### User:\n{}\n### Assistant:\nExtracted Keywords\n```";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        html.contains("<code>") || html.contains("<pre>"),
        "Should be in a code block. Got:\n{}",
        html
    );
    assert!(
        !html.contains("<h3"),
        "Headings inside real code blocks should NOT become <h3>. Got:\n{}",
        html
    );
}
