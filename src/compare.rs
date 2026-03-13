//! Output comparison module for diffing two site builds.
//!
//! Compares the output of rustkyll against Jekyll (or any two directory trees)
//! to identify discrepancies. Supports HTML normalization to ignore insignificant
//! differences like whitespace and attribute ordering.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// The result of comparing two directories of site output.
#[derive(Debug, Clone)]
pub struct ComparisonReport {
    /// Files that match after normalization.
    pub matching: Vec<PathBuf>,
    /// Files that differ, with details about the differences.
    pub differing: Vec<FileDiff>,
    /// Files present in the expected directory but missing from the actual directory.
    pub missing_from_actual: Vec<PathBuf>,
    /// Files present in the actual directory but not in the expected directory.
    pub extra_in_actual: Vec<PathBuf>,
}

impl ComparisonReport {
    /// Returns true if there are no differences at all.
    pub fn is_clean(&self) -> bool {
        self.differing.is_empty()
            && self.missing_from_actual.is_empty()
            && self.extra_in_actual.is_empty()
    }

    /// Total number of files examined.
    pub fn total_files(&self) -> usize {
        self.matching.len()
            + self.differing.len()
            + self.missing_from_actual.len()
            + self.extra_in_actual.len()
    }
}

impl fmt::Display for ComparisonReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Comparison Report")?;
        writeln!(f, "=================")?;
        writeln!(f, "Matching files:      {}", self.matching.len())?;
        writeln!(f, "Differing files:     {}", self.differing.len())?;
        writeln!(f, "Missing from actual: {}", self.missing_from_actual.len())?;
        writeln!(f, "Extra in actual:     {}", self.extra_in_actual.len())?;
        writeln!(f)?;

        if !self.missing_from_actual.is_empty() {
            writeln!(f, "Missing from actual:")?;
            for path in &self.missing_from_actual {
                writeln!(f, "  - {}", path.display())?;
            }
            writeln!(f)?;
        }

        if !self.extra_in_actual.is_empty() {
            writeln!(f, "Extra in actual:")?;
            for path in &self.extra_in_actual {
                writeln!(f, "  + {}", path.display())?;
            }
            writeln!(f)?;
        }

        if !self.differing.is_empty() {
            writeln!(f, "Differing files:")?;
            for diff in &self.differing {
                writeln!(f, "  ~ {}", diff.path.display())?;
                writeln!(f, "    Category: {:?}", diff.category)?;
                if let Some(ref detail) = diff.detail {
                    for line in detail.lines().take(5) {
                        writeln!(f, "    {}", line)?;
                    }
                    let total_lines = detail.lines().count();
                    if total_lines > 5 {
                        writeln!(f, "    ... ({} more lines)", total_lines - 5)?;
                    }
                }
            }
        }

        Ok(())
    }
}

/// A single file that differs between expected and actual output.
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// Relative path of the file.
    pub path: PathBuf,
    /// Category of difference.
    pub category: DiffCategory,
    /// Human-readable detail of the difference (a unified-diff-like summary).
    pub detail: Option<String>,
}

/// Classification of a difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffCategory {
    /// Content text is different (missing/wrong text, broken links, wrong URLs).
    Content,
    /// Structural difference (wrong tags, missing elements, wrong nesting).
    Structural,
    /// Only whitespace differs.
    WhitespaceOnly,
    /// Binary files differ.
    Binary,
}

/// Errors that can occur during comparison.
#[derive(Debug, thiserror::Error)]
pub enum CompareError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("directory does not exist: {0}")]
    DirNotFound(PathBuf),
}

