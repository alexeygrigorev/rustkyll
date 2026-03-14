//! JSON-LD structured data generation and injection.
//!
//! This module generates Schema.org JSON-LD blocks and injects them into
//! rendered HTML pages as a post-processing step. This keeps layout files
//! clean and makes JSON-LD generation a rustkyll feature rather than a
//! template concern.
//!
//! Currently supports:
//! - `Book` + `BreadcrumbList` for book pages
//!
//! The injection happens after template rendering but before writing to disk.

use std::collections::HashMap;

use crate::collection::CollectionItem;
use crate::config::SiteConfig;
use crate::frontmatter::FrontMatter;

/// Inject JSON-LD structured data into rendered HTML based on the layout type.
///
/// This is the main entry point for post-processing. It checks the layout name
/// and generates appropriate JSON-LD if applicable, injecting it before `</body>`.
///
/// Returns the HTML with JSON-LD injected, or the original HTML unchanged if
/// no JSON-LD is applicable for this layout type.
pub fn inject_jsonld(
    html: &str,
    layout_name: &str,
    page_fm: &FrontMatter,
    config: &SiteConfig,
    people: &[CollectionItem],
) -> String {
    let jsonld = match layout_name {
        "book" => generate_book_jsonld(page_fm, config, people),
        _ => return html.to_string(),
    };

    inject_before_body_close(html, &jsonld)
}

/// Generate JSON-LD for a book page.
///
/// Produces a `@graph` containing:
/// - `Book` with name, description, image, url, datePublished, author array, publisher
/// - `BreadcrumbList` with Home > Books > [Book Title]
fn generate_book_jsonld(
    page_fm: &FrontMatter,
    config: &SiteConfig,
    people: &[CollectionItem],
) -> String {
    let site_url = &config.url;
    let site_name = &config.name;

    let title = fm_str(page_fm, "title").unwrap_or_default();
    let page_url = fm_str(page_fm, "url").unwrap_or_default();
    let full_url = format!("{site_url}{page_url}");

    // Build the Book object
    let mut book = serde_json::Map::new();
    book.insert("@type".into(), serde_json::Value::String("Book".into()));
    book.insert("@id".into(), serde_json::Value::String(full_url.clone()));
    book.insert("name".into(), serde_json::Value::String(title.to_string()));

    if let Some(desc) = fm_str(page_fm, "description") {
        if !desc.is_empty() {
            book.insert(
                "description".into(),
                serde_json::Value::String(desc.to_string()),
            );
        }
    }

    if let Some(cover) = fm_str(page_fm, "cover") {
        if !cover.is_empty() {
            book.insert(
                "image".into(),
                serde_json::Value::String(format!("{site_url}/{cover}")),
            );
        }
    }

    book.insert("url".into(), serde_json::Value::String(full_url.clone()));

    if let Some(start) = fm_str(page_fm, "start") {
        // Format date as ISO 8601 (just the date part)
        let date_str = normalize_date(start);
        if !date_str.is_empty() {
            book.insert("datePublished".into(), serde_json::Value::String(date_str));
        }
    }

    // Build author array by resolving slugs against the people collection
    if let Some(authors) = fm_string_array(page_fm, "authors") {
        if !authors.is_empty() {
            let people_map: HashMap<&str, &CollectionItem> = people
                .iter()
                .map(|p| {
                    let short = p
                        .front_matter
                        .get("short")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&p.slug);
                    (short, p)
                })
                .collect();

            let author_values: Vec<serde_json::Value> = authors
                .iter()
                .map(|slug| build_author_person(slug, &people_map, site_url))
                .collect();

            book.insert("author".into(), serde_json::Value::Array(author_values));
        }
    }

    // Publisher
    let mut publisher = serde_json::Map::new();
    publisher.insert(
        "@type".into(),
        serde_json::Value::String("Organization".into()),
    );
    publisher.insert("name".into(), serde_json::Value::String(site_name.clone()));
    publisher.insert("url".into(), serde_json::Value::String(site_url.clone()));
    book.insert("publisher".into(), serde_json::Value::Object(publisher));

    // Build BreadcrumbList
    let breadcrumb = build_breadcrumb_list(
        site_url,
        &[
            ("Home", &format!("{site_url}/")),
            ("Books", &format!("{site_url}/books.html")),
            (title, &full_url),
        ],
    );

    // Wrap in @graph
    let mut root = serde_json::Map::new();
    root.insert(
        "@context".into(),
        serde_json::Value::String("https://schema.org".into()),
    );
    root.insert(
        "@graph".into(),
        serde_json::Value::Array(vec![serde_json::Value::Object(book), breadcrumb]),
    );

    // Serialize with pretty printing for readability
    serde_json::to_string_pretty(&serde_json::Value::Object(root)).unwrap_or_default()
}

