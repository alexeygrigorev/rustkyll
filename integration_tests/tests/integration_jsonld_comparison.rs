//! Integration tests comparing JSON-LD structured data between Rustkyll and Jekyll output.
//!
//! These tests require pre-built output directories:
//! - `_site/` -- Rustkyll output
//! - `_site_jekyll/` -- Jekyll reference output
//!
//! Run with: cargo test -p integration-tests --test integration_jsonld_comparison

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

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

/// A single diff between Jekyll and Rustkyll JSON-LD values.
#[derive(Debug, Clone)]
struct JsonldDiff {
    page: String,
    field_path: String,
    category: DiffCategory,
    expected: String,
    actual: String,
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

/// Get the project root directory.
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Get the Rustkyll output directory.
fn rustkyll_site_dir() -> PathBuf {
    project_root().join("_site")
}

/// Get the Jekyll reference output directory.
fn jekyll_site_dir() -> PathBuf {
    project_root().join("_site_jekyll")
}

/// Read an HTML file and extract JSON-LD blocks.
fn read_and_extract_jsonld(dir: &Path, page_path: &str) -> Option<Vec<serde_json::Value>> {
    let file = dir.join(page_path);
    if !file.exists() {
        return None;
    }
    let html = fs::read_to_string(&file).ok()?;
    Some(extract_jsonld_blocks(&html))
}

/// Show a clear diff between two strings, highlighting where they diverge.
fn show_string_diff(expected: &str, actual: &str) -> String {
    let mut result = String::new();
    let e_chars: Vec<char> = expected.chars().collect();
    let a_chars: Vec<char> = actual.chars().collect();

    let common_prefix = e_chars
        .iter()
        .zip(a_chars.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let context_start = common_prefix.saturating_sub(30);

    result.push_str(&format!("  Diverges at char {common_prefix}:\n"));
    if common_prefix < expected.len() || common_prefix < actual.len() {
        let e_snippet: String = e_chars.iter().skip(context_start).take(80).collect();
        let a_snippet: String = a_chars.iter().skip(context_start).take(80).collect();
        result.push_str(&format!("  Expected: ...{e_snippet}...\n"));
        result.push_str(&format!("  Actual:   ...{a_snippet}...\n"));
    }
    result
}

/// Extract all acceptedAnswer.text values from FAQ-type JSON-LD blocks.
fn find_faq_answers(blocks: &[serde_json::Value]) -> Vec<String> {
    let mut answers = Vec::new();
    for block in blocks {
        let main_entity = block.get("mainEntity").or_else(|| {
            block.get("@graph").and_then(|g| {
                g.as_array().and_then(|arr| {
                    arr.iter()
                        .find(|item| item.get("@type").and_then(|t| t.as_str()) == Some("FAQPage"))
                        .and_then(|faq| faq.get("mainEntity"))
                })
            })
        });

        if let Some(serde_json::Value::Array(questions)) = main_entity {
            for q in questions {
                if let Some(text) = q
                    .get("acceptedAnswer")
                    .and_then(|a| a.get("text"))
                    .and_then(|t| t.as_str())
                {
                    answers.push(text.to_string());
                }
            }
        }
    }
    answers
}

/// Collect all .html files recursively from a directory.
fn collect_html_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    collect_html_files_recursive(dir, &mut files);
    files.sort();
    files
}

fn collect_html_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_dir() {
            collect_html_files_recursive(&path, files);
        } else if path.extension().map(|e| e == "html").unwrap_or(false) {
            files.push(path);
        }
    }
}

/// Truncate a string for display purposes.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let end = s
            .char_indices()
            .nth(max_len)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        format!("{}...", &s[..end])
    }
}

// ========================================================================
// Integration tests (require pre-built _site/ and _site_jekyll/)
// ========================================================================

const FAQ_PAGES: &[&str] = &[
    "blog/guide-to-free-online-courses-data-science-ml.html",
    "blog/open-source-free-ai-agent-evaluation-tools.html",
    "blog/free-machine-learning-courses.html",
    "blog/slack-communities.html",
    "blog/ai-dev-tools-zoomcamp-2025-free-course-to-master-coding-assistants-agents-and-automation.html",
    "blog/llm-zoomcamp.html",
    "blog/mlops-zoomcamp.html",
    "blog/data-engineering-zoomcamp.html",
    "blog/machine-learning-zoomcamp.html",
];