/// Compare two directories file-by-file.
///
/// `expected` is the reference output (e.g., Jekyll's `_site/`).
/// `actual` is the output being tested (e.g., rustkyll's `_site/`).
pub fn compare_directories(
    expected: &Path,
    actual: &Path,
) -> Result<ComparisonReport, CompareError> {
    if !expected.is_dir() {
        return Err(CompareError::DirNotFound(expected.to_path_buf()));
    }
    if !actual.is_dir() {
        return Err(CompareError::DirNotFound(actual.to_path_buf()));
    }

    let expected_files = collect_files(expected)?;
    let actual_files = collect_files(actual)?;

    let mut matching = Vec::new();
    let mut differing = Vec::new();
    let mut missing_from_actual = Vec::new();
    let mut extra_in_actual = Vec::new();

    // Check all expected files
    for rel_path in expected_files.keys() {
        if let Some(actual_path) = actual_files.get(rel_path) {
            let expected_path = &expected_files[rel_path];
            match compare_files(expected_path, actual_path, rel_path)? {
                None => matching.push(rel_path.clone()),
                Some(diff) => differing.push(diff),
            }
        } else {
            missing_from_actual.push(rel_path.clone());
        }
    }

    // Check for extra files in actual
    for rel_path in actual_files.keys() {
        if !expected_files.contains_key(rel_path) {
            extra_in_actual.push(rel_path.clone());
        }
    }

    // Sort for deterministic output
    matching.sort();
    differing.sort_by(|a, b| a.path.cmp(&b.path));
    missing_from_actual.sort();
    extra_in_actual.sort();

    Ok(ComparisonReport {
        matching,
        differing,
        missing_from_actual,
        extra_in_actual,
    })
}

/// Collect all files under a directory into a map of relative_path -> absolute_path.
fn collect_files(dir: &Path) -> Result<BTreeMap<PathBuf, PathBuf>, CompareError> {
    let mut files = BTreeMap::new();
    collect_files_recursive(dir, dir, &mut files)?;
    Ok(files)
}

fn collect_files_recursive(
    base: &Path,
    current: &Path,
    files: &mut BTreeMap<PathBuf, PathBuf>,
) -> Result<(), CompareError> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(base, &path, files)?;
        } else {
            let rel = path.strip_prefix(base).unwrap_or(&path).to_path_buf();
            files.insert(rel, path);
        }
    }
    Ok(())
}

/// Compare two individual files. Returns `None` if they match, `Some(FileDiff)` if they differ.
fn compare_files(
    expected_path: &Path,
    actual_path: &Path,
    rel_path: &Path,
) -> Result<Option<FileDiff>, CompareError> {
    let is_html = matches!(
        rel_path.extension().and_then(|e| e.to_str()),
        Some("html" | "htm")
    );
    let is_xml = matches!(rel_path.extension().and_then(|e| e.to_str()), Some("xml"));
    let is_text = is_html
        || is_xml
        || matches!(
            rel_path.extension().and_then(|e| e.to_str()),
            Some("css" | "js" | "json" | "txt" | "svg" | "md")
        );

    if !is_text {
        // Binary comparison
        let expected_bytes = std::fs::read(expected_path)?;
        let actual_bytes = std::fs::read(actual_path)?;
        if expected_bytes == actual_bytes {
            return Ok(None);
        }
        return Ok(Some(FileDiff {
            path: rel_path.to_path_buf(),
            category: DiffCategory::Binary,
            detail: Some(format!(
                "Binary files differ (expected {} bytes, actual {} bytes)",
                expected_bytes.len(),
                actual_bytes.len()
            )),
        }));
    }

    let expected_content = std::fs::read_to_string(expected_path)?;
    let actual_content = std::fs::read_to_string(actual_path)?;

    // For HTML files, normalize before comparing
    if is_html {
        let norm_expected = normalize_html(&expected_content);
        let norm_actual = normalize_html(&actual_content);

        if norm_expected == norm_actual {
            // Check if only whitespace differs in the original
            if expected_content != actual_content {
                // They match after normalization but differ in raw form -- whitespace only
                return Ok(None); // We consider this a match
            }
            return Ok(None);
        }

        // Find the nature of the difference
        let category = classify_html_diff(&norm_expected, &norm_actual);
        let detail = generate_text_diff(&norm_expected, &norm_actual);

        return Ok(Some(FileDiff {
            path: rel_path.to_path_buf(),
            category,
            detail: Some(detail),
        }));
    }

    // Plain text / XML comparison -- normalize whitespace lightly
    if expected_content == actual_content {
        return Ok(None);
    }

    let norm_expected = normalize_whitespace(&expected_content);
    let norm_actual = normalize_whitespace(&actual_content);

    if norm_expected == norm_actual {
        return Ok(None); // Whitespace-only difference
    }

    let detail = generate_text_diff(&expected_content, &actual_content);

    Ok(Some(FileDiff {
        path: rel_path.to_path_buf(),
        category: DiffCategory::Content,
        detail: Some(detail),
    }))
}