/// Build a Person JSON-LD object for a book author.
fn build_author_person(
    slug: &str,
    people_map: &HashMap<&str, &CollectionItem>,
    site_url: &str,
) -> serde_json::Value {
    let mut person = serde_json::Map::new();
    person.insert("@type".into(), serde_json::Value::String("Person".into()));

    if let Some(item) = people_map.get(slug) {
        // Resolved author -- use their title as name
        let name = item
            .front_matter
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(slug);
        person.insert("name".into(), serde_json::Value::String(name.to_string()));

        let author_url = format!("{site_url}{}", item.url);
        person.insert("url".into(), serde_json::Value::String(author_url));

        if let Some(pic) = item.front_matter.get("picture").and_then(|v| v.as_str()) {
            if !pic.is_empty() {
                // Use the same logic as relative_url filter: prepend baseurl or /
                let image_url = if pic.starts_with('/') {
                    format!("{site_url}{pic}")
                } else {
                    format!("{site_url}/{pic}")
                };
                person.insert("image".into(), serde_json::Value::String(image_url));
            }
        }
    } else {
        // Unresolved author -- use the slug as the name
        person.insert("name".into(), serde_json::Value::String(slug.to_string()));
    }

    serde_json::Value::Object(person)
}

/// Build a BreadcrumbList JSON-LD object from a list of (name, url) pairs.
fn build_breadcrumb_list(_site_url: &str, items: &[(&str, &str)]) -> serde_json::Value {
    let list_items: Vec<serde_json::Value> = items
        .iter()
        .enumerate()
        .map(|(i, (name, url))| {
            let mut item = serde_json::Map::new();
            item.insert("@type".into(), serde_json::Value::String("ListItem".into()));
            item.insert(
                "position".into(),
                serde_json::Value::Number(serde_json::Number::from(i as u64 + 1)),
            );
            item.insert("name".into(), serde_json::Value::String(name.to_string()));
            item.insert("item".into(), serde_json::Value::String(url.to_string()));
            serde_json::Value::Object(item)
        })
        .collect();

    let mut breadcrumb = serde_json::Map::new();
    breadcrumb.insert(
        "@type".into(),
        serde_json::Value::String("BreadcrumbList".into()),
    );
    breadcrumb.insert(
        "itemListElement".into(),
        serde_json::Value::Array(list_items),
    );

    serde_json::Value::Object(breadcrumb)
}

/// Inject a JSON-LD script block before `</body>` in the HTML.
///
/// If no `</body>` tag is found, appends at the end.
fn inject_before_body_close(html: &str, jsonld: &str) -> String {
    let script_block =
        format!("\n  <script type=\"application/ld+json\">\n{jsonld}\n  </script>\n");

    // Case-insensitive search for </body>
    if let Some(pos) = html.to_lowercase().rfind("</body>") {
        let mut result = String::with_capacity(html.len() + script_block.len());
        result.push_str(&html[..pos]);
        result.push_str(&script_block);
        result.push_str(&html[pos..]);
        result
    } else {
        // No </body> tag found -- append at end
        let mut result = html.to_string();
        result.push_str(&script_block);
        result
    }
}

/// Extract a string value from front matter.
fn fm_str<'a>(fm: &'a FrontMatter, key: &str) -> Option<&'a str> {
    fm.get(key).and_then(|v| v.as_str())
}

