//! Unit tests for JSON-LD comparison helpers.
//!
//! The actual integration tests (which require pre-built site output) have been
//! moved to integration_tests/tests/integration_jsonld_comparison.rs.

use std::fmt;

// ========================================================================
// Helpers
// ========================================================================

/// Extract all JSON-LD blocks from an HTML string and parse each one.
fn extract_jsonld_blocks(html: &str) -> Vec<serde_json::Value> {
    let marker = r#"<script type="application/ld+json">"#;
    let end_marker = "</script>";
    let mut results = Vec::new();
    let mut search_from = 0;

    while let Some(start_pos) = html[search_from..].find(marker) {
        let abs_start = search_from + start_pos + marker.len();
        if let Some(end_pos) = html[abs_start..].find(end_marker) {
            let json_str = html[abs_start..abs_start + end_pos].trim();
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                results.push(val);
            }
            search_from = abs_start + end_pos + end_marker.len();
        } else {
            break;
        }
    }
    results
}

/// Category for a JSON-LD diff.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DiffCategory {
    DateFormat,
    BuildTimestamp,
    SmartQuote,
    Transcript,
    Other,
}

impl fmt::Display for DiffCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffCategory::DateFormat => write!(f, "date_format"),
            DiffCategory::BuildTimestamp => write!(f, "build_timestamp"),
            DiffCategory::SmartQuote => write!(f, "smart_quote"),
            DiffCategory::Transcript => write!(f, "transcript"),
            DiffCategory::Other => write!(f, "other"),
        }
    }
}

/// Check if a string looks like a date/datetime (starts with YYYY-MM-DD pattern).
fn looks_like_date(s: &str) -> bool {
    if s.len() < 10 {
        return false;
    }
    let bytes = s.as_bytes();
    bytes[0..4].iter().all(|b| b.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(|b| b.is_ascii_digit())
}

/// Check if two date strings differ only in time/timezone formatting.
fn is_date_format_diff(expected: &str, actual: &str) -> bool {
    if !looks_like_date(expected) || !looks_like_date(actual) {
        return false;
    }
    expected[..10] == actual[..10]
}

/// Check if two values are build timestamps (recent datetimes that differ
/// because builds happened at different times).
fn is_build_timestamp_diff(expected: &str, actual: &str) -> bool {
    if !looks_like_date(expected) || !looks_like_date(actual) {
        return false;
    }
    expected[..10] != actual[..10] && expected.len() > 10 && actual.len() > 10
}

/// Check if two strings differ only in smart quote characters.
fn is_smart_quote_diff(expected: &str, actual: &str) -> bool {
    let normalized_expected = normalize_quotes(expected);
    let normalized_actual = normalize_quotes(actual);
    normalized_expected == normalized_actual && expected != actual
}

/// Normalize smart quotes to ASCII equivalents.
fn normalize_quotes(s: &str) -> String {
    s.replace(['\u{2019}', '\u{2018}'], "'")
        .replace(['\u{201C}', '\u{201D}'], "\"")
}

/// Classify a diff between two JSON-LD values.
fn classify_diff(field_path: &str, expected: &str, actual: &str) -> DiffCategory {
    if field_path.contains("transcript") {
        return DiffCategory::Transcript;
    }
    if field_path.contains("endDate") && is_build_timestamp_diff(expected, actual) {
        return DiffCategory::BuildTimestamp;
    }
    if (field_path.contains("startDate")
        || field_path.contains("endDate")
        || field_path.contains("datePublished"))
        && is_date_format_diff(expected, actual)
    {
        return DiffCategory::DateFormat;
    }
    if is_smart_quote_diff(expected, actual) {
        return DiffCategory::SmartQuote;
    }
    if is_date_format_diff(expected, actual) {
        return DiffCategory::DateFormat;
    }
    DiffCategory::Other
}

/// Recursively compare two JSON values and collect diffs with field paths.
fn compare_json_values(
    expected: &serde_json::Value,
    actual: &serde_json::Value,
    path: &str,
    diffs: &mut Vec<(String, String, String)>,
) {
    match (expected, actual) {
        (serde_json::Value::Object(e_map), serde_json::Value::Object(a_map)) => {
            for (key, e_val) in e_map {
                let field_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match a_map.get(key) {
                    Some(a_val) => compare_json_values(e_val, a_val, &field_path, diffs),
                    None => diffs.push((field_path, format!("{e_val}"), "(missing)".to_string())),
                }
            }
            for (key, a_val) in a_map {
                if !e_map.contains_key(key) {
                    let field_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    diffs.push((field_path, "(missing)".to_string(), format!("{a_val}")));
                }
            }
        }
        (serde_json::Value::Array(e_arr), serde_json::Value::Array(a_arr)) => {
            let max_len = e_arr.len().max(a_arr.len());
            for i in 0..max_len {
                let field_path = format!("{path}[{i}]");
                match (e_arr.get(i), a_arr.get(i)) {
                    (Some(e_val), Some(a_val)) => {
                        compare_json_values(e_val, a_val, &field_path, diffs);
                    }
                    (Some(e_val), None) => {
                        diffs.push((field_path, format!("{e_val}"), "(missing)".to_string()));
                    }
                    (None, Some(a_val)) => {
                        diffs.push((field_path, "(missing)".to_string(), format!("{a_val}")));
                    }
                    (None, None) => {}
                }
            }
        }
        _ => {
            let e_str = json_value_to_string(expected);
            let a_str = json_value_to_string(actual);
            if e_str != a_str {
                diffs.push((path.to_string(), e_str, a_str));
            }
        }
    }
}

/// Convert a JSON value to a comparable string representation.
fn json_value_to_string(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

// ========================================================================
// Unit tests
// ========================================================================

#[test]
fn test_extract_jsonld_blocks_empty_html() {
    let blocks = extract_jsonld_blocks("<html><body>No JSON-LD here</body></html>");
    assert!(blocks.is_empty());
}

#[test]
fn test_extract_jsonld_blocks_single_block() {
    let html = r#"<html><head>
    <script type="application/ld+json">{"@type": "FAQPage", "name": "Test"}</script>
    </head><body></body></html>"#;
    let blocks = extract_jsonld_blocks(html);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0]["@type"], "FAQPage");
    assert_eq!(blocks[0]["name"], "Test");
}

