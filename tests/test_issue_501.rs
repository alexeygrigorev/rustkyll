//! Issue 501: Fix HTML double-escaping in Liquid include parameters.
//!
//! When a Liquid `{% include %}` tag passes HTML content with escaped quotes
//! (`\"`) in parameters, rustkyll must convert them to literal `"` in the
//! output -- NOT to `&quot;`.
//!
//! This is critical for the just-the-docs theme's anchor_headings include,
//! which passes SVG markup and HTML attributes containing escaped quotes.

use std::collections::HashMap;

use liquid::Object;
use rustkyll::template::TemplateEngine;

/// Test that include parameters with escaped quotes produce literal `"` in output.
/// The anchorBody parameter contains SVG with `viewBox=\"0 0 16 16\"`.
#[test]
fn test_include_param_escaped_quotes_become_literal_quotes() {
    let mut includes = HashMap::new();
    includes.insert("test.html".to_string(), "{{ include.body }}".to_string());
    let engine = TemplateEngine::with_includes_map(&includes).unwrap();

    let ctx = Object::new();
    let template =
        r#"{% include test.html body="<svg viewBox=\"0 0 16 16\" aria-hidden=\"true\"></svg>" %}"#;

    let result = engine.parse_and_render(template, &ctx).unwrap();

    // Must contain literal quotes, NOT &quot;
    assert!(
        result.contains(r#"viewBox="0 0 16 16""#),
        "Expected literal quotes in viewBox, got: {}",
        result
    );
    assert!(
        result.contains(r#"aria-hidden="true""#),
        "Expected literal quotes in aria-hidden, got: {}",
        result
    );
    assert!(
        !result.contains("&quot;"),
        "Must NOT contain &quot; HTML entities, got: {}",
        result
    );
}

/// Test that `aria-labelledby=\"%html_id%\"` pattern works correctly.
/// The just-the-docs anchor headings include uses this with a `replace` filter.
#[test]
fn test_include_param_escaped_quotes_with_replace_filter() {
    let mut includes = HashMap::new();
    includes.insert(
        "test.html".to_string(),
        r##"<a {{ include.attrs | replace: '%id%', 'nav' }}>link</a>"##.to_string(),
    );
    let engine = TemplateEngine::with_includes_map(&includes).unwrap();

    let ctx = Object::new();
    let template = r#"{% include test.html attrs="aria-labelledby=\"%id%\"" %}"#;

    let result = engine.parse_and_render(template, &ctx).unwrap();

    assert!(
        result.contains(r#"aria-labelledby="nav""#),
        "Expected literal quotes after replace filter, got: {}",
        result
    );
    assert!(
        !result.contains("&quot;"),
        "Must NOT contain &quot; HTML entities, got: {}",
        result
    );
}

/// Test that include parameters without escaped quotes are unaffected.
#[test]
fn test_include_param_no_escaped_quotes_unchanged() {
    let mut includes = HashMap::new();
    includes.insert("test.html".to_string(), "{{ include.title }}".to_string());
    let engine = TemplateEngine::with_includes_map(&includes).unwrap();

    let ctx = Object::new();
    let template = r#"{% include test.html title="Hello World" %}"#;

    let result = engine.parse_and_render(template, &ctx).unwrap();
    assert_eq!(result, "Hello World");
}

/// Full anchor_headings-style integration test.
/// Simulates the just-the-docs pattern where anchorBody contains SVG
/// and anchorAttrs contains escaped attribute names.
#[test]
fn test_include_anchor_headings_style_escaped_params() {
    let mut includes = HashMap::new();
    // Simplified anchor_headings template that uses both parameters
    includes.insert(
        "vendor/anchor_headings.html".to_string(),
        concat!(
            r##"<a href="#test" class="{{ include.anchorClass }}" "##,
            r##"{{ include.anchorAttrs | replace: '%html_id%', 'test' }}>"##,
            r##"{{ include.anchorBody }}</a>"##,
        )
        .to_string(),
    );
    let engine = TemplateEngine::with_includes_map(&includes).unwrap();

    let ctx = Object::new();
    let template = r##"{% include vendor/anchor_headings.html anchorBody="<svg viewBox=\"0 0 16 16\" aria-hidden=\"true\"><use xlink:href=\"#svg-link\"></use></svg>" anchorClass="anchor-heading" anchorAttrs="aria-labelledby=\"%html_id%\"" %}"##;

    let result = engine.parse_and_render(template, &ctx).unwrap();

    // Check SVG attributes have literal quotes
    assert!(
        result.contains(r#"viewBox="0 0 16 16""#),
        "SVG viewBox should have literal quotes, got: {}",
        result
    );
    assert!(
        result.contains(r#"aria-hidden="true""#),
        "SVG aria-hidden should have literal quotes, got: {}",
        result
    );
    assert!(
        result.contains(r##"xlink:href="#svg-link""##),
        "SVG xlink:href should have literal quotes, got: {}",
        result
    );
    // Check anchor attributes
    assert!(
        result.contains(r#"aria-labelledby="test""#),
        "Anchor aria-labelledby should have literal quotes after replace, got: {}",
        result
    );
    assert!(
        result.contains(r#"class="anchor-heading""#),
        "Anchor class should be present, got: {}",
        result
    );
    // No &quot; anywhere
    assert!(
        !result.contains("&quot;"),
        "Output must NOT contain &quot; entities, got: {}",
        result
    );
}

/// Test with single-quoted escaped parameter values (`\'`).
#[test]
fn test_include_param_escaped_single_quotes() {
    let mut includes = HashMap::new();
    includes.insert("test.html".to_string(), "{{ include.data }}".to_string());
    let engine = TemplateEngine::with_includes_map(&includes).unwrap();

    let ctx = Object::new();
    let template = r"{% include test.html data='it\'s a test' %}";

    let result = engine.parse_and_render(template, &ctx).unwrap();

    assert!(
        result.contains("it's a test"),
        "Expected literal single quote, got: {}",
        result
    );
    assert!(
        !result.contains("&#39;"),
        "Must NOT contain &#39; HTML entities, got: {}",
        result
    );
}

/// Test with non-ASCII / Unicode content in escaped-quote params.
#[test]
fn test_include_param_escaped_quotes_with_unicode() {
    let mut includes = HashMap::new();
    includes.insert("test.html".to_string(), "{{ include.label }}".to_string());
    let engine = TemplateEngine::with_includes_map(&includes).unwrap();

    let ctx = Object::new();
    // Use actual Unicode characters mixed with escaped quotes
    let template = "{% include test.html label=\"\u{4f60}\u{597d} name=\\\"test\\\"\" %}";

    let result = engine.parse_and_render(template, &ctx).unwrap();

    // The escaped quotes should become literal quotes
    assert!(
        !result.contains("&quot;"),
        "Must NOT contain &quot; entities, got: {}",
        result
    );
    // Should contain the Unicode characters
    assert!(
        result.contains("\u{4f60}\u{597d}"),
        "Should contain Unicode characters, got: {}",
        result
    );
}