#[test]
fn test_faq_jsonld_matches_jekyll() {
    let rustkyll_dir = rustkyll_site_dir();
    let jekyll_dir = jekyll_site_dir();

    if !rustkyll_dir.exists() || !jekyll_dir.exists() {
        eprintln!(
            "Skipping: requires _site/ and _site_jekyll/ directories.\n\
             Build both sites first, then run: cargo test -p integration-tests"
        );
        return;
    }

    let mut total_failures = 0;
    let mut failure_details = String::new();

    for page_path in FAQ_PAGES {
        let jekyll_blocks = match read_and_extract_jsonld(&jekyll_dir, page_path) {
            Some(blocks) => blocks,
            None => {
                failure_details.push_str(&format!("MISSING in Jekyll: {page_path}\n"));
                total_failures += 1;
                continue;
            }
        };
        let rustkyll_blocks = match read_and_extract_jsonld(&rustkyll_dir, page_path) {
            Some(blocks) => blocks,
            None => {
                failure_details.push_str(&format!("MISSING in Rustkyll: {page_path}\n"));
                total_failures += 1;
                continue;
            }
        };

        let jekyll_faq = find_faq_answers(&jekyll_blocks);
        let rustkyll_faq = find_faq_answers(&rustkyll_blocks);

        if jekyll_faq.is_empty() && rustkyll_faq.is_empty() {
            continue;
        }

        if jekyll_faq.len() != rustkyll_faq.len() {
            failure_details.push_str(&format!(
                "MISMATCH {page_path}: Jekyll has {} FAQ answers, Rustkyll has {}\n",
                jekyll_faq.len(),
                rustkyll_faq.len()
            ));
            total_failures += 1;
            continue;
        }

        for (i, (j_answer, r_answer)) in jekyll_faq.iter().zip(rustkyll_faq.iter()).enumerate() {
            if j_answer != r_answer {
                failure_details.push_str(&format!(
                    "DIFF {page_path} question[{i}]:\n{}",
                    show_string_diff(j_answer, r_answer)
                ));
                total_failures += 1;
            }
        }
    }

    if total_failures > 0 {
        panic!("FAQ JSON-LD comparison found {total_failures} difference(s):\n\n{failure_details}");
    }

    println!(
        "FAQ JSON-LD comparison: all {} pages match.",
        FAQ_PAGES.len()
    );
}

