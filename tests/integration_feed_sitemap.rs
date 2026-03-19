//! Unit tests for XML parsing helpers used in feed/sitemap validation.
//!
//! The actual integration tests (which build real sites) have been moved to
//! integration_tests/tests/integration_feed_sitemap.rs.

// ============================================================================
// XML parsing helpers
// ============================================================================

/// Extract all text content between matching open/close tags from XML.
/// Does NOT handle nested tags of the same name (fine for feed/sitemap parsing).
fn extract_elements(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut results = Vec::new();
    let mut search_from = 0;

    while let Some(start_pos) = xml[search_from..].find(&open) {
        let abs_start = search_from + start_pos;
        // Find the end of the opening tag (handle attributes)
        if let Some(tag_end) = xml[abs_start..].find('>') {
            let content_start = abs_start + tag_end + 1;
            if let Some(end_pos) = xml[content_start..].find(&close) {
                let content = &xml[content_start..content_start + end_pos];
                results.push(content.to_string());
                search_from = content_start + end_pos + close.len();
            } else {
                break;
            }
        } else {
            break;
        }
    }
    results
}

/// Extract attribute value from an element. Finds first occurrence of
/// `<tag ... attr="value" ...>` and returns the value.
fn extract_attribute(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let open = format!("<{}", tag);
    if let Some(start) = xml.find(&open) {
        // Find the end of this tag
        let tag_region = &xml[start..];
        let end = tag_region.find('>')?;
        let tag_str = &tag_region[..end];

        // Look for attr="value" or attr='value'
        let attr_prefix = format!("{}=\"", attr);
        if let Some(attr_start) = tag_str.find(&attr_prefix) {
            let value_start = attr_start + attr_prefix.len();
            if let Some(value_end) = tag_str[value_start..].find('"') {
                return Some(tag_str[value_start..value_start + value_end].to_string());
            }
        }
        // Try single quotes
        let attr_prefix_sq = format!("{}='", attr);
        if let Some(attr_start) = tag_str.find(&attr_prefix_sq) {
            let value_start = attr_start + attr_prefix_sq.len();
            if let Some(value_end) = tag_str[value_start..].find('\'') {
                return Some(tag_str[value_start..value_start + value_end].to_string());
            }
        }
    }
    None
}

/// Extract all `<link ... href="..." rel="..."/>` style self-closing or open tags.
/// Returns vec of (rel, href) pairs.
fn extract_link_elements(xml: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut search_from = 0;

    while let Some(start) = xml[search_from..].find("<link") {
        let abs_start = search_from + start;
        let tag_region = &xml[abs_start..];
        // Find the end of the tag (either /> or >)
        let end = tag_region
            .find("/>")
            .or_else(|| tag_region.find('>'))
            .unwrap_or(tag_region.len());
        let tag_str = &tag_region[..end];

        let href = extract_attr_from_str(tag_str, "href");
        let rel = extract_attr_from_str(tag_str, "rel");

        if let (Some(h), Some(r)) = (href, rel) {
            results.push((r, h));
        }

        search_from = abs_start + end.max(1);
    }
    results
}

fn extract_attr_from_str(tag_str: &str, attr: &str) -> Option<String> {
    let attr_prefix = format!("{}=\"", attr);
    if let Some(attr_start) = tag_str.find(&attr_prefix) {
        let value_start = attr_start + attr_prefix.len();
        if let Some(value_end) = tag_str[value_start..].find('"') {
            return Some(tag_str[value_start..value_start + value_end].to_string());
        }
    }
    None
}

/// Check if a string looks like RFC 3339 datetime (YYYY-MM-DDTHH:MM:SS...).
fn is_rfc3339_like(s: &str) -> bool {
    let s = s.trim();
    if s.len() < 19 {
        return false;
    }
    // Check YYYY-MM-DDTHH:MM:SS pattern
    let chars: Vec<char> = s.chars().collect();
    chars[4] == '-' && chars[7] == '-' && chars[10] == 'T' && chars[13] == ':' && chars[16] == ':'
}

/// Check if string contains raw Liquid tags.
fn contains_liquid_tags(s: &str) -> bool {
    s.contains("{{") || s.contains("{%")
}

/// Check if two counts are within tolerance (percentage).
fn within_tolerance(a: usize, b: usize, tolerance_pct: f64) -> bool {
    if a == b {
        return true;
    }
    let max = a.max(b) as f64;
    let min = a.min(b) as f64;
    let diff_pct = (max - min) / max * 100.0;
    diff_pct <= tolerance_pct
}

// ============================================================================
// Unit tests for XML helpers
// ============================================================================

