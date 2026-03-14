//! Atom feed generation for blog posts.
//!
//! Generates a valid Atom 1.0 feed (feed.xml) from blog posts,
//! equivalent to the `jekyll-feed` plugin output.

use std::fs;
use std::path::Path;

use crate::collection::CollectionItem;
use crate::config::SiteConfig;

/// Maximum number of posts to include in the feed.
const DEFAULT_MAX_POSTS: usize = 20;

/// Errors that can occur during feed generation.
#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error("I/O error writing feed to {path}: {source}")]
    WriteFile {
        path: String,
        source: std::io::Error,
    },
}

/// Options for feed generation.
#[derive(Debug, Clone)]
pub struct FeedOptions {
    /// Maximum number of posts to include.
    pub max_posts: usize,
}

impl Default for FeedOptions {
    fn default() -> Self {
        Self {
            max_posts: DEFAULT_MAX_POSTS,
        }
    }
}

/// Generate Atom feed XML from a list of blog posts and site configuration.
///
/// Posts are sorted by date descending (newest first) and limited to
/// `options.max_posts`. Posts without a date are excluded.
///
/// Returns the Atom XML as a string.
pub fn generate_atom_feed(
    posts: &[CollectionItem],
    config: &SiteConfig,
    options: &FeedOptions,
) -> String {
    // Filter posts with dates and sort by date descending
    let mut dated_posts: Vec<&CollectionItem> = posts.iter().filter(|p| p.date.is_some()).collect();
    dated_posts.sort_by(|a, b| {
        let da = a.date.as_deref().unwrap_or("");
        let db = b.date.as_deref().unwrap_or("");
        db.cmp(da)
    });
    dated_posts.truncate(options.max_posts);

    // Determine the feed updated time: use the most recent post date, or fallback to now
    let updated = dated_posts
        .first()
        .and_then(|p| p.date.as_deref())
        .map(format_date_to_rfc3339)
        .unwrap_or_else(|| {
            chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S+00:00")
                .to_string()
        });

    let site_url = &config.url;
    let site_title = xml_escape(&config.title);
    let feed_url = format!("{site_url}/feed.xml");

    let mut xml = String::with_capacity(8192);
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
    xml.push_str(
        "  <generator uri=\"https://github.com/rustkyll/rustkyll\">rustkyll</generator>\n",
    );
    xml.push_str(&format!(
        "  <link href=\"{feed_url}\" rel=\"self\" type=\"application/atom+xml\"/>\n"
    ));
    xml.push_str(&format!(
        "  <link href=\"{site_url}/\" rel=\"alternate\" type=\"text/html\"/>\n"
    ));
    xml.push_str(&format!("  <updated>{updated}</updated>\n"));
    xml.push_str(&format!("  <id>{site_url}/feed.xml</id>\n"));
    xml.push_str(&format!("  <title type=\"html\">{site_title}</title>\n"));

    for post in &dated_posts {
        write_entry(&mut xml, post, config);
    }

    xml.push_str("</feed>\n");
    xml
}

/// Write a single Atom entry for a blog post.
fn write_entry(xml: &mut String, post: &CollectionItem, config: &SiteConfig) {
    let site_url = &config.url;
    let post_url = format!("{site_url}{}", post.url);
    let date = post.date.as_deref().unwrap_or("");
    let published = format_date_to_rfc3339(date);

    // Title from front matter, fallback to slug
    let title = post
        .front_matter
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(&post.slug);
    let title_escaped = xml_escape(title);

    // Author from front matter (first author if it's a sequence)
    let author = extract_author(post);

    // Summary/description from front matter
    let description = post
        .front_matter
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    xml.push_str("  <entry>\n");
    xml.push_str(&format!(
        "    <title type=\"html\">{title_escaped}</title>\n"
    ));
    xml.push_str(&format!(
        "    <link href=\"{post_url}\" rel=\"alternate\" type=\"text/html\" title=\"{title_escaped}\"/>\n"
    ));
    xml.push_str(&format!("    <published>{published}</published>\n"));
    xml.push_str(&format!("    <updated>{published}</updated>\n"));
    xml.push_str(&format!("    <id>{post_url}</id>\n"));

    // Content: use HTML content if available, otherwise use excerpt or description
    let content = if !post.html_content.is_empty() {
        Some(&post.html_content)
    } else {
        None
    };

    if let Some(html) = content {
        xml.push_str(&format!(
            "    <content type=\"html\">{}</content>\n",
            xml_escape(html)
        ));
    }

    if let Some(ref desc) = description {
        xml.push_str(&format!(
            "    <summary type=\"html\">{}</summary>\n",
            xml_escape(desc)
        ));
    }

    if let Some(ref author_name) = author {
        xml.push_str("    <author>\n");
        xml.push_str(&format!("      <name>{}</name>\n", xml_escape(author_name)));
        xml.push_str("    </author>\n");
    }

    xml.push_str("  </entry>\n");
}

