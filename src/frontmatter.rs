use std::collections::HashMap;

use pulldown_cmark::{html, Options, Parser};

/// Errors that can occur when parsing a document.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("failed to parse YAML front matter: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// YAML front matter as a flexible key-value map.
pub type FrontMatter = HashMap<String, serde_yaml::Value>;

/// The excerpt separator used in Jekyll-style markdown files.
const EXCERPT_SEPARATOR: &str = "<!--more-->";

/// A parsed document consisting of YAML front matter and markdown body.
#[derive(Debug)]
pub struct Document {
    /// Parsed YAML front matter key-value pairs.
    pub front_matter: FrontMatter,
    /// Raw markdown body (everything after front matter).
    pub content: String,
    /// Content before `<!--more-->` separator, if present.
    pub excerpt: Option<String>,
}

/// Split raw text into optional YAML front matter string and markdown body.
///
/// Front matter is delimited by `---` on its own line at the very start of the file.
/// Returns `(yaml_str, body)`. If no front matter is detected, returns `(None, full_input)`.
fn split_front_matter(input: &str) -> (Option<&str>, &str) {
    // Front matter must start with "---" on the first line.
    let trimmed = input.trim_start_matches('\u{feff}'); // strip BOM if present
    if !trimmed.starts_with("---") {
        return (None, input);
    }

    // Find the closing "---" delimiter. It must appear on its own line
    // after the opening one.
    let after_opening = &trimmed[3..];
    // Skip past the newline after the opening ---
    let rest = if let Some(stripped) = after_opening.strip_prefix('\n') {
        stripped
    } else if let Some(stripped) = after_opening.strip_prefix("\r\n") {
        stripped
    } else {
        // Opening --- is not followed by a newline -- not valid front matter.
        return (None, input);
    };

    // Search for closing --- on its own line.
    for (i, line) in rest.lines().enumerate() {
        if line.trim() == "---" {
            // Calculate byte offset of this closing delimiter in `rest`.
            let byte_offset: usize = rest.lines().take(i).map(|l| l.len() + 1).sum();
            let yaml_str = &rest[..byte_offset];
            // Body starts after the closing --- line.
            let after_close = &rest[byte_offset..];
            // Skip the "---" line itself.
            let body = if let Some(pos) = after_close.find('\n') {
                &after_close[pos + 1..]
            } else {
                // Nothing after the closing ---
                ""
            };
            return (Some(yaml_str), body);
        }
    }

    // No closing delimiter found -- treat entire input as body with no front matter.
    (None, input)
}

/// Extract the excerpt (content before `<!--more-->`) from markdown content.
fn extract_excerpt(content: &str) -> Option<String> {
    content.find(EXCERPT_SEPARATOR).map(|pos| {
        let excerpt = content[..pos].trim().to_string();
        if excerpt.is_empty() {
            String::new()
        } else {
            excerpt
        }
    })
}

/// Parse a string containing optional YAML front matter and a markdown body.
///
/// Returns a `Document` with parsed front matter, raw markdown content,
/// and an optional excerpt (text before `<!--more-->`).
///
/// # Errors
///
/// Returns `ParseError::Yaml` if the front matter block contains invalid YAML.
pub fn parse_document(input: &str) -> Result<Document, ParseError> {
    let (yaml_str, body) = split_front_matter(input);

    let front_matter = match yaml_str {
        Some(yaml) => {
            let parsed: Option<FrontMatter> = serde_yaml::from_str(yaml)?;
            parsed.unwrap_or_default()
        }
        None => FrontMatter::new(),
    };

    let content = body.to_string();
    let excerpt = extract_excerpt(&content);

    Ok(Document {
        front_matter,
        content,
        excerpt,
    })
}