// ──────────────────────────────────────────────
// HTML normalization
// ──────────────────────────────────────────────

/// Normalize HTML for comparison purposes.
///
/// This function:
/// - Collapses whitespace runs into single spaces
/// - Removes leading/trailing whitespace per line
/// - Sorts HTML attributes alphabetically within each tag
/// - Normalizes self-closing tag style
pub fn normalize_html(html: &str) -> String {
    let collapsed = collapse_whitespace(html);
    let trimmed = trim_between_tags(&collapsed);
    sort_html_attributes(&trimmed)
}

/// Collapse runs of whitespace into single spaces and trim.
fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_whitespace = false;
    let mut in_tag = false;
    let mut in_pre = false;

    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Track <pre> blocks -- don't normalize whitespace inside them
        if !in_pre && i + 4 < len && c == '<' {
            let lookahead: String = chars[i..std::cmp::min(i + 5, len)].iter().collect();
            if lookahead.to_lowercase().starts_with("<pre") {
                in_pre = true;
            }
        }
        if in_pre && i + 5 < len && c == '<' {
            let lookahead: String = chars[i..std::cmp::min(i + 6, len)].iter().collect();
            if lookahead.to_lowercase().starts_with("</pre") {
                in_pre = false;
            }
        }

        if in_pre {
            result.push(c);
            i += 1;
            continue;
        }

        if c == '<' {
            in_tag = true;
        }
        if c == '>' {
            in_tag = false;
        }

        if c.is_whitespace() && !in_tag {
            if !in_whitespace {
                result.push(' ');
                in_whitespace = true;
            }
        } else {
            in_whitespace = false;
            result.push(c);
        }
        i += 1;
    }

    result.trim().to_string()
}

/// Remove whitespace between `>` and text, and between text and `<`.
/// This normalizes `> text <` to `>text<`.
fn trim_between_tags(s: &str) -> String {
    // Remove spaces right after '>' (outside tags)
    // Remove spaces right before '<' (outside tags)
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let c = chars[i];
        if c == ' ' {
            // Skip space if it's right after '>'
            if i > 0 && chars[i - 1] == '>' {
                continue;
            }
            // Skip space if it's right before '<'
            if i + 1 < len && chars[i + 1] == '<' {
                continue;
            }
        }
        result.push(c);
    }

    result
}

/// Sort HTML attributes within tags alphabetically.
///
/// Uses a simple state machine to find tag boundaries and reorder attributes.
fn sort_html_attributes(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c == '<' {
            // Read the entire tag
            let tag = read_tag(&mut chars);
            let sorted = sort_single_tag_attributes(&tag);
            result.push_str(&sorted);
        } else {
            result.push(c);
            chars.next();
        }
    }

    result
}

/// Read from '<' to '>' inclusive.
fn read_tag(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut tag = String::new();
    while let Some(&c) = chars.peek() {
        tag.push(c);
        chars.next();
        if c == '>' {
            break;
        }
    }
    tag
}

/// Sort attributes within a single HTML tag string.
///
/// Handles: `<tag attr1="val1" attr2="val2">` and self-closing variants.
fn sort_single_tag_attributes(tag: &str) -> String {
    // Don't sort closing tags, comments, doctypes, or processing instructions
    if tag.starts_with("</")
        || tag.starts_with("<!--")
        || tag.starts_with("<!")
        || tag.starts_with("<?")
    {
        return tag.to_string();
    }

    // Parse tag name and attributes
    let inner = if tag.ends_with("/>") {
        &tag[1..tag.len() - 2]
    } else if tag.ends_with('>') {
        &tag[1..tag.len() - 1]
    } else {
        return tag.to_string();
    };

    let inner = inner.trim();

    // Split tag name from attributes
    let (tag_name, attrs_str) = match inner.find(|c: char| c.is_whitespace()) {
        Some(pos) => (&inner[..pos], inner[pos..].trim()),
        None => {
            // No attributes
            return tag.to_string();
        }
    };

    if attrs_str.is_empty() {
        return tag.to_string();
    }

    // Parse attributes
    let attrs = parse_attributes(attrs_str);

    // Sort attributes by name
    let mut sorted_attrs: Vec<(String, Option<String>)> = attrs;
    sorted_attrs.sort_by(|a, b| a.0.cmp(&b.0));

    // Reconstruct tag
    let mut result = String::from("<");
    result.push_str(tag_name);
    for (name, value) in &sorted_attrs {
        result.push(' ');
        result.push_str(name);
        if let Some(val) = value {
            result.push('=');
            result.push('"');
            result.push_str(val);
            result.push('"');
        }
    }
    if tag.ends_with("/>") {
        result.push_str(" />");
    } else {
        result.push('>');
    }

    result
}