/// Extract a string array from front matter.
///
/// Handles both YAML sequences and single string values.
fn fm_string_array(fm: &FrontMatter, key: &str) -> Option<Vec<String>> {
    let val = fm.get(key)?;
    match val {
        serde_yaml::Value::Sequence(seq) => {
            let strs: Vec<String> = seq
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if strs.is_empty() {
                None
            } else {
                Some(strs)
            }
        }
        serde_yaml::Value::String(s) => Some(vec![s.clone()]),
        _ => None,
    }
}

/// Normalize a date string to ISO 8601 format (YYYY-MM-DDTHH:MM:SS+00:00).
///
/// Handles various formats:
/// - "2020-12-14 00:00:00" -> "2020-12-14T00:00:00+00:00"
/// - "2020-12-14" -> "2020-12-14T00:00:00+00:00"
fn normalize_date(date_str: &str) -> String {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // If it already has a T, assume it's already ISO-ish
    if trimmed.contains('T') {
        return trimmed.to_string();
    }

    // Try "YYYY-MM-DD HH:MM:SS" format
    if let Some((date_part, time_part)) = trimmed.split_once(' ') {
        return format!("{date_part}T{time_part}+00:00");
    }

    // Just a date: "YYYY-MM-DD"
    format!("{trimmed}T00:00:00+00:00")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(url: &str, name: &str) -> SiteConfig {
        SiteConfig {
            url: url.to_string(),
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn make_person(slug: &str, title: &str, picture: Option<&str>) -> CollectionItem {
        let mut fm = FrontMatter::new();
        fm.insert("short".into(), serde_yaml::Value::String(slug.to_string()));
        fm.insert("title".into(), serde_yaml::Value::String(title.to_string()));
        if let Some(pic) = picture {
            fm.insert("picture".into(), serde_yaml::Value::String(pic.to_string()));
        }
        CollectionItem {
            slug: slug.to_string(),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: format!("/people/{slug}.html"),
            date: None,
            collection_name: "people".to_string(),
            source_path: String::new(),
        }
    }

    fn make_book_fm(
        title: &str,
        authors: &[&str],
        cover: Option<&str>,
        start: Option<&str>,
        description: Option<&str>,
    ) -> FrontMatter {
        let mut fm = FrontMatter::new();
        fm.insert("title".into(), serde_yaml::Value::String(title.to_string()));
        fm.insert(
            "url".into(),
            serde_yaml::Value::String("/books/test-book.html".to_string()),
        );
        let author_seq: Vec<serde_yaml::Value> = authors
            .iter()
            .map(|a| serde_yaml::Value::String(a.to_string()))
            .collect();
        fm.insert("authors".into(), serde_yaml::Value::Sequence(author_seq));
        if let Some(c) = cover {
            fm.insert("cover".into(), serde_yaml::Value::String(c.to_string()));
        }
        if let Some(s) = start {
            fm.insert("start".into(), serde_yaml::Value::String(s.to_string()));
        }
        if let Some(d) = description {
            fm.insert(
                "description".into(),
                serde_yaml::Value::String(d.to_string()),
            );
        }
        fm
    }

    #[test]
    fn test_book_jsonld_has_correct_type() {
        let config = make_config("https://example.com", "Test Site");
        let people = vec![make_person("author1", "Author One", None)];
        let fm = make_book_fm("My Book", &["author1"], None, None, None);

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let graph = parsed["@graph"].as_array().unwrap();
        assert!(graph.iter().any(|item| item["@type"] == "Book"));
        assert!(graph.iter().any(|item| item["@type"] == "BreadcrumbList"));
    }

    #[test]
    fn test_book_jsonld_name_matches_title() {
        let config = make_config("https://example.com", "Test Site");
        let people = vec![];
        let fm = make_book_fm("Machine Learning Bookcamp", &[], None, None, None);

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let book = parsed["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["@type"] == "Book")
            .unwrap();
        assert_eq!(book["name"], "Machine Learning Bookcamp");
    }

    #[test]
    fn test_book_jsonld_url_includes_site_url() {
        let config = make_config("https://datatalks.club", "DTC");
        let people = vec![];
        let fm = make_book_fm("Test", &[], None, None, None);

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let book = parsed["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["@type"] == "Book")
            .unwrap();
        assert_eq!(book["url"], "https://datatalks.club/books/test-book.html");
    }

    #[test]
    fn test_book_jsonld_author_resolution() {
        let config = make_config("https://example.com", "Test");
        let people = vec![make_person(
            "alexeygrigorev",
            "Alexey Grigorev",
            Some("images/authors/alexeygrigorev.jpg"),
        )];
        let fm = make_book_fm("Test", &["alexeygrigorev"], None, None, None);

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let book = parsed["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["@type"] == "Book")
            .unwrap();

        let authors = book["author"].as_array().unwrap();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0]["@type"], "Person");
        assert_eq!(authors[0]["name"], "Alexey Grigorev");
        assert!(authors[0]["url"]
            .as_str()
            .unwrap()
            .contains("/people/alexeygrigorev.html"));
    }

    #[test]
    fn test_book_jsonld_author_resolution_from_any_collection() {
        // Verify that author resolution works regardless of the collection name.
        // The JSON-LD code does not assume "people" -- it takes a flat slice of items.
        let config = make_config("https://example.com", "Test");
        // Create an author item with collection_name "team" instead of "people"
        let mut fm = FrontMatter::new();
        fm.insert(
            "short".into(),
            serde_yaml::Value::String("jdoe".to_string()),
        );
        fm.insert(
            "title".into(),
            serde_yaml::Value::String("Jane Doe".to_string()),
        );
        let team_member = CollectionItem {
            slug: "jdoe".to_string(),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            url: "/team/jdoe.html".to_string(),
            date: None,
            collection_name: "team".to_string(),
            source_path: String::new(),
        };

        let book_fm = make_book_fm("My Book", &["jdoe"], None, None, None);
        let jsonld = generate_book_jsonld(&book_fm, &config, &[team_member]);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let book = parsed["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["@type"] == "Book")
            .unwrap();

        let authors = book["author"].as_array().unwrap();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0]["name"], "Jane Doe");
        assert!(
            authors[0]["url"]
                .as_str()
                .unwrap()
                .contains("/team/jdoe.html"),
            "Author URL should reference the team collection path"
        );
    }

    #[test]
    fn test_book_jsonld_multiple_authors() {
        let config = make_config("https://example.com", "Test");
        let people = vec![
            make_person("author1", "Author One", None),
            make_person("author2", "Author Two", None),
        ];
        let fm = make_book_fm("Test", &["author1", "author2"], None, None, None);

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let book = parsed["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["@type"] == "Book")
            .unwrap();

        let authors = book["author"].as_array().unwrap();
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0]["name"], "Author One");
        assert_eq!(authors[1]["name"], "Author Two");
    }

    #[test]
    fn test_book_jsonld_no_description_omitted() {
        let config = make_config("https://example.com", "Test");
        let people = vec![];
        let fm = make_book_fm("Test", &[], None, None, None);

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let book = parsed["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["@type"] == "Book")
            .unwrap();

        assert!(book.get("description").is_none());
    }

    #[test]
    fn test_book_jsonld_with_description() {
        let config = make_config("https://example.com", "Test");
        let people = vec![];
        let fm = make_book_fm("Test", &[], None, None, Some("A great book"));

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let book = parsed["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["@type"] == "Book")
            .unwrap();

        assert_eq!(book["description"], "A great book");
    }

    #[test]
    fn test_book_jsonld_date_published() {
        let config = make_config("https://example.com", "Test");
        let people = vec![];
        let fm = make_book_fm("Test", &[], None, Some("2021-11-15 00:00:00"), None);

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let book = parsed["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["@type"] == "Book")
            .unwrap();

        assert_eq!(book["datePublished"], "2021-11-15T00:00:00+00:00");
    }

    #[test]
    fn test_breadcrumb_has_three_items() {
        let config = make_config("https://example.com", "Test");
        let people = vec![];
        let fm = make_book_fm("My Book Title", &[], None, None, None);

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let breadcrumb = parsed["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["@type"] == "BreadcrumbList")
            .unwrap();

        let items = breadcrumb["itemListElement"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["position"], 1);
        assert_eq!(items[0]["name"], "Home");
        assert_eq!(items[1]["position"], 2);
        assert_eq!(items[1]["name"], "Books");
        assert_eq!(items[2]["position"], 3);
        assert_eq!(items[2]["name"], "My Book Title");
    }

    #[test]
    fn test_book_jsonld_publisher() {
        let config = make_config("https://example.com", "DataTalks.Club");
        let people = vec![];
        let fm = make_book_fm("Test", &[], None, None, None);

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let book = parsed["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["@type"] == "Book")
            .unwrap();

        assert_eq!(book["publisher"]["@type"], "Organization");
        assert_eq!(book["publisher"]["name"], "DataTalks.Club");
    }

    #[test]
    fn test_inject_before_body_close() {
        let html = "<html><body><p>Hello</p></body></html>";
        let jsonld = r#"{"@type": "Book"}"#;
        let result = inject_before_body_close(html, jsonld);

        assert!(result.contains(r#"<script type="application/ld+json">"#));
        assert!(result.contains(r#"{"@type": "Book"}"#));
        // JSON-LD should come before </body>
        let script_pos = result.find("application/ld+json").unwrap();
        let body_pos = result.find("</body>").unwrap();
        assert!(script_pos < body_pos);
    }

    #[test]
    fn test_inject_no_body_tag() {
        let html = "<html><p>Hello</p></html>";
        let jsonld = r#"{"@type": "Book"}"#;
        let result = inject_before_body_close(html, jsonld);

        // Should still contain the JSON-LD (appended at end)
        assert!(result.contains(r#"<script type="application/ld+json">"#));
    }

    #[test]
    fn test_inject_jsonld_non_book_layout_unchanged() {
        let html = "<html><body><p>Hello</p></body></html>";
        let fm = FrontMatter::new();
        let config = make_config("https://example.com", "Test");
        let people = vec![];

        let result = inject_jsonld(html, "post", &fm, &config, &people);
        assert_eq!(result, html);
    }

    #[test]
    fn test_inject_jsonld_book_layout_adds_jsonld() {
        let html = "<html><body><p>Hello</p></body></html>";
        let fm = make_book_fm("Test Book", &[], None, None, None);
        let config = make_config("https://example.com", "Test");
        let people = vec![];

        let result = inject_jsonld(html, "book", &fm, &config, &people);
        assert!(result.contains("application/ld+json"));
        assert!(result.contains("Book"));
    }

    #[test]
    fn test_normalize_date_with_time() {
        assert_eq!(
            normalize_date("2020-12-14 00:00:00"),
            "2020-12-14T00:00:00+00:00"
        );
    }

    #[test]
    fn test_normalize_date_without_time() {
        assert_eq!(normalize_date("2020-12-14"), "2020-12-14T00:00:00+00:00");
    }

    #[test]
    fn test_normalize_date_empty() {
        assert_eq!(normalize_date(""), "");
    }

    #[test]
    fn test_book_jsonld_with_cover() {
        let config = make_config("https://example.com", "Test");
        let people = vec![];
        let fm = make_book_fm("Test", &[], Some("images/books/cover.jpg"), None, None);

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        let parsed: serde_json::Value = serde_json::from_str(&jsonld).unwrap();

        let book = parsed["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["@type"] == "Book")
            .unwrap();

        assert_eq!(book["image"], "https://example.com/images/books/cover.jpg");
    }

    #[test]
    fn test_generated_jsonld_is_valid_json() {
        let config = make_config("https://datatalks.club", "DataTalks.Club");
        let people = vec![make_person(
            "alexeygrigorev",
            "Alexey Grigorev",
            Some("images/authors/alexeygrigorev.jpg"),
        )];
        let fm = make_book_fm(
            "Machine Learning Bookcamp",
            &["alexeygrigorev"],
            Some("images/books/ml-bookcamp/cover.jpg"),
            Some("2020-12-14 00:00:00"),
            Some("Learn ML by doing projects"),
        );

        let jsonld = generate_book_jsonld(&fm, &config, &people);
        // Must be valid JSON
        let result: Result<serde_json::Value, _> = serde_json::from_str(&jsonld);
        assert!(
            result.is_ok(),
            "Generated JSON-LD must be valid JSON: {}",
            jsonld
        );
    }
}
