//! Integration tests for people page generation.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use liquid::model::ValueView;
use liquid::Object;

use rustkyll::collection::{self, CollectionItem};
use rustkyll::config::SiteConfig;
use rustkyll::data;
use rustkyll::generator;
use rustkyll::template::layout::LayoutEngine;

fn site_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datatalksclub.github.io")
}

static CONFIG: LazyLock<SiteConfig> =
    LazyLock::new(|| SiteConfig::from_file(&site_dir().join("_config.yml")).unwrap());

struct PeopleFixture {
    people: Vec<CollectionItem>,
    site_context: Object,
    layout_engine: LayoutEngine,
}

static FIXTURE: LazyLock<PeopleFixture> = LazyLock::new(|| {
    let (people, _) = collection::load_collection("people", &site_dir(), &CONFIG).unwrap();
    let (posts, _) = collection::load_collection("posts", &site_dir(), &CONFIG).unwrap();
    let (books, _) = collection::load_collection("books", &site_dir(), &CONFIG).unwrap();
    let data_tree = data::load_data(&site_dir().join("_data")).unwrap();

    let mut colls = HashMap::new();
    colls.insert("posts".to_string(), posts);
    colls.insert("books".to_string(), books);
    colls.insert("people".to_string(), people.clone());

    let site_context =
        generator::build_site_context(&CONFIG, &colls, &data_tree, Some(&site_dir()), &[]);
    let layout_engine =
        LayoutEngine::new(&site_dir().join("_layouts"), &site_dir().join("_includes")).unwrap();

    PeopleFixture {
        people,
        site_context,
        layout_engine,
    }
});

static PEOPLE_OUTPUT: LazyLock<tempfile::TempDir> = LazyLock::new(|| {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = generator::generate_collection_pages(
        &FIXTURE.people,
        "people",
        &CONFIG,
        &FIXTURE.layout_engine,
        &FIXTURE.site_context,
        tmp.path(),
    )
    .unwrap();
    assert!(
        result.generated >= 424,
        "Expected 424+ people pages, got {} generated ({} skipped, {} errors)",
        result.generated,
        result.skipped,
        result.errors.len(),
    );
    tmp
});

#[test]
fn test_generate_people_pages_count() {
    // Force lazy evaluation
    let _output = &*PEOPLE_OUTPUT;
}

#[test]
fn test_generated_alexeygrigorev_content() {
    let html = fs::read_to_string(PEOPLE_OUTPUT.path().join("people/alexeygrigorev.html")).unwrap();

    assert!(html.contains("Alexey Grigorev"), "Should contain name");
    assert!(
        html.contains("images/authors/alexeygrigorev.jpg"),
        "Should contain profile image path"
    );
    assert!(
        html.contains("https://twitter.com/Al_Grigor"),
        "Should contain Twitter link"
    );
    assert!(
        html.contains("https://linkedin.com/in/agrigorev"),
        "Should contain LinkedIn link"
    );
    assert!(
        html.contains("https://github.com/alexeygrigorev"),
        "Should contain GitHub link"
    );
    assert!(
        html.contains("https://alexeygrigorev.com/"),
        "Should contain web link"
    );
    assert!(
        html.contains("founder of DataTalks.Club"),
        "Should contain bio"
    );
}

#[test]
fn test_generated_chiphuyen_content() {
    let html = fs::read_to_string(PEOPLE_OUTPUT.path().join("people/chiphuyen.html")).unwrap();

    assert!(html.contains("Chip Huyen"), "Should contain name");
    assert!(
        html.contains("Stanford University"),
        "Should contain Stanford mention"
    );
    assert!(
        html.contains("twitter.com/chipro"),
        "Should contain Twitter"
    );
    assert!(
        html.contains("linkedin.com/in/chiphuyen"),
        "Should contain LinkedIn"
    );
    assert!(
        html.contains("github.com/chiphuyen"),
        "Should contain GitHub"
    );
    assert!(html.contains("huyenchip.com"), "Should contain web");
}

#[test]
fn test_generated_alexeygrigorev_has_jsonld() {
    let html = fs::read_to_string(PEOPLE_OUTPUT.path().join("people/alexeygrigorev.html")).unwrap();

    assert!(
        html.contains(r#"<script type="application/ld+json">"#),
        "Should contain JSON-LD script block"
    );
    assert!(
        html.contains(r#""@type": "Person""#),
        "Should contain Person type in JSON-LD"
    );
    assert!(
        html.contains("Alexey Grigorev"),
        "JSON-LD should contain person name"
    );
}

#[test]
fn test_generated_alexeygrigorev_has_articles_section() {
    let html = fs::read_to_string(PEOPLE_OUTPUT.path().join("people/alexeygrigorev.html")).unwrap();

    assert!(
        html.contains("<h3>Articles</h3>"),
        "Should contain Articles section for a person with posts"
    );
}

#[test]
fn test_people_array_includes_short_and_content() {
    let (people, _) = collection::load_collection("people", &site_dir(), &CONFIG).unwrap();

    let mut colls = HashMap::new();
    colls.insert("people".to_string(), people);
    let data = data::DataTree::new();
    let ctx = generator::build_site_context(&CONFIG, &colls, &data, None, &[]);

    if let Some(liquid::model::Value::Array(arr)) = ctx.get("people") {
        let alexey = arr.iter().find(|item| {
            if let liquid::model::Value::Object(obj) = item {
                obj.get("short")
                    .map(|v| v.render().to_string() == "alexeygrigorev")
                    .unwrap_or(false)
            } else {
                false
            }
        });
        assert!(
            alexey.is_some(),
            "Expected to find alexeygrigorev in people array"
        );
        if let Some(liquid::model::Value::Object(obj)) = alexey {
            assert!(
                obj.get("title").is_some(),
                "People object should have title"
            );
            assert!(
                obj.get("content").is_some(),
                "People object should have content"
            );
        }
    } else {
        panic!("Expected people Array in context");
    }
}
