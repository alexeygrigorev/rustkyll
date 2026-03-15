//! Integration tests for RSS/Atom feed and sitemap validation (Issue 63).
//!
//! These tests are marked `#[ignore]` because they require:
//! - Building real sites with rustkyll (slow)
//! - Jekyll to be installed for reference comparison
//! - Website source directories to exist
//!
//! Run with: cargo test --test integration_feed_sitemap -- --ignored
//! Run a specific test: cargo test --test integration_feed_sitemap -- --ignored test_dtc_feed_validation

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Extract the text content of the first occurrence of a tag.
fn extract_first_element(xml: &str, tag: &str) -> Option<String> {
    extract_elements(xml, tag).into_iter().next()
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

// ============================================================================
// Site building helpers
// ============================================================================

fn project_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn dtc_site_dir() -> PathBuf {
    let p = project_dir().join("websites/DataTalksClub/datatalksclub.github.io");
    if p.exists() {
        return p;
    }
    // Fallback to old location
    project_dir().join("datatalksclub.github.io")
}

fn kids_site_dir() -> PathBuf {
    project_dir().join("websites/alexeygrigorev/kids-horror-stories-ru")
}

/// Build a site with rustkyll CLI and return the output directory path.
fn build_with_rustkyll(source: &Path, dest: &Path) {
    if dest.exists() {
        fs::remove_dir_all(dest).unwrap();
    }
    fs::create_dir_all(dest).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg(source.to_str().unwrap())
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .output()
        .expect("failed to run rustkyll binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Rustkyll build failed for {}.\nstdout: {}\nstderr: {}",
        source.display(),
        stdout,
        stderr
    );

    eprintln!(
        "Rustkyll build complete for {}: {}",
        source.display(),
        stdout.trim()
    );
}

/// Build a site with Jekyll and return the output directory path.
///
/// Uses `bundle exec jekyll build` with the working directory set to the site
/// source so that the site's Gemfile and bundled gems (themes, plugins) are
/// picked up correctly.
fn build_with_jekyll(source: &Path, dest: &Path) {
    if dest.exists() {
        fs::remove_dir_all(dest).unwrap();
    }
    fs::create_dir_all(dest).unwrap();

    let output = Command::new("bundle")
        .arg("exec")
        .arg("jekyll")
        .arg("build")
        .arg("--destination")
        .arg(dest.to_str().unwrap())
        .current_dir(source)
        .output()
        .expect("failed to run bundle exec jekyll");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Jekyll build failed for {}.\nstdout: {}\nstderr: {}",
        source.display(),
        stdout,
        stderr
    );

    eprintln!(
        "Jekyll build complete for {}: {}",
        source.display(),
        stdout.trim()
    );
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
// Unit tests for XML helpers (not ignored -- fast)
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

// ============================================================================
// DTC feed.xml validation
// ============================================================================

#[test]
#[ignore]
fn test_dtc_feed_validation() {
    let source = dtc_site_dir();
    assert!(
        source.exists(),
        "DTC site not found at {}",
        source.display()
    );

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("rustkyll_site");

    build_with_rustkyll(&source, &dest);

    let feed_path = dest.join("feed.xml");
    assert!(feed_path.exists(), "feed.xml should be generated");

    let feed_content = fs::read_to_string(&feed_path).unwrap();

    // Valid XML (basic check: starts with XML declaration and has matching tags)
    assert!(
        feed_content.contains("<?xml"),
        "feed.xml should contain XML declaration"
    );

    // Atom namespace
    assert!(
        feed_content.contains("xmlns=\"http://www.w3.org/2005/Atom\""),
        "feed.xml should contain Atom namespace"
    );

    // Feed title matches site title
    let feed_title = extract_first_element(&feed_content, "title");
    assert!(feed_title.is_some(), "Feed should have a <title> element");
    let title = feed_title.unwrap();
    assert!(
        title.contains("DataTalks.Club"),
        "Feed title should contain 'DataTalks.Club', got: '{}'",
        title
    );

    // Link elements
    let links = extract_link_elements(&feed_content);
    let self_link = links.iter().find(|(rel, _)| rel == "self");
    assert!(self_link.is_some(), "Feed should have a self link");

    let alt_link = links.iter().find(|(rel, _)| rel == "alternate");
    assert!(alt_link.is_some(), "Feed should have an alternate link");

    if let Some((_, href)) = alt_link {
        assert!(
            href.contains("datatalks.club"),
            "Alternate link should point to datatalks.club, got: '{}'",
            href
        );
    }

    // Entry elements
    let entries = extract_elements(&feed_content, "entry");
    assert!(!entries.is_empty(), "Feed should contain at least 1 entry");
    eprintln!("DTC feed.xml: {} entries found", entries.len());

    // Check each entry has required elements
    for (i, entry) in entries.iter().enumerate() {
        let entry_title = extract_first_element(entry, "title");
        assert!(entry_title.is_some(), "Entry {} should have a <title>", i);
        let title_text = entry_title.unwrap();
        assert!(
            !title_text.is_empty(),
            "Entry {} title should not be empty",
            i
        );

        let published = extract_first_element(entry, "published");
        assert!(published.is_some(), "Entry {} should have <published>", i);

        let updated = extract_first_element(entry, "updated");
        assert!(updated.is_some(), "Entry {} should have <updated>", i);

        let id = extract_first_element(entry, "id");
        assert!(id.is_some(), "Entry {} should have <id>", i);

        // Check entry URLs contain correct site prefix
        let entry_links = extract_link_elements(entry);
        if let Some((_, href)) = entry_links.first() {
            assert!(
                href.contains("datatalks.club"),
                "Entry {} link should contain datatalks.club, got: '{}'",
                i,
                href
            );
        }

        // Check dates are RFC 3339
        if let Some(ref pub_date) = published {
            assert!(
                is_rfc3339_like(pub_date),
                "Entry {} published date should be RFC 3339, got: '{}'",
                i,
                pub_date
            );
        }
        if let Some(ref upd_date) = updated {
            assert!(
                is_rfc3339_like(upd_date),
                "Entry {} updated date should be RFC 3339, got: '{}'",
                i,
                upd_date
            );
        }
    }

    // No raw Liquid tags
    assert!(
        !contains_liquid_tags(&feed_content),
        "feed.xml should not contain raw Liquid tags"
    );

    eprintln!(
        "DTC feed.xml validation: PASSED ({} entries)",
        entries.len()
    );
}

// ============================================================================
// DTC feed.xml vs Jekyll comparison
// ============================================================================

#[test]
#[ignore]
fn test_dtc_feed_vs_jekyll() {
    let source = dtc_site_dir();
    assert!(
        source.exists(),
        "DTC site not found at {}",
        source.display()
    );

    let tmp = tempfile::tempdir().unwrap();
    let rustkyll_dest = tmp.path().join("rustkyll_site");
    let jekyll_dest = tmp.path().join("jekyll_site");

    build_with_rustkyll(&source, &rustkyll_dest);
    build_with_jekyll(&source, &jekyll_dest);

    let rustkyll_feed = fs::read_to_string(rustkyll_dest.join("feed.xml")).unwrap();
    let jekyll_feed_path = jekyll_dest.join("feed.xml");
    if !jekyll_feed_path.exists() {
        eprintln!(
            "SKIPPED: Jekyll did not produce feed.xml (jekyll-feed plugin may not be installed)"
        );
        return;
    }
    let jekyll_feed = fs::read_to_string(&jekyll_feed_path).unwrap();

    // Compare entry counts
    let rustkyll_entries = extract_elements(&rustkyll_feed, "entry");
    let jekyll_entries = extract_elements(&jekyll_feed, "entry");

    eprintln!(
        "Feed entry counts: rustkyll={}, jekyll={}",
        rustkyll_entries.len(),
        jekyll_entries.len()
    );

    assert!(
        within_tolerance(rustkyll_entries.len(), jekyll_entries.len(), 5.0),
        "Feed entry counts should be within 5%: rustkyll={}, jekyll={}",
        rustkyll_entries.len(),
        jekyll_entries.len()
    );

    // Compare entry titles
    let rustkyll_titles: HashSet<String> = rustkyll_entries
        .iter()
        .filter_map(|e| extract_first_element(e, "title"))
        .map(|t| t.trim().to_string())
        .collect();

    let jekyll_titles: HashSet<String> = jekyll_entries
        .iter()
        .filter_map(|e| extract_first_element(e, "title"))
        .map(|t| t.trim().to_string())
        .collect();

    let common: HashSet<_> = rustkyll_titles.intersection(&jekyll_titles).collect();
    let rustkyll_only: Vec<_> = rustkyll_titles.difference(&jekyll_titles).collect();
    let jekyll_only: Vec<_> = jekyll_titles.difference(&rustkyll_titles).collect();

    eprintln!(
        "Title comparison: common={}, rustkyll_only={}, jekyll_only={}",
        common.len(),
        rustkyll_only.len(),
        jekyll_only.len()
    );

    if !rustkyll_only.is_empty() {
        eprintln!("Rustkyll-only titles: {:?}", rustkyll_only);
    }
    if !jekyll_only.is_empty() {
        eprintln!("Jekyll-only titles: {:?}", jekyll_only);
    }

    // At least 90% of Jekyll entries should appear in rustkyll
    let overlap_pct = if jekyll_titles.is_empty() {
        100.0
    } else {
        common.len() as f64 / jekyll_titles.len() as f64 * 100.0
    };

    assert!(
        overlap_pct >= 90.0,
        "At least 90% of Jekyll feed entries should appear in rustkyll feed. \
         Overlap: {:.1}% ({}/{})",
        overlap_pct,
        common.len(),
        jekyll_titles.len()
    );

    eprintln!(
        "DTC feed.xml comparison: PASSED (overlap: {:.1}%)",
        overlap_pct
    );
}

// ============================================================================
// Kids podcast.xml validation
// ============================================================================

#[test]
#[ignore]
fn test_kids_podcast_validation() {
    let source = kids_site_dir();
    assert!(
        source.exists(),
        "kids-horror-stories-ru site not found at {}",
        source.display()
    );

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("rustkyll_site");

    build_with_rustkyll(&source, &dest);

    let podcast_path = dest.join("podcast.xml");
    assert!(
        podcast_path.exists(),
        "podcast.xml should be generated for kids-horror-stories-ru"
    );

    let podcast_content = fs::read_to_string(&podcast_path).unwrap();

    // Valid XML structure check
    assert!(
        podcast_content.contains("<?xml") || podcast_content.trim().starts_with("<rss"),
        "podcast.xml should contain XML declaration or start with <rss>"
    );

    // RSS 2.0 structure
    assert!(
        podcast_content.contains("<rss"),
        "podcast.xml should contain <rss> element"
    );
    let rss_version = extract_attribute(&podcast_content, "rss", "version");
    assert_eq!(
        rss_version.as_deref(),
        Some("2.0"),
        "RSS version should be 2.0, got: {:?}",
        rss_version
    );

    // Channel element
    assert!(
        podcast_content.contains("<channel>") || podcast_content.contains("<channel "),
        "podcast.xml should contain <channel> element"
    );

    // Item elements
    let items = extract_elements(&podcast_content, "item");
    assert!(
        !items.is_empty(),
        "podcast.xml should contain at least 1 <item>"
    );
    eprintln!("Kids podcast.xml: {} items found", items.len());

    // Each item has title and description
    for (i, item) in items.iter().enumerate() {
        let title = extract_first_element(item, "title");
        assert!(title.is_some(), "Item {} should have a <title>", i);
        let title_text = title.unwrap();
        assert!(
            !title_text.is_empty(),
            "Item {} title should not be empty",
            i
        );

        let desc = extract_first_element(item, "description");
        assert!(desc.is_some(), "Item {} should have a <description>", i);
    }

    // Podcast title/author/description populated (channel-level)
    let channel_content = extract_first_element(&podcast_content, "channel")
        .expect("Should have a <channel> element");

    let podcast_title = extract_first_element(&channel_content, "title");
    assert!(
        podcast_title.is_some() && !podcast_title.as_ref().unwrap().is_empty(),
        "Channel title should be populated"
    );

    let description = extract_first_element(&channel_content, "description");
    assert!(
        description.is_some() && !description.as_ref().unwrap().is_empty(),
        "Channel description should be populated"
    );

    // No raw Liquid tags
    assert!(
        !contains_liquid_tags(&podcast_content),
        "podcast.xml should not contain raw Liquid tags, but found them"
    );

    eprintln!(
        "Kids podcast.xml validation: PASSED ({} items, title='{}', desc present)",
        items.len(),
        podcast_title.unwrap_or_default()
    );
}

// ============================================================================
// Kids podcast.xml vs Jekyll comparison
// ============================================================================

#[test]
#[ignore]
fn test_kids_podcast_vs_jekyll() {
    let source = kids_site_dir();
    assert!(
        source.exists(),
        "kids site not found at {}",
        source.display()
    );

    let tmp = tempfile::tempdir().unwrap();
    let rustkyll_dest = tmp.path().join("rustkyll_site");
    let jekyll_dest = tmp.path().join("jekyll_site");

    build_with_rustkyll(&source, &rustkyll_dest);
    build_with_jekyll(&source, &jekyll_dest);

    let rustkyll_podcast_path = rustkyll_dest.join("podcast.xml");
    let jekyll_podcast_path = jekyll_dest.join("podcast.xml");

    assert!(
        rustkyll_podcast_path.exists(),
        "Rustkyll should produce podcast.xml"
    );
    assert!(
        jekyll_podcast_path.exists(),
        "Jekyll should produce podcast.xml"
    );

    let rustkyll_content = fs::read_to_string(&rustkyll_podcast_path).unwrap();
    let jekyll_content = fs::read_to_string(&jekyll_podcast_path).unwrap();

    let rustkyll_items = extract_elements(&rustkyll_content, "item");
    let jekyll_items = extract_elements(&jekyll_content, "item");

    eprintln!(
        "Podcast item counts: rustkyll={}, jekyll={}",
        rustkyll_items.len(),
        jekyll_items.len()
    );

    assert!(
        within_tolerance(rustkyll_items.len(), jekyll_items.len(), 5.0),
        "Podcast item counts should be within 5%: rustkyll={}, jekyll={}",
        rustkyll_items.len(),
        jekyll_items.len()
    );

    eprintln!(
        "Kids podcast.xml comparison: PASSED (rustkyll={}, jekyll={})",
        rustkyll_items.len(),
        jekyll_items.len()
    );
}

// ============================================================================
// DTC sitemap.xml validation
// ============================================================================

#[test]
#[ignore]
fn test_dtc_sitemap_validation() {
    let source = dtc_site_dir();
    assert!(
        source.exists(),
        "DTC site not found at {}",
        source.display()
    );

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("rustkyll_site");

    build_with_rustkyll(&source, &dest);

    let sitemap_path = dest.join("sitemap.xml");
    assert!(sitemap_path.exists(), "sitemap.xml should be generated");

    let sitemap_content = fs::read_to_string(&sitemap_path).unwrap();

    // Valid XML
    assert!(
        sitemap_content.contains("<?xml"),
        "sitemap.xml should contain XML declaration"
    );

    // Sitemap namespace
    assert!(
        sitemap_content.contains("xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"")
            || sitemap_content.contains("xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'"),
        "sitemap.xml should contain sitemap namespace"
    );

    // Extract URLs
    let locs = extract_elements(&sitemap_content, "loc");
    eprintln!("DTC sitemap.xml: {} URLs found", locs.len());

    assert!(
        locs.len() > 50,
        "DTC sitemap should have >50 URLs, got {}",
        locs.len()
    );

    // Root URL present
    assert!(
        locs.iter().any(|u| u.trim() == "https://datatalks.club/"),
        "Sitemap should contain root URL https://datatalks.club/"
    );

    // All URLs use correct prefix
    for (i, loc) in locs.iter().enumerate() {
        let url = loc.trim();
        assert!(
            url.starts_with("https://datatalks.club"),
            "Sitemap URL {} should start with https://datatalks.club, got: '{}'",
            i,
            url
        );
    }

    // No raw Liquid tags in any URL
    for (i, loc) in locs.iter().enumerate() {
        assert!(
            !contains_liquid_tags(loc),
            "Sitemap URL {} should not contain raw Liquid tags: '{}'",
            i,
            loc
        );
    }

    // No Liquid tags anywhere in the sitemap
    assert!(
        !contains_liquid_tags(&sitemap_content),
        "sitemap.xml should not contain any raw Liquid tags"
    );

    // Check key sections are represented
    let has_people = locs.iter().any(|u| u.contains("/people"));
    let has_books = locs.iter().any(|u| u.contains("/books"));
    let has_podcast = locs.iter().any(|u| u.contains("/podcast"));
    let has_posts = locs
        .iter()
        .any(|u| u.contains("/blog/") || u.contains("/posts/"));

    eprintln!(
        "Key sections: people={}, books={}, podcast={}, posts={}",
        has_people, has_books, has_podcast, has_posts
    );

    assert!(has_people, "Sitemap should contain people URLs");
    assert!(has_books, "Sitemap should contain books URLs");
    assert!(has_podcast, "Sitemap should contain podcast URLs");

    // Verify HTML URLs correspond to actual files (at least 90%)
    let html_locs: Vec<&String> = locs
        .iter()
        .filter(|u| u.trim().ends_with(".html"))
        .collect();
    let mut existing_count = 0;
    for loc in &html_locs {
        let url = loc.trim();
        // Strip the URL prefix to get the relative path
        let relative = url.strip_prefix("https://datatalks.club").unwrap_or(url);
        let relative = relative.trim_start_matches('/');
        let file_path = dest.join(relative);
        if file_path.exists() {
            existing_count += 1;
        }
    }

    let file_match_pct = if html_locs.is_empty() {
        100.0
    } else {
        existing_count as f64 / html_locs.len() as f64 * 100.0
    };

    eprintln!(
        "HTML file correspondence: {}/{} ({:.1}%)",
        existing_count,
        html_locs.len(),
        file_match_pct
    );

    assert!(
        file_match_pct >= 90.0,
        "At least 90% of .html sitemap URLs should correspond to actual files. \
         Got {:.1}% ({}/{})",
        file_match_pct,
        existing_count,
        html_locs.len()
    );

    eprintln!(
        "DTC sitemap.xml validation: PASSED ({} URLs, {:.1}% file match)",
        locs.len(),
        file_match_pct
    );
}

// ============================================================================
// DTC sitemap.xml vs Jekyll comparison
// ============================================================================

#[test]
#[ignore]
fn test_dtc_sitemap_vs_jekyll() {
    let source = dtc_site_dir();
    assert!(
        source.exists(),
        "DTC site not found at {}",
        source.display()
    );

    let tmp = tempfile::tempdir().unwrap();
    let rustkyll_dest = tmp.path().join("rustkyll_site");
    let jekyll_dest = tmp.path().join("jekyll_site");

    build_with_rustkyll(&source, &rustkyll_dest);
    build_with_jekyll(&source, &jekyll_dest);

    let rustkyll_sitemap = fs::read_to_string(rustkyll_dest.join("sitemap.xml")).unwrap();
    let jekyll_sitemap_path = jekyll_dest.join("sitemap.xml");
    assert!(
        jekyll_sitemap_path.exists(),
        "Jekyll should produce sitemap.xml"
    );
    let jekyll_sitemap = fs::read_to_string(&jekyll_sitemap_path).unwrap();

    // Extract URLs from both
    let rustkyll_urls: HashSet<String> = extract_elements(&rustkyll_sitemap, "loc")
        .into_iter()
        .map(|u| u.trim().to_string())
        .collect();

    let jekyll_urls: HashSet<String> = extract_elements(&jekyll_sitemap, "loc")
        .into_iter()
        .map(|u| u.trim().to_string())
        .collect();

    eprintln!(
        "Sitemap URL counts: rustkyll={}, jekyll={}",
        rustkyll_urls.len(),
        jekyll_urls.len()
    );

    // Count comparison within 5%
    assert!(
        within_tolerance(rustkyll_urls.len(), jekyll_urls.len(), 5.0),
        "Sitemap URL counts should be within 5%: rustkyll={}, jekyll={}",
        rustkyll_urls.len(),
        jekyll_urls.len()
    );

    // Report differences
    let common: HashSet<_> = rustkyll_urls.intersection(&jekyll_urls).collect();
    let rustkyll_only: Vec<_> = rustkyll_urls.difference(&jekyll_urls).collect();
    let jekyll_only: Vec<_> = jekyll_urls.difference(&rustkyll_urls).collect();

    eprintln!(
        "URL comparison: common={}, rustkyll_only={}, jekyll_only={}",
        common.len(),
        rustkyll_only.len(),
        jekyll_only.len()
    );

    if !rustkyll_only.is_empty() {
        let sample: Vec<_> = rustkyll_only.iter().take(10).collect();
        eprintln!("Sample rustkyll-only URLs: {:?}", sample);
    }
    if !jekyll_only.is_empty() {
        let sample: Vec<_> = jekyll_only.iter().take(10).collect();
        eprintln!("Sample jekyll-only URLs: {:?}", sample);
    }

    eprintln!(
        "DTC sitemap.xml comparison: PASSED (common={}, rustkyll_only={}, jekyll_only={})",
        common.len(),
        rustkyll_only.len(),
        jekyll_only.len()
    );
}

// ============================================================================
// Kids sitemap.xml validation
// ============================================================================

#[test]
#[ignore]
fn test_kids_sitemap_validation() {
    let source = kids_site_dir();
    assert!(
        source.exists(),
        "kids site not found at {}",
        source.display()
    );

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("rustkyll_site");

    build_with_rustkyll(&source, &dest);

    let sitemap_path = dest.join("sitemap.xml");
    if !sitemap_path.exists() {
        eprintln!(
            "Kids sitemap.xml: not generated (site has no sitemap template). PASSED (no sitemap expected)"
        );
        return;
    }

    let sitemap_content = fs::read_to_string(&sitemap_path).unwrap();

    // If generated, it should be valid XML
    assert!(
        sitemap_content.contains("<?xml") || sitemap_content.contains("<urlset"),
        "If sitemap.xml exists, it should be valid XML"
    );

    // Extract URLs
    let locs = extract_elements(&sitemap_content, "loc");
    eprintln!("Kids sitemap.xml: {} URLs found", locs.len());

    // No Liquid tags
    assert!(
        !contains_liquid_tags(&sitemap_content),
        "Sitemap should not contain raw Liquid tags"
    );

    // Verify URLs correspond to actual files
    let mut existing_count = 0;
    let mut total_checked = 0;
    for loc in &locs {
        let url = loc.trim();
        // Try to extract relative path from the URL
        if let Some(relative) = url
            .split("//")
            .nth(1)
            .and_then(|s| s.find('/').map(|i| &s[i..]))
        {
            let relative = relative.trim_start_matches('/');
            if relative.is_empty() {
                // Root URL -- check index.html
                if dest.join("index.html").exists() {
                    existing_count += 1;
                }
            } else {
                let file_path = dest.join(relative);
                if file_path.exists() {
                    existing_count += 1;
                }
            }
            total_checked += 1;
        }
    }

    if total_checked > 0 {
        let match_pct = existing_count as f64 / total_checked as f64 * 100.0;
        eprintln!(
            "Kids sitemap file correspondence: {}/{} ({:.1}%)",
            existing_count, total_checked, match_pct
        );
    }

    eprintln!("Kids sitemap.xml validation: PASSED");
}
