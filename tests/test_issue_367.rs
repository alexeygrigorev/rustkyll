//! TDD tests for issue 367: DTC URL asterisk rendering in markdown.
//!
//! Asterisks inside URL-like link text should not be parsed as emphasis markers.
//! This tests both the kramdown parser and the pulldown-cmark (markdownify) path.

use rustkyll::kramdown_parser;

// ============================================================================
// Kramdown parser tests
// ============================================================================

#[test]
fn test_issue367_url_asterisks_not_parsed_as_emphasis() {
    // URL with asterisks in query params should not produce <em> tags
    let input = "[https://example.com/?a=1*foo*bar](https://example.com/?a=1*foo*bar)";
    let html = kramdown_parser::to_html(input);
    assert!(
        !html.contains("<em>"),
        "URL asterisks should not create <em> tags. Got: {:?}",
        html
    );
    assert!(
        html.contains("1*foo*bar"),
        "Asterisks should be preserved as literal characters. Got: {:?}",
        html
    );
}

#[test]
fn test_issue367_oreilly_url_pattern() {
    // Real-world O'Reilly URL pattern with _gl query parameters
    let input = "[https://site.com/?_gl=1*abc*_ga*123](https://site.com/?_gl=1*abc*_ga*123)";
    let html = kramdown_parser::to_html(input);
    assert!(
        !html.contains("<em>"),
        "*abc* should NOT be wrapped in <em>. Got: {:?}",
        html
    );
    assert!(
        html.contains("1*abc*_ga*123"),
        "URL query params should be preserved. Got: {:?}",
        html
    );
}

#[test]
fn test_issue367_normal_emphasis_still_works() {
    // Regular emphasis must still work
    let input = "text *emphasis* more";
    let html = kramdown_parser::to_html(input);
    assert!(
        html.contains("<em>emphasis</em>"),
        "Normal emphasis should still work. Got: {:?}",
        html
    );
}

#[test]
fn test_issue367_emphasis_in_non_url_link_text() {
    // Emphasis inside non-URL link text should still work
    let input = "[regular *emphasis* in link](https://example.com)";
    let html = kramdown_parser::to_html(input);
    assert!(
        html.contains("<em>emphasis</em>"),
        "Emphasis in non-URL link text should work. Got: {:?}",
        html
    );
}

#[test]
fn test_issue367_unicode_with_url_asterisks() {
    // Unicode content mixed with URL asterisks
    let input =
        "[https://example.com/p\u{00e4}th?q=1*\u{00fc}ber*x](https://example.com/p\u{00e4}th)";
    let html = kramdown_parser::to_html(input);
    assert!(
        !html.contains("<em>"),
        "URL asterisks with Unicode should not create emphasis. Got: {:?}",
        html
    );
}

#[test]
fn test_issue367_multiple_asterisk_pairs_in_url() {
    // Multiple asterisk pairs in a URL query string
    let input = "[https://example.com/?_gl=1*95hemv*_ga*MTA2ODM2NTQzNi4xNjU1NjQ3NTg4*_ga_092EL089CH*MTY2OTMxNzY4MC4yMi4wLjE2NjkzMTc2ODAuNjAuMC4w](https://example.com/)";
    let html = kramdown_parser::to_html(input);
    assert!(
        !html.contains("<em>"),
        "Multiple asterisk pairs in URL should not create emphasis. Got: {:?}",
        html
    );
    assert!(
        html.contains("1*95hemv*"),
        "Asterisks should be literal. Got: {:?}",
        html
    );
}

#[test]
fn test_issue367_full_oreilly_url_kramdown() {
    // The exact URL from the DTC reliable-machine-learning page (kramdown path)
    let input = "maybe [https://www.oreilly.com/library/view/practical-fairness/9781492075721/?_gl=1*95hemv*_ga*MTA2ODM2NTQzNi4xNjU1NjQ3NTg4*_ga_092EL089CH*MTY3MDI2NTc4Ny4zLjEuMTY3MDI2NTg2NS41Ny4wLjA](https://www.oreilly.com/library/view/practical-fairness/9781492075721/?_gl=1*95hemv*_ga*MTA2ODM2NTQzNi4xNjU1NjQ3NTg4*_ga_092EL089CH*MTY3MDI2NTc4Ny4zLjEuMTY3MDI2NTg2NS41Ny4wLjA). if you want";
    let html = kramdown_parser::to_html(input);
    assert!(
        !html.contains("<em>95hemv</em>"),
        "URL query param should not be emphasis. Got: {:?}",
        html
    );
    assert!(
        !html.contains("<em>MTA2"),
        "URL query param should not be emphasis. Got: {:?}",
        html
    );
    assert!(
        html.contains("<a href="),
        "Should parse as a link. Got: {:?}",
        html
    );
}

// ============================================================================
// Markdownify (pulldown-cmark) path tests
// ============================================================================

#[test]
fn test_issue367_full_oreilly_via_markdownify() {
    // Issue 378: The markdownify filter (markdown_to_html_for_filter) should NOT protect
    // URL asterisks, matching Jekyll/kramdown behavior where asterisks produce <em> tags.
    // The markdown_to_html_with_options pipeline STILL protects them (tested here).
    let input = "maybe [https://www.oreilly.com/library/view/practical-fairness/9781492075721/?_gl=1*95hemv*_ga*MTA2ODM2NTQzNi4xNjU1NjQ3NTg4*_ga_092EL089CH*MTY3MDI2NTc4Ny4zLjEuMTY3MDI2NTg2NS41Ny4wLjA](https://www.oreilly.com/library/view/practical-fairness/9781492075721/?_gl=1*95hemv*_ga*MTA2ODM2NTQzNi4xNjU1NjQ3NTg4*_ga_092EL089CH*MTY3MDI2NTc4Ny4zLjEuMTY3MDI2NTg2NS41Ny4wLjA). if you want";
    // Test kramdown mode via markdown_to_html_with_options (still protects URL asterisks)
    let html =
        rustkyll::frontmatter::markdown_to_html_with_options(input, true, true, false, false);
    assert!(
        !html.contains("<em>95hemv</em>"),
        "URL query param should not be emphasis via markdown_to_html_with_options. Got: {:?}",
        html
    );
    assert!(
        !html.contains("<em>MTA2"),
        "URL query param should not be emphasis via markdown_to_html_with_options. Got: {:?}",
        html
    );
    assert!(
        html.contains("<a href="),
        "Should parse as a link via markdown_to_html_with_options. Got: {:?}",
        html
    );
}
