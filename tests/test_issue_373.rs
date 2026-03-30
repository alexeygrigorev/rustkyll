//! Tests for issue 373: DTC block elements leaking outside list containers.
//!
//! When `markdownify` produces block-level elements (like `<h3>` from `###`)
//! inside a list item context, pulldown-cmark can terminate the list prematurely.
//! These tests verify that `<h3>` and `<p>` elements stay nested inside `<li>`,
//! matching kramdown's behavior.
//!
//! The underlying fix was implemented in issue #385 via `renest_heading_after_list`.
//! These tests provide additional regression coverage for the specific scenarios
//! described in issue #373.

// ============================================================================
// Unit: Heading inside list item via markdownify
// ============================================================================

#[test]
fn test_issue373_heading_inside_list_item_stays_nested() {
    // Markdown text containing a bullet list where a list item continuation
    // includes ### Heading -- pass through markdown_to_html_for_filter.
    // Verify the resulting HTML has <h3> nested inside <li>, not leaked
    // as a sibling after </ul>.
    let input = "\
- Start of list item<br />\n\
### Important heading<br />\n\
continuation text";

    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    // The heading should become an <h3> tag
    assert!(
        html.contains("<h3"),
        "### Important heading should become <h3>. Got:\n{}",
        html
    );

    // The heading should be inside <li>, not leaked after </ul>
    if let Some(ul_close_pos) = html.rfind("</ul>") {
        let after_ul = &html[ul_close_pos..];
        assert!(
            !after_ul.contains("<h3"),
            "<h3> should not appear after </ul>. Got:\n{}",
            html
        );
    }
}

#[test]
fn test_issue373_multiple_headings_and_p_inside_list_item() {
    // Markdown text with ### User:\n{}\n### Assistant:\nExtracted Keywords
    // inside a list item -- verify both <h3> tags and intervening <p> remain
    // inside <li>.
    let input = "\
- Prepare an instruction format dataset.<br />\n\
```### System: You are an expert.<br />\n\
### User:<br />\n\
{}<br />\n\
### Assistant:<br />\n\
Extracted Keywords: {}```<br />\n\
continuation text";

    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    // Both headings should be <h3> tags
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

    // All block elements should be inside <li>, not leaked after </ul>
    if let Some(ul_close_pos) = html.rfind("</ul>") {
        let after_ul = &html[ul_close_pos..];
        assert!(
            !after_ul.contains("<h3"),
            "No <h3> should leak after </ul>. Got:\n{}",
            html
        );
        assert!(
            !after_ul.contains("<p>{}<br /></p>"),
            "<p> with {{}} should not leak after </ul>. Got:\n{}",
            html
        );
    }
}

#[test]
fn test_issue373_heading_in_code_block_not_converted() {
    // Markdown text with a code block (triple-backtick) containing ###
    // Verify the ### inside code is NOT converted to <h3> (it should remain
    // as code).
    let input = "```\n### User:\n{}\n### Assistant:\nExtracted Keywords\n```";

    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    assert!(
        html.contains("<code>") || html.contains("<pre>"),
        "Should produce a code block. Got:\n{}",
        html
    );
    assert!(
        !html.contains("<h3"),
        "### inside code block should NOT become <h3>. Got:\n{}",
        html
    );
}

// ============================================================================
// Unit: Regression safety for other list patterns
// ============================================================================

#[test]
fn test_issue373_regression_simple_numbered_list() {
    let input = "1. foo\n2. bar";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        html.contains("<ol>"),
        "Numbered list should produce <ol>. Got:\n{}",
        html
    );
    assert!(
        html.contains("<li>"),
        "Should have <li> items. Got:\n{}",
        html
    );
    assert!(html.contains("foo"), "Should contain 'foo'. Got:\n{}", html);
    assert!(html.contains("bar"), "Should contain 'bar'. Got:\n{}", html);
}

#[test]
fn test_issue373_regression_nested_bullet_list() {
    let input = "- outer\n  - inner1\n  - inner2\n- outer2";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    // Should have nested <ul> structure
    let ul_count = html.matches("<ul>").count();
    assert!(
        ul_count >= 2,
        "Should have nested <ul> (at least 2). Got {} in:\n{}",
        ul_count,
        html
    );
    assert!(
        html.contains("inner1"),
        "Should contain 'inner1'. Got:\n{}",
        html
    );
    assert!(
        html.contains("outer2"),
        "Should contain 'outer2'. Got:\n{}",
        html
    );
}

#[test]
fn test_issue373_regression_list_followed_by_paragraph() {
    // A list item followed by a paragraph (no heading) -- verify the paragraph
    // is properly placed relative to the list.
    let input = "- item1\n- item2\n\nParagraph after the list.";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    assert!(html.contains("<ul>"), "Should have <ul>. Got:\n{}", html);
    assert!(
        html.contains("Paragraph after the list."),
        "Paragraph should be present. Got:\n{}",
        html
    );
    // The paragraph should come after the list closes
    if let Some(ul_close) = html.rfind("</ul>") {
        let after_ul = &html[ul_close..];
        assert!(
            after_ul.contains("Paragraph after the list"),
            "Paragraph should appear after </ul>. Got:\n{}",
            html
        );
    }
}

#[test]
fn test_issue373_unicode_emoji_in_list_item() {
    // Include a test with emoji/Unicode content in the list item text.
    let input = "- Hello \u{1F60A} world<br />\n\
\u{25E6} bullet item<br />\n\
### Heading with \u{1F4DA} emoji<br />\n\
more text";

    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    assert!(
        html.contains('\u{1F60A}'),
        "Should preserve U+1F60A smiley emoji. Got:\n{}",
        html
    );
    assert!(
        html.contains('\u{25E6}'),
        "Should preserve U+25E6 bullet character. Got:\n{}",
        html
    );
    assert!(
        html.contains('\u{1F4DA}'),
        "Should preserve U+1F4DA books emoji. Got:\n{}",
        html
    );
}