/// Parse HTML attributes from a string like `class="foo" id="bar" disabled`.
/// Returns a vec of (name, optional_value) pairs.
fn parse_attributes(s: &str) -> Vec<(String, Option<String>)> {
    let mut attrs = Vec::new();
    let mut chars = s.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        if chars.peek().is_none() {
            break;
        }

        // Read attribute name
        let mut name = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() || c == '/' || c == '>' {
                break;
            }
            name.push(c);
            chars.next();
        }

        if name.is_empty() {
            // Skip unexpected characters
            chars.next();
            continue;
        }

        // Check for '='
        if chars.peek() == Some(&'=') {
            chars.next(); // consume '='

            // Read value
            let value = if chars.peek() == Some(&'"') {
                chars.next(); // consume opening quote
                let mut val = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '"' {
                        chars.next(); // consume closing quote
                        break;
                    }
                    val.push(c);
                    chars.next();
                }
                val
            } else if chars.peek() == Some(&'\'') {
                chars.next(); // consume opening quote
                let mut val = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '\'' {
                        chars.next();
                        break;
                    }
                    val.push(c);
                    chars.next();
                }
                val
            } else {
                // Unquoted value
                let mut val = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || c == '>' || c == '/' {
                        break;
                    }
                    val.push(c);
                    chars.next();
                }
                val
            };

            attrs.push((name, Some(value)));
        } else {
            // Boolean attribute (no value)
            attrs.push((name, None));
        }
    }

    attrs
}

