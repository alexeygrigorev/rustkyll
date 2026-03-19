//! Integration tests for site context building.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

use liquid::model::{Value as LiquidValue, ValueView};

use rustkyll::collection;
use rustkyll::config::SiteConfig;
use rustkyll::data;
use rustkyll::generator;

fn site_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datatalksclub.github.io")
}

static CONFIG: LazyLock<SiteConfig> =
    LazyLock::new(|| SiteConfig::from_file(&site_dir().join("_config.yml")).unwrap());

static DATA: LazyLock<data::DataTree> =
    LazyLock::new(|| data::load_data(&site_dir().join("_data")).unwrap());

#[test]
fn test_site_context_has_data_events() {
    if !site_dir().exists() {
        return;
    }
    let colls = HashMap::new();
    let ctx = generator::build_site_context(&CONFIG, &colls, &DATA, None, &[]);

    let data_val = ctx.get("data").expect("site should have data");
    if let LiquidValue::Object(data_obj) = data_val {
        let events = data_obj.get("events").expect("data should have events");
        if let LiquidValue::Array(arr) = events {
            assert!(arr.len() > 100, "Expected 100+ events, got {}", arr.len());
        } else {
            panic!("Expected events to be an array");
        }
    } else {
        panic!("Expected data to be an object");
    }
}

#[test]
fn test_site_context_github_url_resolves() {
    if !site_dir().exists() {
        return;
    }
    let colls = HashMap::new();
    let data = data::DataTree::new();
    let ctx = generator::build_site_context(&CONFIG, &colls, &data, Some(&site_dir()), &[]);

    // DTC config does NOT have jekyll-github-metadata in its plugins list,
    // so site.github.repository_url should be Nil (not resolved from git remote).
    // This verifies the gating behavior introduced in issue 229.
    let github = ctx.get("github").expect("site should have github");
    if let LiquidValue::Object(github_obj) = github {
        let repo_url = github_obj
            .get("repository_url")
            .expect("should have repository_url");
        assert_eq!(
            *repo_url,
            LiquidValue::Nil,
            "repo URL should be nil when jekyll-github-metadata plugin is not configured"
        );
    } else {
        panic!("Expected github to be an object");
    }
}

#[test]
fn test_site_context_has_url_and_name() {
    if !site_dir().exists() {
        return;
    }
    let colls = HashMap::new();
    let data = data::DataTree::new();
    let ctx = generator::build_site_context(&CONFIG, &colls, &data, None, &[]);

    assert_eq!(
        ctx.get("url"),
        Some(&LiquidValue::scalar("https://datatalks.club"))
    );
    assert_eq!(
        ctx.get("name"),
        Some(&LiquidValue::scalar("DataTalks.Club"))
    );
}

#[test]
fn test_site_context_multiple_collections() {
    if !site_dir().exists() {
        return;
    }
    let (posts, _) = collection::load_collection("posts", &site_dir(), &CONFIG).unwrap();
    let (books, _) = collection::load_collection("books", &site_dir(), &CONFIG).unwrap();
    let (people, _) = collection::load_collection("people", &site_dir(), &CONFIG).unwrap();
    let (podcast, _) = collection::load_collection("podcast", &site_dir(), &CONFIG).unwrap();

    let mut colls = HashMap::new();
    colls.insert("posts".to_string(), posts);
    colls.insert("books".to_string(), books);
    colls.insert("people".to_string(), people);
    colls.insert("podcast".to_string(), podcast);

    let data = data::DataTree::new();
    let ctx = generator::build_site_context(&CONFIG, &colls, &data, None, &[]);

    assert!(ctx.get("posts").is_some(), "Should have posts");
    assert!(ctx.get("books").is_some(), "Should have books");
    assert!(ctx.get("people").is_some(), "Should have people");
    assert!(ctx.get("podcast").is_some(), "Should have podcast");
}
