//! Sitemap XML generation.
//!
//! Generates a standard `sitemap.xml` listing all pages and collection items
//! with their full URLs. This is the programmatic equivalent of the Jekyll
//! sitemap template found in a typical Jekyll site's `sitemap.xml`.

use std::fmt::Write;
use std::fs;
use std::path::Path;

use crate::collection::{CollectionItem, Page};
use crate::config::SiteConfig;
use crate::frontmatter::FrontMatter;

/// A single entry in the sitemap.
#[derive(Debug, Clone, PartialEq)]
pub struct SitemapEntry {
    /// Full URL (e.g. `https://example.com/books/my-book.html`).
    pub loc: String,
}

/// Collect sitemap entries from collection items and standalone pages.
///
/// The `base_url` is the site URL (e.g. `https://example.com`) and is
/// prepended to each item's relative URL.
///
/// Collection items and pages are included in the order they appear. The
/// root URL (`base_url + "/"`) is always included as the first entry.
///
/// Mirrors `jekyll-sitemap`'s exclusions:
/// - Documents in collections configured with `output: false` are omitted
///   (no HTML is emitted for them, so their URLs would 404). Collections
///   absent from the config default to `output: true` and are included.
/// - Pages and documents with `sitemap: false` front matter are omitted.
pub fn collect_entries(
    base_url: &str,
    collections: &[(String, Vec<CollectionItem>)],
    pages: &[Page],
    config: &SiteConfig,
) -> Vec<SitemapEntry> {
    let base = base_url.trim_end_matches('/');
    let mut entries = Vec::new();

    // Root URL is always first
    entries.push(SitemapEntry {
        loc: format!("{}/", base),
    });

    // Standalone pages (excluding index which is already the root)
    for page in pages {
        let url = &page.url;
        if url == "/" || url == "/index.html" {
            continue;
        }
        if is_sitemap_false(&page.front_matter) {
            continue;
        }
        entries.push(SitemapEntry {
            loc: format!("{}{}", base, url),
        });
    }

    // Collection items
    for (name, items) in collections {
        // Skip collections explicitly configured with `output: false`; their
        // documents are never written to `_site`. Collections absent from the
        // config default to `output: true` and are included.
        if let Some(collection) = config.collection(name) {
            if !collection.output {
                continue;
            }
        }
        for item in items {
            if is_sitemap_false(&item.front_matter) {
                continue;
            }
            entries.push(SitemapEntry {
                loc: format!("{}{}", base, item.url),
            });
        }
    }

    entries
}

/// Returns true if the front matter has `sitemap: false`.
///
/// Mirrors `jekyll-sitemap`, which omits any page or document whose front
/// matter sets `sitemap: false`. A missing key (or any non-`false` value)
/// means the item is included.
fn is_sitemap_false(front_matter: &FrontMatter) -> bool {
    front_matter
        .get("sitemap")
        .and_then(|v| v.as_bool())
        .is_some_and(|b| !b)
}

/// Escape a string for use in XML content.
///
/// Replaces `&`, `<`, `>`, `"`, and `'` with their XML entity equivalents.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Render sitemap entries as a valid XML sitemap string.
pub fn render_xml(entries: &[SitemapEntry]) -> String {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

    for entry in entries {
        let _ = write!(
            xml,
            "    <url>\n        <loc>{}</loc>\n    </url>\n",
            xml_escape(&entry.loc)
        );
    }

    xml.push_str("</urlset>\n");
    xml
}

/// Generate sitemap.xml and write it to `output_dir/sitemap.xml`.
///
/// Backwards-compatible wrapper that applies no site-config-based exclusions
/// (all collections treated as `output: true`). Prefer
/// [`generate_sitemap_with_config`] for the real build path so that
/// `output: false` collections are excluded.
///
/// # Errors
///
/// Returns an I/O error if the output directory cannot be created or
/// the file cannot be written.
pub fn generate_sitemap(
    base_url: &str,
    collections: &[(String, Vec<CollectionItem>)],
    pages: &[Page],
    output_dir: &Path,
) -> Result<usize, std::io::Error> {
    generate_sitemap_with_config(
        base_url,
        collections,
        pages,
        &SiteConfig::default(),
        output_dir,
    )
}

