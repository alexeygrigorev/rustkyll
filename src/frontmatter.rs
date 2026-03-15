use std::collections::HashMap;

use pulldown_cmark::{html, Options, Parser};

/// Errors that can occur when parsing a document.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("failed to parse YAML front matter: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("failed to parse YAML front matter (lenient): {0}")]
    YamlLenient(#[from] crate::yaml::YamlParseError),
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

    // Search for closing --- on its own line by scanning for line boundaries
    // directly in the byte string. This correctly handles both LF and CRLF
    // line endings without cumulative byte offset drift (which was the cause
    // of the Unicode slicing panic in issue #78).
    let mut line_start = 0;
    while line_start < rest.len() {
        // Find where this line ends (at the next \n, or end of string).
        let newline_pos = rest[line_start..].find('\n');
        let line_end = newline_pos.map(|p| line_start + p).unwrap_or(rest.len());

        // Extract the line content (without the trailing \n).
        let line = &rest[line_start..line_end];

        if line.trim() == "---" {
            // YAML content is everything before this line's start.
            let yaml_str = &rest[..line_start];
            // Body starts after the closing --- line (past the \n).
            let body = if line_end < rest.len() {
                &rest[line_end + 1..]
            } else {
                ""
            };
            return (Some(yaml_str), body);
        }

        // Advance to the next line. If no newline was found, we're done.
        match newline_pos {
            Some(_) => line_start = line_end + 1,
            None => break,
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
            let parsed: Option<FrontMatter> = crate::yaml::from_str_lenient(yaml)?;
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
    // D5: Enable smart punctuation to match kramdown's smart quote behavior.
    // kramdown converts straight quotes to curly quotes by default.
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    // Escape parenthesis-style ordered list markers (e.g., "1) text") because
    // kramdown does not support `)` as a list delimiter -- only `.`.
    // pulldown-cmark (CommonMark) would treat these as ordered lists.
    let markdown = escape_paren_list_markers(markdown);

    // Protect Liquid tags from smart punctuation by replacing quotes inside
    // {% %} and {{ }} patterns with placeholders.
    let protected = protect_liquid_quotes(&markdown);

    let parser = Parser::new_ext(&protected, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Restore protected quotes
    let html_output = restore_liquid_quotes(&html_output);

    // Apply kramdown compatibility post-processing
    crate::kramdown::postprocess(&html_output)
}

/// Escape parenthesis-style ordered list markers to prevent pulldown-cmark
/// from treating them as ordered lists. Kramdown only uses `.` as a list
/// delimiter, not `)`, so `1) text` should be treated as a paragraph.
///
/// This converts `1) ` at the start of a line to `1\) ` so the backslash
/// escapes the parenthesis in CommonMark. Only applies outside of code blocks
/// and HTML blocks.
fn escape_paren_list_markers(markdown: &str) -> String {
    let mut result = String::with_capacity(markdown.len());
    let mut in_code_block = false;
    let mut in_html_block = false;

    for line in markdown.split('\n') {
        if !result.is_empty() {
            result.push('\n');
        }

        // Track fenced code blocks
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            result.push_str(line);
            continue;
        }

        if in_code_block {
            result.push_str(line);
            continue;
        }

        // Track HTML blocks (simple heuristic: lines starting with <)
        if trimmed.starts_with('<') && !trimmed.starts_with("</") {
            in_html_block = true;
        }
        if in_html_block {
            result.push_str(line);
            // End HTML block on blank line or closing tag
            if trimmed.is_empty() {
                in_html_block = false;
            }
            continue;
        }

        // Check for N) pattern at start of line (with optional leading whitespace)
        let leading_spaces = line.len() - trimmed.len();
        if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
            // Check for more digits followed by ") "
            let digits_end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            let after_digits = &rest[digits_end..];
            if after_digits.starts_with(") ") || after_digits == ")" {
                // Escape the closing parenthesis
                result.push_str(&line[..leading_spaces]);
                let digit_count = trimmed.len() - rest.len() + digits_end;
                result.push_str(&trimmed[..digit_count]);
                result.push_str("\\)");
                result.push_str(&after_digits[1..]); // skip the original )
                continue;
            }
        }

        result.push_str(line);
    }

    result
}

/// Replace double quotes inside Liquid tags with a placeholder to prevent
/// smart punctuation from converting them to curly quotes.
fn protect_liquid_quotes(input: &str) -> String {
    // Sentinel that won't appear in normal text and won't be modified by markdown
    const QUOTE_PLACEHOLDER: &str = "\x00QUOT\x00";

    let mut result = String::with_capacity(input.len());
    let mut remaining = input;

    while !remaining.is_empty() {
        // Find next Liquid tag opening
        let tag_start = remaining.find("{%").or_else(|| remaining.find("{{"));

        if let Some(start) = tag_start {
            // Copy everything before the tag
            result.push_str(&remaining[..start]);

            let opener = &remaining[start..start + 2];
            let closer = if opener == "{%" { "%}" } else { "}}" };

            if let Some(end) = remaining[start + 2..].find(closer) {
                let tag_end = start + 2 + end + closer.len();
                let tag_content = &remaining[start..tag_end];
                // Replace double quotes inside the tag with placeholder
                result.push_str(&tag_content.replace('"', QUOTE_PLACEHOLDER));
                remaining = &remaining[tag_end..];
            } else {
                // No closing tag found, copy rest as-is
                result.push_str(remaining);
                return result;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Restore placeholders back to double quotes.
fn restore_liquid_quotes(input: &str) -> String {
    const QUOTE_PLACEHOLDER: &str = "\x00QUOT\x00";
    input.replace(QUOTE_PLACEHOLDER, "\"")
}

/// Dedent lines inside HTML blocks that have 4+ spaces of leading whitespace.
///
/// In CommonMark, 4+ spaces of indentation creates an indented code block.
/// When Liquid includes produce HTML output with indentation (e.g., from
/// `{% for %}` loops), the indented `<a>`, `<div>`, `<h3>` tags get treated
/// as code blocks by pulldown-cmark, causing them to be HTML-escaped inside
/// `<pre><code>` blocks.
///
/// Jekyll uses kramdown, which is more lenient about indentation inside HTML.
/// This function normalizes the indentation to prevent the code-block issue
/// while preserving actual indented code blocks (those not containing HTML tags).
///
/// The algorithm: reduce any line indented with 4+ spaces to 2 spaces if it
/// looks like it contains an HTML tag (starts with `<` after trimming) or is
/// a blank line within an HTML context.
pub fn dedent_html_lines(content: &str) -> String {
    let mut result = String::with_capacity(content.len());

    for line in content.split('\n') {
        let trimmed = line.trim_start();
        let leading_spaces = line.len() - trimmed.len();

        // Only modify lines with 4+ spaces that look like HTML
        if leading_spaces >= 4 && looks_like_html(trimmed) {
            // Reduce to at most 3 spaces (prevent code-block interpretation)
            let new_indent = leading_spaces.min(3);
            for _ in 0..new_indent {
                result.push(' ');
            }
            result.push_str(trimmed);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    // Remove trailing newline that we added if original didn't end with one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Check if a trimmed line looks like it contains HTML content.
///
/// Returns true for lines that start with an HTML tag, end with an HTML tag,
/// or contain common HTML patterns. Returns false for plain text that should
/// be treated as potential indented code blocks.
fn looks_like_html(trimmed: &str) -> bool {
    if trimmed.is_empty() {
        return false;
    }

    // Lines starting with HTML tags
    if trimmed.starts_with('<') {
        return true;
    }

    // Lines starting with HTML closing tags
    if trimmed.starts_with("</") {
        return true;
    }

    // Lines that end with an HTML tag (e.g., content followed by </div>)
    if trimmed.ends_with('>') {
        return true;
    }

    false
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
        assert!(html.contains("<h2"), "Should contain h2 tag. Got: {}", html);
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
        assert!(
            html.contains("<pre"),
            "Should contain a pre tag. Got: {}",
            html
        );
        assert!(
            html.contains("<code>"),
            "Should contain a code tag. Got: {}",
            html
        );
        assert!(
            html.contains("code here"),
            "Should contain code content. Got: {}",
            html
        );
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
        // The Liquid tag structure is preserved. Note: smart punctuation (D5)
        // converts straight quotes to curly quotes in text context, which is
        // the same behavior as kramdown. In the real pipeline, Liquid tags are
        // resolved before markdown conversion, so this only affects edge cases.
        assert!(
            html.contains("{% include youtube.html"),
            "Liquid tag should be preserved. Got: {}",
            html
        );
        assert!(
            html.contains("abc123"),
            "Liquid tag parameters should be preserved. Got: {}",
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
        assert!(html.contains("<h2"), "Should contain h2 tag. Got: {}", html);
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

    // ========================================================================
    // Issue 43: Duplicate keys in front matter
    // ========================================================================

    #[test]
    fn test_front_matter_duplicate_keys_last_wins() {
        let input = "---\ntitle: First Title\nlayout: post\ntitle: Second Title\n---\nBody here";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Second Title")
        );
        assert_eq!(
            doc.front_matter.get("layout").and_then(Value::as_str),
            Some("post")
        );
    }

    // ========================================================================
    // Issue 71: dedent_html_lines tests
    // ========================================================================

    #[test]
    fn test_dedent_html_lines_reduces_indented_html_tags() {
        let input = "    <a href=\"/test.html\">Link</a>\n    <div>Content</div>";
        let result = dedent_html_lines(input);
        // Should reduce 4-space indent to 3 spaces (below code-block threshold)
        assert!(
            !result.starts_with("    <a"),
            "Should reduce indentation below 4 spaces, got: {:?}",
            result
        );
        assert!(
            result.contains("<a href=\"/test.html\">Link</a>"),
            "HTML content should be preserved, got: {:?}",
            result
        );
    }

    #[test]
    fn test_dedent_html_lines_preserves_non_html_indentation() {
        // Plain text with 4+ spaces should NOT be dedented (it's a code block)
        let input = "    let x = 42;";
        let result = dedent_html_lines(input);
        assert_eq!(
            result, input,
            "Non-HTML indented lines should be preserved as-is"
        );
    }

    #[test]
    fn test_dedent_html_lines_preserves_less_than_4_spaces() {
        let input = "  <div>OK</div>";
        let result = dedent_html_lines(input);
        assert_eq!(result, input, "Lines with <4 spaces should be unchanged");
    }

    #[test]
    fn test_dedent_html_lines_handles_deep_indentation() {
        let input = "        <h3>Title</h3>";
        let result = dedent_html_lines(input);
        assert!(
            result.contains("<h3>Title</h3>"),
            "Content should be preserved"
        );
        let leading_spaces = result.len() - result.trim_start().len();
        assert!(
            leading_spaces <= 3,
            "Leading spaces should be at most 3, got {}",
            leading_spaces
        );
    }

    #[test]
    fn test_dedent_html_lines_mixed_content() {
        let input = "## Heading\n\n<div class=\"wrapper\">\n    <a href=\"/test\">Link</a>\n    <h3>Title</h3>\n</div>\n\n## Another heading";
        let result = dedent_html_lines(input);
        assert!(
            result.contains("## Heading"),
            "Markdown headings should be preserved"
        );
        assert!(
            result.contains("## Another heading"),
            "Markdown headings should be preserved"
        );
        assert!(
            result.contains("<a href=\"/test\">Link</a>"),
            "HTML links should be preserved"
        );
        // The indented <a> tag should no longer have 4+ spaces
        assert!(
            !result.contains("    <a href"),
            "Indented HTML should be dedented"
        );
    }

    #[test]
    fn test_dedent_html_lines_related_posts_pattern() {
        // Simulates what Liquid outputs after processing related-posts.html include
        let input = r#"<div class="related-posts-section">
  <h2 class="related-posts-title">Related Posts</h2>
  <div class="related-posts-grid">
    <a href="/blog/test.html" class="related-post-card">
      <div class="related-post-content">
        <h3 class="related-post-title">Test Course</h3>
      </div>
    </a>
  </div>
</div>"#;
        let result = dedent_html_lines(input);
        // After dedenting, the markdown processor should not escape the HTML
        let html = markdown_to_html(&result);
        assert!(
            html.contains("<h3 class=\"related-post-title\">Test Course</h3>"),
            "h3 tags should render as HTML, not be escaped. Got: {}",
            html
        );
        assert!(
            html.contains("<a href=\"/blog/test.html\""),
            "Links should render as HTML. Got: {}",
            html
        );
        assert!(
            !html.contains("&lt;a href"),
            "Links should NOT be HTML-escaped. Got: {}",
            html
        );
        assert!(
            !html.contains("<pre><code>"),
            "Should not produce code blocks. Got: {}",
            html
        );
    }

    #[test]
    fn test_dedent_html_lines_preserves_fenced_code_blocks() {
        // Fenced code blocks (```) should not be affected since they use
        // backtick fencing, not indentation
        let input = "```\n    <div>code example</div>\n```";
        let result = dedent_html_lines(input);
        // The <div> inside fenced code is still HTML-looking, but the fenced
        // code block markers ensure it's treated as code by the markdown parser
        let html = markdown_to_html(&result);
        assert!(
            html.contains("<code>"),
            "Fenced code block should still work"
        );
    }

    #[test]
    fn test_markdown_with_embedded_html_after_liquid() {
        // Simulates a markdown file that contains HTML from a Liquid include,
        // which is the pattern for blog posts with {% include related-posts.html %}
        let input = r#"## Introduction

Some markdown text here.

<div class="related-posts-section">
  <h2 class="related-posts-title">Related Posts</h2>
  <div class="related-posts-grid">
    <a href="/blog/course.html" class="related-post-card">
      <div class="related-post-content">
        <h3 class="related-post-title">Course Title</h3>
        <p class="related-post-excerpt">Description here</p>
      </div>
    </a>
  </div>
</div>
"#;
        let dedented = dedent_html_lines(input);
        let html = markdown_to_html(&dedented);

        // Markdown heading should be converted
        assert!(
            html.contains("Introduction</h2>") && html.contains("<h2"),
            "Markdown heading should be converted to HTML. Got: {}",
            html
        );

        // Embedded HTML should be preserved as-is
        assert!(
            html.contains("<h3 class=\"related-post-title\">Course Title</h3>"),
            "Include output HTML should not be escaped. Got: {}",
            html
        );
        assert!(
            !html.contains("&lt;h3"),
            "HTML tags should not be escaped. Got: {}",
            html
        );
    }

    #[test]
    fn test_markdown_headings_with_liquid_html() {
        // Simulates a standalone page like books.md with markdown headings
        // mixed with Liquid-generated HTML
        let input = r#"# Book of the Week

Each week we have a book author coming.

## How it works

* Register on DataTalks.Club
* Join the channel

## Upcoming books

<section class="upcoming-books">
  <div class="books">
    <div class="book-card">Book 1</div>
  </div>
</section>

## Archive

<ul>
  <li>Past book 1</li>
</ul>
"#;
        let dedented = dedent_html_lines(input);
        let html = markdown_to_html(&dedented);

        assert!(
            html.contains("Book of the Week</h1>") && html.contains("<h1"),
            "h1 missing. Got: {}",
            html
        );
        assert!(
            html.contains("How it works</h2>") && html.contains("<h2"),
            "h2 'How it works' missing. Got: {}",
            html
        );
        assert!(
            html.contains("Upcoming books</h2>"),
            "h2 'Upcoming books' missing. Got: {}",
            html
        );
        assert!(
            html.contains("Archive</h2>"),
            "h2 'Archive' missing. Got: {}",
            html
        );
        assert!(
            html.contains("<li>Register on DataTalks.Club</li>"),
            "list items missing"
        );
    }

    // ========================================================================
    // Issue 78: Unicode byte boundary panic with CRLF line endings
    // ========================================================================

    #[test]
    fn test_unicode_curly_quote_lf() {
        // U+2019 RIGHT SINGLE QUOTATION MARK (3 bytes in UTF-8)
        let input = "---\ntitle: 'Strategic Positioning\u{2019}'\n---\nBody here";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Strategic Positioning\u{2019}")
        );
        assert_eq!(doc.content, "Body here");
    }

    #[test]
    fn test_unicode_curly_quote_crlf() {
        // This is the exact reproduction case from issue #78.
        // CRLF line endings + U+2019 curly quote caused a byte boundary panic.
        let input = "---\r\ntitle: 'Strategic Positioning\u{2019}'\r\n---\r\nBody here";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Strategic Positioning\u{2019}")
        );
        assert_eq!(doc.content, "Body here");
    }

    #[test]
    fn test_unicode_emoji_crlf() {
        // 4-byte emoji with CRLF line endings
        let input = "---\r\ntitle: 'Hello \u{1F600} World'\r\nlayout: post\r\n---\r\nBody content";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Hello \u{1F600} World")
        );
        assert_eq!(doc.content, "Body content");
    }

    #[test]
    fn test_unicode_cjk_crlf() {
        // CJK characters (3 bytes each) with CRLF
        let input = "---\r\ntitle: '\u{4F60}\u{597D}\u{4E16}\u{754C}'\r\n---\r\nBody";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("\u{4F60}\u{597D}\u{4E16}\u{754C}")
        );
        assert_eq!(doc.content, "Body");
    }

    #[test]
    fn test_unicode_in_body_crlf() {
        let input = "---\r\ntitle: Test\r\n---\r\nBody with \u{2019}curly\u{2019} quotes";
        let doc = parse_document(input).unwrap();
        assert_eq!(doc.content, "Body with \u{2019}curly\u{2019} quotes");
    }

    #[test]
    fn test_crlf_ascii_only() {
        let input = "---\r\ntitle: Hello\r\nlayout: post\r\n---\r\nBody content here";
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
    fn test_crlf_long_frontmatter_with_unicode() {
        // 50+ lines to accumulate offset drift, with Unicode on the last line
        let mut input = String::from("---\r\n");
        for i in 0..55 {
            input.push_str(&format!("key{}: value{}\r\n", i, i));
        }
        input.push_str("special: 'quote\u{2019}mark'\r\n");
        input.push_str("---\r\n");
        input.push_str("Body after long frontmatter");

        let doc = parse_document(&input).unwrap();
        assert_eq!(
            doc.front_matter.get("special").and_then(Value::as_str),
            Some("quote\u{2019}mark")
        );
        assert_eq!(doc.content, "Body after long frontmatter");
    }

    #[test]
    fn test_mixed_line_endings() {
        // Mix of LF and CRLF within the same file
        let input = "---\ntitle: 'Mixed \u{2019} endings'\r\nlayout: post\n---\r\nBody here";
        let doc = parse_document(&input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Mixed \u{2019} endings")
        );
        assert_eq!(doc.content, "Body here");
    }

    #[test]
    fn test_empty_front_matter_crlf() {
        let input = "---\r\n---\r\nBody after empty front matter";
        let doc = parse_document(input).unwrap();
        assert!(doc.front_matter.is_empty());
        assert_eq!(doc.content, "Body after empty front matter");
    }

    #[test]
    fn test_bom_crlf_unicode() {
        // BOM + CRLF + multi-byte characters
        let input = "\u{feff}---\r\ntitle: 'Hello \u{2019}World\u{2019}'\r\n---\r\nBody";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Hello \u{2019}World\u{2019}")
        );
        assert_eq!(doc.content, "Body");
    }

    #[test]
    fn test_crlf_podcast_pattern_with_curly_quotes() {
        // Simulates the actual DTC podcast episode that triggered the panic.
        // The curly quote U+2019 appears in the title value.
        let input = "---\r\ntitle: \"Building a Sustainable Data Freelancing Career: Market Validation, Client Acquisition & Strategic Positioning\u{2019}\"\r\nseason: 7\r\nepisode: 6\r\nguests:\r\n- jakobgraff\r\nimage: images/podcast/ab-testing.jpg\r\nids:\r\n  anchor: AB-Testing-e1eq73v\r\n  youtube: 0Gqx1LtqRZU\r\n---\r\nTranscript content here.";
        let doc = parse_document(input).unwrap();
        assert!(
            doc.front_matter.get("title").is_some(),
            "title should be parsed"
        );
        assert_eq!(doc.content, "Transcript content here.");
    }

    #[test]
    fn test_split_front_matter_crlf_closing_delimiter() {
        // Verify that the closing --- with CRLF is detected correctly
        let input = "---\r\ntitle: Test\r\n---\r\n";
        let (yaml, body) = split_front_matter(input);
        assert!(yaml.is_some(), "YAML should be detected");
        assert!(body.is_empty() || body.trim().is_empty());
    }

    #[test]
    fn test_unicode_at_exact_offset_boundary() {
        // Construct input where the multi-byte character would be at the exact
        // position where the old code's cumulative drift would cause a panic.
        // With CRLF, each line undercounts by 1 byte. After N lines the drift is N bytes.
        // Place a 3-byte character so the old offset would land inside it.
        let mut input = String::from("---\r\n");
        // 10 lines of short content to create 10-byte drift
        for _ in 0..10 {
            input.push_str("k: v\r\n");
        }
        // Add a line with a multi-byte char near where the drift would cause slicing
        input.push_str("z: '\u{2019}\u{2019}\u{2019}'\r\n");
        input.push_str("---\r\n");
        input.push_str("Body");

        let doc = parse_document(&input).unwrap();
        assert_eq!(doc.content, "Body");
        assert_eq!(
            doc.front_matter.get("z").and_then(Value::as_str),
            Some("\u{2019}\u{2019}\u{2019}")
        );
    }

    // ========================================================================
    // escape_paren_list_markers tests
    // ========================================================================

    #[test]
    fn test_escape_paren_list_markers_basic() {
        let input = "1) First item\n2) Second item";
        let result = escape_paren_list_markers(input);
        assert_eq!(result, "1\\) First item\n2\\) Second item");
    }

    #[test]
    fn test_escape_paren_list_markers_dot_style_unaffected() {
        let input = "1. First item\n2. Second item";
        let result = escape_paren_list_markers(input);
        assert_eq!(
            result, input,
            "Dot-style list markers should not be escaped"
        );
    }

    #[test]
    fn test_escape_paren_list_markers_inside_code_block() {
        let input = "```\n1) code line\n```\n1) outside code";
        let result = escape_paren_list_markers(input);
        assert!(
            result.contains("1) code line"),
            "Should not escape inside code blocks. Got: {}",
            result
        );
        assert!(
            result.contains("1\\) outside code"),
            "Should escape outside code blocks. Got: {}",
            result
        );
    }

    #[test]
    fn test_escape_paren_list_markers_mid_sentence() {
        let input = "This has 1) in the middle";
        let result = escape_paren_list_markers(input);
        assert_eq!(result, input, "Should not escape when not at start of line");
    }

    #[test]
    fn test_escape_paren_list_markers_multi_digit() {
        let input = "10) Tenth item";
        let result = escape_paren_list_markers(input);
        assert_eq!(result, "10\\) Tenth item");
    }

    #[test]
    fn test_escape_paren_list_markers_renders_as_paragraph() {
        // Verify that after escaping, markdown_to_html produces a <p> tag, not <ol>
        let input = "1) First item";
        let html = markdown_to_html(input);
        assert!(
            !html.contains("<ol>"),
            "Escaped paren marker should not produce <ol>. Got: {}",
            html
        );
        assert!(
            html.contains("<p>"),
            "Escaped paren marker should produce <p>. Got: {}",
            html
        );
    }
}