/// Convert a markdown string to HTML.
///
/// Supports headings, paragraphs, links, images, bold/italic, blockquotes,
/// code blocks, lists, horizontal rules, and raw HTML passthrough
/// (including Liquid-like tags such as `{% include ... %}`).
pub fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    // ========================================================================
    // Front matter splitting tests
    // ========================================================================

    #[test]
    fn test_parse_standard_front_matter() {
        let input = "---\ntitle: Hello\nlayout: post\n---\nBody content here";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Hello")
        );
        assert_eq!(
            doc.front_matter.get("layout").and_then(Value::as_str),
            Some("post")
        );
        assert_eq!(doc.content, "Body content here");
    }

    #[test]
    fn test_parse_no_front_matter() {
        let input = "Just some markdown\n\nWith paragraphs.";
        let doc = parse_document(input).unwrap();
        assert!(doc.front_matter.is_empty());
        assert_eq!(doc.content, input);
    }

    #[test]
    fn test_parse_empty_front_matter() {
        let input = "---\n---\nBody after empty front matter";
        let doc = parse_document(input).unwrap();
        assert!(doc.front_matter.is_empty());
        assert_eq!(doc.content, "Body after empty front matter");
    }

    #[test]
    fn test_hr_in_body_not_confused_with_front_matter() {
        let input = "---\ntitle: Test\n---\nSome text\n\n---\n\nMore text after HR";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Test")
        );
        assert!(doc.content.contains("---"));
        assert!(doc.content.contains("More text after HR"));
    }

    #[test]
    fn test_front_matter_with_blank_line_after_opening() {
        let input = "---\n\ntitle: Test\n---\nBody";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Test")
        );
        assert_eq!(doc.content, "Body");
    }

    // ========================================================================
    // YAML value types tests
    // ========================================================================

    #[test]
    fn test_yaml_simple_string() {
        let input = "---\ntitle: \"Test Title\"\n---\n";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Test Title")
        );
    }

    #[test]
    fn test_yaml_inline_list() {
        let input = "---\nauthors: [alice, bob]\n---\n";
        let doc = parse_document(input).unwrap();
        let authors = doc.front_matter.get("authors").unwrap();
        let seq = authors.as_sequence().unwrap();
        assert_eq!(seq.len(), 2);
        assert_eq!(seq[0].as_str(), Some("alice"));
        assert_eq!(seq[1].as_str(), Some("bob"));
    }

    #[test]
    fn test_yaml_block_list() {
        let input = "---\ntags:\n- analytics\n- clustering\n---\n";
        let doc = parse_document(input).unwrap();
        let tags = doc.front_matter.get("tags").unwrap();
        let seq = tags.as_sequence().unwrap();
        assert_eq!(seq.len(), 2);
        assert_eq!(seq[0].as_str(), Some("analytics"));
        assert_eq!(seq[1].as_str(), Some("clustering"));
    }

    #[test]
    fn test_yaml_nested_object() {
        let input = "---\nids:\n  anchor: ABC\n  youtube: XYZ\n---\n";
        let doc = parse_document(input).unwrap();
        let ids = doc.front_matter.get("ids").unwrap();
        let map = ids.as_mapping().unwrap();
        assert_eq!(
            map.get(Value::String("anchor".into()))
                .and_then(Value::as_str),
            Some("ABC")
        );
        assert_eq!(
            map.get(Value::String("youtube".into()))
                .and_then(Value::as_str),
            Some("XYZ")
        );
    }

    #[test]
    fn test_yaml_date_value() {
        let input = "---\nstart: 2020-12-14 00:00:00\n---\n";
        let doc = parse_document(input).unwrap();
        // serde_yaml parses bare dates/datetimes as strings
        let start = doc.front_matter.get("start").unwrap();
        // It should be preserved as some kind of value (string or tagged)
        assert!(start.as_str().is_some() || start.is_string());
    }

    #[test]
    fn test_yaml_null_empty_value() {
        let input = "---\ndescription:\n---\n";
        let doc = parse_document(input).unwrap();
        let desc = doc.front_matter.get("description").unwrap();
        assert!(desc.is_null());
    }

    // ========================================================================
    // Excerpt extraction tests
    // ========================================================================

    #[test]
    fn test_excerpt_with_separator() {
        let input = "---\ntitle: Test\n---\nFirst paragraph.\n\n<!--more-->\n\nRest of content.";
        let doc = parse_document(input).unwrap();
        assert_eq!(doc.excerpt, Some("First paragraph.".to_string()));
        assert!(doc.content.contains("Rest of content."));
    }

    #[test]
    fn test_excerpt_without_separator() {
        let input = "---\ntitle: Test\n---\nJust content, no separator.";
        let doc = parse_document(input).unwrap();
        assert!(doc.excerpt.is_none());
    }

    #[test]
    fn test_excerpt_separator_at_beginning() {
        let input = "---\ntitle: Test\n---\n<!--more-->\nContent after.";
        let doc = parse_document(input).unwrap();
        assert_eq!(doc.excerpt, Some(String::new()));
    }

    // ========================================================================
    // Markdown to HTML conversion tests
    // ========================================================================

    #[test]
    fn test_md_heading() {
        let html = markdown_to_html("## Hello");
        assert!(html.contains("<h2>"));
        assert!(html.contains("Hello"));
        assert!(html.contains("</h2>"));
    }

    #[test]
    fn test_md_bold_italic() {
        let html = markdown_to_html("This is **bold** and *italic* text.");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_md_link() {
        let html = markdown_to_html("[text](https://example.com)");
        assert!(html.contains("<a href=\"https://example.com\">text</a>"));
    }

    #[test]
    fn test_md_code_block() {
        let html = markdown_to_html("```\ncode here\n```");
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code>"));
        assert!(html.contains("code here"));
    }

    #[test]
    fn test_md_blockquote() {
        let html = markdown_to_html("> This is a quote");
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("This is a quote"));
    }

    #[test]
    fn test_md_raw_html_passthrough() {
        let html = markdown_to_html("<figure><img src=\"test.jpg\"></figure>");
        assert!(html.contains("<figure>"));
        assert!(html.contains("<img src=\"test.jpg\">"));
        assert!(html.contains("</figure>"));
    }

    #[test]
    fn test_md_liquid_tags_preserved() {
        let input = "Some text\n\n{% include youtube.html video_id=\"abc123\" %}\n\nMore text";
        let html = markdown_to_html(input);
        assert!(
            html.contains("{% include youtube.html video_id=\"abc123\" %}"),
            "Liquid tag should be preserved. Got: {}",
            html
        );
    }

    // ========================================================================
    // Integration tests with real Jekyll content patterns
    // ========================================================================

    #[test]
    fn test_real_post_pattern() {
        let input = r#"---
layout: post
title: 'Customer Segmentation with RFM+'
subtitle: Build a 5D RFM+ framework
description: Customer segmentation with limited data.
image: images/posts/2020-11-29-segmentation/cover.jpg
authors:
- nishantmohan
tags:
- analytics
- clustering
datepublished: '2020-11-29'
date: '2020-11-29'
---

## Background

There's a specific part of job-hunting that I look forward to.

<!--more-->

## Introduction

They asked me to perform customer segmentation.

<figure>
<img src="/images/posts/test.jpg" />
</figure>

{% include youtube.html video_id="pWqD7SGuihs" %}
"#;
        let doc = parse_document(input).unwrap();

        // Verify front matter fields
        assert_eq!(
            doc.front_matter.get("layout").and_then(Value::as_str),
            Some("post")
        );
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Customer Segmentation with RFM+")
        );

        // Authors list
        let authors = doc.front_matter.get("authors").unwrap();
        let seq = authors.as_sequence().unwrap();
        assert_eq!(seq.len(), 1);
        assert_eq!(seq[0].as_str(), Some("nishantmohan"));

        // Tags list
        let tags = doc.front_matter.get("tags").unwrap();
        let tag_seq = tags.as_sequence().unwrap();
        assert_eq!(tag_seq.len(), 2);

        // Date string
        assert_eq!(
            doc.front_matter.get("date").and_then(Value::as_str),
            Some("2020-11-29")
        );

        // Excerpt
        assert!(doc.excerpt.is_some());
        let excerpt = doc.excerpt.unwrap();
        assert!(excerpt.contains("Background"));
        assert!(!excerpt.contains("Introduction"));

        // Content
        assert!(doc.content.contains("Introduction"));
        assert!(doc.content.contains("{% include youtube.html"));

        // HTML conversion
        let html = markdown_to_html(&doc.content);
        assert!(html.contains("<h2>"));
        assert!(html.contains("<figure>"));
        assert!(html.contains("{% include youtube.html"));
    }

    #[test]
    fn test_real_people_pattern() {
        let input = r#"---
short: 16rahuljain
title: "Rahul Jain"
picture: "images/authors/16rahuljain.jpg"
linkedin: 16rahuljain

---

Rahul has over 12 years of experience in data and engineering."#;
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("short").and_then(Value::as_str),
            Some("16rahuljain")
        );
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Rahul Jain")
        );
        assert_eq!(
            doc.front_matter.get("picture").and_then(Value::as_str),
            Some("images/authors/16rahuljain.jpg")
        );
        assert_eq!(
            doc.front_matter.get("linkedin").and_then(Value::as_str),
            Some("16rahuljain")
        );
        assert!(doc.content.contains("Rahul has over 12 years"));
    }

    #[test]
    fn test_real_book_pattern_deeply_nested() {
        let input = r#"---
title: "Machine Learning Bookcamp"
description: "Book of the Week"
start: 2020-12-14 00:00:00
end: 2020-12-18 23:59:59
authors: [alexeygrigorev]
links:
  - text: Book's page on Manning
    link: http://bit.ly/mlbookcamp
  - text: Book's GitHub repository
    link: https://github.com/alexeygrigorev/mlbookcamp-code
archive:
- name: Vladimir Finkelshtein
  text: "First question."
  replies:
  - name: Alexey Grigorev
    text: "Answer here."
---

Book description body.
"#;
        let doc = parse_document(input).unwrap();

        // Inline list
        let authors = doc.front_matter.get("authors").unwrap();
        assert_eq!(authors.as_sequence().unwrap().len(), 1);

        // Nested links
        let links = doc.front_matter.get("links").unwrap();
        let links_seq = links.as_sequence().unwrap();
        assert_eq!(links_seq.len(), 2);
        let first_link = links_seq[0].as_mapping().unwrap();
        assert_eq!(
            first_link
                .get(Value::String("text".into()))
                .and_then(Value::as_str),
            Some("Book's page on Manning")
        );

        // Deeply nested archive with replies
        let archive = doc.front_matter.get("archive").unwrap();
        let archive_seq = archive.as_sequence().unwrap();
        assert_eq!(archive_seq.len(), 1);
        let first_entry = archive_seq[0].as_mapping().unwrap();
        let replies = first_entry
            .get(Value::String("replies".into()))
            .unwrap()
            .as_sequence()
            .unwrap();
        assert_eq!(replies.len(), 1);
        assert_eq!(
            replies[0]
                .as_mapping()
                .unwrap()
                .get(Value::String("name".into()))
                .and_then(Value::as_str),
            Some("Alexey Grigorev")
        );
    }

    #[test]
    fn test_real_podcast_pattern_nested_ids_and_links() {
        let input = r#"---
title: 'A/B Testing'
short: A/B Testing
season: 7
episode: 6
guests:
- jakobgraff
image: images/podcast/ab-testing.jpg
ids:
  anchor: AB-Testing-e1eq73v
  youtube: 0Gqx1LtqRZU
links:
  anchor: https://anchor.fm/datatalksclub/episodes/AB-Testing-e1eq73v
  apple: https://podcasts.apple.com/podcast/id1541710331
  spotify: https://open.spotify.com/episode/3LhBOO1UANCGbOwkntZt4j
  youtube: https://www.youtube.com/watch?v=0Gqx1LtqRZU
---

Transcript content here.
"#;
        let doc = parse_document(input).unwrap();

        // Nested ids map
        let ids = doc.front_matter.get("ids").unwrap();
        let ids_map = ids.as_mapping().unwrap();
        assert_eq!(
            ids_map
                .get(Value::String("anchor".into()))
                .and_then(Value::as_str),
            Some("AB-Testing-e1eq73v")
        );
        assert_eq!(
            ids_map
                .get(Value::String("youtube".into()))
                .and_then(Value::as_str),
            Some("0Gqx1LtqRZU")
        );

        // Nested links map (as a mapping, not a sequence)
        let links = doc.front_matter.get("links").unwrap();
        let links_map = links.as_mapping().unwrap();
        assert_eq!(
            links_map
                .get(Value::String("spotify".into()))
                .and_then(Value::as_str),
            Some("https://open.spotify.com/episode/3LhBOO1UANCGbOwkntZt4j")
        );

        // Season/episode as integers
        let season = doc.front_matter.get("season").unwrap();
        assert_eq!(season.as_u64(), Some(7));
    }
}
