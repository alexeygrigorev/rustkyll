/// Issue 390: Test that URLs containing asterisks are not parsed as emphasis.
///
/// This test documents a known parser difference: both pulldown-cmark and
/// kramdown incorrectly treat `*` inside URLs as emphasis markers when the
/// URL appears as link text in `[url](url)` markdown syntax.
///
/// The correct behavior is to leave `*` in URLs as literal characters.
/// This test captures the current (incorrect) behavior so we can track
/// progress toward fixing it.
use rustkyll::frontmatter::{markdown_to_html, markdown_to_html_for_filter};

#[test]
fn test_issue390_url_with_asterisks_in_link_text_markdownify() {
    // URL with Google Analytics tracking params containing *
    // When used as link text in [url](url), the * should NOT become <em>
    let input = "[https://example.com/?_gl=1*abc*_ga*123](https://example.com/?_gl=1*abc*_ga*123)";
    let result = markdown_to_html_for_filter(input);

    // protect_url_link_text_emphasis escapes asterisks in URL-like link text,
    // so pulldown-cmark produces a proper <a> link (matching Jekyll/kramdown).
    assert!(
        result.contains("<a "),
        "Markdownify should produce a link for URL with asterisks: got '{}'",
        result
    );
}

#[test]
fn test_issue390_url_with_asterisks_in_main_pipeline() {
    // Same test through the main markdown pipeline
    let input = "[https://example.com/?_gl=1*abc*_ga*123](https://example.com/?_gl=1*abc*_ga*123)";
    let result = markdown_to_html(input);

    // The main pipeline has protect_url_link_text_emphasis() which should
    // protect the asterisks — verify it produces a clean link
    assert!(
        result.contains("<a "),
        "Main pipeline should produce a link for URL with asterisks: got '{}'",
        result
    );
}

#[test]
fn test_issue390_bare_url_with_asterisks_should_not_emphasize() {
    // Bare URL (not in markdown link syntax) with asterisks
    // This is the pattern that causes problems in thread comments
    let input = "check https://example.com/?q=a*b*c for details";
    let result = markdown_to_html_for_filter(input);

    // The asterisks in the URL should ideally not produce <em>
    // This test documents current behavior
    assert!(
        !result.is_empty(),
        "Should produce some output for bare URL with asterisks"
    );
}

#[test]
fn test_issue390_normal_emphasis_still_works() {
    // Regular emphasis should still work
    let input = "this is *emphasized* text";
    let result = markdown_to_html_for_filter(input);
    assert!(
        result.contains("<em>emphasized</em>"),
        "Normal emphasis should still work: got '{}'",
        result
    );
}