#[test]
fn test_extract_elements_atom_feed() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test Blog</title>
  <entry>
    <title>Post One</title>
    <link href="https://example.com/post-one.html" rel="alternate"/>
    <published>2024-01-15T00:00:00+00:00</published>
    <updated>2024-01-15T00:00:00+00:00</updated>
    <id>https://example.com/post-one.html</id>
  </entry>
  <entry>
    <title>Post Two</title>
    <link href="https://example.com/post-two.html" rel="alternate"/>
    <published>2024-01-10T00:00:00+00:00</published>
    <updated>2024-01-10T00:00:00+00:00</updated>
    <id>https://example.com/post-two.html</id>
  </entry>
</feed>"#;

    let entries = extract_elements(xml, "entry");
    assert_eq!(entries.len(), 2, "Should find 2 entries");

    let titles = extract_elements(xml, "title");
    assert_eq!(titles.len(), 3, "Should find 3 titles (1 feed + 2 entries)");
    assert_eq!(titles[0], "Test Blog");
    assert_eq!(titles[1], "Post One");
    assert_eq!(titles[2], "Post Two");

    let published = extract_elements(xml, "published");
    assert_eq!(published.len(), 2);
    assert!(is_rfc3339_like(&published[0]));
}

#[test]
fn test_extract_elements_sitemap() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>https://example.com/</loc></url>
  <url><loc>https://example.com/about.html</loc></url>
  <url><loc>https://example.com/posts/hello.html</loc></url>
</urlset>"#;

    let locs = extract_elements(xml, "loc");
    assert_eq!(locs.len(), 3);
    assert_eq!(locs[0], "https://example.com/");
    assert_eq!(locs[1], "https://example.com/about.html");
    assert_eq!(locs[2], "https://example.com/posts/hello.html");
}

#[test]
fn test_extract_elements_invalid_xml_no_panic() {
    // Incomplete tag -- should return empty, not panic
    let xml = "<entry><title>Oops";
    let titles = extract_elements(xml, "title");
    assert!(titles.is_empty(), "Should not find complete elements");

    // Empty string
    let empty = extract_elements("", "entry");
    assert!(empty.is_empty());

    // Malformed
    let malformed = extract_elements("<<<not xml at all>>>", "entry");
    assert!(malformed.is_empty());
}

#[test]
fn test_extract_elements_rss() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>My Podcast</title>
    <item>
      <title>Episode 1</title>
      <description>First episode</description>
    </item>
    <item>
      <title>Episode 2</title>
      <description>Second episode</description>
    </item>
  </channel>
</rss>"#;

    let items = extract_elements(xml, "item");
    assert_eq!(items.len(), 2);

    let titles = extract_elements(xml, "title");
    assert_eq!(titles.len(), 3); // channel title + 2 item titles
    assert_eq!(titles[0], "My Podcast");
}

#[test]
fn test_is_rfc3339_like() {
    assert!(is_rfc3339_like("2024-01-15T00:00:00+00:00"));
    assert!(is_rfc3339_like("2024-01-15T12:30:45Z"));
    assert!(!is_rfc3339_like("2024-01-15"));
    assert!(!is_rfc3339_like("not a date"));
    assert!(!is_rfc3339_like(""));
}

#[test]
fn test_contains_liquid_tags() {
    assert!(contains_liquid_tags("Hello {{ name }}"));
    assert!(contains_liquid_tags("{% for x in items %}"));
    assert!(!contains_liquid_tags("Hello World"));
    assert!(!contains_liquid_tags("<loc>https://example.com/</loc>"));
}

#[test]
fn test_within_tolerance() {
    assert!(within_tolerance(100, 100, 5.0));
    assert!(within_tolerance(100, 95, 5.0));
    assert!(within_tolerance(100, 96, 5.0));
    assert!(!within_tolerance(100, 90, 5.0));
    assert!(within_tolerance(0, 0, 5.0));
}

#[test]
fn test_extract_attribute() {
    let xml = r#"<rss version="2.0"><channel></channel></rss>"#;
    let version = extract_attribute(xml, "rss", "version");
    assert_eq!(version.as_deref(), Some("2.0"));
}

#[test]
fn test_extract_link_elements() {
    let xml = r#"
<feed>
  <link href="https://example.com/feed.xml" rel="self" type="application/atom+xml"/>
  <link href="https://example.com/" rel="alternate" type="text/html"/>
</feed>"#;

    let links = extract_link_elements(xml);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].0, "self");
    assert!(links[0].1.contains("feed.xml"));
    assert_eq!(links[1].0, "alternate");
}
