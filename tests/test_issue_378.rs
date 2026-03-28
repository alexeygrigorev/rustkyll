//! TDD tests for issue 378/402: DTC reliable-machine-learning URL asterisk in markdownify.
//!
//! The markdownify filter (markdown_to_html_for_filter) SHOULD protect URL asterisks,
//! matching Jekyll/kramdown's behavior where `[url_with_asterisks](url)` produces a proper
//! `<a>` link. Jekyll/kramdown parses the link structure first, so asterisks inside
//! the link text do not become emphasis markers.
//! All pipelines (markdown_to_html, markdown_to_html_with_options, markdown_to_html_for_filter)
//! should protect URL asterisks in link text.

// ============================================================================
// markdownify filter path: should protect URL asterisks (match Jekyll)
// ============================================================================

#[test]
fn test_issue378_markdownify_url_asterisks_produce_link() {
    // In the markdownify filter, URL asterisks in [url](url) should produce <a> links
    // matching Jekyll/kramdown behavior.
    let input = "[https://example.com/?a=1*foo*bar](https://example.com/?a=1*foo*bar)";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        html.contains("<a href="),
        "markdownify should produce a proper link from URL with asterisks (matching Jekyll). Got: {:?}",
        html
    );
    assert!(
        !html.contains("<em>foo</em>"),
        "markdownify should NOT produce <em> from URL asterisks. Got: {:?}",
        html
    );
}

#[test]
fn test_issue378_markdownify_oreilly_url_pattern_produces_link() {
    // The O'Reilly URL pattern with _gl query parameters should produce <a> link
    let input = "[https://site.com/?_gl=1*abc*_ga](https://site.com/?_gl=1*abc*_ga)";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        html.contains("<a href="),
        "markdownify should produce a link from URL with asterisks. Got: {:?}",
        html
    );
}

#[test]
fn test_issue378_markdownify_normal_emphasis_still_works() {
    let input = "text *emphasis* more";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        html.contains("<em>emphasis</em>"),
        "Normal emphasis should still work in markdownify. Got: {:?}",
        html
    );
}

#[test]
fn test_issue378_markdownify_unicode_with_url_asterisks() {
    let input =
        "[https://example.com/p\u{00e4}th?q=1*\u{00fc}ber*x](https://example.com/p\u{00e4}th)";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    // With asterisks protected, should produce a proper link
    assert!(
        html.contains("<a href="),
        "Should produce a link without encoding issues. Got: {:?}",
        html
    );
}

// ============================================================================
// Non-markdownify pipelines: should STILL protect URL asterisks
// ============================================================================

#[test]
fn test_issue378_non_markdownify_still_protects_url_asterisks() {
    let input = "[https://example.com/?a=1*foo*bar](https://example.com/?a=1*foo*bar)";
    let html =
        rustkyll::frontmatter::markdown_to_html_with_options(input, true, true, false, false);
    assert!(
        !html.contains("<em>"),
        "Non-markdownify pipeline should still protect URL asterisks. Got: {:?}",
        html
    );
}

#[test]
fn test_issue378_full_oreilly_url_via_markdownify() {
    // The exact URL from the DTC reliable-machine-learning page, via markdownify.
    // Jekyll/kramdown produces a proper <a> link (stripping query params in the cached output),
    // so rustkyll's markdownify must also produce an <a> link (not <em> tags).
    let input = "maybe [https://www.oreilly.com/library/view/practical-fairness/9781492075721/?_gl=1*95hemv*_ga*MTA2ODM2NTQzNi4xNjU1NjQ3NTg4*_ga_092EL089CH*MTY3MDI2NTc4Ny4zLjEuMTY3MDI2NTg2NS41Ny4wLjA](https://www.oreilly.com/library/view/practical-fairness/9781492075721/?_gl=1*95hemv*_ga*MTA2ODM2NTQzNi4xNjU1NjQ3NTg4*_ga_092EL089CH*MTY3MDI2NTc4Ny4zLjEuMTY3MDI2NTg2NS41Ny4wLjA). if you want";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        html.contains("<a href="),
        "markdownify should produce a proper link (matching Jekyll). Got: {:?}",
        html
    );
    assert!(
        !html.contains("<em>95hemv</em>"),
        "markdownify should NOT produce <em> from URL asterisks. Got: {:?}",
        html
    );
    assert!(
        !html.contains("<em>MTA2"),
        "markdownify should NOT produce <em> from URL asterisks. Got: {:?}",
        html
    );
}