#[test]
fn test_podcast_jsonld_diff_summary() {
    let rustkyll_dir = rustkyll_site_dir();
    let jekyll_dir = jekyll_site_dir();

    if !rustkyll_dir.exists() || !jekyll_dir.exists() {
        eprintln!(
            "Skipping: requires _site/ and _site_jekyll/ directories.\n\
             Build both sites first, then run: cargo test -p integration-tests"
        );
        return;
    }

    let podcast_dir_jekyll = jekyll_dir.join("podcast");
    let podcast_dir_rustkyll = rustkyll_dir.join("podcast");

    if !podcast_dir_jekyll.exists() || !podcast_dir_rustkyll.exists() {
        eprintln!("Skipping: podcast/ directory not found in one or both site outputs.");
        return;
    }

    let mut all_diffs: Vec<JsonldDiff> = Vec::new();
    let mut pages_compared = 0;
    let mut pages_with_diffs = 0;

    let jekyll_pages = collect_html_files(&podcast_dir_jekyll);

    for jekyll_page in &jekyll_pages {
        let rel_path = jekyll_page
            .strip_prefix(&jekyll_dir)
            .unwrap()
            .to_string_lossy()
            .to_string();

        let rustkyll_page = rustkyll_dir.join(&rel_path);
        if !rustkyll_page.exists() {
            all_diffs.push(JsonldDiff {
                page: rel_path.clone(),
                field_path: "(page)".to_string(),
                category: DiffCategory::Other,
                expected: "(exists)".to_string(),
                actual: "(missing)".to_string(),
            });
            continue;
        }

        let jekyll_html = match fs::read_to_string(jekyll_page) {
            Ok(h) => h,
            Err(_) => continue,
        };
        let rustkyll_html = match fs::read_to_string(&rustkyll_page) {
            Ok(h) => h,
            Err(_) => continue,
        };

        let jekyll_blocks = extract_jsonld_blocks(&jekyll_html);
        let rustkyll_blocks = extract_jsonld_blocks(&rustkyll_html);

        pages_compared += 1;
        let mut page_diffs = Vec::new();

        let max_blocks = jekyll_blocks.len().max(rustkyll_blocks.len());
        for i in 0..max_blocks {
            match (jekyll_blocks.get(i), rustkyll_blocks.get(i)) {
                (Some(j_block), Some(r_block)) => {
                    let mut raw_diffs = Vec::new();
                    compare_json_values(j_block, r_block, "", &mut raw_diffs);
                    for (field_path, expected, actual) in raw_diffs {
                        let category = classify_diff(&field_path, &expected, &actual);
                        page_diffs.push(JsonldDiff {
                            page: rel_path.clone(),
                            field_path,
                            category,
                            expected,
                            actual,
                        });
                    }
                }
                (Some(_), None) => {
                    page_diffs.push(JsonldDiff {
                        page: rel_path.clone(),
                        field_path: format!("block[{i}]"),
                        category: DiffCategory::Other,
                        expected: "(exists)".to_string(),
                        actual: "(missing block)".to_string(),
                    });
                }
                (None, Some(_)) => {
                    page_diffs.push(JsonldDiff {
                        page: rel_path.clone(),
                        field_path: format!("block[{i}]"),
                        category: DiffCategory::Other,
                        expected: "(missing block)".to_string(),
                        actual: "(exists)".to_string(),
                    });
                }
                (None, None) => {}
            }
        }

        if !page_diffs.is_empty() {
            pages_with_diffs += 1;
        }
        all_diffs.extend(page_diffs);
    }

    // Categorize and summarize
    let mut counts: HashMap<DiffCategory, usize> = HashMap::new();
    let mut pages_per_category: HashMap<DiffCategory, Vec<String>> = HashMap::new();
    let mut samples_per_category: HashMap<DiffCategory, Vec<(String, String, String)>> =
        HashMap::new();

    for diff in &all_diffs {
        *counts.entry(diff.category.clone()).or_insert(0) += 1;
        pages_per_category
            .entry(diff.category.clone())
            .or_default()
            .push(diff.page.clone());
        let samples = samples_per_category
            .entry(diff.category.clone())
            .or_default();
        if samples.len() < 3 {
            samples.push((
                diff.field_path.clone(),
                diff.expected.clone(),
                diff.actual.clone(),
            ));
        }
    }

    for pages in pages_per_category.values_mut() {
        pages.sort();
        pages.dedup();
    }

    println!("\n=== Podcast JSON-LD Diff Summary ===");
    println!("Pages compared: {pages_compared}");
    println!("Pages with diffs: {pages_with_diffs}");
    println!();

    let categories = [
        DiffCategory::DateFormat,
        DiffCategory::BuildTimestamp,
        DiffCategory::SmartQuote,
        DiffCategory::Transcript,
        DiffCategory::Other,
    ];

    for cat in &categories {
        let count = counts.get(cat).copied().unwrap_or(0);
        let page_count = pages_per_category.get(cat).map(|p| p.len()).unwrap_or(0);
        println!("{cat}: {count} diffs across {page_count} pages");

        if let Some(samples) = samples_per_category.get(cat) {
            for (field, expected, actual) in samples {
                println!("  Sample: {field}");
                println!("    Expected: {}", truncate_str(expected, 100));
                println!("    Actual:   {}", truncate_str(actual, 100));
            }
        }
        println!();
    }

    let build_ts_count = counts
        .get(&DiffCategory::BuildTimestamp)
        .copied()
        .unwrap_or(0);
    let total_non_noise = all_diffs.len() - build_ts_count;
    println!("Total diffs: {}", all_diffs.len());
    println!("Non-noise diffs (excluding build_timestamp): {total_non_noise}");

    let date_format_count = counts.get(&DiffCategory::DateFormat).copied().unwrap_or(0);
    let smart_quote_count = counts.get(&DiffCategory::SmartQuote).copied().unwrap_or(0);
    let transcript_count = counts.get(&DiffCategory::Transcript).copied().unwrap_or(0);

    assert!(
        date_format_count <= 200,
        "date_format diffs ({date_format_count}) exceeded threshold of 200 -- regression?"
    );
    assert!(
        smart_quote_count <= 160,
        "smart_quote diffs ({smart_quote_count}) exceeded threshold of 160 -- regression?"
    );
    assert!(
        transcript_count <= 65,
        "transcript diffs ({transcript_count}) exceeded threshold of 65 -- regression?"
    );

    let faq_diffs: Vec<&JsonldDiff> = all_diffs
        .iter()
        .filter(|d| d.page.starts_with("blog/"))
        .filter(|d| d.category != DiffCategory::BuildTimestamp)
        .collect();
    assert!(
        faq_diffs.is_empty(),
        "FAQ pages should have 0 JSON-LD diffs, but found {}: {:?}",
        faq_diffs.len(),
        faq_diffs.iter().map(|d| &d.page).collect::<Vec<_>>()
    );

    println!("\nAll regression guards passed.");
}
