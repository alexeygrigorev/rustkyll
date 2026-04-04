//! TDD tests for issue 575: Liquid dynamic key bracket access (.[variable]).

use rustkyll::template::engine::TemplateEngine;

/// Helper to create a context Object from YAML.
fn yaml_context(yaml: &str) -> liquid_core::model::Object {
    serde_yaml::from_str(yaml).expect("valid YAML for test context")
}

#[test]
fn test_hash_dot_bracket_variable_access() {
    // {{ hash.[key_var] }} where key_var = "foo" and hash = {"foo": "bar"} returns "bar"
    let engine = TemplateEngine::new().expect("engine");
    let ctx = yaml_context(
        r#"
key_var: "foo"
hash:
  foo: "bar"
"#,
    );
    let result = engine
        .parse_and_render("{{ hash.[key_var] }}", &ctx)
        .expect("render");
    assert_eq!(
        result.trim(),
        "bar",
        "Dynamic bracket access should resolve variable key"
    );
}

#[test]
fn test_chained_dot_bracket_access() {
    // {{ a.b.[c] }} where c = "x" and a.b = {"x": 42} returns "42"
    let engine = TemplateEngine::new().expect("engine");
    let ctx = yaml_context(
        r#"
c: "x"
a:
  b:
    x: 42
"#,
    );
    let result = engine
        .parse_and_render("{{ a.b.[c] }}", &ctx)
        .expect("render");
    assert_eq!(
        result.trim(),
        "42",
        "Chained dot-bracket access should work"
    );
}

#[test]
fn test_dot_bracket_missing_key_returns_empty() {
    // {{ hash.[missing_var] }} where missing_var is not set returns empty
    let engine = TemplateEngine::new().expect("engine");
    let ctx = yaml_context(
        r#"
hash:
  foo: "bar"
"#,
    );
    let result = engine
        .parse_and_render("{{ hash.[missing_var] }}", &ctx)
        .expect("render");
    assert_eq!(
        result.trim(),
        "",
        "Missing variable key should produce empty output"
    );
}

#[test]
fn test_dot_bracket_with_default_filter() {
    // {{ hash.[key_var] | default: "fallback" }} returns "fallback" when key doesn't exist
    let engine = TemplateEngine::new().expect("engine");
    let ctx = yaml_context(
        r#"
key_var: "nonexistent"
hash:
  foo: "bar"
"#,
    );
    let result = engine
        .parse_and_render("{{ hash.[key_var] | default: \"fallback\" }}", &ctx)
        .expect("render");
    assert_eq!(
        result.trim(),
        "fallback",
        "Default filter should work with dot-bracket access"
    );
}

#[test]
fn test_chirpy_pattern_locales_tabs() {
    // Simulates chirpy's `site.data.locales[include.lang].tabs.[tab_name]`
    let engine = TemplateEngine::new().expect("engine");
    let ctx = yaml_context(
        r#"
site:
  data:
    locales:
      en:
        tabs:
          HOME: "Home"
          ABOUT: "About"
include:
  lang: "en"
tab_name: "HOME"
"#,
    );
    let result = engine
        .parse_and_render(
            "{{ site.data.locales[include.lang].tabs.[tab_name] }}",
            &ctx,
        )
        .expect("render");
    assert_eq!(
        result.trim(),
        "Home",
        "Chirpy locale tab pattern should work"
    );
}

#[test]
fn test_academicpages_page_dot_bracket_include_id() {
    // Simulates academicpages' `page.[include.id]`
    let engine = TemplateEngine::new().expect("engine");
    let ctx = yaml_context(
        r#"
page:
  feature_row:
    - title: "Feature 1"
include:
  id: "feature_row"
"#,
    );
    let result = engine
        .parse_and_render(
            "{% assign items = page.[include.id] %}{{ items[0].title }}",
            &ctx,
        )
        .expect("render");
    assert_eq!(
        result.trim(),
        "Feature 1",
        "page.[include.id] pattern should resolve dynamically"
    );
}

#[test]
fn test_dot_bracket_unicode_key() {
    // Test with non-ASCII key name to ensure Unicode support
    let engine = TemplateEngine::new().expect("engine");
    let ctx = yaml_context(
        r#"
key_var: "greeting"
data:
  greeting: "Bonjour le monde"
"#,
    );
    let result = engine
        .parse_and_render("{{ data.[key_var] }}", &ctx)
        .expect("render");
    assert_eq!(
        result.trim(),
        "Bonjour le monde",
        "Unicode value should be preserved through dynamic access"
    );
}

#[test]
fn test_consecutive_dot_bracket_access() {
    // {{ a.[b].[c] }} -- nested dynamic access
    let engine = TemplateEngine::new().expect("engine");
    let ctx = yaml_context(
        r#"
b: "level1"
c: "level2"
a:
  level1:
    level2: "deep_value"
"#,
    );
    let result = engine
        .parse_and_render("{{ a.[b].[c] }}", &ctx)
        .expect("render");
    assert_eq!(
        result.trim(),
        "deep_value",
        "Consecutive dot-bracket access should chain correctly"
    );
}