#[test]
fn test_extract_jsonld_blocks_multiple_blocks() {
    let html = r#"<html><head>
    <script type="application/ld+json">{"@type": "FAQPage"}</script>
    <script type="application/ld+json">{"@type": "BreadcrumbList"}</script>
    </head><body></body></html>"#;
    let blocks = extract_jsonld_blocks(html);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0]["@type"], "FAQPage");
    assert_eq!(blocks[1]["@type"], "BreadcrumbList");
}

#[test]
fn test_extract_jsonld_blocks_with_unicode() {
    let html = r#"<html><head>
    <script type="application/ld+json">{"@type": "FAQPage", "name": "Caf\u00e9 \u00fc\u00f1\u00ef\u00e7\u00f8\u00f0\u00e9"}</script>
    </head><body></body></html>"#;
    let blocks = extract_jsonld_blocks(html);
    assert_eq!(blocks.len(), 1);
}

#[test]
fn test_classify_diff_date_format() {
    let cat = classify_diff(
        "@graph[1].startDate",
        "2025-11-07 00:00:00 +0100",
        "2025-11-07",
    );
    assert_eq!(cat, DiffCategory::DateFormat);
}

#[test]
fn test_classify_diff_build_timestamp() {
    let cat = classify_diff(
        "@graph[1].endDate",
        "2026-03-18 14:30:00 +0100",
        "2026-03-17 09:15:00 +0000",
    );
    assert_eq!(cat, DiffCategory::BuildTimestamp);
}

#[test]
fn test_classify_diff_smart_quote() {
    let cat = classify_diff(
        "@graph[0].about[2].description",
        "He\u{2019}s a great speaker",
        "He's a great speaker",
    );
    assert_eq!(cat, DiffCategory::SmartQuote);
}

#[test]
fn test_classify_diff_transcript() {
    let cat = classify_diff(
        "@graph[2].transcript",
        "Hello [0:00] world",
        "Hello [0.0] world",
    );
    assert_eq!(cat, DiffCategory::Transcript);
}

#[test]
fn test_classify_diff_other() {
    let cat = classify_diff("description", "detecti...", "detectio..");
    assert_eq!(cat, DiffCategory::Other);
}

#[test]
fn test_compare_json_values_identical() {
    let a: serde_json::Value = serde_json::json!({"name": "Test", "value": 42});
    let mut diffs = Vec::new();
    compare_json_values(&a, &a, "", &mut diffs);
    assert!(diffs.is_empty());
}

#[test]
fn test_compare_json_values_different() {
    let a: serde_json::Value = serde_json::json!({"name": "Alice"});
    let b: serde_json::Value = serde_json::json!({"name": "Bob"});
    let mut diffs = Vec::new();
    compare_json_values(&a, &b, "", &mut diffs);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].0, "name");
    assert_eq!(diffs[0].1, "Alice");
    assert_eq!(diffs[0].2, "Bob");
}

#[test]
fn test_compare_json_values_nested() {
    let a: serde_json::Value = serde_json::json!({"@graph": [{"@type": "A"}, {"@type": "B"}]});
    let b: serde_json::Value = serde_json::json!({"@graph": [{"@type": "A"}, {"@type": "C"}]});
    let mut diffs = Vec::new();
    compare_json_values(&a, &b, "", &mut diffs);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].0, "@graph[1].@type");
}

#[test]
fn test_is_smart_quote_diff_true() {
    assert!(is_smart_quote_diff(
        "He\u{2019}s a \u{201C}great\u{201D} speaker",
        "He's a \"great\" speaker"
    ));
}

#[test]
fn test_is_smart_quote_diff_false_other_changes() {
    assert!(!is_smart_quote_diff("Hello world", "Hello earth"));
}

#[test]
fn test_looks_like_date() {
    assert!(looks_like_date("2025-11-07"));
    assert!(looks_like_date("2025-11-07 00:00:00 +0100"));
    assert!(!looks_like_date("hello"));
    assert!(!looks_like_date("12345"));
}
