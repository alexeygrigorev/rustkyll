//! Integration tests for book page generation.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

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

struct BooksFixture {
    books: Vec<CollectionItem>,
    site_context: Object,
    layout_engine: LayoutEngine,
}

static FIXTURE: LazyLock<BooksFixture> = LazyLock::new(|| {
    let (books, _) = collection::load_collection("books", &site_dir(), &CONFIG).unwrap();
    let (people, _) = collection::load_collection("people", &site_dir(), &CONFIG).unwrap();
    let (posts, _) = collection::load_collection("posts", &site_dir(), &CONFIG).unwrap();
    let data_tree = data::load_data(&site_dir().join("_data")).unwrap();

    let mut colls = HashMap::new();
    colls.insert("posts".to_string(), posts);
    colls.insert("books".to_string(), books.clone());
    colls.insert("people".to_string(), people);

    let site_context =
        generator::build_site_context(&CONFIG, &colls, &data_tree, Some(&site_dir()), &[]);
    let layout_engine =
        LayoutEngine::new(&site_dir().join("_layouts"), &site_dir().join("_includes")).unwrap();

    BooksFixture {
        books,
        site_context,
        layout_engine,
    }
});

fn render_book(slug: &str) -> String {
    let book = FIXTURE.books.iter().find(|b| b.slug == slug).unwrap();
    let mut fm = book.front_matter.clone();
    fm.insert("url".into(), serde_yaml::Value::String(book.url.clone()));
    FIXTURE
        .layout_engine
        .render_page("book", &book.html_content, &fm, &FIXTURE.site_context)
        .unwrap()
}

static BOOKS_OUTPUT: LazyLock<tempfile::TempDir> = LazyLock::new(|| {
    let tmp = tempfile::TempDir::new().unwrap();
    let result = generator::generate_collection_pages(
        &FIXTURE.books,
        "books",
        &CONFIG,
        &FIXTURE.layout_engine,
        &FIXTURE.site_context,
        tmp.path(),
    )
    .unwrap();

    assert!(
        result.generated >= 97,
        "Expected at least 97 books generated, got {} (skipped: {}, errors: {:?})",
        result.generated,
        result.skipped,
        result.errors
    );
    assert_eq!(
        result.generated + result.errors.len(),
        98,
        "Expected 98 total books attempted (99 files minus _template.md)"
    );
    tmp
});

#[test]
fn test_generate_all_books_count() {
    if !site_dir().exists() {
        return;
    }
    let _output = &*BOOKS_OUTPUT;
}

#[test]
fn test_render_ml_bookcamp() {
    if !site_dir().exists() {
        return;
    }
    let html = render_book("20201214-ml-bookcamp");

    assert!(html.contains("Machine Learning Bookcamp"));
    assert!(html.contains("Alexey Grigorev"));
    assert!(html.contains("/people/alexeygrigorev.html"));
    assert!(html.contains("images/books/20201214-ml-bookcamp/cover.jpg"));
    assert!(html.contains("14 Dec 2020"), "Should contain start date");
    // End date: YAML has "2020-12-18 23:59:59" which is treated as UTC.
    // Jekyll converts to system timezone (CET = UTC+1), rolling to Dec 19.
    assert!(
        html.contains("19 Dec 2020") || html.contains("18 Dec 2020"),
        "Should contain end date (19 Dec in CET, 18 Dec in UTC)"
    );
}

#[test]
fn test_ml_bookcamp_external_links() {
    if !site_dir().exists() {
        return;
    }
    let html = render_book("20201214-ml-bookcamp");

    assert!(html.contains("Book&#39;s page on Manning") || html.contains("Book's page on Manning"),);
    assert!(html.contains("http://bit.ly/mlbookcamp"));
    assert!(html.contains("https://mlbookcamp.com/"));
    assert!(html.contains("https://github.com/alexeygrigorev/mlbookcamp-code"));
    assert!(html.contains("target=\"_blank\""));
}

