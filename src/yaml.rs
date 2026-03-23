//! YAML parsing utilities that handle duplicate keys with last-value-wins semantics.
//!
//! Ruby's YAML parser (Psych) silently accepts duplicate keys, using the last
//! occurrence. `serde_yaml` 0.9 rejects them. This module bridges the gap by
//! implementing a custom YAML loader via `yaml-rust2` that tolerates duplicates,
//! then converting to `serde_yaml::Value` for downstream consumption.

use std::collections::BTreeMap;
use std::mem;

use yaml_rust2::parser::{Event, MarkedEventReceiver, Parser, Tag};
use yaml_rust2::scanner::{Marker, ScanError, TScalarStyle};
use yaml_rust2::yaml::{Hash, Yaml};

/// Errors from lenient YAML parsing.
#[derive(Debug, thiserror::Error)]
pub enum YamlParseError {
    #[error("YAML scan/parse error: {0}")]
    Scan(#[from] ScanError),

    #[error("failed to deserialize YAML value: {0}")]
    Conversion(String),
}

/// A YAML loader that accepts duplicate keys with last-value-wins semantics.
///
/// This is a fork of `yaml_rust2::YamlLoader` with the single change that
/// duplicate mapping keys silently overwrite instead of returning an error.
#[derive(Default)]
struct LenientYamlLoader {
    docs: Vec<Yaml>,
    doc_stack: Vec<(Yaml, usize)>,
    key_stack: Vec<Yaml>,
    anchor_map: BTreeMap<usize, Yaml>,
    error: Option<ScanError>,
}

impl MarkedEventReceiver for LenientYamlLoader {
    fn on_event(&mut self, ev: Event, mark: Marker) {
        if self.error.is_some() {
            return;
        }
        if let Err(e) = self.on_event_impl(ev, mark) {
            self.error = Some(e);
        }
    }
}

impl LenientYamlLoader {
    fn on_event_impl(&mut self, ev: Event, mark: Marker) -> Result<(), ScanError> {
        match ev {
            Event::DocumentStart | Event::Nothing | Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentEnd => match self.doc_stack.len() {
                0 => self.docs.push(Yaml::BadValue),
                1 => self.docs.push(self.doc_stack.pop().unwrap().0),
                _ => unreachable!(),
            },
            Event::SequenceStart(aid, _) => {
                self.doc_stack.push((Yaml::Array(Vec::new()), aid));
            }
            Event::SequenceEnd => {
                let node = self.doc_stack.pop().unwrap();
                self.insert_new_node(node, mark)?;
            }
            Event::MappingStart(aid, _) => {
                self.doc_stack.push((Yaml::Hash(Hash::new()), aid));
                self.key_stack.push(Yaml::BadValue);
            }
            Event::MappingEnd => {
                self.key_stack.pop().unwrap();
                let node = self.doc_stack.pop().unwrap();
                self.insert_new_node(node, mark)?;
            }
            Event::Scalar(v, style, aid, tag) => {
                let node = if style != TScalarStyle::Plain {
                    Yaml::String(v)
                } else if let Some(Tag {
                    ref handle,
                    ref suffix,
                }) = tag
                {
                    if handle == "tag:yaml.org,2002:" {
                        match suffix.as_ref() {
                            "bool" => match v.as_str() {
                                "true" | "True" | "TRUE" => Yaml::Boolean(true),
                                "false" | "False" | "FALSE" => Yaml::Boolean(false),
                                _ => Yaml::BadValue,
                            },
                            "int" => match v.parse::<i64>() {
                                Err(_) => Yaml::BadValue,
                                Ok(v) => Yaml::Integer(v),
                            },
                            "float" => match parse_f64(&v) {
                                Some(_) => Yaml::Real(v),
                                None => Yaml::BadValue,
                            },
                            "null" => match v.as_ref() {
                                "~" | "null" => Yaml::Null,
                                _ => Yaml::BadValue,
                            },
                            _ => Yaml::String(v),
                        }
                    } else {
                        Yaml::String(v)
                    }
                } else {
                    let parsed = Yaml::from_str(&v);
                    // yaml_rust2 does NOT implement YAML 1.1 sexagesimal
                    // (base-60) parsing. Ruby's Psych converts e.g. `0:36`
                    // to 36.0. We intentionally keep the original human-readable
                    // string (e.g. "0:36") instead, as an improvement over Jekyll.
                    if matches!(parsed, Yaml::String(_)) {
                        if is_sexagesimal(&v) {
                            // Keep original string like "0:36" or "1:05:30"
                            Yaml::String(v)
                        } else {
                            parsed
                        }
                    } else {
                        parsed
                    }
                };

                self.insert_new_node((node, aid), mark)?;
            }
            Event::Alias(id) => {
                let n = match self.anchor_map.get(&id) {
                    Some(v) => v.clone(),
                    None => Yaml::BadValue,
                };
                self.insert_new_node((n, 0), mark)?;
            }
        }
        Ok(())
    }

