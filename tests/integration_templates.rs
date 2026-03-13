//! Integration tests for template rendering with real site layouts and includes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

use liquid::model::Value as LiquidValue;
use liquid::Object;

use rustkyll::template::layout::LayoutEngine;

fn site_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datatalksclub.github.io")
}

static LAYOUT_ENGINE: LazyLock<LayoutEngine> = LazyLock::new(|| {
    LayoutEngine::new(&site_dir().join("_layouts"), &site_dir().join("_includes")).unwrap()
});

/// Build a minimal site context for layout tests that need headers/footers.
fn minimal_site_context() -> Object {
    let mut site = Object::new();
    site.insert("name".into(), LiquidValue::scalar("DataTalks.Club"));
    site.insert("url".into(), LiquidValue::scalar("https://datatalks.club"));
    site.insert("twitter".into(), LiquidValue::scalar("@DataTalksClub"));

    let mut data = Object::new();
    let mut nav = Object::new();
    nav.insert("top".into(), LiquidValue::Array(vec![]));
    nav.insert("bottom".into(), LiquidValue::Array(vec![]));
    data.insert("navigation".into(), LiquidValue::Object(nav));
    data.insert("header".into(), LiquidValue::Object(Object::new()));
    site.insert("data".into(), LiquidValue::Object(data));

    let mut github = Object::new();
    github.insert(
        "repository_url".into(),
        LiquidValue::scalar("https://github.com/test/repo"),
    );
    site.insert("github".into(), LiquidValue::Object(github));

    site
}

#[test]
fn test_layout_engine_creates_with_real_dirs() {
    let engine = &*LAYOUT_ENGINE;
    assert_eq!(engine.layout_names().len(), 6);
}

#[test]
fn test_render_home_layout_with_real_includes() {
    let mut fm = HashMap::new();
    fm.insert(
        "title".to_string(),
        serde_yaml::Value::String("Welcome".to_string()),
    );

    let site = minimal_site_context();
    let output = LAYOUT_ENGINE
        .render("home", "<p>Hello</p>", &fm, &site)
        .unwrap();

    assert!(output.contains("<html"));
    assert!(output.contains("<head>"));
    assert!(output.contains("<p>Hello</p>"));
    assert!(output.contains("DataTalks.Club"));
}

#[test]
fn test_render_page_layout_with_subscribe_include() {
    let mut fm = HashMap::new();
    fm.insert(
        "title".to_string(),
        serde_yaml::Value::String("Test Page".to_string()),
    );

    let site = minimal_site_context();
    let output = LAYOUT_ENGINE
        .render("page", "Page content here", &fm, &site)
        .unwrap();

    assert!(output.contains("Page content here"));
    assert!(
        output.contains("mc-embedded-subscribe-form"),
        "Should contain subscribe form"
    );
}