/// Generate sitemap.xml, honoring the site config's collection `output` flags
/// and per-item/page `sitemap: false` front matter, and write it to
/// `output_dir/sitemap.xml`.
///
/// # Errors
///
/// Returns an I/O error if the output directory cannot be created or
/// the file cannot be written.
pub fn generate_sitemap_with_config(
    base_url: &str,
    collections: &[(String, Vec<CollectionItem>)],
    pages: &[Page],
    config: &SiteConfig,
    output_dir: &Path,
) -> Result<usize, std::io::Error> {
    let entries = collect_entries(base_url, collections, pages, config);
    let xml = render_xml(&entries);

    fs::create_dir_all(output_dir)?;
    fs::write(output_dir.join("sitemap.xml"), &xml)?;

    Ok(entries.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::FrontMatter;

    fn make_collection_item(slug: &str, url: &str, collection: &str) -> CollectionItem {
        CollectionItem {
            slug: slug.to_string(),
            front_matter: FrontMatter::new(),
            content: String::new(),
            html_content: String::new(),
            excerpt: None,
            excerpt_html: None,
            url: url.to_string(),
            date: None,
            collection_name: collection.to_string(),
            source_path: format!("_{collection}/{slug}.md"),
            id: format!("/{collection}/{slug}"),
        }
    }

    fn make_page(slug: &str, url: &str) -> Page {
        Page {
            slug: slug.to_string(),
            front_matter: FrontMatter::new(),
            content: String::new(),
            html_content: String::new(),
            url: url.to_string(),
            source_path: format!("{slug}.md"),
        }
    }

    // ========================================================================
    // XML escaping
    // ========================================================================

    #[test]
    fn test_xml_escape_ampersand() {
        assert_eq!(xml_escape("foo&bar"), "foo&amp;bar");
    }

    #[test]
    fn test_xml_escape_angle_brackets() {
        assert_eq!(xml_escape("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn test_xml_escape_quotes() {
        assert_eq!(xml_escape("a\"b'c"), "a&quot;b&apos;c");
    }

    #[test]
    fn test_xml_escape_no_change() {
        assert_eq!(xml_escape("hello world"), "hello world");
    }

    // ========================================================================
    // Entry collection
    // ========================================================================

    #[test]
    fn test_collect_entries_root_always_first() {
        let entries = collect_entries("https://example.com", &[], &[], &SiteConfig::default());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].loc, "https://example.com/");
    }

    #[test]
    fn test_collect_entries_strips_trailing_slash_from_base() {
        let entries = collect_entries("https://example.com/", &[], &[], &SiteConfig::default());
        assert_eq!(entries[0].loc, "https://example.com/");
    }

    #[test]
    fn test_collect_entries_includes_pages() {
        let pages = vec![
            make_page("events", "/events.html"),
            make_page("books", "/books.html"),
        ];
        let entries = collect_entries("https://example.com", &[], &pages, &SiteConfig::default());
        assert_eq!(entries.len(), 3); // root + 2 pages
        assert_eq!(entries[1].loc, "https://example.com/events.html");
        assert_eq!(entries[2].loc, "https://example.com/books.html");
    }

    #[test]
    fn test_collect_entries_skips_index_page() {
        let pages = vec![
            make_page("index", "/index.html"),
            make_page("about", "/about.html"),
        ];
        let entries = collect_entries("https://example.com", &[], &pages, &SiteConfig::default());
        assert_eq!(entries.len(), 2); // root + about (index is skipped)
    }

    #[test]
    fn test_collect_entries_skips_root_slash_page() {
        let pages = vec![make_page("index", "/")];
        let entries = collect_entries("https://example.com", &[], &pages, &SiteConfig::default());
        assert_eq!(entries.len(), 1); // only root
    }

    #[test]
    fn test_collect_entries_includes_collection_items() {
        let items = vec![
            make_collection_item("alice", "/people/alice.html", "people"),
            make_collection_item("bob", "/people/bob.html", "people"),
        ];
        let collections = vec![("people".to_string(), items)];
        let entries = collect_entries(
            "https://example.com",
            &collections,
            &[],
            &SiteConfig::default(),
        );
        assert_eq!(entries.len(), 3); // root + 2 people
        assert_eq!(entries[1].loc, "https://example.com/people/alice.html");
    }

    #[test]
    fn test_collect_entries_multiple_collections() {
        let people = vec![make_collection_item(
            "alice",
            "/people/alice.html",
            "people",
        )];
        let books = vec![make_collection_item("ml", "/books/ml.html", "books")];
        let collections = vec![("people".to_string(), people), ("books".to_string(), books)];
        let entries = collect_entries(
            "https://example.com",
            &collections,
            &[],
            &SiteConfig::default(),
        );
        assert_eq!(entries.len(), 3); // root + 1 person + 1 book
    }

    #[test]
    fn test_collect_entries_pages_before_collections() {
        let pages = vec![make_page("about", "/about.html")];
        let items = vec![make_collection_item(
            "alice",
            "/people/alice.html",
            "people",
        )];
        let collections = vec![("people".to_string(), items)];
        let entries = collect_entries(
            "https://example.com",
            &collections,
            &pages,
            &SiteConfig::default(),
        );
        assert_eq!(entries[0].loc, "https://example.com/");
        assert_eq!(entries[1].loc, "https://example.com/about.html");
        assert_eq!(entries[2].loc, "https://example.com/people/alice.html");
    }

    // ========================================================================
    // Exclusions: output:false collections and sitemap:false front matter
    // ========================================================================

    /// Build a `SiteConfig` declaring a single collection with the given
    /// `output` flag. Other collections are absent (default `output: true`).
    fn config_with_collection(name: &str, output: bool) -> SiteConfig {
        let yaml = format!(
            "collections:\n  {name}:\n    output: {output}\n    permalink: /{name}/:title/\n"
        );
        SiteConfig::from_yaml_str(&yaml).unwrap()
    }

    /// Build front matter with `sitemap: false`.
    fn front_matter_sitemap_false() -> FrontMatter {
        let mut fm = FrontMatter::new();
        fm.insert("sitemap".to_string(), serde_yaml::Value::Bool(false));
        fm
    }

    #[test]
    fn test_collect_entries_omits_output_false_collection_docs() {
        let items = vec![make_collection_item("hidden", "/notes/hidden/", "notes")];
        let collections = vec![("notes".to_string(), items)];
        let config = config_with_collection("notes", false);
        let entries = collect_entries("https://example.com", &collections, &[], &config);
        // Only the root remains; the output:false doc is omitted.
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].loc, "https://example.com/");
        assert!(
            !entries.iter().any(|e| e.loc.contains("/notes/hidden/")),
            "output:false collection doc should be omitted"
        );
    }

    #[test]
    fn test_collect_entries_omits_page_with_sitemap_false() {
        let mut excluded = make_page("excluded", "/excluded.html");
        excluded.front_matter = front_matter_sitemap_false();
        let pages = vec![make_page("about", "/about.html"), excluded];
        let entries = collect_entries("https://example.com", &[], &pages, &SiteConfig::default());
        assert_eq!(entries.len(), 2); // root + about
        assert!(
            entries
                .iter()
                .any(|e| e.loc == "https://example.com/about.html"),
            "normal page should be included"
        );
        assert!(
            !entries.iter().any(|e| e.loc.contains("/excluded.html")),
            "page with sitemap:false should be omitted"
        );
    }

    #[test]
    fn test_collect_entries_omits_output_true_doc_with_sitemap_false() {
        let mut hidden = make_collection_item("hidden", "/people/hidden.html", "people");
        hidden.front_matter = front_matter_sitemap_false();
        let visible = make_collection_item("alice", "/people/alice.html", "people");
        let collections = vec![("people".to_string(), vec![visible, hidden])];
        let config = config_with_collection("people", true);
        let entries = collect_entries("https://example.com", &collections, &[], &config);
        assert_eq!(entries.len(), 2); // root + alice
        assert!(
            entries.iter().any(|e| e.loc.contains("/people/alice.html")),
            "output:true doc without sitemap:false should be included"
        );
        assert!(
            !entries
                .iter()
                .any(|e| e.loc.contains("/people/hidden.html")),
            "output:true doc with sitemap:false should be omitted"
        );
    }

    #[test]
    fn test_collect_entries_includes_positive_cases() {
        // Normal page.
        let page = make_page("about", "/about.html");
        // Normal output:true collection doc.
        let people = vec![make_collection_item(
            "alice",
            "/people/alice.html",
            "people",
        )];
        // Config-absent collection (posts) doc -- defaults to output:true.
        let posts = vec![make_collection_item(
            "first",
            "/2020/01/01/first.html",
            "posts",
        )];
        let collections = vec![("people".to_string(), people), ("posts".to_string(), posts)];
        // Config declares only `people` as output:true; `posts` is absent.
        let config = config_with_collection("people", true);
        let entries = collect_entries("https://example.com", &collections, &[page], &config);
        assert_eq!(entries.len(), 4); // root + about + alice + posts doc
        assert!(entries.iter().any(|e| e.loc.contains("/about.html")));
        assert!(entries.iter().any(|e| e.loc.contains("/people/alice.html")));
        assert!(
            entries
                .iter()
                .any(|e| e.loc.contains("/2020/01/01/first.html")),
            "config-absent collection (posts) should default to included"
        );
    }

    #[test]
    fn test_collect_entries_minimal_repro_single_loc() {
        // Mirrors the issue's minimal reproduction: an output:false `notes`
        // collection doc plus a `sitemap: false` page should both be dropped,
        // leaving exactly the site root.
        let index = make_page("index", "/");
        let mut excluded = make_page("excluded", "/excluded.html");
        excluded.front_matter = front_matter_sitemap_false();
        let pages = vec![index, excluded];
        let notes = vec![make_collection_item("hidden", "/notes/hidden/", "notes")];
        let collections = vec![("notes".to_string(), notes)];
        let config = config_with_collection("notes", false);
        let entries = collect_entries("https://example.com", &collections, &pages, &config);
        assert_eq!(
            entries.len(),
            1,
            "expected exactly one <loc> (the site root)"
        );
        assert_eq!(entries[0].loc, "https://example.com/");
    }

    // ========================================================================
    // XML rendering
    // ========================================================================

    #[test]
    fn test_render_xml_header() {
        let xml = render_xml(&[]);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"));
        assert!(xml.contains("</urlset>"));
    }

    #[test]
    fn test_render_xml_single_entry() {
        let entries = vec![SitemapEntry {
            loc: "https://example.com/".to_string(),
        }];
        let xml = render_xml(&entries);
        assert!(xml.contains("<loc>https://example.com/</loc>"));
    }

    #[test]
    fn test_render_xml_escapes_ampersand_in_url() {
        let entries = vec![SitemapEntry {
            loc: "https://example.com/page?a=1&b=2".to_string(),
        }];
        let xml = render_xml(&entries);
        assert!(xml.contains("<loc>https://example.com/page?a=1&amp;b=2</loc>"));
    }

    #[test]
    fn test_render_xml_multiple_entries() {
        let entries = vec![
            SitemapEntry {
                loc: "https://example.com/".to_string(),
            },
            SitemapEntry {
                loc: "https://example.com/about.html".to_string(),
            },
        ];
        let xml = render_xml(&entries);
        let url_count = xml.matches("<url>").count();
        assert_eq!(url_count, 2);
    }

    #[test]
    fn test_render_xml_is_valid_structure() {
        let entries = vec![SitemapEntry {
            loc: "https://example.com/".to_string(),
        }];
        let xml = render_xml(&entries);
        // Verify basic XML structure
        assert!(xml.starts_with("<?xml"));
        assert!(xml.contains("<urlset"));
        assert!(xml.ends_with("</urlset>\n"));
        // Each <url> has a matching </url>
        assert_eq!(xml.matches("<url>").count(), xml.matches("</url>").count());
    }

    // ========================================================================
    // Full pipeline: generate_sitemap writes file
    // ========================================================================

    #[test]
    fn test_generate_sitemap_writes_file() {
        let dir = tempfile::tempdir().unwrap();
        let count = generate_sitemap("https://example.com", &[], &[], dir.path()).unwrap();
        assert_eq!(count, 1); // just root
        let path = dir.path().join("sitemap.xml");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("<loc>https://example.com/</loc>"));
    }

    #[test]
    fn test_generate_sitemap_with_collections_and_pages() {
        let dir = tempfile::tempdir().unwrap();
        let pages = vec![make_page("about", "/about.html")];
        let items = vec![make_collection_item(
            "alice",
            "/people/alice.html",
            "people",
        )];
        let collections = vec![("people".to_string(), items)];
        let count =
            generate_sitemap("https://example.com", &collections, &pages, dir.path()).unwrap();
        assert_eq!(count, 3);
        let content = std::fs::read_to_string(dir.path().join("sitemap.xml")).unwrap();
        assert!(content.contains("https://example.com/about.html"));
        assert!(content.contains("https://example.com/people/alice.html"));
    }

    #[test]
    fn test_generate_sitemap_with_config_excludes_output_false_and_sitemap_false() {
        let dir = tempfile::tempdir().unwrap();
        let mut excluded = make_page("excluded", "/excluded.html");
        excluded.front_matter = front_matter_sitemap_false();
        let pages = vec![make_page("index", "/"), excluded];
        let notes = vec![make_collection_item("hidden", "/notes/hidden/", "notes")];
        let collections = vec![("notes".to_string(), notes)];
        let config = config_with_collection("notes", false);
        let count = generate_sitemap_with_config(
            "https://example.com",
            &collections,
            &pages,
            &config,
            dir.path(),
        )
        .unwrap();
        assert_eq!(count, 1); // only root
        let content = std::fs::read_to_string(dir.path().join("sitemap.xml")).unwrap();
        assert_eq!(content.matches("<loc>").count(), 1);
        assert!(content.contains("<loc>https://example.com/</loc>"));
        assert!(!content.contains("/notes/hidden/"));
        assert!(!content.contains("/excluded.html"));
    }

    #[test]
    fn test_generate_sitemap_creates_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("sub").join("dir");
        let count = generate_sitemap("https://example.com", &[], &[], &nested).unwrap();
        assert_eq!(count, 1);
        assert!(nested.join("sitemap.xml").exists());
    }

    // ========================================================================
    // Integration: real site data
    // ========================================================================

    #[test]
    fn test_sitemap_with_real_collections() {
        use crate::config::SiteConfig;
        use std::path::PathBuf;

        let site_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let config = SiteConfig::from_file(&site_dir.join("_config.yml")).unwrap();

        let mut collections_vec = Vec::new();
        for name in config.collections.keys() {
            let (items, _) = crate::collection::load_collection(name, &site_dir, &config).unwrap();
            collections_vec.push((name.clone(), items));
        }

        // Sort collection names for deterministic output
        collections_vec.sort_by(|a, b| a.0.cmp(&b.0));

        // Also load posts
        let (posts, _) = crate::collection::load_collection("posts", &site_dir, &config).unwrap();
        collections_vec.push(("posts".to_string(), posts));

        let (pages, _) = crate::collection::load_pages(&site_dir, &config).unwrap();

        let entries = collect_entries(&config.url, &collections_vec, &pages, &config);

        // Should have a substantial number of entries
        assert!(
            entries.len() > 5,
            "Expected 5+ sitemap entries, got {}",
            entries.len()
        );

        // Root should be first
        assert_eq!(entries[0].loc, "https://datatalks.club/");

        // All entries should start with the site URL
        for entry in &entries {
            assert!(
                entry.loc.starts_with("https://datatalks.club"),
                "Entry should start with site URL: {}",
                entry.loc
            );
        }

        // Verify the XML renders without panic
        let xml = render_xml(&entries);
        assert!(xml.contains("<urlset"));
        assert!(xml.contains("</urlset>"));

        // Should contain known URLs from the site
        assert!(
            entries.iter().any(|e| e.loc.contains("/people/")),
            "Should have people URLs"
        );
        assert!(
            entries.iter().any(|e| e.loc.contains("/books/")),
            "Should have book URLs"
        );
        assert!(
            entries.iter().any(|e| e.loc.contains("/podcast/")),
            "Should have podcast URLs"
        );
    }
}
