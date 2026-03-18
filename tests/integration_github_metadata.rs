//! Integration tests for site.github metadata (build_revision, url).
//!
//! Tests for issue 210: Fix choosealicense.com JSON-LD and CSS version hash.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use liquid::model::{Value as LiquidValue, ValueView};

use rustkyll::collection;
use rustkyll::config::SiteConfig;
use rustkyll::data;
use rustkyll::generator;
use rustkyll::template::layout::LayoutEngine;

/// Helper: create a minimal SiteConfig with the given URL.
fn config_with_url(url: &str) -> SiteConfig {
    let yaml = format!("url: \"{url}\"\nname: \"Test\"\ntitle: \"Test Site\"\nbaseurl: \"\"");
    serde_yaml::from_str(&yaml).unwrap()
}

/// Helper: initialize a git repo in a temp dir with a commit containing non-ASCII message.
fn init_git_repo(dir: &Path) -> String {
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .output()
        .expect("git config email failed");

    Command::new("git")
        .args([
            "config",
            "user.name",
            "Test \u{0410}\u{0432}\u{0442}\u{043e}\u{0440}",
        ])
        .current_dir(dir)
        .output()
        .expect("git config name failed");

    // Create a file with non-ASCII content
    std::fs::write(
        dir.join("README.md"),
        "# \u{041f}\u{0440}\u{043e}\u{0435}\u{043a}\u{0442} \u{0442}\u{0435}\u{0441}\u{0442}\nUnicode: \u{00e9}\u{00f1}\u{00fc}",
    )
    .unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .output()
        .expect("git add failed");

    // Commit with non-ASCII message
    Command::new("git")
        .args([
            "commit",
            "-m",
            "\u{0418}\u{0441}\u{043f}\u{0440}\u{0430}\u{0432}\u{043b}\u{0435}\u{043d}\u{0438}\u{0435} \u{00b7} fix \u{2014} initial",
        ])
        .current_dir(dir)
        .output()
        .expect("git commit failed");

    // Get the expected SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .expect("git rev-parse failed");

    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

// ============================================================================
// Unit tests for site.github fields in build_site_context
// ============================================================================

#[test]
fn test_github_build_revision_populated_from_git_head() {
    let tmp = tempfile::tempdir().unwrap();
    let expected_sha = init_git_repo(tmp.path());

    let config = config_with_url("https://example.com");
    let colls = HashMap::new();
    let data_tree = data::DataTree::new();
    let ctx = generator::build_site_context(&config, &colls, &data_tree, Some(tmp.path()), &[]);

    let github = ctx.get("github").expect("site should have github");
    if let LiquidValue::Object(github_obj) = github {
        let revision = github_obj
            .get("build_revision")
            .expect("should have build_revision");
        if let LiquidValue::Scalar(s) = revision {
            let sha = s.to_kstr().to_string();
            assert_eq!(sha.len(), 40, "SHA should be 40 chars, got: {sha}");
            assert!(
                sha.chars().all(|c| c.is_ascii_hexdigit()),
                "SHA should be hex: {sha}"
            );
            assert_eq!(sha, expected_sha, "SHA should match git HEAD");
        } else {
            panic!("Expected scalar value for build_revision, got: {revision:?}");
        }
    } else {
        panic!("Expected github to be an object");
    }
}

#[test]
fn test_github_build_revision_empty_for_non_git_dir() {
    let tmp = tempfile::tempdir().unwrap();
    // No git init -- just a plain directory

    let config = config_with_url("https://example.com");
    let colls = HashMap::new();
    let data_tree = data::DataTree::new();
    let ctx = generator::build_site_context(&config, &colls, &data_tree, Some(tmp.path()), &[]);

    let github = ctx.get("github").expect("site should have github");
    if let LiquidValue::Object(github_obj) = github {
        let revision = github_obj
            .get("build_revision")
            .expect("should have build_revision");
        // Should be empty string or Nil -- must not panic
        match revision {
            LiquidValue::Scalar(s) => {
                let val = s.to_kstr().to_string();
                assert!(
                    val.is_empty(),
                    "Should be empty for non-git dir, got: {val}"
                );
            }
            LiquidValue::Nil => {} // Also acceptable
            other => panic!("Expected empty string or Nil, got: {other:?}"),
        }
    } else {
        panic!("Expected github to be an object");
    }
}

#[test]
fn test_github_url_from_config() {
    let config = config_with_url("https://choosealicense.com");
    let colls = HashMap::new();
    let data_tree = data::DataTree::new();
    let ctx = generator::build_site_context(&config, &colls, &data_tree, None, &[]);

    let github = ctx.get("github").expect("site should have github");
    if let LiquidValue::Object(github_obj) = github {
        let url = github_obj.get("url").expect("should have url");
        assert_eq!(
            *url,
            LiquidValue::scalar("https://choosealicense.com"),
            "site.github.url should match config url"
        );
    } else {
        panic!("Expected github to be an object");
    }
}

#[test]
fn test_github_url_empty_when_config_url_empty() {
    let config = config_with_url("");
    let colls = HashMap::new();
    let data_tree = data::DataTree::new();
    let ctx = generator::build_site_context(&config, &colls, &data_tree, None, &[]);

    let github = ctx.get("github").expect("site should have github");
    if let LiquidValue::Object(github_obj) = github {
        let url = github_obj.get("url").expect("should have url");
        assert_eq!(
            *url,
            LiquidValue::scalar(""),
            "site.github.url should be empty when config url is empty"
        );
    } else {
        panic!("Expected github to be an object");
    }
}

// ============================================================================
// Integration tests: template rendering with site.github fields
// ============================================================================

/// Helper: run a minimal build pipeline for a tiny site.
fn mini_build(site_dir: &Path, output_dir: &Path) {
    let config = SiteConfig::from_file(&site_dir.join("_config.yml")).unwrap();

    let data_dir = site_dir.join("_data");
    let data_tree = if data_dir.exists() {
        data::load_data(&data_dir).unwrap()
    } else {
        data::DataTree::new()
    };

    let mut collections = HashMap::new();
    for collection_name in config.collections.keys() {
        let (items, _) = collection::load_collection(collection_name, site_dir, &config).unwrap();
        collections.insert(collection_name.clone(), items);
    }

    let (pages, _) = collection::load_pages(site_dir, &config).unwrap();

    let site_context =
        generator::build_site_context(&config, &collections, &data_tree, Some(site_dir), &pages);

    let layout_engine =
        LayoutEngine::new(&site_dir.join("_layouts"), &site_dir.join("_includes")).unwrap();

    if output_dir.exists() {
        std::fs::remove_dir_all(output_dir).unwrap();
    }
    std::fs::create_dir_all(output_dir).unwrap();

    // Generate standalone pages (which includes index.html etc.)
    generator::generate_standalone_pages(
        &pages,
        &config,
        &layout_engine,
        &site_context,
        output_dir,
    )
    .unwrap();
}

#[test]
fn test_css_version_hash_renders_in_template() {
    let tmp = tempfile::tempdir().unwrap();
    let site_dir = tmp.path();
    let expected_sha = init_git_repo(site_dir);

    // Create _config.yml
    std::fs::write(
        site_dir.join("_config.yml"),
        "url: \"https://example.com\"\nname: \"T\u{00e9}st\"\ntitle: \"T\u{00e9}st\"\n",
    )
    .unwrap();

    // Create _layouts directory with a default layout
    std::fs::create_dir_all(site_dir.join("_layouts")).unwrap();
    std::fs::write(
        site_dir.join("_layouts").join("default.html"),
        "<html><head><link href=\"style.css?v={{ site.github.build_revision }}\"></head>\
         <body>{{ content }}</body></html>",
    )
    .unwrap();

    // Create index.html with front matter
    std::fs::write(
        site_dir.join("index.html"),
        "---\nlayout: default\ntitle: \"P\u{00e1}gina\"\n---\n<p>Contenu fran\u{00e7}ais</p>\n",
    )
    .unwrap();

    // Build the site
    let output_dir = site_dir.join("_site");
    mini_build(site_dir, &output_dir);

    let output = std::fs::read_to_string(output_dir.join("index.html")).unwrap();
    assert!(
        output.contains(&format!("style.css?v={expected_sha}")),
        "Output should contain CSS version hash with SHA. Output:\n{output}"
    );
    assert!(
        !output.contains("?v=''"),
        "Output should not contain empty ?v=''. Output:\n{output}"
    );
    assert!(
        !output.contains("?v=\"\""),
        "Output should not contain empty ?v=\"\". Output:\n{output}"
    );
}

#[test]
fn test_jsonld_breadcrumb_renders_absolute_urls() {
    let tmp = tempfile::tempdir().unwrap();
    let site_dir = tmp.path();
    let _sha = init_git_repo(site_dir);

    // Create _config.yml with non-ASCII site name
    std::fs::write(
        site_dir.join("_config.yml"),
        "url: \"https://example.com\"\nname: \"\u{041f}\u{0440}\u{043e}\u{0435}\u{043a}\u{0442}\"\ntitle: \"\u{041f}\u{0440}\u{043e}\u{0435}\u{043a}\u{0442}\"\n",
    )
    .unwrap();

    // Create _layouts directory with a layout that uses site.github.url
    std::fs::create_dir_all(site_dir.join("_layouts")).unwrap();
    std::fs::write(
        site_dir.join("_layouts").join("default.html"),
        "<html><body>\
         <script type=\"application/ld+json\">\
         {\"@id\": \"{{ site.github.url }}{{ page.url }}\"}\
         </script>\
         <h1>\u{041e} \u{043f}\u{0440}\u{043e}\u{0435}\u{043a}\u{0442}\u{0435}</h1>\
         {{ content }}</body></html>",
    )
    .unwrap();

    // Create about/index.html
    std::fs::create_dir_all(site_dir.join("about")).unwrap();
    std::fs::write(
        site_dir.join("about").join("index.html"),
        "---\nlayout: default\ntitle: \"\u{041e} \u{043f}\u{0440}\u{043e}\u{0435}\u{043a}\u{0442}\u{0435}\"\npermalink: /about/\n---\n<p>\u{0422}\u{0435}\u{043a}\u{0441}\u{0442}</p>\n",
    )
    .unwrap();

    // Build the site
    let output_dir = site_dir.join("_site");
    mini_build(site_dir, &output_dir);

    let output = std::fs::read_to_string(output_dir.join("about").join("index.html")).unwrap();
    assert!(
        output.contains("\"@id\": \"https://example.com/about/\""),
        "Output should contain absolute URL in JSON-LD. Output:\n{output}"
    );
    assert!(
        !output.contains("\"@id\": \"/about/\""),
        "Output should not contain relative URL in JSON-LD. Output:\n{output}"
    );
}
