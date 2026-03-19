//! Unit tests for performance-related features (Issues 49, 57).
//!
//! Large-site integration tests have been moved to
//! integration_tests/tests/integration_performance.rs.

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use rustkyll::template::engine::CachedSiteContext;

// ============================================================================
// Unit: CachedSiteContext
// ============================================================================

#[test]
fn test_cached_site_context_creation() {
    use liquid::model::Value as LiquidValue;
    use liquid::Object;

    // Build a large site Object with 1000+ keys
    let mut site = Object::new();
    let mut posts = Vec::new();
    for i in 0..1000 {
        let mut post = Object::new();
        post.insert("title".into(), LiquidValue::scalar(format!("Post {}", i)));
        post.insert("url".into(), LiquidValue::scalar(format!("/post/{}", i)));
        post.insert("content".into(), LiquidValue::scalar("Some content here"));
        posts.push(LiquidValue::Object(post));
    }
    site.insert("posts".into(), LiquidValue::Array(posts));
    site.insert("url".into(), LiquidValue::scalar("https://example.com"));

    // CachedSiteContext::new should succeed and be reusable
    let cached = CachedSiteContext::new(&site);

    // Verify it can be used multiple times (the whole point of caching)
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let template = engine.parse("{{ site.url }}").unwrap();

    let mut ctx = Object::new();
    ctx.insert("page".into(), LiquidValue::Object(Object::new()));
    ctx.insert("content".into(), LiquidValue::scalar(""));

    let result1 = engine
        .render_with_cached_site(&template, &ctx, &cached)
        .unwrap();
    let result2 = engine
        .render_with_cached_site(&template, &ctx, &cached)
        .unwrap();

    assert_eq!(result1, "https://example.com");
    assert_eq!(result2, "https://example.com");
}

#[test]
fn test_cached_site_context_page_specific_variables() {
    use liquid::model::Value as LiquidValue;
    use liquid::Object;

    let mut site = Object::new();
    site.insert("name".into(), LiquidValue::scalar("Test Site"));

    let cached = CachedSiteContext::new(&site);
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let template = engine.parse("{{ page.title }} on {{ site.name }}").unwrap();

    // Render with different page titles using the same cached site
    let mut ctx1 = Object::new();
    let mut page1 = Object::new();
    page1.insert("title".into(), LiquidValue::scalar("Page One"));
    ctx1.insert("page".into(), LiquidValue::Object(page1));
    ctx1.insert("content".into(), LiquidValue::scalar(""));

    let mut ctx2 = Object::new();
    let mut page2 = Object::new();
    page2.insert("title".into(), LiquidValue::scalar("Page Two"));
    ctx2.insert("page".into(), LiquidValue::Object(page2));
    ctx2.insert("content".into(), LiquidValue::scalar(""));

    let result1 = engine
        .render_with_cached_site(&template, &ctx1, &cached)
        .unwrap();
    let result2 = engine
        .render_with_cached_site(&template, &ctx2, &cached)
        .unwrap();

    assert_eq!(result1, "Page One on Test Site");
    assert_eq!(result2, "Page Two on Test Site");
}

// ============================================================================
// Unit: Slim site context optimization (Issue 57)
// ============================================================================

#[test]
fn test_slim_site_context_excludes_large_arrays() {
    use liquid::model::Value as LiquidValue;
    use rustkyll::collection::CollectionItem;
    use rustkyll::config::SiteConfig;
    use rustkyll::generator;

    let mut fm = HashMap::new();
    fm.insert(
        "title".to_string(),
        serde_yaml::Value::String("Test Episode".to_string()),
    );
    fm.insert(
        "season".to_string(),
        serde_yaml::Value::Number(serde_yaml::Number::from(1)),
    );
    let transcript: Vec<serde_yaml::Value> = (0..100)
        .map(|i| {
            let mut map = serde_yaml::Mapping::new();
            map.insert(
                serde_yaml::Value::String("line".to_string()),
                serde_yaml::Value::String(format!("Line {}", i)),
            );
            serde_yaml::Value::Mapping(map)
        })
        .collect();
    fm.insert(
        "transcript".to_string(),
        serde_yaml::Value::Sequence(transcript),
    );
    fm.insert(
        "guests".to_string(),
        serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("alice".to_string()),
            serde_yaml::Value::String("bob".to_string()),
        ]),
    );

    let item = CollectionItem {
        slug: "test-episode".to_string(),
        url: "/podcast/test-episode.html".to_string(),
        date: Some("2024-01-01".to_string()),
        front_matter: fm,
        content: "raw content".to_string(),
        html_content: "<p>rendered content</p>".to_string(),
        excerpt: None,
        collection_name: "podcast".to_string(),
        source_path: "_podcast/test-episode.md".to_string(),
        id: "/podcast/test-episode".to_string(),
    };

    let mut collections = HashMap::new();
    collections.insert("podcast".to_string(), vec![item]);

    let config = SiteConfig::default();
    let data = BTreeMap::new();
    let pages = vec![];

    let site = generator::build_site_context(&config, &collections, &data, None, &pages);

    if let Some(LiquidValue::Array(podcast_arr)) = site.get("podcast") {
        assert_eq!(podcast_arr.len(), 1);

        if let LiquidValue::Object(ref episode) = podcast_arr[0] {
            assert!(
                episode.get("title").is_some(),
                "Title should be in slim site context"
            );
            assert!(
                episode.get("season").is_some(),
                "Season should be in slim site context"
            );
            assert!(
                episode.get("guests").is_some(),
                "Small array (guests) should be in slim site context"
            );
            assert!(
                episode.get("transcript").is_none(),
                "Large array (transcript) should be excluded from slim site context"
            );
            assert!(
                episode.get("content").is_some(),
                "Content should be in slim site context"
            );
        } else {
            panic!("Expected Object in podcast array");
        }
    } else {
        panic!("Expected podcast array in site context");
    }
}

