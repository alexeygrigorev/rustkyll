//! Tests for issue #565: Liquid capture tag whitespace preservation.
//!
//! Verifies that `{% capture %}` preserves all whitespace inside the block,
//! matching Jekyll's behavior.

use liquid::Object;
use rustkyll::template::TemplateEngine;

#[test]
fn test_capture_preserves_leading_trailing_spaces() {
    let eng = TemplateEngine::new().unwrap();
    let ctx = Object::new();
    let output = eng
        .parse_and_render("{% capture foo %}  hello  {% endcapture %}{{ foo }}", &ctx)
        .unwrap();
    assert_eq!(
        output, "  hello  ",
        "capture must preserve leading/trailing spaces"
    );
}

#[test]
fn test_capture_preserves_newlines_in_multiline_content() {
    let eng = TemplateEngine::new().unwrap();
    let ctx = Object::new();
    let output = eng
        .parse_and_render(
            "{% capture foo %}\n  <article>hello</article>\n{% endcapture %}{{ foo }}",
            &ctx,
        )
        .unwrap();
    assert_eq!(
        output, "\n  <article>hello</article>\n",
        "capture must preserve newlines in multi-line content"
    );
}

#[test]
fn test_capture_with_strip_newlines_preserves_spaces() {
    // This is the chirpy pattern: capture multi-line HTML, then strip_newlines
    let eng = TemplateEngine::new().unwrap();
    let ctx = Object::new();
    let output = eng
        .parse_and_render(
            "{% capture foo %}\n  <article>hello</article>\n{% endcapture %}{{ foo | strip_newlines }}",
            &ctx,
        )
        .unwrap();
    assert_eq!(
        output, "  <article>hello</article>",
        "strip_newlines should remove \\n but preserve leading/trailing spaces"
    );
}

#[test]
fn test_capture_with_unicode_content_preserves_whitespace() {
    let eng = TemplateEngine::new().unwrap();
    let ctx = Object::new();
    let output = eng
        .parse_and_render(
            "{% capture foo %}  caf\u{00e9} \u{00fc}ber  {% endcapture %}{{ foo }}",
            &ctx,
        )
        .unwrap();
    assert_eq!(
        output, "  caf\u{00e9} \u{00fc}ber  ",
        "capture must preserve whitespace around Unicode content"
    );
}
