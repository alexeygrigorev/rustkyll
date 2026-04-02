//! TDD tests for issue 542: Support `includes_dir` config setting.
//!
//! When `_config.yml` has `includes_dir: custom_dir`, includes from that directory
//! should override includes from the default `_includes/` directory.

use std::fs;
use std::path::Path;

use rustkyll::config::SiteConfig;
use rustkyll::frontmatter::FrontMatter;
use rustkyll::template::engine::load_includes_merged;
use rustkyll::template::layout::LayoutEngine;

/// Create a minimal site fixture with a custom includes_dir.
fn create_custom_includes_fixture(tmp: &Path) {
    fs::write(
        tmp.join("_config.yml"),
        "\
title: Test Site
includes_dir: custom_inc
",
    )
    .unwrap();

    fs::create_dir_all(tmp.join("_layouts")).unwrap();
    fs::create_dir_all(tmp.join("_includes")).unwrap();
    fs::create_dir_all(tmp.join("custom_inc")).unwrap();

    // Default includes (fallback)
    fs::write(
        tmp.join("_includes/header.html"),
        "<header>default-header</header>",
    )
    .unwrap();
    fs::write(
        tmp.join("_includes/footer.html"),
        "<footer>default-footer</footer>",
    )
    .unwrap();

    // Custom includes (override header, not footer)
    fs::write(
        tmp.join("custom_inc/header.html"),
        "<header>custom-header</header>",
    )
    .unwrap();

    // Layout that uses both includes
    fs::write(
        tmp.join("_layouts/default.html"),
        "---\n---\n{% include header.html %}\n<main>{{ content }}</main>\n{% include footer.html %}\n",
    )
    .unwrap();
}

/// Create a fixture that tests subdirectory override behavior.
fn create_subdir_includes_fixture(tmp: &Path) {
    fs::write(
        tmp.join("_config.yml"),
        "\
title: Subdir Test
includes_dir: docs/_includes
",
    )
    .unwrap();

    fs::create_dir_all(tmp.join("_layouts")).unwrap();
    fs::create_dir_all(tmp.join("_includes/theme")).unwrap();
    fs::create_dir_all(tmp.join("docs/_includes/theme")).unwrap();

    // Default: empty stub
    fs::write(
        tmp.join("_includes/theme/analytics.html"),
        "<!-- no analytics -->",
    )
    .unwrap();

    // Custom: real analytics script
    fs::write(
        tmp.join("docs/_includes/theme/analytics.html"),
        "<script src=\"https://analytics.example.com/script.js\"></script>",
    )
    .unwrap();

    fs::write(
        tmp.join("_layouts/default.html"),
        "---\n---\n<html><head>{% include theme/analytics.html %}</head><body>{{ content }}</body></html>\n",
    )
    .unwrap();
}

fn empty_front_matter() -> FrontMatter {
    FrontMatter::new()
}

fn empty_site_context() -> liquid::Object {
    let mut site = liquid::Object::new();
    site.insert("title".into(), liquid::model::Value::scalar("Test"));
    site
}

#[test]
fn test_config_includes_dir_parsed() {
    let tmp = tempfile::tempdir().unwrap();
    create_custom_includes_fixture(tmp.path());

    let config = SiteConfig::from_file(&tmp.path().join("_config.yml")).unwrap();
    assert_eq!(config.includes_dir, "custom_inc");
}

#[test]
fn test_merged_includes_override_in_layout_engine() {
    let tmp = tempfile::tempdir().unwrap();
    create_custom_includes_fixture(tmp.path());

    let config = SiteConfig::from_file(&tmp.path().join("_config.yml")).unwrap();
    let default_includes = tmp.path().join("_includes");
    let custom_includes = tmp.path().join(&config.includes_dir);

    let engine = LayoutEngine::new_with_merged_includes(
        &tmp.path().join("_layouts"),
        &default_includes,
        &custom_includes,
    )
    .unwrap();

    let fm = empty_front_matter();
    let site = empty_site_context();
    let output = engine
        .render("default", "<p>Hello</p>", &fm, &site)
        .unwrap();

    // header.html should be from custom_inc (override)
    assert!(
        output.contains("custom-header"),
        "Custom includes_dir should override default header: got {}",
        output
    );
    assert!(
        !output.contains("default-header"),
        "Default header should NOT appear when overridden: got {}",
        output
    );

    // footer.html should be from _includes (fallback)
    assert!(
        output.contains("default-footer"),
        "Default includes should be used as fallback for footer: got {}",
        output
    );
}

