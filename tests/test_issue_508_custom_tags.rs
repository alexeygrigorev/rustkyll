//! Tests for issue #508: al-folio custom Liquid tags registration.
//!
//! Verifies that `{% details %}` and `{% file_exists %}` tags are properly
//! registered and produce correct output.

use liquid::Object;
use rustkyll::template::TemplateEngine;

#[test]
fn test_details_block_produces_html() {
    let eng = TemplateEngine::new().unwrap();
    let ctx = Object::new();
    let output = eng
        .parse_and_render(
            "{% details Click to expand %}Some **bold** content{% enddetails %}",
            &ctx,
        )
        .unwrap();
    assert!(
        output.contains("<details>"),
        "Should produce <details> element: {}",
        output
    );
    assert!(
        output.contains("<summary>Click to expand</summary>"),
        "Should produce <summary> with text: {}",
        output
    );
    assert!(
        output.contains("</details>"),
        "Should close </details>: {}",
        output
    );
    assert!(
        output.contains("<strong>bold</strong>"),
        "Should render Markdown in body: {}",
        output
    );
}

#[test]
fn test_details_block_unicode_content() {
    let eng = TemplateEngine::new().unwrap();
    let ctx = Object::new();
    let output = eng
        .parse_and_render(
            "{% details Erweiterung %}Inhalt mit Umlauten: aou{% enddetails %}",
            &ctx,
        )
        .unwrap();
    assert!(
        output.contains("<summary>Erweiterung</summary>"),
        "Should handle Unicode in summary: {}",
        output
    );
    assert!(
        output.contains("Umlauten"),
        "Should preserve Unicode in body: {}",
        output
    );
}

#[test]
fn test_file_exists_tag_true() {
    let eng = TemplateEngine::new().unwrap();
    let mut site = Object::new();
    site.insert("source".into(), liquid::model::Value::scalar("."));
    let mut ctx = Object::new();
    ctx.insert("site".into(), liquid::model::Value::Object(site));
    let output = eng
        .parse_and_render("{% file_exists Cargo.toml %}", &ctx)
        .unwrap();
    assert_eq!(output, "true", "Cargo.toml should exist: {}", output);
}

#[test]
fn test_file_exists_tag_false() {
    let eng = TemplateEngine::new().unwrap();
    let mut site = Object::new();
    site.insert("source".into(), liquid::model::Value::scalar("."));
    let mut ctx = Object::new();
    ctx.insert("site".into(), liquid::model::Value::Object(site));
    let output = eng
        .parse_and_render("{% file_exists no_such_file_ever.xyz %}", &ctx)
        .unwrap();
    assert_eq!(
        output, "false",
        "Nonexistent file should be false: {}",
        output
    );
}

#[test]
fn test_file_exists_in_capture_block() {
    let eng = TemplateEngine::new().unwrap();
    let mut site = Object::new();
    site.insert("source".into(), liquid::model::Value::scalar("."));
    let mut ctx = Object::new();
    ctx.insert("site".into(), liquid::model::Value::Object(site));
    let output = eng
        .parse_and_render(
            r#"{% capture exists %}{% file_exists Cargo.toml %}{% endcapture %}{% if exists == "true" %}yes{% else %}no{% endif %}"#,
            &ctx,
        )
        .unwrap();
    assert_eq!(
        output, "yes",
        "file_exists in capture pattern should work: {}",
        output
    );
}