#[test]
fn test_slim_site_context_keeps_small_arrays() {
    use rustkyll::collection::CollectionItem;
    use rustkyll::config::SiteConfig;
    use rustkyll::generator;

    let mut fm = HashMap::new();
    fm.insert(
        "title".to_string(),
        serde_yaml::Value::String("Alice Smith".to_string()),
    );
    fm.insert(
        "short".to_string(),
        serde_yaml::Value::String("alice".to_string()),
    );
    fm.insert(
        "tags".to_string(),
        serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("ml".to_string()),
            serde_yaml::Value::String("data".to_string()),
            serde_yaml::Value::String("ai".to_string()),
        ]),
    );

    let item = CollectionItem {
        slug: "alice".to_string(),
        url: "/people/alice.html".to_string(),
        date: None,
        front_matter: fm,
        content: "Bio here".to_string(),
        html_content: "<p>Bio here</p>".to_string(),
        excerpt: None,
        collection_name: "people".to_string(),
        source_path: "_people/alice.md".to_string(),
        id: "/people/alice".to_string(),
    };

    let mut collections = HashMap::new();
    collections.insert("people".to_string(), vec![item]);

    let config = SiteConfig::default();
    let data = BTreeMap::new();
    let pages = vec![];

    let site = generator::build_site_context(&config, &collections, &data, None, &pages);

    if let Some(liquid::model::Value::Array(people_arr)) = site.get("people") {
        assert_eq!(people_arr.len(), 1);
        if let liquid::model::Value::Object(ref person) = people_arr[0] {
            assert!(person.get("title").is_some(), "Title should be present");
            assert!(person.get("short").is_some(), "Short should be present");
            assert!(
                person.get("tags").is_some(),
                "Small array should be present"
            );
            assert!(person.get("content").is_some(), "Content should be present");
        } else {
            panic!("Expected Object");
        }
    } else {
        panic!("Expected people array");
    }
}

// ============================================================================
// Unit: Performance micro-benchmarks (Issue 57)
// ============================================================================

#[test]
fn test_render_1000_for_loop_iterations() {
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let mut ctx = liquid::Object::new();

    let items: Vec<liquid::model::Value> = (0..1000)
        .map(|i| liquid::model::Value::scalar(format!("item-{}", i)))
        .collect();
    ctx.insert("items".into(), liquid::model::Value::Array(items));

    let template_str = "{% for item in items %}{{ item }}\n{% endfor %}";

    let start = Instant::now();
    let result = engine.parse_and_render(template_str, &ctx);
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Template should render successfully");
    let output = result.unwrap();
    assert!(
        output.contains("item-0"),
        "Output should contain first item"
    );
    assert!(
        output.contains("item-999"),
        "Output should contain last item"
    );
    assert!(
        elapsed.as_millis() < 500,
        "1000 for-loop iterations should complete in under 500ms, took {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_render_deeply_nested_object_access() {
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let mut ctx = liquid::Object::new();

    let mut people = Vec::new();
    for i in 0..100 {
        let mut person = liquid::Object::new();
        person.insert(
            "name".into(),
            liquid::model::Value::scalar(format!("Person {}", i)),
        );
        person.insert(
            "bio".into(),
            liquid::model::Value::scalar("A short biography"),
        );
        people.push(liquid::model::Value::Object(person));
    }

    let mut data_obj = liquid::Object::new();
    data_obj.insert("people".into(), liquid::model::Value::Array(people));

    let mut site = liquid::Object::new();
    site.insert("data".into(), liquid::model::Value::Object(data_obj));

    ctx.insert("site".into(), liquid::model::Value::Object(site));

    let template_str = "{% for person in site.data.people %}{{ person.name }}\n{% endfor %}";

    let start = Instant::now();
    let result = engine.parse_and_render(template_str, &ctx);
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Person 0"));
    assert!(output.contains("Person 99"));
    assert!(
        elapsed.as_millis() < 50,
        "Deeply nested access should complete in under 50ms, took {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_render_100_templates_sequentially() {
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();

    let start = Instant::now();
    for i in 0..100 {
        let mut ctx = liquid::Object::new();
        ctx.insert("n".into(), liquid::model::Value::scalar(i as i64));
        ctx.insert(
            "title".into(),
            liquid::model::Value::scalar(format!("Page {}", i)),
        );

        let template_str = "<h1>{{ title }}</h1><p>Number: {{ n }}</p>";
        let result = engine.parse_and_render(template_str, &ctx);
        assert!(result.is_ok());
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 200,
        "100 sequential template renders should complete in under 200ms, took {}ms",
        elapsed.as_millis()
    );
}