/// Extract the first author name from a post's front matter.
///
/// Handles both a single string `author: "name"` and a sequence
/// `authors: ["name1", "name2"]`.
fn extract_author(post: &CollectionItem) -> Option<String> {
    // Try `author` field first (string)
    if let Some(author) = post.front_matter.get("author").and_then(|v| v.as_str()) {
        return Some(author.to_string());
    }

    // Try `authors` field (sequence)
    if let Some(serde_yaml::Value::Sequence(authors)) = post.front_matter.get("authors") {
        if let Some(first) = authors.first().and_then(|v| v.as_str()) {
            return Some(first.to_string());
        }
    }

    None
}

/// Format a date string (YYYY-MM-DD) to RFC 3339 / Atom format.
///
/// Returns `YYYY-MM-DDT00:00:00+00:00` for date-only strings.
/// Already-formatted strings are returned as-is.
fn format_date_to_rfc3339(date: &str) -> String {
    let trimmed = date.trim();
    // If it already contains 'T', assume it's already formatted
    if trimmed.contains('T') {
        return trimmed.to_string();
    }
    // Date-only: append time and timezone
    format!("{trimmed}T00:00:00+00:00")
}

/// Escape XML special characters in a string.
fn xml_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            _ => result.push(ch),
        }
    }
    result
}