    fn insert_new_node(&mut self, node: (Yaml, usize), _mark: Marker) -> Result<(), ScanError> {
        if node.1 > 0 {
            self.anchor_map.insert(node.1, node.0.clone());
        }
        if self.doc_stack.is_empty() {
            self.doc_stack.push(node);
        } else {
            let parent = self.doc_stack.last_mut().unwrap();
            match *parent {
                (Yaml::Array(ref mut v), _) => v.push(node.0),
                (Yaml::Hash(ref mut h), _) => {
                    let cur_key = self.key_stack.last_mut().unwrap();
                    if cur_key.is_badvalue() {
                        *cur_key = node.0;
                    } else {
                        let mut newkey = Yaml::BadValue;
                        mem::swap(&mut newkey, cur_key);
                        // YAML 1.1 merge key: when key is "<<", merge the
                        // value's mapping entries into the current mapping.
                        // Keys already present are NOT overwritten (first wins
                        // for merge, matching Ruby/Jekyll behavior).
                        if newkey == Yaml::String("<<".to_string()) {
                            match node.0 {
                                Yaml::Hash(ref merge_hash) => {
                                    for (k, v) in merge_hash {
                                        // Only insert if key doesn't already exist
                                        if !h.contains_key(k) {
                                            h.insert(k.clone(), v.clone());
                                        }
                                    }
                                }
                                Yaml::Array(ref arr) => {
                                    // Merge multiple mappings in order
                                    for item in arr {
                                        if let Yaml::Hash(ref merge_hash) = item {
                                            for (k, v) in merge_hash {
                                                if !h.contains_key(k) {
                                                    h.insert(k.clone(), v.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    // Non-mapping value for << key: just insert normally
                                    h.insert(newkey, node.0);
                                }
                            }
                        } else {
                            // Silently overwrite on duplicate key (last-value-wins).
                            h.insert(newkey, node.0);
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    fn load_from_str(source: &str) -> Result<Vec<Yaml>, ScanError> {
        let mut parser = Parser::new(source.chars());
        let mut loader = LenientYamlLoader::default();
        parser.load(&mut loader, true)?;
        if let Some(e) = loader.error {
            Err(e)
        } else {
            Ok(loader.docs)
        }
    }
}

/// Check whether a string matches the YAML 1.1 sexagesimal (base-60) pattern.
///
/// Sexagesimal values are colon-separated digit groups like `0:36`, `1:30`, `1:30:00`.
/// Ruby/Psych converts these to floats (e.g. `0:36` -> `36.0`). We intentionally
/// keep the original human-readable string instead.
fn is_sexagesimal(v: &str) -> bool {
    let parts: Vec<&str> = v.split(':').collect();
    if parts.len() < 2 {
        return false;
    }
    parts
        .iter()
        .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

// Copied from yaml-rust2 internals for scalar parsing.
fn parse_f64(v: &str) -> Option<f64> {
    match v {
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => Some(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => Some(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => Some(f64::NAN),
        _ if v.as_bytes().iter().any(u8::is_ascii_digit) => v.parse::<f64>().ok(),
        _ => None,
    }
}

/// Parse a YAML string into a `serde_yaml::Value`, tolerating duplicate keys.
///
/// When duplicate keys are found in a mapping, the last value wins (matching
/// Ruby/Jekyll behavior). Returns `serde_yaml::Value::Null` for empty documents.
///
/// # Errors
///
/// Returns `YamlParseError::Scan` if the YAML is syntactically invalid.
pub fn parse_yaml_lenient(input: &str) -> Result<serde_yaml::Value, YamlParseError> {
    let docs = LenientYamlLoader::load_from_str(input)?;
    if docs.is_empty() {
        return Ok(serde_yaml::Value::Null);
    }
    Ok(yaml_to_serde(&docs[0]))
}

/// Convert a `yaml_rust2::Yaml` value to a `serde_yaml::Value`.
fn yaml_to_serde(yaml: &Yaml) -> serde_yaml::Value {
    match yaml {
        Yaml::Real(s) => {
            if let Ok(f) = s.parse::<f64>() {
                // If the float is a whole number (like 6.0) and the original
                // string contains a decimal point, preserve as string to match
                // Jekyll/Ruby behavior where 6.0 renders as "6.0", not "6".
                if f.fract() == 0.0 && s.contains('.') {
                    serde_yaml::Value::String(s.clone())
                } else {
                    serde_yaml::Value::Number(serde_yaml::Number::from(f))
                }
            } else {
                serde_yaml::Value::String(s.clone())
            }
        }
        Yaml::Integer(i) => serde_yaml::Value::Number(serde_yaml::Number::from(*i)),
        Yaml::String(s) => serde_yaml::Value::String(s.clone()),
        Yaml::Boolean(b) => serde_yaml::Value::Bool(*b),
        Yaml::Array(arr) => {
            let items: Vec<serde_yaml::Value> = arr.iter().map(yaml_to_serde).collect();
            serde_yaml::Value::Sequence(items)
        }
        Yaml::Hash(hash) => {
            let mut mapping = serde_yaml::Mapping::new();
            for (k, v) in hash {
                mapping.insert(yaml_to_serde(k), yaml_to_serde(v));
            }
            serde_yaml::Value::Mapping(mapping)
        }
        Yaml::Null | Yaml::BadValue => serde_yaml::Value::Null,
        Yaml::Alias(_) => serde_yaml::Value::Null,
    }
}

/// Parse a YAML string into a deserialized type `T`, tolerating duplicate keys.
///
/// This first parses via the lenient loader to handle duplicates, converts to
/// `serde_yaml::Value`, then deserializes into the target type.
///
/// # Errors
///
/// Returns `YamlParseError::Scan` for syntax errors, or
/// `YamlParseError::Conversion` if the value cannot be deserialized into `T`.
pub fn from_str_lenient<T: serde::de::DeserializeOwned>(input: &str) -> Result<T, YamlParseError> {
    let value = parse_yaml_lenient(input)?;
    serde_yaml::from_value(value).map_err(|e| YamlParseError::Conversion(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ========================================================================
    // Basic parsing
    // ========================================================================

    #[test]
    fn test_parse_simple_mapping() {
        let yaml = "name: test\nversion: 1\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(mapping.get("name").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(mapping.get("version").and_then(|v| v.as_u64()), Some(1));
    }

    #[test]
    fn test_parse_empty_string() {
        let value = parse_yaml_lenient("").unwrap();
        assert!(value.is_null());
    }

    #[test]
    fn test_parse_sequence() {
        let yaml = "- one\n- two\n- three\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let seq = value.as_sequence().unwrap();
        assert_eq!(seq.len(), 3);
    }

    // ========================================================================
    // Duplicate key handling (Issue 43)
    // ========================================================================

    #[test]
    fn test_duplicate_top_level_key_last_wins() {
        let yaml = "url: first\nurl: second\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(mapping.get("url").and_then(|v| v.as_str()), Some("second"));
    }

    #[test]
    fn test_duplicate_nested_key_last_wins() {
        let yaml = "sass:\n  style: expanded\n  style: compressed\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        let sass = mapping.get("sass").unwrap().as_mapping().unwrap();
        assert_eq!(
            sass.get("style").and_then(|v| v.as_str()),
            Some("compressed")
        );
    }

    #[test]
    fn test_duplicate_key_different_types() {
        let yaml = "foo: 1\nfoo: bar\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(mapping.get("foo").and_then(|v| v.as_str()), Some("bar"));
    }

    #[test]
    fn test_triple_duplicate_key() {
        let yaml = "key: first\nkey: second\nkey: third\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(mapping.get("key").and_then(|v| v.as_str()), Some("third"));
    }

    #[test]
    fn test_deserialize_with_duplicates() {
        #[derive(serde::Deserialize, Debug)]
        struct Config {
            url: String,
            name: String,
        }
        let yaml = "url: first\nname: test\nurl: second\n";
        let config: Config = from_str_lenient(yaml).unwrap();
        assert_eq!(config.url, "second");
        assert_eq!(config.name, "test");
    }

    #[test]
    fn test_duplicate_in_front_matter_style() {
        let yaml = "title: First Title\nlayout: post\ntitle: Second Title\n";
        let fm: HashMap<String, serde_yaml::Value> = from_str_lenient(yaml).unwrap();
        assert_eq!(
            fm.get("title").and_then(|v| v.as_str()),
            Some("Second Title")
        );
        assert_eq!(fm.get("layout").and_then(|v| v.as_str()), Some("post"));
    }

    #[test]
    fn test_realistic_bitcoin_config_pattern() {
        let yaml = r#"
title: Bitcoin
url: https://bitcoin.org
permalink: /:title.html
collections:
  posts:
    output: true
    permalink: /blog/:title.html
plugins:
  - jekyll-redirect-from
plugins:
  - jekyll-redirect-from
  - jekyll-sitemap
"#;
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        // The second `plugins` list should win
        let plugins = mapping.get("plugins").unwrap().as_sequence().unwrap();
        assert_eq!(plugins.len(), 2);
    }

    // ========================================================================
    // Data type preservation
    // ========================================================================

    #[test]
    fn test_preserves_booleans() {
        let yaml = "flag: true\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(mapping.get("flag").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn test_preserves_integers() {
        let yaml = "count: 42\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(mapping.get("count").and_then(|v| v.as_u64()), Some(42));
    }

    #[test]
    fn test_preserves_floats() {
        let yaml = "pi: 3.14\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(mapping.get("pi").and_then(|v| v.as_f64()), Some(3.14));
    }

    #[test]
    fn test_preserves_null() {
        let yaml = "empty:\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        assert!(mapping.get("empty").unwrap().is_null());
    }

    #[test]
    fn test_invalid_yaml_returns_error() {
        let result = parse_yaml_lenient(":\n  - :\n    invalid: [unclosed");
        assert!(result.is_err());
    }

    // ========================================================================
    // Sexagesimal timestamp handling (Issues 101, 155, 161)
    //
    // YAML 1.1 interprets colon-separated values like 0:30 as sexagesimal
    // (base-60) floats. Ruby/Jekyll converts these to floats (e.g. 36.0).
    // We intentionally keep the original human-readable string (e.g. "0:36")
    // as an improvement over Jekyll. This is a known acceptable difference.
    // ========================================================================

    #[test]
    fn test_sexagesimal_short_timestamp_stays_as_string() {
        // 0:30 stays as "0:30" (not converted to "30.0" like Jekyll)
        let value = parse_yaml_lenient("time: 0:30").unwrap();
        let mapping = value.as_mapping().unwrap();
        let time = mapping.get("time").unwrap();
        assert_eq!(time.as_str(), Some("0:30"));
    }

    #[test]
    fn test_sexagesimal_zero_timestamp_stays_as_string() {
        // 0:00 stays as "0:00"
        let value = parse_yaml_lenient("time: 0:00").unwrap();
        let mapping = value.as_mapping().unwrap();
        let time = mapping.get("time").unwrap();
        assert_eq!(time.as_str(), Some("0:00"));
    }

    #[test]
    fn test_sexagesimal_hour_minute_second_stays_as_string() {
        // 1:30:00 stays as "1:30:00" (not converted to "5400.0")
        let value = parse_yaml_lenient("time: 1:30:00").unwrap();
        let mapping = value.as_mapping().unwrap();
        let time = mapping.get("time").unwrap();
        assert_eq!(time.as_str(), Some("1:30:00"));
    }

    #[test]
    fn test_sexagesimal_various_podcast_timestamps_stay_human_readable() {
        let yaml = r#"
transcript:
  - time: 0:00
    sec: 0
  - time: 0:12
    sec: 12
  - time: 0:30
    sec: 30
  - time: 0:41
    sec: 41
"#;
        let value = parse_yaml_lenient(yaml).unwrap();
        let transcript = value
            .as_mapping()
            .unwrap()
            .get("transcript")
            .unwrap()
            .as_sequence()
            .unwrap();

        let expected_times = ["0:00", "0:12", "0:30", "0:41"];
        let expected_secs = [0, 12, 30, 41];

        for (i, item) in transcript.iter().enumerate() {
            let m = item.as_mapping().unwrap();
            assert_eq!(
                m.get("time").unwrap().as_str(),
                Some(expected_times[i]),
                "time[{}] should be string '{}'",
                i,
                expected_times[i]
            );
            assert_eq!(
                m.get("sec").unwrap().as_u64(),
                Some(expected_secs[i]),
                "sec[{}] should be integer {}",
                i,
                expected_secs[i]
            );
        }
    }

    #[test]
    fn test_quoted_timestamps_stay_string() {
        // Quoted timestamps should stay as strings
        let value = parse_yaml_lenient("time: '1:05'").unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(mapping.get("time").unwrap().as_str(), Some("1:05"));
    }

    #[test]
    fn test_is_sexagesimal_function() {
        assert!(is_sexagesimal("0:36"));
        assert!(is_sexagesimal("0:00"));
        assert!(is_sexagesimal("0:01"));
        assert!(is_sexagesimal("1:30"));
        assert!(is_sexagesimal("1:30:00"));
        assert!(is_sexagesimal("2:30:45"));
        // Not sexagesimal
        assert!(!is_sexagesimal("hello"));
        assert!(!is_sexagesimal("42"));
        assert!(!is_sexagesimal(":30"));
        assert!(!is_sexagesimal("0:"));
        assert!(!is_sexagesimal("https://example.com"));
    }

    #[test]
    fn test_url_not_parsed_as_sexagesimal() {
        // URLs contain colons but should not be parsed as sexagesimal
        // because they contain non-digit characters
        let value = parse_yaml_lenient("url: https://example.com").unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(
            mapping.get("url").and_then(|v| v.as_str()),
            Some("https://example.com")
        );
    }

    #[test]
    fn test_sexagesimal_1_05_30_stays_as_string() {
        // 1:05:30 stays as "1:05:30" (not "3930.0")
        let value = parse_yaml_lenient("time: 1:05:30").unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(mapping.get("time").unwrap().as_str(), Some("1:05:30"));
    }

    #[test]
    fn test_datetime_with_timezone_offset_preserved() {
        // Dates with timezone offsets must stay as strings with the original offset
        let yaml = "date: 2025-11-07 00:00:00 +0100";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        let date_val = mapping.get("date").unwrap();
        assert_eq!(
            date_val.as_str().unwrap(),
            "2025-11-07 00:00:00 +0100",
            "datetime with timezone offset must be preserved as-is"
        );
    }

    #[test]
    fn test_datetime_with_timezone_offset_unicode_key() {
        // Non-ASCII content alongside date fields with timezone offsets
        let yaml = "date: 2025-11-07 00:00:00 +0200\ntitle: \u{00dc}bersicht";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(
            mapping.get("date").unwrap().as_str().unwrap(),
            "2025-11-07 00:00:00 +0200"
        );
        assert_eq!(
            mapping.get("title").unwrap().as_str().unwrap(),
            "\u{00dc}bersicht"
        );
    }

    // ========================================================================
    // YAML merge key (<<) support
    // ========================================================================

    #[test]
    fn test_yaml_merge_key_basic() {
        let yaml = "defaults: &DEF\n  color: red\n  size: 10\nitem:\n  <<: *DEF\n  name: widget\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        let item = mapping.get("item").unwrap().as_mapping().unwrap();
        // Merged keys from anchor
        assert_eq!(item.get("color").and_then(|v| v.as_str()), Some("red"));
        assert_eq!(item.get("size").and_then(|v| v.as_u64()), Some(10));
        // Own key
        assert_eq!(item.get("name").and_then(|v| v.as_str()), Some("widget"));
    }

    #[test]
    fn test_yaml_merge_key_own_values_override() {
        let yaml = "base: &BASE\n  mode: debug\n  level: 1\nchild:\n  <<: *BASE\n  mode: release\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        let child = mapping.get("child").unwrap().as_mapping().unwrap();
        // Own value overrides merged value
        assert_eq!(child.get("mode").and_then(|v| v.as_str()), Some("release"));
        // Merged value preserved
        assert_eq!(child.get("level").and_then(|v| v.as_u64()), Some(1));
    }

    #[test]
    fn test_yaml_merge_key_multiple_anchors() {
        let yaml = "a: &A\n  x: 1\nb: &B\n  y: 2\nc:\n  <<: [*A, *B]\n  z: 3\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        let c = mapping.get("c").unwrap().as_mapping().unwrap();
        assert_eq!(c.get("x").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(c.get("y").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(c.get("z").and_then(|v| v.as_u64()), Some(3));
    }

    #[test]
    fn test_yaml_merge_key_unicode_values() {
        // Merge keys with non-ASCII content (Cyrillic, CJK)
        let yaml = "defaults: &DEF\n  greeting: \"\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}\"\n  lang: \"\u{4e2d}\u{6587}\"\nitem:\n  <<: *DEF\n  name: test\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        let item = mapping.get("item").unwrap().as_mapping().unwrap();
        assert_eq!(
            item.get("greeting").and_then(|v| v.as_str()),
            Some("\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}")
        );
        assert_eq!(
            item.get("lang").and_then(|v| v.as_str()),
            Some("\u{4e2d}\u{6587}")
        );
    }

    // ========================================================================
    // Float preservation (6.0 stays "6.0", not "6")
    // ========================================================================

    #[test]
    fn test_yaml_whole_float_preserved_as_string() {
        let yaml = "version: 6.0\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        // 6.0 should be preserved as string "6.0" (not collapsed to integer 6)
        let version = mapping.get("version").unwrap();
        assert_eq!(version.as_str(), Some("6.0"));
    }

    #[test]
    fn test_yaml_fractional_float_stays_number() {
        let yaml = "ratio: 1.5\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        let ratio = mapping.get("ratio").unwrap();
        // 1.5 has a fractional part, stays as number
        assert!(ratio.is_number(), "1.5 should be a number, got {:?}", ratio);
        assert!((ratio.as_f64().unwrap() - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_yaml_integer_stays_integer() {
        let yaml = "count: 42\n";
        let value = parse_yaml_lenient(yaml).unwrap();
        let mapping = value.as_mapping().unwrap();
        let count = mapping.get("count").unwrap();
        assert_eq!(count.as_u64(), Some(42));
    }
}
