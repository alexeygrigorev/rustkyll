//! TDD tests for issue #584: Liquid whitespace trimming eats adjacent tag output.
//!
//! `{{- -}}` dash-trimming must only strip template-level whitespace (newlines
//! from source), not the rendered output of neighboring `{{ ' ' }}` tags.

/// Helper: render a Liquid template with the given globals.
fn render(template_str: &str, globals: &liquid::Object) -> String {
    let parser = liquid::ParserBuilder::with_stdlib()
        .build()
        .expect("failed to build liquid parser");
    let template = parser
        .parse(template_str)
        .unwrap_or_else(|e| panic!("failed to parse template {:?}: {}", template_str, e));
    template
        .render(globals)
        .unwrap_or_else(|e| panic!("failed to render template {:?}: {}", template_str, e))
}

#[test]
fn test_issue584_dash_trim_does_not_eat_space_tag_output() {
    // {{- 1 -}} followed by {{ ' ' }} followed by {{- "min" -}}
    // The space must survive: "1 min", not "1min"
    let globals = liquid::Object::new();
    let result = render("{{- 1 -}}\n{{ ' ' }}\n{{- \"min\" -}}", &globals);
    assert_eq!(
        result, "1 min",
        "Whitespace trim must not consume output of {{ ' ' }} tag"
    );
}

#[test]
fn test_issue584_dash_trim_preserves_expression_output() {
    // {{- "a" -}} followed by {{ "X" }} followed by {{- "b" -}}
    // Must produce "aXb"
    let globals = liquid::Object::new();
    let result = render("{{- \"a\" -}}\n{{ \"X\" }}\n{{- \"b\" -}}", &globals);
    assert_eq!(
        result, "aXb",
        "Whitespace trim must not consume output of adjacent expression tags"
    );
}

#[test]
fn test_issue584_normal_trimming_still_works() {
    // Simple case: leading/trailing template whitespace stripped
    let globals = liquid::Object::new();
    let result = render("  {{- \"hello\" -}}  ", &globals);
    assert_eq!(
        result, "hello",
        "Normal whitespace trimming must still work"
    );
}

#[test]
fn test_issue584_no_trim_spaces_preserved() {
    // Without dash syntax, spaces between tags are preserved
    let globals = liquid::Object::new();
    let result = render("{{ \"a\" }} {{ \"b\" }}", &globals);
    assert_eq!(
        result, "a b",
        "Without dash syntax, spaces should be preserved"
    );
}

#[test]
fn test_issue584_left_trim_on_non_newline_space() {
    // {{ 'hello' }}{{- ' world' -}} — left-trim should strip template ws only
    let globals = liquid::Object::new();
    let result = render("{{ 'hello' }}{{- ' world' -}}", &globals);
    assert_eq!(
        result, "hello world",
        "Left-trim should not eat rendered output of preceding tag"
    );
}

#[test]
fn test_issue584_unicode_space_preserved() {
    // Unicode content with space between trimmed tags
    let globals = liquid::Object::new();
    let result = render("{{- \"Привет\" -}}\n{{ ' ' }}\n{{- \"мир\" -}}", &globals);
    assert_eq!(
        result, "Привет мир",
        "Unicode content with space separator must be preserved"
    );
}

#[test]
fn test_issue584_multiline_template_ws_stripped() {
    // Multiple newlines between tags should be stripped by {{-
    let globals = liquid::Object::new();
    let result = render("{{- \"a\" -}}\n\n\n{{- \"b\" -}}", &globals);
    assert_eq!(
        result, "ab",
        "Multiple newlines (template whitespace) should be stripped"
    );
}
