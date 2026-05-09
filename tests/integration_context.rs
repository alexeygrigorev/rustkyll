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
fn test_site_context_github_url_nil_without_plugin() {
    if !site_dir().exists() {
        return;
    }
    let colls = HashMap::new();
    let data = data::DataTree::new();
    let ctx = generator::build_site_context(&CONFIG, &colls, &data, Some(&site_dir()), &[]);

    // The checked-in DTC fixture has a github-pages Gemfile, which
    // auto-activates jekyll-github-metadata for local Bundler builds.
    let github = ctx.get("github").expect("site should have github");
    if let LiquidValue::Object(github_obj) = github {
        assert!(
            github_obj.get("repository_url").is_some(),
            "repository_url should be populated when the github-pages Gemfile is present"
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

/// Issue #585: site.collections must be an iterable array with .label, .docs, .output
#[test]
fn test_site_collections_is_iterable_array() {
    use rustkyll::collection::CollectionItem;
    use rustkyll::config::CollectionConfig;

    // Build a config with 2 custom collections + posts
    let mut config = SiteConfig::default();
    config.collections.insert(
        "portfolio".to_string(),
        CollectionConfig {
            output: true,
            permalink: String::new(),
            sort_by: None,
        },
    );
    config.collections.insert(
        "talks".to_string(),
        CollectionConfig {
            output: false,
            permalink: String::new(),
            sort_by: None,
        },
    );
    config.collections.insert(
        "posts".to_string(),
        CollectionConfig {
            output: true,
            permalink: "/:title/".to_string(),
            sort_by: None,
        },
    );

    // Create collection items with non-ASCII titles
    let mut portfolio_fm = HashMap::new();
    portfolio_fm.insert(
        "title".to_string(),
        serde_yaml::Value::String("Proj\u{00e9}ct Alpha".to_string()),
    );
    let mut talks_fm = HashMap::new();
    talks_fm.insert(
        "title".to_string(),
        serde_yaml::Value::String("\u{00dc}ber Talk".to_string()),
    );

    let mut colls: HashMap<String, Vec<CollectionItem>> = HashMap::new();
    colls.insert(
        "portfolio".to_string(),
        vec![CollectionItem {
            slug: "project-alpha".to_string(),
            front_matter: portfolio_fm,
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            excerpt_html: None,
            url: "/portfolio/project-alpha/".to_string(),
            date: None,
            collection_name: "portfolio".to_string(),
            source_path: "_portfolio/project-alpha.md".to_string(),
            id: "/portfolio/project-alpha".to_string(),
        }],
    );
    colls.insert(
        "talks".to_string(),
        vec![CollectionItem {
            slug: "uber-talk".to_string(),
            front_matter: talks_fm,
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            excerpt_html: None,
            url: "/talks/uber-talk/".to_string(),
            date: None,
            collection_name: "talks".to_string(),
            source_path: "_talks/uber-talk.md".to_string(),
            id: "/talks/uber-talk".to_string(),
        }],
    );
    colls.insert("posts".to_string(), vec![]);

    let data = data::DataTree::new();
    let ctx = generator::build_site_context(&config, &colls, &data, None, &[]);

    // site.collections must exist and be an array
    let collections_val = ctx
        .get("collections")
        .expect("site.collections should exist");
    let collections_arr = match collections_val {
        LiquidValue::Array(arr) => arr,
        other => panic!(
            "site.collections should be an array, got: {:?}",
            other.type_name()
        ),
    };

    // Should have 3 collections (portfolio, posts, talks)
    assert_eq!(
        collections_arr.len(),
        3,
        "Expected 3 collections, got {}",
        collections_arr.len()
    );

    // Collections should be sorted alphabetically by label
    let labels: Vec<String> = collections_arr
        .iter()
        .map(|c| {
            if let LiquidValue::Object(obj) = c {
                if let Some(LiquidValue::Scalar(s)) = obj.get("label") {
                    s.to_kstr().to_string()
                } else {
                    panic!("collection should have .label as scalar");
                }
            } else {
                panic!("each collection should be an object");
            }
        })
        .collect();
    assert_eq!(labels, vec!["portfolio", "posts", "talks"]);

    // Check portfolio: output=true, 1 doc with non-ASCII title
    if let LiquidValue::Object(portfolio) = &collections_arr[0] {
        // .output should be true
        assert_eq!(
            portfolio.get("output"),
            Some(&LiquidValue::scalar(true)),
            "portfolio.output should be true"
        );
        // .docs should be an array with 1 item
        if let Some(LiquidValue::Array(docs)) = portfolio.get("docs") {
            assert_eq!(docs.len(), 1, "portfolio should have 1 doc");
        } else {
            panic!("portfolio.docs should be an array");
        }
    }

    // Check talks: output=false
    if let LiquidValue::Object(talks) = &collections_arr[2] {
        assert_eq!(
            talks.get("output"),
            Some(&LiquidValue::scalar(false)),
            "talks.output should be false"
        );
    }

    // Check posts included
    assert!(
        labels.contains(&"posts".to_string()),
        "posts collection should be in site.collections"
    );
}