#[test]
fn test_merged_includes_subdirectory_override() {
    let tmp = tempfile::tempdir().unwrap();
    create_subdir_includes_fixture(tmp.path());

    let config = SiteConfig::from_file(&tmp.path().join("_config.yml")).unwrap();
    let default_includes = tmp.path().join("_includes");
    let custom_includes = tmp.path().join(&config.includes_dir);

    let engine = LayoutEngine::new_with_merged_includes(
        &tmp.path().join("_layouts"),
        &default_includes,
        &custom_includes,
    )
    .unwrap();

    let fm = empty_front_matter();
    let site = empty_site_context();
    let output = engine
        .render("default", "<p>Content</p>", &fm, &site)
        .unwrap();

    // analytics should come from docs/_includes, not the empty stub
    assert!(
        output.contains("analytics.example.com"),
        "Subdirectory includes should be overridden by custom includes_dir: got {}",
        output
    );
    assert!(
        !output.contains("no analytics"),
        "Default stub should not appear: got {}",
        output
    );
}

#[test]
fn test_default_includes_dir_unchanged_behavior() {
    let tmp = tempfile::tempdir().unwrap();

    fs::write(tmp.path().join("_config.yml"), "title: Default Test\n").unwrap();
    fs::create_dir_all(tmp.path().join("_layouts")).unwrap();
    fs::create_dir_all(tmp.path().join("_includes")).unwrap();

    fs::write(
        tmp.path().join("_includes/nav.html"),
        "<nav>Навигация</nav>",
    )
    .unwrap();

    fs::write(
        tmp.path().join("_layouts/default.html"),
        "---\n---\n{% include nav.html %}\n{{ content }}\n",
    )
    .unwrap();

    let config = SiteConfig::from_file(&tmp.path().join("_config.yml")).unwrap();
    assert_eq!(
        config.includes_dir, "_includes",
        "Default should be _includes"
    );

    let default_includes = tmp.path().join("_includes");
    let custom_includes = tmp.path().join(&config.includes_dir);

    let engine = LayoutEngine::new_with_merged_includes(
        &tmp.path().join("_layouts"),
        &default_includes,
        &custom_includes,
    )
    .unwrap();

    let fm = empty_front_matter();
    let site = empty_site_context();
    let output = engine
        .render("default", "<p>Привет мир</p>", &fm, &site)
        .unwrap();

    assert!(
        output.contains("Навигация"),
        "Default includes should work when includes_dir is not set: got {}",
        output
    );
    assert!(
        output.contains("Привет мир"),
        "Unicode content should be preserved: got {}",
        output
    );
}

#[test]
fn test_load_includes_merged_function_directly() {
    let tmp = tempfile::tempdir().unwrap();
    let default_dir = tmp.path().join("_includes");
    let custom_dir = tmp.path().join("custom_inc");

    fs::create_dir_all(&default_dir).unwrap();
    fs::create_dir_all(&custom_dir).unwrap();

    fs::write(default_dir.join("shared.html"), "default-shared").unwrap();
    fs::write(default_dir.join("only-default.html"), "only-in-default").unwrap();
    fs::write(custom_dir.join("shared.html"), "custom-shared").unwrap();
    fs::write(custom_dir.join("only-custom.html"), "only-in-custom").unwrap();

    let merged = load_includes_merged(&default_dir, &custom_dir).unwrap();

    assert_eq!(merged.get("shared.html").unwrap(), "custom-shared");
    assert_eq!(merged.get("only-default.html").unwrap(), "only-in-default");
    assert_eq!(merged.get("only-custom.html").unwrap(), "only-in-custom");
    assert_eq!(merged.len(), 3);
}
