//! TDD tests for issue 390: intra-word emphasis in markdownify (kramdown boundary matching).
//!
//! Kramdown treats `*` in `sh*t` as opening emphasis (intra-word), while pulldown-cmark
//! (CommonMark) does not. This causes downstream differences where the emphasis span
//! absorbs text up to the next `*`.

#[test]
fn test_issue390_intraword_asterisk_opens_emphasis_in_markdownify() {
    // In kramdown, `sh*t ton of money` with a later `*closing*` produces:
    // sh<em>t ton of money...text *closing</em>
    // because the * in sh*t opens emphasis and closes at the next *.
    let input = "spend sh*t ton of money on RedShift\n> *Can you say anything?*\nMore text";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    // Kramdown: sh<em>t ton of money...
    assert!(
        html.contains("sh<em>t ton of money"),
        "Intra-word * should open <em> in markdownify. Got: {:?}",
        html
    );
}

#[test]
fn test_issue390_intraword_emphasis_absorbs_next_em_tag() {
    // The emphasis opened by sh*t should absorb the next <em> that pulldown-cmark
    // would have created, converting the opening <em> to literal *.
    let input = "spend sh*t ton of money on RedShift\n> *Can you say anything?*\nMore text";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    // The * before "Can you say" should be literal (inside the emphasis span)
    assert!(
        html.contains("*Can you say anything?</em>")
            || html.contains("*Can you say anything?</em>"),
        "The * before 'Can you say' should be literal text inside <em>. Got: {:?}",
        html
    );
}

#[test]
fn test_issue390_normal_emphasis_not_affected() {
    // Normal emphasis like *text* should not be affected
    let input = "This is *important* text";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        html.contains("<em>important</em>"),
        "Normal emphasis should work. Got: {:?}",
        html
    );
}

#[test]
fn test_issue390_literal_asterisk_without_later_emphasis() {
    // If there's an intra-word * but no later emphasis to absorb, keep it literal
    let input = "5*3 is fifteen";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        html.contains("5*3"),
        "Intra-word * with no later emphasis should stay literal. Got: {:?}",
        html
    );
}

#[test]
fn test_issue390_actual_dtc_analytics_engineering_pattern() {
    // This is the actual pattern from the DTC analytics engineering book page.
    // After newline_to_br, the text looks like this (simplified):
    let input = "If you're on AWS and don't wanna spend sh*t ton of money on setting up a RedShift cluster that will be idle most of the time, go with RedShift Serverless<br />\n> *Can you say anything regarding dbt certificates, in particular how complex they are?*<br />\nI think you'll find this article useful";
    let html = rustkyll::frontmatter::markdown_to_html_for_filter(input);
    assert!(
        html.contains("sh<em>"),
        "sh*t should open emphasis in markdownify (matching kramdown). Got: {:?}",
        html
    );
}
