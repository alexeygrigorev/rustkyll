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
                    // (base-60) parsing. Ruby's Psych does, converting
                    // e.g. `0:36` to 36.0. Match Ruby here for Jekyll compat.
                    if matches!(parsed, Yaml::String(_)) {
                        if let Some(f) = parse_sexagesimal(&v) {
                            // Store as string "N.0" so Liquid renders it exactly
                            // as Ruby/Jekyll does (e.g. "36.0" not "36").
                            Yaml::String(format_sexagesimal_float(f))
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
                        // Silently overwrite on duplicate key (last-value-wins).
                        h.insert(newkey, node.0);
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

/// Format a sexagesimal float value to match Ruby's display: always show `.0`
/// for whole numbers (e.g., `36.0` not `36`), matching Jekyll's template output.
fn format_sexagesimal_float(f: f64) -> String {
    if f.fract() == 0.0 {
        format!("{f:.1}")
    } else {
        format!("{f}")
    }
}

/// Parse a YAML 1.1 sexagesimal (base-60) value to match Ruby/Psych behavior.
///
/// Colon-separated digit groups are interpreted as base-60:
///   `0:36`    -> 0*60 + 36 = 36.0
///   `1:30`    -> 1*60 + 30 = 90.0
///   `1:30:00` -> 1*3600 + 30*60 + 0 = 5400.0
fn parse_sexagesimal(v: &str) -> Option<f64> {
    let parts: Vec<&str> = v.split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    for part in &parts {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }
    let mut result = 0.0f64;
    let mut multiplier = 1.0f64;
    for part in parts.iter().rev() {
        let val: f64 = part.parse().ok()?;
        result += val * multiplier;
        multiplier *= 60.0;
    }
    Some(result)
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
                serde_yaml::Value::Number(serde_yaml::Number::from(f))
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
    // Sexagesimal timestamp handling (Issues 101, 155)
    //
    // YAML 1.1 interprets colon-separated values like 0:30 as sexagesimal
    // (base-60) floats. Ruby's Psych parser does this, so `0:36` becomes
    // 36.0 in Jekyll templates. We match Ruby's behavior to produce
    // identical output and eliminate DOM diffs.
    // ========================================================================

    #[test]
    fn test_sexagesimal_short_timestamp_becomes_float_string() {
        // 0:30 -> "30.0" (string matching Ruby/Jekyll rendering)
        let value = parse_yaml_lenient("time: 0:30").unwrap();
        let mapping = value.as_mapping().unwrap();
        let time = mapping.get("time").unwrap();
        assert_eq!(time.as_str(), Some("30.0"));
    }

    #[test]
    fn test_sexagesimal_zero_timestamp_becomes_float_string() {
        // 0:00 -> "0.0"
        let value = parse_yaml_lenient("time: 0:00").unwrap();
        let mapping = value.as_mapping().unwrap();
        let time = mapping.get("time").unwrap();
        assert_eq!(time.as_str(), Some("0.0"));
    }

    #[test]
    fn test_sexagesimal_hour_minute_second_becomes_float_string() {
        // 1:30:00 -> "5400.0"
        let value = parse_yaml_lenient("time: 1:30:00").unwrap();
        let mapping = value.as_mapping().unwrap();
        let time = mapping.get("time").unwrap();
        assert_eq!(time.as_str(), Some("5400.0"));
    }

    #[test]
    fn test_sexagesimal_various_podcast_timestamps() {
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

        let expected_times = ["0.0", "12.0", "30.0", "41.0"];
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
        // Quoted timestamps should stay as strings (not sexagesimal)
        let value = parse_yaml_lenient("time: '1:05'").unwrap();
        let mapping = value.as_mapping().unwrap();
        assert_eq!(mapping.get("time").unwrap().as_str(), Some("1:05"));
    }

    #[test]
    fn test_sexagesimal_parse_function() {
        assert_eq!(parse_sexagesimal("0:36"), Some(36.0));
        assert_eq!(parse_sexagesimal("0:00"), Some(0.0));
        assert_eq!(parse_sexagesimal("0:01"), Some(1.0));
        assert_eq!(parse_sexagesimal("1:30"), Some(90.0));
        assert_eq!(parse_sexagesimal("1:30:00"), Some(5400.0));
        assert_eq!(parse_sexagesimal("2:30:45"), Some(9045.0));
        // Not sexagesimal
        assert_eq!(parse_sexagesimal("hello"), None);
        assert_eq!(parse_sexagesimal("42"), None);
        assert_eq!(parse_sexagesimal(":30"), None);
        assert_eq!(parse_sexagesimal("0:"), None);
        assert_eq!(parse_sexagesimal("https://example.com"), None);
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
}