/// Normalize whitespace in non-HTML text: collapse runs of whitespace, trim lines.
fn normalize_whitespace(s: &str) -> String {
    s.lines()
        .map(|line| {
            let mut result = String::new();
            let mut in_ws = false;
            for c in line.chars() {
                if c.is_whitespace() {
                    if !in_ws {
                        result.push(' ');
                        in_ws = true;
                    }
                } else {
                    in_ws = false;
                    result.push(c);
                }
            }
            result.trim().to_string()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// ──────────────────────────────────────────────
// Diff generation
// ──────────────────────────────────────────────

/// Generate a simple line-by-line diff between two strings.
/// Returns a human-readable summary of the differences.
pub fn generate_text_diff(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();

    let mut diff_lines = Vec::new();
    let max_len = std::cmp::max(expected_lines.len(), actual_lines.len());
    let mut diff_count = 0;
    let max_diffs = 20; // Limit output size

    for i in 0..max_len {
        let exp = expected_lines.get(i).copied();
        let act = actual_lines.get(i).copied();

        match (exp, act) {
            (Some(e), Some(a)) if e != a => {
                diff_count += 1;
                if diff_count <= max_diffs {
                    diff_lines.push(format!("Line {}:", i + 1));
                    diff_lines.push(format!("  - {}", e));
                    diff_lines.push(format!("  + {}", a));
                }
            }
            (Some(e), None) => {
                diff_count += 1;
                if diff_count <= max_diffs {
                    diff_lines.push(format!("Line {}: missing in actual", i + 1));
                    diff_lines.push(format!("  - {}", e));
                }
            }
            (None, Some(a)) => {
                diff_count += 1;
                if diff_count <= max_diffs {
                    diff_lines.push(format!("Line {}: extra in actual", i + 1));
                    diff_lines.push(format!("  + {}", a));
                }
            }
            _ => {}
        }
    }

    if diff_count > max_diffs {
        diff_lines.push(format!(
            "... and {} more differences",
            diff_count - max_diffs
        ));
    }

    if diff_lines.is_empty() {
        "No line-level differences found (possible encoding difference)".to_string()
    } else {
        diff_lines.join("\n")
    }
}

/// Classify an HTML diff based on the nature of the normalized content difference.
fn classify_html_diff(norm_expected: &str, norm_actual: &str) -> DiffCategory {
    // Strip all tags and compare text content
    let text_expected = strip_tags(norm_expected);
    let text_actual = strip_tags(norm_actual);

    if text_expected != text_actual {
        return DiffCategory::Content;
    }

    // Text is the same but HTML differs -- structural difference
    DiffCategory::Structural
}

/// Strip all HTML tags, leaving only text content.
fn strip_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ─── HTML normalization tests ───

    #[test]
    fn test_collapse_whitespace() {
        assert_eq!(collapse_whitespace("  hello   world  "), "hello world");
    }

    #[test]
    fn test_collapse_whitespace_preserves_pre() {
        let html = "<p>text</p><pre>  indented\n  code  </pre><p>more</p>";
        let result = collapse_whitespace(html);
        assert!(result.contains("  indented\n  code  "));
    }

    #[test]
    fn test_sort_attributes_simple() {
        let html = r#"<div class="foo" id="bar">"#;
        let sorted = sort_html_attributes(html);
        assert_eq!(sorted, r#"<div class="foo" id="bar">"#);

        let html = r#"<div id="bar" class="foo">"#;
        let sorted = sort_html_attributes(html);
        assert_eq!(sorted, r#"<div class="foo" id="bar">"#);
    }

    #[test]
    fn test_sort_attributes_self_closing() {
        let html = r#"<img width="100" src="a.png" />"#;
        let sorted = sort_html_attributes(html);
        assert_eq!(sorted, r#"<img src="a.png" width="100" />"#);
    }

    #[test]
    fn test_sort_attributes_boolean() {
        let html = r#"<input type="text" disabled>"#;
        let sorted = sort_html_attributes(html);
        assert_eq!(sorted, r#"<input disabled type="text">"#);
    }

    #[test]
    fn test_sort_attributes_closing_tag_unchanged() {
        let html = "</div>";
        let sorted = sort_html_attributes(html);
        assert_eq!(sorted, "</div>");
    }

    #[test]
    fn test_sort_attributes_comment_unchanged() {
        let html = "<!-- a comment -->";
        let sorted = sort_html_attributes(html);
        assert_eq!(sorted, "<!-- a comment -->");
    }

    #[test]
    fn test_normalize_html_full() {
        let html1 = r#"<div id="x"   class="y">
            Hello   World
        </div>"#;
        let html2 = r#"<div class="y" id="x">Hello World</div>"#;

        assert_eq!(normalize_html(html1), normalize_html(html2));
    }

    #[test]
    fn test_normalize_whitespace_text() {
        let text = "  hello   world  \n\n  foo  bar  ";
        assert_eq!(normalize_whitespace(text), "hello world\nfoo bar");
    }

    // ─── Diff classification tests ───

    #[test]
    fn test_classify_content_diff() {
        let expected = "<p>Hello World</p>";
        let actual = "<p>Hello Mars</p>";
        assert_eq!(classify_html_diff(expected, actual), DiffCategory::Content);
    }

    #[test]
    fn test_classify_structural_diff() {
        let expected = "<div><p>Hello</p></div>";
        let actual = "<span><p>Hello</p></span>";
        assert_eq!(
            classify_html_diff(expected, actual),
            DiffCategory::Structural
        );
    }

    // ─── strip_tags tests ───

    #[test]
    fn test_strip_tags() {
        assert_eq!(strip_tags("<p>Hello <b>World</b></p>"), "Hello World");
        assert_eq!(strip_tags("no tags"), "no tags");
        assert_eq!(strip_tags("<br/>"), "");
    }

    // ─── Text diff tests ───

    #[test]
    fn test_generate_text_diff_identical() {
        let diff = generate_text_diff("hello\nworld", "hello\nworld");
        assert!(diff.contains("No line-level differences"));
    }

    #[test]
    fn test_generate_text_diff_different() {
        let diff = generate_text_diff("hello\nworld", "hello\nearth");
        assert!(diff.contains("Line 2:"));
        assert!(diff.contains("- world"));
        assert!(diff.contains("+ earth"));
    }

    #[test]
    fn test_generate_text_diff_extra_lines() {
        let diff = generate_text_diff("a\nb", "a\nb\nc");
        assert!(diff.contains("extra in actual"));
    }

    #[test]
    fn test_generate_text_diff_missing_lines() {
        let diff = generate_text_diff("a\nb\nc", "a\nb");
        assert!(diff.contains("missing in actual"));
    }

    // ─── parse_attributes tests ───

    #[test]
    fn test_parse_attributes_basic() {
        let attrs = parse_attributes(r#"class="foo" id="bar""#);
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0], ("class".to_string(), Some("foo".to_string())));
        assert_eq!(attrs[1], ("id".to_string(), Some("bar".to_string())));
    }

    #[test]
    fn test_parse_attributes_boolean() {
        let attrs = parse_attributes("disabled checked");
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0], ("disabled".to_string(), None));
        assert_eq!(attrs[1], ("checked".to_string(), None));
    }

    #[test]
    fn test_parse_attributes_single_quotes() {
        let attrs = parse_attributes("class='foo'");
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0], ("class".to_string(), Some("foo".to_string())));
    }

    // ─── Directory comparison tests ───

    #[test]
    fn test_compare_directories_identical() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().join("expected");
        let actual = dir.path().join("actual");

        fs::create_dir_all(&expected).unwrap();
        fs::create_dir_all(&actual).unwrap();

        fs::write(expected.join("index.html"), "<p>Hello</p>").unwrap();
        fs::write(actual.join("index.html"), "<p>Hello</p>").unwrap();

        let report = compare_directories(&expected, &actual).unwrap();
        assert!(report.is_clean());
        assert_eq!(report.matching.len(), 1);
    }

    #[test]
    fn test_compare_directories_whitespace_only_diff() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().join("expected");
        let actual = dir.path().join("actual");

        fs::create_dir_all(&expected).unwrap();
        fs::create_dir_all(&actual).unwrap();

        fs::write(expected.join("index.html"), "<p>  Hello  </p>").unwrap();
        fs::write(actual.join("index.html"), "<p> Hello </p>").unwrap();

        let report = compare_directories(&expected, &actual).unwrap();
        assert!(report.is_clean());
        assert_eq!(report.matching.len(), 1);
    }

    #[test]
    fn test_compare_directories_attribute_order_diff() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().join("expected");
        let actual = dir.path().join("actual");

        fs::create_dir_all(&expected).unwrap();
        fs::create_dir_all(&actual).unwrap();

        fs::write(
            expected.join("page.html"),
            r#"<div id="x" class="y">Hello</div>"#,
        )
        .unwrap();
        fs::write(
            actual.join("page.html"),
            r#"<div class="y" id="x">Hello</div>"#,
        )
        .unwrap();

        let report = compare_directories(&expected, &actual).unwrap();
        assert!(report.is_clean());
    }

    #[test]
    fn test_compare_directories_content_diff() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().join("expected");
        let actual = dir.path().join("actual");

        fs::create_dir_all(&expected).unwrap();
        fs::create_dir_all(&actual).unwrap();

        fs::write(expected.join("index.html"), "<p>Hello</p>").unwrap();
        fs::write(actual.join("index.html"), "<p>Goodbye</p>").unwrap();

        let report = compare_directories(&expected, &actual).unwrap();
        assert!(!report.is_clean());
        assert_eq!(report.differing.len(), 1);
        assert_eq!(report.differing[0].category, DiffCategory::Content);
    }

    #[test]
    fn test_compare_directories_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().join("expected");
        let actual = dir.path().join("actual");

        fs::create_dir_all(&expected).unwrap();
        fs::create_dir_all(&actual).unwrap();

        fs::write(expected.join("index.html"), "<p>Hello</p>").unwrap();
        fs::write(expected.join("about.html"), "<p>About</p>").unwrap();
        fs::write(actual.join("index.html"), "<p>Hello</p>").unwrap();

        let report = compare_directories(&expected, &actual).unwrap();
        assert!(!report.is_clean());
        assert_eq!(report.missing_from_actual.len(), 1);
        assert_eq!(report.missing_from_actual[0], PathBuf::from("about.html"));
    }

    #[test]
    fn test_compare_directories_extra_file() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().join("expected");
        let actual = dir.path().join("actual");

        fs::create_dir_all(&expected).unwrap();
        fs::create_dir_all(&actual).unwrap();

        fs::write(expected.join("index.html"), "<p>Hello</p>").unwrap();
        fs::write(actual.join("index.html"), "<p>Hello</p>").unwrap();
        fs::write(actual.join("extra.html"), "<p>Extra</p>").unwrap();

        let report = compare_directories(&expected, &actual).unwrap();
        assert!(!report.is_clean());
        assert_eq!(report.extra_in_actual.len(), 1);
    }

    #[test]
    fn test_compare_directories_nested() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().join("expected");
        let actual = dir.path().join("actual");

        fs::create_dir_all(expected.join("sub")).unwrap();
        fs::create_dir_all(actual.join("sub")).unwrap();

        fs::write(expected.join("sub/page.html"), "<p>Nested</p>").unwrap();
        fs::write(actual.join("sub/page.html"), "<p>Nested</p>").unwrap();

        let report = compare_directories(&expected, &actual).unwrap();
        assert!(report.is_clean());
        assert_eq!(report.matching.len(), 1);
    }

    #[test]
    fn test_compare_directories_binary_files() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().join("expected");
        let actual = dir.path().join("actual");

        fs::create_dir_all(&expected).unwrap();
        fs::create_dir_all(&actual).unwrap();

        fs::write(expected.join("image.png"), [0u8, 1, 2, 3]).unwrap();
        fs::write(actual.join("image.png"), [0u8, 1, 2, 3]).unwrap();

        let report = compare_directories(&expected, &actual).unwrap();
        assert!(report.is_clean());

        // Different binary files
        fs::write(actual.join("image.png"), [4u8, 5, 6]).unwrap();
        let report = compare_directories(&expected, &actual).unwrap();
        assert!(!report.is_clean());
        assert_eq!(report.differing[0].category, DiffCategory::Binary);
    }

    #[test]
    fn test_compare_directories_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let result = compare_directories(&dir.path().join("nope"), &dir.path().join("also_nope"));
        assert!(result.is_err());
    }

    #[test]
    fn test_report_display() {
        let report = ComparisonReport {
            matching: vec![PathBuf::from("index.html")],
            differing: vec![FileDiff {
                path: PathBuf::from("about.html"),
                category: DiffCategory::Content,
                detail: Some("Line 1: different".to_string()),
            }],
            missing_from_actual: vec![PathBuf::from("missing.html")],
            extra_in_actual: vec![PathBuf::from("extra.html")],
        };
        let output = format!("{}", report);
        assert!(output.contains("Matching files:      1"));
        assert!(output.contains("Differing files:     1"));
        assert!(output.contains("Missing from actual: 1"));
        assert!(output.contains("Extra in actual:     1"));
    }

    #[test]
    fn test_report_total_files() {
        let report = ComparisonReport {
            matching: vec![PathBuf::from("a.html"), PathBuf::from("b.html")],
            differing: vec![FileDiff {
                path: PathBuf::from("c.html"),
                category: DiffCategory::Content,
                detail: None,
            }],
            missing_from_actual: vec![PathBuf::from("d.html")],
            extra_in_actual: vec![],
        };
        assert_eq!(report.total_files(), 4);
    }

    #[test]
    fn test_xml_whitespace_normalization() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().join("expected");
        let actual = dir.path().join("actual");

        fs::create_dir_all(&expected).unwrap();
        fs::create_dir_all(&actual).unwrap();

        fs::write(
            expected.join("sitemap.xml"),
            "<url>  <loc>http://a.com</loc>  </url>",
        )
        .unwrap();
        fs::write(
            actual.join("sitemap.xml"),
            "<url> <loc>http://a.com</loc> </url>",
        )
        .unwrap();

        let report = compare_directories(&expected, &actual).unwrap();
        assert!(report.is_clean());
    }

    #[test]
    fn test_structural_diff_classification() {
        let expected = "<div><p>Hello</p></div>";
        let actual = "<section><p>Hello</p></section>";
        assert_eq!(
            classify_html_diff(expected, actual),
            DiffCategory::Structural
        );
    }
}
