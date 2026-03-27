//! TDD tests for issue 386: DTC graph-data definition list rendering in markdownify.
//!
//! Kramdown renders `term\n: definition` as `<dl><dt><dd>`. When this pattern
//! appears in `newline_to_br | markdownify` output (e.g., `term<br />\n: definition`),
//! rustkyll must produce the same `<dl>` structure.

// ============================================================================
// Unit: Definition list pattern detection
// ============================================================================

#[test]
fn test_issue386_definition_list_basic() {
    // Simple definition list pattern: term<br />\n: definition
    let input = "term<br />\n: definition";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    assert!(
        html.contains("<dl>"),
        "Should produce <dl> for definition list pattern. Got:\n{}",
        html
    );
    assert!(
        html.contains("<dt>term<br /></dt>"),
        "Should produce <dt> with term. Got:\n{}",
        html
    );
    assert!(
        html.contains("<dd>definition</dd>"),
        "Should produce <dd> with definition. Got:\n{}",
        html
    );
}

#[test]
fn test_issue386_definition_list_in_ordered_list() {
    // The exact pattern from the graph-data page
    let input = "1. Snap: http://snap.stanford.edu/<br />\n\
                 2. Kaggle: https://www.kaggle.com/<br />\n\
                 3. Or, this GitHub<br />\n\
                 : [https://github.com/awesomedata/awesome-public-datasets](https://github.com/awesomedata/awesome-public-datasets)";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    // Items 1 and 2 should be normal list items
    assert!(
        html.contains("Snap:"),
        "Item 1 should be present. Got:\n{}",
        html
    );
    assert!(
        html.contains("Kaggle:"),
        "Item 2 should be present. Got:\n{}",
        html
    );

    // Item 3 should contain a definition list
    assert!(
        html.contains("<dl>"),
        "Item 3 should contain a <dl>. Got:\n{}",
        html
    );
    assert!(
        html.contains("<dt>Or, this GitHub<br /></dt>"),
        "Should produce <dt> for item 3 term. Got:\n{}",
        html
    );
    assert!(
        html.contains("<dd>"),
        "Should produce <dd> for item 3 definition. Got:\n{}",
        html
    );
    assert!(
        html.contains("awesome-public-datasets</a>"),
        "Link in definition should be rendered. Got:\n{}",
        html
    );
}

#[test]
fn test_issue386_no_definition_list_for_colon_mid_line() {
    // A colon in the middle of text should NOT trigger definition list
    let input = "Just some text: with a colon";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    assert!(
        !html.contains("<dl>"),
        "Colon mid-line should not produce <dl>. Got:\n{}",
        html
    );
}

#[test]
fn test_issue386_no_definition_list_for_colon_at_end_of_line() {
    // A colon at end of line followed by a numbered list is NOT definition syntax
    let input = "I get my graph data from:<br />\n1. First item";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    assert!(
        !html.contains("<dl>"),
        "Colon at end of line should not produce <dl>. Got:\n{}",
        html
    );
}

#[test]
fn test_issue386_definition_list_with_link() {
    // Definition containing a markdown link
    let input = "Term<br />\n: [link text](http://example.com)";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    assert!(html.contains("<dl>"), "Should produce <dl>. Got:\n{}", html);
    assert!(
        html.contains("<dd>") && html.contains("</dd>"),
        "Should produce <dd>. Got:\n{}",
        html
    );
    assert!(
        html.contains(r#"<a href="http://example.com">link text</a>"#),
        "Link in definition should be rendered as <a>. Got:\n{}",
        html
    );
}

#[test]
fn test_issue386_definition_list_with_bold() {
    // Definition containing bold text
    let input = "Term<br />\n: **bold** definition";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    assert!(html.contains("<dl>"), "Should produce <dl>. Got:\n{}", html);
    assert!(
        html.contains("<strong>bold</strong>"),
        "Bold in definition should be rendered. Got:\n{}",
        html
    );
}

#[test]
fn test_issue386_definition_list_unicode() {
    // Non-ASCII content in term and definition
    let input = "Terme fran\u{00E7}ais<br />\n: d\u{00E9}finition avec accent \u{00E8}";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    assert!(
        html.contains("<dl>"),
        "Should produce <dl> for unicode content. Got:\n{}",
        html
    );
    assert!(
        html.contains("fran\u{00E7}ais"),
        "Unicode term should be preserved. Got:\n{}",
        html
    );
    assert!(
        html.contains("d\u{00E9}finition"),
        "Unicode definition should be preserved. Got:\n{}",
        html
    );
}

#[test]
fn test_issue386_no_regression_on_ordered_unordered_transition() {
    // Issue #362 pattern: ordered-to-unordered list transition must stay nested
    let input = "1. Item<br />\n- Subitem";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);

    // Must NOT produce <dl> for a dash-list continuation
    assert!(
        !html.contains("<dl>"),
        "Ordered-to-unordered transition should not produce <dl>. Got:\n{}",
        html
    );
}
