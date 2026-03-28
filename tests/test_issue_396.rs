//! TDD tests for issue 396: Normalize newlines in HTML attribute values.
//!
//! Jekyll/kramdown normalizes newlines inside HTML attribute values to spaces
//! only for **inline** HTML (rendered inside `<p>` tags). Block-level HTML
//! elements like `<figure>`, `<div>` are passed through verbatim.

#[test]
fn test_issue396_inline_html_newline_in_alt_attribute() {
    // The mojombo-blog case: <a><img> is inline HTML, wrapped in <p> by pulldown-cmark
    // Newlines inside attribute values should be normalized to spaces.
    let input = "<a rel=\"license\" href=\"http://creativecommons.org/licenses/by-nc-sa/3.0/us/\"><img alt=\"Creative\nCommons License\" style=\"border-width:0\" src=\"http://i.creativecommons.org/l/by-nc-sa/3.0/us/80x15.png\" /></a>";
    let result = rustkyll::frontmatter::markdown_to_html(input);
    assert!(
        result.contains("alt=\"Creative Commons License\""),
        "Newline in alt attribute should be normalized to space for inline HTML. Got: {:?}",
        result
    );
    assert!(
        !result.contains("Creative\nCommons"),
        "Should not contain newline in attribute value for inline HTML. Got: {:?}",
        result
    );
}

#[test]
fn test_issue396_block_html_newline_in_title_preserved() {
    // <div> is a block-level element -- Jekyll/kramdown passes it through verbatim.
    // Newlines in attributes should be PRESERVED.
    let input = "<div title=\"one\ntwo\nthree\">content</div>";
    let result = rustkyll::frontmatter::markdown_to_html(input);
    assert!(
        result.contains("title=\"one\ntwo\nthree\""),
        "Block-level HTML should preserve newlines in attributes. Got: {:?}",
        result
    );
}

#[test]
fn test_issue396_single_line_attribute_unchanged() {
    let input = "<img alt=\"no newlines\" src=\"test.png\" />";
    let result = rustkyll::frontmatter::markdown_to_html(input);
    assert!(
        result.contains("alt=\"no newlines\""),
        "Single-line attributes should be unchanged. Got: {:?}",
        result
    );
}

#[test]
fn test_issue396_inline_multi_attr_newline_in_one() {
    // <img> alone is inline HTML, wrapped in <p> by pulldown-cmark.
    // Only the attribute with the newline should be normalized.
    let input = "<img alt=\"Creative\nCommons\" class=\"license\" src=\"test.png\" />";
    let result = rustkyll::frontmatter::markdown_to_html(input);
    assert!(
        result.contains("alt=\"Creative Commons\""),
        "Newline in alt should be normalized for inline HTML. Got: {:?}",
        result
    );
    assert!(
        result.contains("class=\"license\""),
        "Other attributes should be preserved. Got: {:?}",
        result
    );
}

#[test]
fn test_issue396_block_figure_newline_in_alt_preserved() {
    // The DTC case: <figure><img> is block-level HTML.
    // Newlines in alt attribute should be PRESERVED.
    let input = "<figure>\n<img src=\"test.png\" alt=\"ML Zoomcamp \nleaderboard showing bonus points\" />\n</figure>\n";
    let result = rustkyll::frontmatter::markdown_to_html_with_options(
        input, true, // kramdown mode
        true, false, false,
    );
    assert!(
        result.contains("alt=\"ML Zoomcamp \nleaderboard"),
        "Block-level HTML should preserve newlines in attributes. Got: {:?}",
        result
    );
}

#[test]
fn test_issue396_unicode_in_inline_attribute_with_newline() {
    // Non-ASCII content in inline HTML attribute values with newlines.
    // <span> is inline, should be wrapped in <p> and normalized.
    let input = "<span title=\"Geb\u{00E4}ude\n\u{00FC}bersicht\">content</span>";
    let result = rustkyll::frontmatter::markdown_to_html(input);
    assert!(
        result.contains("title=\"Geb\u{00E4}ude \u{00FC}bersicht\""),
        "Unicode content with newline should be normalized in inline HTML. Got: {:?}",
        result
    );
}

#[test]
fn test_issue396_with_options_kramdown_inline() {
    // <img> alone is inline HTML in kramdown mode too.
    let input = "<img alt=\"Creative\nCommons License\" src=\"test.png\" />";
    let result = rustkyll::frontmatter::markdown_to_html_with_options(
        input, true,  // add_code_classes (kramdown mode)
        true,  // enable_smart_punctuation
        false, // enable_hardbreaks
        false, // enable_autolink
    );
    assert!(
        result.contains("alt=\"Creative Commons License\""),
        "markdown_to_html_with_options should normalize inline HTML. Got: {:?}",
        result
    );
}
