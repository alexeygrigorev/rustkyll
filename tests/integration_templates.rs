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
    if !site_dir().exists() {
        return;
    }
    let engine = &*LAYOUT_ENGINE;
    assert_eq!(engine.layout_names().len(), 6);
}

#[test]
fn test_render_home_layout_with_real_includes() {
    if !site_dir().exists() {
        return;
    }
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
    if !site_dir().exists() {
        return;
    }
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

// ========================================================================
// Issue 29: page.previous / page.next template rendering
// ========================================================================

#[test]
fn test_prev_next_template_renders_next_title() {
    if !site_dir().exists() {
        return;
    }
    let engine = rustkyll::template::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();

    let mut next_obj = Object::new();
    next_obj.insert("title".into(), LiquidValue::scalar("Next Post Title"));
    next_obj.insert("url".into(), LiquidValue::scalar("/blog/next.html"));
    ctx.insert(
        "page".into(),
        LiquidValue::Object({
            let mut page = Object::new();
            page.insert("next".into(), LiquidValue::Object(next_obj));
            page
        }),
    );

    let result = engine
        .parse_and_render("{{ page.next.title }}", &ctx)
        .unwrap();
    assert_eq!(result.trim(), "Next Post Title");
}

#[test]
fn test_prev_next_template_conditional_first_post() {
    if !site_dir().exists() {
        return;
    }
    let engine = rustkyll::template::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();

    ctx.insert(
        "page".into(),
        LiquidValue::Object({
            let mut page = Object::new();
            page.insert("previous".into(), LiquidValue::Nil);
            page.insert(
                "next".into(),
                LiquidValue::Object({
                    let mut next = Object::new();
                    next.insert("title".into(), LiquidValue::scalar("Second"));
                    next
                }),
            );
            page
        }),
    );

    let result = engine
        .parse_and_render("{% if page.previous %}yes{% else %}no{% endif %}", &ctx)
        .unwrap();
    assert_eq!(result.trim(), "no");
}

#[test]
fn test_prev_next_template_conditional_last_post() {
    if !site_dir().exists() {
        return;
    }
    let engine = rustkyll::template::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();

    ctx.insert(
        "page".into(),
        LiquidValue::Object({
            let mut page = Object::new();
            page.insert(
                "previous".into(),
                LiquidValue::Object({
                    let mut prev = Object::new();
                    prev.insert("title".into(), LiquidValue::scalar("First"));
                    prev
                }),
            );
            page.insert("next".into(), LiquidValue::Nil);
            page
        }),
    );

    let result = engine
        .parse_and_render("{% if page.next %}yes{% else %}no{% endif %}", &ctx)
        .unwrap();
    assert_eq!(result.trim(), "no");
}

#[test]
fn test_prev_next_template_renders_previous_url() {
    if !site_dir().exists() {
        return;
    }
    let engine = rustkyll::template::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();

    ctx.insert(
        "page".into(),
        LiquidValue::Object({
            let mut page = Object::new();
            page.insert(
                "previous".into(),
                LiquidValue::Object({
                    let mut prev = Object::new();
                    prev.insert("url".into(), LiquidValue::scalar("/blog/prev-post.html"));
                    prev.insert("title".into(), LiquidValue::scalar("Previous"));
                    prev
                }),
            );
            page
        }),
    );

    let result = engine
        .parse_and_render("{{ page.previous.url }}", &ctx)
        .unwrap();
    assert_eq!(result.trim(), "/blog/prev-post.html");
}

// ========================================================================
// Issue 30: Missing filters template rendering
// ========================================================================

#[test]
fn test_number_of_words_integration() {
    if !site_dir().exists() {
        return;
    }
    let engine = rustkyll::template::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();
    ctx.insert(
        "content".into(),
        LiquidValue::scalar("hello beautiful world"),
    );

    let result = engine
        .parse_and_render("{{ content | number_of_words }}", &ctx)
        .unwrap();
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_number_of_words_auto_mode_integration() {
    if !site_dir().exists() {
        return;
    }
    let engine = rustkyll::template::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();
    ctx.insert(
        "content".into(),
        LiquidValue::scalar("Hello \u{4e16}\u{754c}"),
    );

    // 'auto' mode: 1 Latin word + 2 CJK characters = 3
    let result = engine
        .parse_and_render("{{ content | number_of_words: 'auto' }}", &ctx)
        .unwrap();
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_number_of_words_no_arg_regression_integration() {
    if !site_dir().exists() {
        return;
    }
    let engine = rustkyll::template::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();
    ctx.insert("content".into(), LiquidValue::scalar("one two three four"));

    // No argument: whitespace splitting
    let result = engine
        .parse_and_render("{{ content | number_of_words }}", &ctx)
        .unwrap();
    assert_eq!(result.trim(), "4");
}

#[test]
fn test_xml_escape_integration() {
    if !site_dir().exists() {
        return;
    }
    let engine = rustkyll::template::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();
    ctx.insert(
        "text".into(),
        LiquidValue::scalar("<b>bold & \"quoted\"</b>"),
    );

    let result = engine
        .parse_and_render("{{ text | xml_escape }}", &ctx)
        .unwrap();
    assert_eq!(
        result.trim(),
        "&lt;b&gt;bold &amp; &quot;quoted&quot;&lt;/b&gt;"
    );
}

#[test]
fn test_truncatewords_integration() {
    if !site_dir().exists() {
        return;
    }
    let engine = rustkyll::template::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();
    ctx.insert("text".into(), LiquidValue::scalar("a b c d"));

    let result = engine
        .parse_and_render("{{ text | truncatewords: 2 }}", &ctx)
        .unwrap();
    assert_eq!(result.trim(), "a b...");
}

#[test]
fn test_group_by_integration() {
    if !site_dir().exists() {
        return;
    }
    let engine = rustkyll::template::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();

    let items = LiquidValue::Array(vec![
        LiquidValue::Object({
            let mut o = Object::new();
            o.insert("type".into(), LiquidValue::scalar("fruit"));
            o.insert("name".into(), LiquidValue::scalar("apple"));
            o
        }),
        LiquidValue::Object({
            let mut o = Object::new();
            o.insert("type".into(), LiquidValue::scalar("veggie"));
            o.insert("name".into(), LiquidValue::scalar("carrot"));
            o
        }),
        LiquidValue::Object({
            let mut o = Object::new();
            o.insert("type".into(), LiquidValue::scalar("fruit"));
            o.insert("name".into(), LiquidValue::scalar("banana"));
            o
        }),
    ]);
    ctx.insert("items".into(), items);

    let result = engine
        .parse_and_render(
            "{% assign groups = items | group_by: \"type\" %}{% for g in groups %}{{ g.name }}:{{ g.size }};{% endfor %}",
            &ctx,
        )
        .unwrap();
    // BTreeMap ensures deterministic ordering: "fruit" before "veggie"
    assert_eq!(result.trim(), "fruit:2;veggie:1;");
}

// ========================================================================
// Unit: `reversed` modifier in for loops
// ========================================================================

#[test]
fn test_for_loop_reversed() {
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();
    ctx.insert(
        "items".into(),
        LiquidValue::Array(vec![
            LiquidValue::scalar("a"),
            LiquidValue::scalar("b"),
            LiquidValue::scalar("c"),
        ]),
    );
    let result = engine
        .parse_and_render(
            "{% for item in items reversed %}{{ item }} {% endfor %}",
            &ctx,
        )
        .unwrap();
    assert_eq!(result, "c b a ");
}

#[test]
fn test_for_loop_reversed_with_limit() {
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();
    ctx.insert(
        "items".into(),
        LiquidValue::Array(vec![
            LiquidValue::scalar("a"),
            LiquidValue::scalar("b"),
            LiquidValue::scalar("c"),
            LiquidValue::scalar("d"),
        ]),
    );
    let result = engine
        .parse_and_render(
            "{% for item in items reversed limit:2 %}{{ item }} {% endfor %}",
            &ctx,
        )
        .unwrap();
    // The liquid crate applies limit first (takes first 2: a, b), then reverses (b, a)
    assert_eq!(result, "b a ");
}

#[test]
fn test_for_loop_reversed_empty_array() {
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let mut ctx = Object::new();
    ctx.insert("items".into(), LiquidValue::Array(vec![]));
    let result = engine
        .parse_and_render(
            "{% for item in items reversed %}{{ item }} {% endfor %}",
            &ctx,
        )
        .unwrap();
    assert_eq!(result, "");
}

// ========================================================================
// Unit: Template rendering with collection iteration
// ========================================================================

#[test]
fn test_for_loop_over_site_collection() {
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let mut site = Object::new();
    let stories: Vec<LiquidValue> = (0..5)
        .map(|i| {
            let mut obj = Object::new();
            obj.insert("title".into(), LiquidValue::scalar(format!("Story {}", i)));
            LiquidValue::Object(obj)
        })
        .collect();
    site.insert("stories".into(), LiquidValue::Array(stories));

    let mut ctx = Object::new();
    ctx.insert("site".into(), LiquidValue::Object(site));

    let result = engine
        .parse_and_render("{% for post in site.stories %}X{% endfor %}", &ctx)
        .unwrap();
    assert_eq!(result, "XXXXX");
}

#[test]
fn test_for_loop_over_site_collection_reversed() {
    let engine = rustkyll::template::engine::TemplateEngine::new().unwrap();
    let mut site = Object::new();
    let stories: Vec<LiquidValue> = (1..=3)
        .map(|i| {
            let mut obj = Object::new();
            obj.insert("num".into(), LiquidValue::scalar(i as i64));
            LiquidValue::Object(obj)
        })
        .collect();
    site.insert("stories".into(), LiquidValue::Array(stories));

    let mut ctx = Object::new();
    ctx.insert("site".into(), LiquidValue::Object(site));

    let result = engine
        .parse_and_render(
            "{% for post in site.stories reversed %}{{ post.num }} {% endfor %}",
            &ctx,
        )
        .unwrap();
    assert_eq!(result, "3 2 1 ");
}
