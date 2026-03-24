//! TDD tests for issue 296: DTC remaining DOM diffs.

// === Category A: JSON-LD description diffs ===

#[test]
fn test_issue296_strip_html_trailing_whitespace_acceptable() {
    // Jekyll's document.content for cross-referenced items returns raw markdown,
    // which may have trailing spaces from the source file. rustkyll uses rendered
    // HTML which strips trailing spaces. Both behaviors are acceptable.
    // This test documents the expected behavior.
    let _html = "<p>Text with trailing space in source.</p>\n";
    let result = rustkyll::frontmatter::markdown_to_html_with_options(
        "Text with trailing space in source. \n",
        true,
        true,
        false,
        false,
    );
    // The trailing space before \n in the source may or may not be preserved
    // in the rendered HTML - both are acceptable
    assert!(
        result.contains("Text with trailing space in source."),
        "Content should be preserved. Got: {:?}",
        result
    );
}

#[test]
fn test_issue296_markdown_paragraph_double_newline() {
    // Multi-paragraph content should have double newline between paragraphs
    // in the rendered HTML, matching kramdown output.
    let md = "First paragraph.\n\nSecond paragraph.\n";
    let html = rustkyll::frontmatter::markdown_to_html_with_options(md, true, true, false, false);
    assert!(
        html.contains("</p>\n\n<p>"),
        "Should have double newline between paragraphs. Got: {:?}",
        html
    );
}

#[test]
fn test_issue296_unicode_in_descriptions() {
    // Accented characters and special punctuation must be preserved in
    // rendered content (used for JSON-LD descriptions).
    let md = "Ren\u{00e9} Descartes est un philosophe fran\u{00e7}ais.\n";
    let html = rustkyll::frontmatter::markdown_to_html_with_options(md, true, true, false, false);
    assert!(
        html.contains("Ren\u{00e9}"),
        "Accented characters should be preserved. Got: {:?}",
        html
    );
    assert!(
        html.contains("fran\u{00e7}ais"),
        "Cedilla should be preserved. Got: {:?}",
        html
    );
}

#[test]
fn test_issue296_description_with_markdown_link() {
    // When content has markdown links, the rendered HTML converts them to <a> tags.
    // After strip_html, only the link text remains (no URL).
    // Jekyll preserves the raw markdown link syntax because it uses raw content
    // for cross-referenced items. This difference is acceptable.
    let md = "David is the founder of [Accents Welcome](https://accentswelcome.com), a school.\n";
    let html = rustkyll::frontmatter::markdown_to_html_with_options(md, true, true, false, false);
    assert!(
        html.contains("Accents Welcome"),
        "Link text should be present. Got: {:?}",
        html
    );
    assert!(
        html.contains("<a"),
        "Link should be rendered as HTML anchor. Got: {:?}",
        html
    );
}

// === Newline in raw HTML alt attribute ===

#[test]
fn test_issue296_raw_html_alt_newline_preserved() {
    // Raw HTML blocks should preserve literal newlines in attribute values.
    // The markdown source has: alt="ML Zoomcamp \nleaderboard..."
    // This should pass through unchanged.
    let md = "<figure>\n<img src=\"test.png\" alt=\"ML Zoomcamp \nleaderboard showing bonus points\" />\n</figure>\n";
    let html = rustkyll::frontmatter::markdown_to_html_with_options(md, true, true, false, false);
    assert!(
        html.contains("alt=\"ML Zoomcamp \nleaderboard"),
        "Newline in alt attribute should be preserved in raw HTML block. Got: {:?}",
        html
    );
}

// === Podcast guest bio newline preservation ===

#[test]
fn test_issue296_multi_paragraph_content_preserves_double_newline() {
    // Multi-paragraph bios should have \n\n between paragraphs in rendered HTML,
    // which after strip_html produces \n\n in the text (matching Jekyll).
    let md = "First paragraph about the guest.\n\nSecond paragraph with more details.\n";
    let html = rustkyll::frontmatter::markdown_to_html_with_options(md, true, true, false, false);
    // After strip_html, the \n\n should be preserved
    let stripped = html.replace("<p>", "").replace("</p>", "");
    assert!(
        stripped.contains("guest.\n\nSecond"),
        "Double newline between paragraphs should be preserved after strip_html. Got: {:?}",
        stripped
    );
}

// === Non-ASCII / Unicode content tests ===

#[test]
fn test_issue296_unicode_utn_description() {
    // Test with Universidad Tecnologica Nacional content
    let md = "She graduated from Universidad Tecnol\u{00f3}gica Nacional (UTN - FRBA).\n";
    let html = rustkyll::frontmatter::markdown_to_html_with_options(md, true, true, false, false);
    assert!(
        html.contains("Tecnol\u{00f3}gica"),
        "Accented o should be preserved. Got: {:?}",
        html
    );
    assert!(
        html.contains("(UTN - FRBA)"),
        "Parentheses and hyphens should be preserved. Got: {:?}",
        html
    );
}