/// Generate the Atom feed and write it to `feed.xml` in the output directory.
///
/// # Errors
///
/// Returns `FeedError::WriteFile` if the file cannot be written.
pub fn write_atom_feed(
    posts: &[CollectionItem],
    config: &SiteConfig,
    options: &FeedOptions,
    output_dir: &Path,
) -> Result<(), FeedError> {
    let xml = generate_atom_feed(posts, config, options);
    let feed_path = output_dir.join("feed.xml");

    if let Some(parent) = feed_path.parent() {
        fs::create_dir_all(parent).map_err(|e| FeedError::WriteFile {
            path: feed_path.display().to_string(),
            source: e,
        })?;
    }

    fs::write(&feed_path, xml).map_err(|e| FeedError::WriteFile {
        path: feed_path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SiteConfig;
    use crate::frontmatter::FrontMatter;

    fn test_config() -> SiteConfig {
        SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test Blog".to_string(),
            title: "Test Blog".to_string(),
            permalink: "/blog/:title.html".to_string(),
            ..Default::default()
        }
    }

    fn make_post(
        slug: &str,
        title: &str,
        date: &str,
        content: &str,
        description: Option<&str>,
        authors: Vec<&str>,
    ) -> CollectionItem {
        let mut fm = FrontMatter::new();
        fm.insert("title".into(), serde_yaml::Value::String(title.to_string()));
        if let Some(desc) = description {
            fm.insert(
                "description".into(),
                serde_yaml::Value::String(desc.to_string()),
            );
        }
        if authors.len() == 1 {
            fm.insert(
                "author".into(),
                serde_yaml::Value::String(authors[0].to_string()),
            );
        } else if !authors.is_empty() {
            let seq: Vec<serde_yaml::Value> = authors
                .iter()
                .map(|a| serde_yaml::Value::String(a.to_string()))
                .collect();
            fm.insert("authors".into(), serde_yaml::Value::Sequence(seq));
        }

        CollectionItem {
            slug: slug.to_string(),
            front_matter: fm,
            content: content.to_string(),
            html_content: format!("<p>{content}</p>"),
            excerpt: None,
            url: format!("/blog/{slug}.html"),
            date: Some(date.to_string()),
            collection_name: "posts".to_string(),
            source_path: format!("_posts/{slug}.md"),
        }
    }

    // ========================================================================
    // Basic feed structure
    // ========================================================================

    #[test]
    fn test_feed_has_xml_declaration() {
        let config = test_config();
        let posts = vec![];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
    }

    #[test]
    fn test_feed_has_atom_namespace() {
        let config = test_config();
        let posts = vec![];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("xmlns=\"http://www.w3.org/2005/Atom\""));
    }

    #[test]
    fn test_feed_has_title() {
        let config = test_config();
        let posts = vec![];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("<title type=\"html\">Test Blog</title>"));
    }

    #[test]
    fn test_feed_has_self_link() {
        let config = test_config();
        let posts = vec![];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("href=\"https://example.com/feed.xml\" rel=\"self\""));
    }

    #[test]
    fn test_feed_has_alternate_link() {
        let config = test_config();
        let posts = vec![];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("href=\"https://example.com/\" rel=\"alternate\""));
    }

    #[test]
    fn test_feed_has_id() {
        let config = test_config();
        let posts = vec![];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("<id>https://example.com/feed.xml</id>"));
    }

    #[test]
    fn test_feed_has_generator() {
        let config = test_config();
        let posts = vec![];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("<generator"));
        assert!(xml.contains("rustkyll</generator>"));
    }

    #[test]
    fn test_feed_generator_uri_is_generic() {
        let config = test_config();
        let posts = vec![];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(
            !xml.contains("DataTalksClub"),
            "Feed generator URI should not contain org-specific references"
        );
        assert!(
            xml.contains("uri=\"https://github.com/rustkyll/rustkyll\""),
            "Feed generator URI should use generic rustkyll URL"
        );
    }

    // ========================================================================
    // Entries
    // ========================================================================

    #[test]
    fn test_feed_entry_has_title() {
        let config = test_config();
        let posts = vec![make_post(
            "hello",
            "Hello World",
            "2024-01-15",
            "content",
            None,
            vec!["alice"],
        )];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("<title type=\"html\">Hello World</title>"));
    }

    #[test]
    fn test_feed_entry_has_link() {
        let config = test_config();
        let posts = vec![make_post(
            "hello",
            "Hello World",
            "2024-01-15",
            "content",
            None,
            vec![],
        )];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("href=\"https://example.com/blog/hello.html\""));
    }

    #[test]
    fn test_feed_entry_has_published_date() {
        let config = test_config();
        let posts = vec![make_post(
            "hello",
            "Hello",
            "2024-01-15",
            "content",
            None,
            vec![],
        )];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("<published>2024-01-15T00:00:00+00:00</published>"));
    }

    #[test]
    fn test_feed_entry_has_updated_date() {
        let config = test_config();
        let posts = vec![make_post(
            "hello",
            "Hello",
            "2024-01-15",
            "content",
            None,
            vec![],
        )];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("<updated>2024-01-15T00:00:00+00:00</updated>"));
    }

    #[test]
    fn test_feed_entry_has_id() {
        let config = test_config();
        let posts = vec![make_post(
            "hello",
            "Hello",
            "2024-01-15",
            "content",
            None,
            vec![],
        )];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("<id>https://example.com/blog/hello.html</id>"));
    }

    #[test]
    fn test_feed_entry_has_content() {
        let config = test_config();
        let posts = vec![make_post(
            "hello",
            "Hello",
            "2024-01-15",
            "My content",
            None,
            vec![],
        )];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("<content type=\"html\">"));
        assert!(xml.contains("My content"));
    }

    #[test]
    fn test_feed_entry_has_summary_from_description() {
        let config = test_config();
        let posts = vec![make_post(
            "hello",
            "Hello",
            "2024-01-15",
            "content",
            Some("A great post"),
            vec![],
        )];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("<summary type=\"html\">A great post</summary>"));
    }

    #[test]
    fn test_feed_entry_has_author() {
        let config = test_config();
        let posts = vec![make_post(
            "hello",
            "Hello",
            "2024-01-15",
            "content",
            None,
            vec!["alice"],
        )];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("<author>"));
        assert!(xml.contains("<name>alice</name>"));
    }

    #[test]
    fn test_feed_entry_author_from_authors_list() {
        let config = test_config();
        let posts = vec![make_post(
            "hello",
            "Hello",
            "2024-01-15",
            "content",
            None,
            vec!["bob", "charlie"],
        )];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("<name>bob</name>"));
    }

    // ========================================================================
    // Sorting and limits
    // ========================================================================

    #[test]
    fn test_feed_posts_sorted_newest_first() {
        let config = test_config();
        let posts = vec![
            make_post("old", "Old Post", "2023-01-01", "old", None, vec![]),
            make_post("new", "New Post", "2024-06-15", "new", None, vec![]),
            make_post("mid", "Mid Post", "2024-01-15", "mid", None, vec![]),
        ];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        let new_pos = xml.find("New Post").expect("New Post should appear");
        let mid_pos = xml.find("Mid Post").expect("Mid Post should appear");
        let old_pos = xml.find("Old Post").expect("Old Post should appear");
        assert!(new_pos < mid_pos, "New should come before Mid");
        assert!(mid_pos < old_pos, "Mid should come before Old");
    }

    #[test]
    fn test_feed_respects_max_posts() {
        let config = test_config();
        let posts: Vec<CollectionItem> = (0..30)
            .map(|i| {
                make_post(
                    &format!("post-{i}"),
                    &format!("Post {i}"),
                    &format!("2024-01-{:02}", (i % 28) + 1),
                    "content",
                    None,
                    vec![],
                )
            })
            .collect();
        let options = FeedOptions { max_posts: 10 };
        let xml = generate_atom_feed(&posts, &config, &options);
        let entry_count = xml.matches("<entry>").count();
        assert_eq!(entry_count, 10);
    }

    #[test]
    fn test_feed_excludes_posts_without_date() {
        let config = test_config();
        let mut post_no_date =
            make_post("no-date", "No Date", "2024-01-01", "content", None, vec![]);
        post_no_date.date = None;
        let posts = vec![
            make_post(
                "with-date",
                "With Date",
                "2024-01-01",
                "content",
                None,
                vec![],
            ),
            post_no_date,
        ];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        let entry_count = xml.matches("<entry>").count();
        assert_eq!(entry_count, 1);
        assert!(xml.contains("With Date"));
        assert!(!xml.contains("No Date"));
    }

    // ========================================================================
    // XML escaping
    // ========================================================================

    #[test]
    fn test_xml_escape_ampersand() {
        assert_eq!(xml_escape("A & B"), "A &amp; B");
    }

    #[test]
    fn test_xml_escape_angle_brackets() {
        assert_eq!(xml_escape("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn test_xml_escape_quotes() {
        assert_eq!(xml_escape("it's \"great\""), "it&apos;s &quot;great&quot;");
    }

    #[test]
    fn test_feed_escapes_title_with_special_chars() {
        let config = test_config();
        let posts = vec![make_post(
            "special",
            "Tips & Tricks: <Rust>",
            "2024-01-15",
            "content",
            None,
            vec![],
        )];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        assert!(xml.contains("Tips &amp; Tricks: &lt;Rust&gt;"));
    }

    // ========================================================================
    // Date formatting
    // ========================================================================

    #[test]
    fn test_format_date_only() {
        assert_eq!(
            format_date_to_rfc3339("2024-01-15"),
            "2024-01-15T00:00:00+00:00"
        );
    }

    #[test]
    fn test_format_already_rfc3339() {
        assert_eq!(
            format_date_to_rfc3339("2024-01-15T14:30:00+00:00"),
            "2024-01-15T14:30:00+00:00"
        );
    }

    // ========================================================================
    // Write to disk
    // ========================================================================

    #[test]
    fn test_write_atom_feed_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config();
        let posts = vec![make_post(
            "hello",
            "Hello",
            "2024-01-15",
            "content",
            None,
            vec![],
        )];
        write_atom_feed(&posts, &config, &FeedOptions::default(), dir.path()).unwrap();
        let feed_path = dir.path().join("feed.xml");
        assert!(feed_path.exists());
        let content = fs::read_to_string(&feed_path).unwrap();
        assert!(content.starts_with("<?xml"));
        assert!(content.contains("<entry>"));
    }

    #[test]
    fn test_write_atom_feed_empty_posts() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config();
        write_atom_feed(&[], &config, &FeedOptions::default(), dir.path()).unwrap();
        let content = fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        assert!(content.starts_with("<?xml"));
        assert!(!content.contains("<entry>"));
    }

    // ========================================================================
    // Feed updated time
    // ========================================================================

    #[test]
    fn test_feed_updated_uses_newest_post_date() {
        let config = test_config();
        let posts = vec![
            make_post("old", "Old", "2023-06-01", "old", None, vec![]),
            make_post("new", "New", "2024-03-15", "new", None, vec![]),
        ];
        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());
        // The first <updated> in the feed header should be the newest post date
        let feed_updated_start = xml.find("<updated>").unwrap();
        let feed_updated_end = xml[feed_updated_start..].find("</updated>").unwrap();
        let feed_updated = &xml[feed_updated_start + 9..feed_updated_start + feed_updated_end];
        assert_eq!(feed_updated, "2024-03-15T00:00:00+00:00");
    }

    // ========================================================================
    // Integration: real DataTalks.Club posts
    // ========================================================================

    #[test]
    fn test_feed_with_real_posts() {
        let site_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let config = SiteConfig::from_file(&site_dir.join("_config.yml")).unwrap();
        let (posts, _errors) =
            crate::collection::load_collection("posts", &site_dir, &config).unwrap();

        // Should have some posts
        assert!(!posts.is_empty(), "Should load at least some posts");

        let xml = generate_atom_feed(&posts, &config, &FeedOptions::default());

        // Basic structure checks
        assert!(xml.starts_with("<?xml"));
        assert!(xml.contains("xmlns=\"http://www.w3.org/2005/Atom\""));
        assert!(xml.contains("<title type=\"html\">DataTalks.Club</title>"));
        assert!(xml.contains("https://datatalks.club/feed.xml"));
        assert!(xml.contains("<entry>"));

        // Should have at most DEFAULT_MAX_POSTS entries
        let entry_count = xml.matches("<entry>").count();
        assert!(entry_count <= DEFAULT_MAX_POSTS);
        assert!(entry_count > 0);

        // Each entry should have required Atom elements
        assert_eq!(
            xml.matches("<entry>").count(),
            xml.matches("</entry>").count()
        );
        assert_eq!(
            xml.matches("<entry>").count(),
            xml.matches("<published>").count()
        );
    }
}