#[test]
fn test_ml_bookcamp_qa_archive() {
    if !site_dir().exists() {
        return;
    }
    let html = render_book("20201214-ml-bookcamp");

    assert!(html.contains("Questions and Answers"));
    assert!(html.contains("Vladimir Finkelshtein"));
    assert!(html.contains("Wendy Mak"));
    assert!(html.contains("Neal Lathia"));
    assert!(html.contains("book-archive-reply"));
    assert!(html.contains("book-archive-thread"));
}

#[test]
fn test_render_data_teams() {
    if !site_dir().exists() {
        return;
    }
    let html = render_book("20210201-data-teams");

    assert!(html.contains("Data Teams"));
    assert!(html.contains("Jesse Anderson"));
    assert!(html.contains("book-archive-thread"));
}

#[test]
fn test_book_without_archive() {
    if !site_dir().exists() {
        return;
    }
    let book = FIXTURE
        .books
        .iter()
        .find(|b| !b.front_matter.contains_key("archive"));

    if let Some(book) = book {
        let mut fm = book.front_matter.clone();
        fm.insert("url".into(), serde_yaml::Value::String(book.url.clone()));

        let html = FIXTURE
            .layout_engine
            .render_page("book", &book.html_content, &fm, &FIXTURE.site_context)
            .unwrap();

        assert!(!html.contains("Questions and Answers"));
    }
}

#[test]
fn test_book_participation_instructions() {
    if !site_dir().exists() {
        return;
    }
    let html = render_book("20201214-ml-bookcamp");

    assert!(html.contains("/slack.html"));
    assert!(html.contains("#book-of-the-week"));
    assert!(html.contains("/books.html"));
}

#[test]
fn test_book_css_classes() {
    if !site_dir().exists() {
        return;
    }
    let html = render_book("20201214-ml-bookcamp");

    assert!(html.contains("content-book"));
    assert!(html.contains("content-book-image-container"));
    assert!(html.contains("content-book-description"));
}

#[test]
fn test_generated_book_files_content() {
    if !site_dir().exists() {
        return;
    }
    let _output = &*BOOKS_OUTPUT;

    let ml_path = BOOKS_OUTPUT.path().join("books/20201214-ml-bookcamp.html");
    assert!(ml_path.exists());
    let ml_content = fs::read_to_string(&ml_path).unwrap();
    assert!(ml_content.contains("Machine Learning Bookcamp"));
    assert!(ml_content.contains("Alexey Grigorev"));

    let dt_path = BOOKS_OUTPUT.path().join("books/20210201-data-teams.html");
    assert!(dt_path.exists());
    let dt_content = fs::read_to_string(&dt_path).unwrap();
    assert!(dt_content.contains("Data Teams"));

    let rl_path = BOOKS_OUTPUT
        .path()
        .join("books/20210111-reinforcement-learning.html");
    assert!(rl_path.exists());
    let rl_content = fs::read_to_string(&rl_path).unwrap();
    assert!(rl_content.contains("Reinforcement Learning"));
}

#[test]
fn test_generated_books_are_valid_html() {
    if !site_dir().exists() {
        return;
    }
    let _output = &*BOOKS_OUTPUT;
    let books_dir = BOOKS_OUTPUT.path().join("books");
    for entry in fs::read_dir(&books_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e == "html").unwrap_or(false) {
            let content = fs::read_to_string(&path).unwrap();
            assert!(!content.is_empty(), "File should not be empty: {:?}", path);
            assert!(content.contains("<html"), "Missing <html: {:?}", path);
            assert!(content.contains("<body"), "Missing <body: {:?}", path);
        }
    }
}

#[test]
fn test_qa_text_newline_to_br_markdownify() {
    if !site_dir().exists() {
        return;
    }
    let html = render_book("20201214-ml-bookcamp");
    // D3: Void elements are normalized to not have self-closing slash
    assert!(
        html.contains("<br>") || html.contains("<br />"),
        "Q&A text should contain <br> from newline_to_br filter"
    );
}

#[test]
fn test_book_author_resolution() {
    if !site_dir().exists() {
        return;
    }
    let html = render_book("20201214-ml-bookcamp");
    assert!(html.contains("Alexey Grigorev"));
    assert!(html.contains("/people/alexeygrigorev.html"));
}