#[test]
fn test_issue296_unicode_cyrillic_in_description() {
    // Cyrillic characters should be preserved in descriptions
    let md = "\u{041c}\u{043e}\u{0441}\u{043a}\u{0432}\u{0430} - \u{0433}\u{043e}\u{0440}\u{043e}\u{0434} \u{043c}\u{043e}\u{0439}.\n";
    let html = rustkyll::frontmatter::markdown_to_html_with_options(md, true, true, false, false);
    assert!(
        html.contains("\u{041c}\u{043e}\u{0441}\u{043a}\u{0432}\u{0430}"),
        "Cyrillic should be preserved. Got: {:?}",
        html
    );
}

#[test]
fn test_issue296_unicode_emoji_in_description() {
    // Emoji should be preserved in descriptions
    let md = "Data scientist and ML engineer \u{1f680}\u{1f4ca}.\n";
    let html = rustkyll::frontmatter::markdown_to_html_with_options(md, true, true, false, false);
    assert!(
        html.contains("\u{1f680}"),
        "Rocket emoji should be preserved. Got: {:?}",
        html
    );
}

// === Acceptable diff filter tests ===

#[test]
fn test_issue296_acceptable_diff_filter_trailing_space() {
    // This is a documentation test: the acceptable-diff filter should catch
    // differences where only trailing whitespace differs between Jekyll and
    // rustkyll JSON-LD descriptions.
    //
    // Jekyll (raw md): "...new professionals in this area. \n"
    // Rustkyll (HTML):  "...new professionals in this area.\n"
    //
    // After rstrip(): both become "...new professionals in this area."
    // This is an acceptable diff.
    let jekyll = "text with trailing space. \n";
    let rustkyll = "text with trailing space.\n";
    assert_eq!(
        jekyll.trim_end(),
        rustkyll.trim_end(),
        "After trimming trailing whitespace, both should match"
    );
}

#[test]
fn test_issue296_acceptable_diff_filter_markdown_link() {
    // Documentation test: the acceptable-diff filter catches diffs where
    // Jekyll preserves markdown link syntax but rustkyll strips it.
    let jekyll = "founder of [Accents Welcome](https://accentswelcome.com), a school.";
    let rustkyll = "founder of Accents Welcome, a school.";
    // After stripping markdown links from Jekyll's version:
    let stripped = jekyll.replace(
        "[Accents Welcome](https://accentswelcome.com)",
        "Accents Welcome",
    );
    assert_eq!(
        stripped, rustkyll,
        "After stripping markdown links, both should match"
    );
}

#[test]
fn test_issue296_debug_raw_html_processing() {
    // Debug test: trace where newline in HTML attribute gets converted to space
    let input = "<figure>\n<img src=\"test.png\" alt=\"ML Zoomcamp \nleaderboard\" />\n</figure>\n";

    // Raw pulldown-cmark preserves the newline
    let mut html_output = String::new();
    use pulldown_cmark::{Options, Parser};
    let parser = Parser::new_ext(input, Options::empty());
    pulldown_cmark::html::push_html(&mut html_output, parser);
    assert!(
        html_output.contains("Zoomcamp \nleaderboard"),
        "pulldown-cmark should preserve newline. Got: {:?}",
        html_output
    );

    // Check pre-processing steps individually
    let step1 = rustkyll::kramdown::process_markdown_attribute(input);
    assert!(
        step1.contains("Zoomcamp \nleaderboard"),
        "process_markdown_attribute broke it: {:?}",
        step1
    );

    let step2 = rustkyll::kramdown::split_text_after_html_block_close(&step1);
    assert!(
        step2.contains("Zoomcamp \nleaderboard"),
        "split_text_after_html_block_close broke it: {:?}",
        step2
    );

    let step3 = rustkyll::kramdown::collapse_blank_lines_between_list_items(&step2);
    assert!(
        step3.contains("Zoomcamp \nleaderboard"),
        "collapse_blank_lines_between_list_items broke it: {:?}",
        step3
    );

    // Now check postprocess
    let postprocessed = rustkyll::kramdown::postprocess(&html_output);
    if postprocessed.contains("Zoomcamp  leaderboard") {
        eprintln!("postprocess converted newline to space!");
    }
    assert!(
        postprocessed.contains("Zoomcamp \nleaderboard"),
        "postprocess should preserve newline in HTML attributes. Got: {:?}",
        postprocessed
    );
}
