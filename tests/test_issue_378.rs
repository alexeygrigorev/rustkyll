//! TDD tests for issue 378: DTC reliable-machine-learning URL asterisk emphasis in markdownify.
//!
//! The markdownify filter (markdown_to_html_for_filter) should NOT protect URL asterisks,
//! matching Jekyll/kramdown's behavior where asterisks in URLs produce <em> tags.
//! The other pipelines (markdown_to_html, markdown_to_html_with_options) should still protect them.

// ============================================================================
// markdownify filter path: should NOT protect URL asterisks (match Jekyll)
// ============================================================================

#[test]
fn test_issue378_markdownify_url_asterisks_produce_emphasis() {
    // In the markdownify filter, URL asterisks should be treated as emphasis (matching Jekyll)
    let input = "[https://example.com/?a=1*foo*bar](https://example.com/?a=1*foo*bar)";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        html.contains("<em>"),
        "markdownify should produce <em> from URL asterisks (matching Jekyll). Got: {:?}",
        html
    );
    assert!(
        !html.contains("<a href="),
        "markdownify should NOT produce a proper link (matching Jekyll's broken behavior). Got: {:?}",
        html
    );
}

#[test]
fn test_issue378_markdownify_oreilly_url_pattern() {
    // With only one asterisk pair (*abc*), pulldown-cmark wraps it in <em>,
    // but when there's an odd number of asterisks, the link syntax breaks differently.
    // The key assertion: markdownify should NOT produce a clean <a> link.
    let input = "[https://site.com/?_gl=1*abc*_ga](https://site.com/?_gl=1*abc*_ga)";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        !html.contains("<a href="),
        "markdownify should NOT produce a clean link from URL with asterisks. Got: {:?}",
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
    // With asterisks not protected, pulldown-cmark may produce <em> tags
    // The key thing is no encoding issues occur
    assert!(
        !html.is_empty(),
        "Should produce output without encoding issues. Got: {:?}",
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
    // The exact URL from the DTC reliable-machine-learning page, via markdownify
    let input = "maybe [https://www.oreilly.com/library/view/practical-fairness/9781492075721/?_gl=1*95hemv*_ga*MTA2ODM2NTQzNi4xNjU1NjQ3NTg4*_ga_092EL089CH*MTY3MDI2NTc4Ny4zLjEuMTY3MDI2NTg2NS41Ny4wLjA](https://www.oreilly.com/library/view/practical-fairness/9781492075721/?_gl=1*95hemv*_ga*MTA2ODM2NTQzNi4xNjU1NjQ3NTg4*_ga_092EL089CH*MTY3MDI2NTc4Ny4zLjEuMTY3MDI2NTg2NS41Ny4wLjA). if you want";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    // Jekyll produces <em> tags from the asterisks, breaking the link
    assert!(
        html.contains("<em>"),
        "markdownify should produce <em> from URL asterisks (matching Jekyll). Got: {:?}",
        html
    );
    assert!(
        !html.contains("<a href=\"https://www.oreilly.com"),
        "markdownify should NOT produce a clean link (matching Jekyll). Got: {:?}",
        html
    );
}
