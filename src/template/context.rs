use std::collections::HashSet;

use liquid::model::Value as LiquidValue;
use liquid::Object;

/// Convert a `serde_yaml::Value` to a `liquid::model::Value`.
///
/// Recursively converts all YAML types to their Liquid equivalents:
/// - String -> Scalar (string)
/// - Integer -> Scalar (integer)
/// - Float -> Scalar (float)
/// - Bool -> Scalar (bool)
/// - Null -> Nil
/// - Sequence -> Array
/// - Mapping -> Object
///
/// Note: date-only string expansion (`YYYY-MM-DD` -> `YYYY-MM-DD 00:00:00 +0000`)
/// is only applied to the `date` key in mappings (see `yaml_mapping_to_object`),
/// matching Jekyll's behavior where only the special `date` field is converted
/// to a Ruby Time object. Other date-like strings (e.g., `dateadded`, `start`)
/// remain as plain date strings since Ruby YAML parses them as Date objects
/// whose `to_s` returns just `YYYY-MM-DD`.
pub fn yaml_to_liquid(yaml: &serde_yaml::Value) -> LiquidValue {
    match yaml {
        serde_yaml::Value::String(s) => LiquidValue::scalar(s.clone()),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                LiquidValue::scalar(i)
            } else if let Some(f) = n.as_f64() {
                LiquidValue::scalar(f)
            } else {
                // Fallback: render number as string
                LiquidValue::scalar(n.to_string())
            }
        }
        serde_yaml::Value::Bool(b) => LiquidValue::scalar(*b),
        serde_yaml::Value::Null => LiquidValue::Nil,
        serde_yaml::Value::Sequence(seq) => {
            let arr: Vec<LiquidValue> = seq.iter().map(yaml_to_liquid).collect();
            LiquidValue::Array(arr)
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = yaml_mapping_to_object(map);
            let key_order: Vec<LiquidValue> = map
                .keys()
                .filter_map(|k| {
                    if let serde_yaml::Value::String(s) = k {
                        Some(LiquidValue::scalar(s.clone()))
                    } else {
                        None
                    }
                })
                .collect();
            if !key_order.is_empty() {
                obj.insert(
                    crate::template::filters::jsonify::KEY_ORDER_FIELD.into(),
                    LiquidValue::Array(key_order),
                );
            }
            LiquidValue::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => {
            // Strip the tag and convert the inner value
            yaml_to_liquid(&tagged.value)
        }
    }
}

/// Expand a date-only string to full datetime format to match Jekyll behavior.
///
/// In Ruby YAML (used by Jekyll), bare date values like `2025-11-07` are parsed
/// as Time objects and render as `"2025-11-07 00:00:00 +0100"` (with local timezone).
/// In our Rust YAML parser, they remain plain strings. This function detects
/// date-only strings matching `YYYY-MM-DD` and expands them to
/// `"YYYY-MM-DD 00:00:00 +0000"` (using UTC since we don't track the build timezone).
///
/// Strings that don't match the exact `YYYY-MM-DD` pattern are returned unchanged.
#[cfg(test)]
fn expand_date_only_string(s: &str) -> String {
    expand_date_only_string_with_tz(s, None)
}

/// Normalize an ISO 8601 / RFC 3339 datetime string to Jekyll's format.
///
/// Jekyll renders dates with timezone as `"YYYY-MM-DD HH:MM:SS +HHMM"` (space-separated,
/// no colon in the offset). This function converts formats like:
/// - `"2024-11-23T20:16:52+08:00"` -> `"2024-11-23 20:16:52 +0800"`
/// - `"2024-01-15T14:30:00Z"` -> `"2024-01-15 14:30:00 +0000"`
/// - `"2024-06-15T10:00:00-05:00"` -> `"2024-06-15 10:00:00 -0500"`
///
/// Returns `None` if the string doesn't match a recognized ISO 8601 format with timezone.
fn normalize_iso8601_date_with_tz(s: &str) -> Option<String> {
    // Try RFC 3339 (e.g. "2024-11-23T20:16:52+08:00" or "2024-01-15T14:30:00Z")
    let dt = chrono::DateTime::parse_from_rfc3339(s).ok()?;
    let total_secs = dt.offset().local_minus_utc();
    let sign = if total_secs >= 0 { '+' } else { '-' };
    let abs_secs = total_secs.unsigned_abs();
    let hours = abs_secs / 3600;
    let minutes = (abs_secs % 3600) / 60;
    Some(format!(
        "{} {sign}{hours:02}{minutes:02}",
        dt.format("%Y-%m-%d %H:%M:%S")
    ))
}

pub(crate) fn expand_date_only_string_with_tz(s: &str, site_tz: Option<chrono_tz::Tz>) -> String {
    // Try to parse as a date or date+time that needs expansion to full datetime.
    // Already-complete datetimes (with timezone offset) pass through unchanged.

    // First, try to parse a NaiveDateTime from various partial formats.
    // If successful, format it as "YYYY-MM-DD HH:MM:SS +HHMM".
    let parsed_dt = None
        // YYYY-MM-DD (date only, 10 chars)
        .or_else(|| {
            if s.len() == 10 {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
            } else {
                None
            }
        })
        // YYYY/MM/DD (date only with slashes, 10 chars)
        .or_else(|| {
            if s.len() == 10 {
                chrono::NaiveDate::parse_from_str(s, "%Y/%m/%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
            } else {
                None
            }
        })
        // YYYY/MM/DD HH:MM (slash date with time, no seconds)
        .or_else(|| chrono::NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M").ok())
        // YYYY-MM-DD HH:MM (dash date with time, no seconds -- must NOT match
        // YYYY-MM-DD HH:MM:SS which is already a full datetime without tz)
        .or_else(|| {
            // Only match if there are exactly 16 chars (YYYY-MM-DD HH:MM)
            // to avoid matching "YYYY-MM-DD HH:MM:SS" or "YYYY-MM-DD HH:MM:SS +HHMM"
            if s.len() == 16 {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").ok()
            } else {
                None
            }
        });

    if let Some(dt) = parsed_dt {
        let date_str = dt.format("%Y-%m-%d %H:%M:%S").to_string();
        if let Some(tz) = site_tz {
            use chrono::TimeZone;
            if let chrono::LocalResult::Single(loc) | chrono::LocalResult::Ambiguous(loc, _) =
                tz.from_local_datetime(&dt)
            {
                use chrono::Offset;
                let offset = loc.offset().fix();
                let total_secs = offset.local_minus_utc();
                let sign = if total_secs >= 0 { '+' } else { '-' };
                let abs_secs = total_secs.unsigned_abs();
                let hours = abs_secs / 3600;
                let minutes = (abs_secs % 3600) / 60;
                return format!("{date_str} {sign}{hours:02}{minutes:02}");
            }
        }
        format!("{date_str} +0000")
    } else {
        // Try to normalize ISO 8601 / RFC 3339 dates that already have timezone info.
        // Jekyll renders these as "YYYY-MM-DD HH:MM:SS +HHMM" (space-separated, no colon).
        // e.g. "2024-11-23T20:16:52+08:00" -> "2024-11-23 20:16:52 +0800"
        if let Some(normalized) = normalize_iso8601_date_with_tz(s) {
            return normalized;
        }
        s.to_string()
    }
}

/// Convert a `serde_yaml::Mapping` to a `liquid::Object`.
///
/// Keys are converted to strings (non-string keys use their debug representation).
/// Values are recursively converted via `yaml_to_liquid`.
///
/// The special `date` key gets date-only string expansion (`YYYY-MM-DD` ->
/// `YYYY-MM-DD 00:00:00 +0000`) to match Jekyll's behavior, where the `date`
/// frontmatter field is converted to a Ruby Time object. Other date-like fields
/// (e.g., `dateadded`, `start`) remain as plain `YYYY-MM-DD` strings, matching
/// Ruby's Date#to_s behavior.
pub fn yaml_mapping_to_object(mapping: &serde_yaml::Mapping) -> Object {
    yaml_mapping_to_object_with_tz(mapping, None)
}

pub fn yaml_mapping_to_object_with_tz(
    mapping: &serde_yaml::Mapping,
    site_tz: Option<chrono_tz::Tz>,
) -> Object {
    let mut obj = Object::new();
    for (key, value) in mapping {
        let key_str = match key {
            serde_yaml::Value::String(s) => s.clone(),
            other => format!("{other:?}"),
        };
        let liquid_value = if key_str == "date" {
            if let serde_yaml::Value::String(s) = value {
                LiquidValue::scalar(expand_date_only_string_with_tz(s, site_tz))
            } else {
                yaml_to_liquid(value)
            }
        } else {
            yaml_to_liquid(value)
        };
        obj.insert(key_str.into(), liquid_value);
    }
    obj
}

/// Normalize the `date` field in a front matter HashMap using the site timezone.
///
/// This function applies `expand_date_only_string_with_tz` to the `date` key
/// in the front matter, converting partial date formats like `YYYY/MM/DD HH:MM`
/// to the full `YYYY-MM-DD HH:MM:SS +HHMM` format that Jekyll produces when
/// rendering `page.date` in templates.
///
/// This must be called before passing front matter to the render context,
/// because `yaml_to_liquid` (used in `build_render_context_page_only`) does
/// not perform date normalization on individual string values.
pub fn normalize_frontmatter_date(
    front_matter: &mut std::collections::HashMap<String, serde_yaml::Value>,
    site_tz: Option<chrono_tz::Tz>,
) {
    if let Some(serde_yaml::Value::String(date_str)) = front_matter.get("date") {
        let expanded = expand_date_only_string_with_tz(date_str, site_tz);
        if expanded != *date_str {
            front_matter.insert("date".into(), serde_yaml::Value::String(expanded));
        }
    }
}

/// Normalize a Liquid value so that arrays of objects have uniform keys.
///
/// When Liquid iterates over an array of objects in a `{% for %}` loop,
/// accessing a key that exists on some objects but not others causes an
/// "Unknown index" error. This function prevents that by ensuring every
/// object in an array has the same set of keys (the union of all keys
/// across all objects in that array). Missing keys are filled with `Nil`.
///
/// The normalization is recursive: it processes arrays inside objects,
/// arrays inside arrays, and objects inside arrays at all nesting levels.
pub fn normalize_arrays(value: LiquidValue) -> LiquidValue {
    match value {
        LiquidValue::Array(arr) => {
            // First, recursively normalize all elements
            let arr: Vec<LiquidValue> = arr.into_iter().map(normalize_arrays).collect();

            // Check if this is an array of objects
            let has_objects = arr.iter().any(|v| matches!(v, LiquidValue::Object(_)));
            if !has_objects {
                return LiquidValue::Array(arr);
            }

            // Collect the union of all keys across all objects in the array
            let mut all_keys = HashSet::new();
            for item in &arr {
                if let LiquidValue::Object(obj) = item {
                    for (key, _) in obj.iter() {
                        all_keys.insert(key.to_string());
                    }
                }
            }

            // Pad each object with Nil for any missing keys
            let arr = arr
                .into_iter()
                .map(|item| {
                    if let LiquidValue::Object(mut obj) = item {
                        for key in &all_keys {
                            if obj.get(key.as_str()).is_none() {
                                obj.insert(key.clone().into(), LiquidValue::Nil);
                            }
                        }
                        LiquidValue::Object(obj)
                    } else {
                        item
                    }
                })
                .collect();

            LiquidValue::Array(arr)
        }
        LiquidValue::Object(obj) => {
            let mut new_obj = Object::new();
            for (key, val) in obj.into_iter() {
                new_obj.insert(key, normalize_arrays(val));
            }
            LiquidValue::Object(new_obj)
        }
        other => other,
    }
}

/// Build a Liquid `Object` with `page` and `site` namespaces from YAML data.
///
/// This is a convenience helper for constructing a template context where
/// page-level front matter is accessible as `page.title`, `page.layout`, etc.
/// and site-level data is accessible as `site.title`, `site.data`, etc.
pub fn build_context(
    page_yaml: Option<&serde_yaml::Mapping>,
    site_yaml: Option<&serde_yaml::Mapping>,
) -> Object {
    let mut ctx = Object::new();

    if let Some(page) = page_yaml {
        ctx.insert(
            "page".into(),
            LiquidValue::Object(yaml_mapping_to_object(page)),
        );
    }

    if let Some(site) = site_yaml {
        ctx.insert(
            "site".into(),
            LiquidValue::Object(yaml_mapping_to_object(site)),
        );
    }

    ctx
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value as YamlValue;

    #[test]
    fn test_convert_string() {
        let yaml = YamlValue::String("hello".into());
        let liquid = yaml_to_liquid(&yaml);
        assert_eq!(liquid, LiquidValue::scalar("hello"));
    }

    #[test]
    fn test_convert_integer() {
        let yaml: YamlValue = serde_yaml::from_str("42").unwrap();
        let liquid = yaml_to_liquid(&yaml);
        assert_eq!(liquid, LiquidValue::scalar(42i64));
    }

    #[test]
    fn test_convert_float() {
        let yaml: YamlValue = serde_yaml::from_str("1.23").unwrap();
        let liquid = yaml_to_liquid(&yaml);
        assert_eq!(liquid, LiquidValue::scalar(1.23f64));
    }

    #[test]
    fn test_convert_bool() {
        let yaml = YamlValue::Bool(true);
        let liquid = yaml_to_liquid(&yaml);
        assert_eq!(liquid, LiquidValue::scalar(true));
    }

    #[test]
    fn test_convert_null() {
        let yaml = YamlValue::Null;
        let liquid = yaml_to_liquid(&yaml);
        assert_eq!(liquid, LiquidValue::Nil);
    }

    #[test]
    fn test_convert_sequence() {
        let yaml: YamlValue = serde_yaml::from_str("[1, 2, 3]").unwrap();
        let liquid = yaml_to_liquid(&yaml);
        let expected = LiquidValue::Array(vec![
            LiquidValue::scalar(1i64),
            LiquidValue::scalar(2i64),
            LiquidValue::scalar(3i64),
        ]);
        assert_eq!(liquid, expected);
    }

    #[test]
    fn test_convert_mapping() {
        let yaml: YamlValue = serde_yaml::from_str("title: Hello\ncount: 5").unwrap();
        let liquid = yaml_to_liquid(&yaml);
        if let LiquidValue::Object(obj) = liquid {
            assert_eq!(obj.get("title"), Some(&LiquidValue::scalar("Hello")));
            assert_eq!(obj.get("count"), Some(&LiquidValue::scalar(5i64)));
        } else {
            panic!("Expected Object, got {:?}", liquid);
        }
    }

    #[test]
    fn test_convert_nested_structure() {
        let yaml_str = r#"
page:
  title: "Test"
  tags:
    - rust
    - web
  meta:
    author: "Alice"
    count: 10
"#;
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let liquid = yaml_to_liquid(&yaml);
        if let LiquidValue::Object(root) = &liquid {
            if let Some(LiquidValue::Object(page)) = root.get("page") {
                assert_eq!(page.get("title"), Some(&LiquidValue::scalar("Test")));
                if let Some(LiquidValue::Array(tags)) = page.get("tags") {
                    assert_eq!(tags.len(), 2);
                    assert_eq!(tags[0], LiquidValue::scalar("rust"));
                } else {
                    panic!("Expected tags array");
                }
                if let Some(LiquidValue::Object(meta)) = page.get("meta") {
                    assert_eq!(meta.get("author"), Some(&LiquidValue::scalar("Alice")));
                    assert_eq!(meta.get("count"), Some(&LiquidValue::scalar(10i64)));
                } else {
                    panic!("Expected meta object");
                }
            } else {
                panic!("Expected page object");
            }
        } else {
            panic!("Expected root Object");
        }
    }

    #[test]
    fn test_yaml_mapping_to_object() {
        let yaml: YamlValue = serde_yaml::from_str("a: 1\nb: hello").unwrap();
        if let YamlValue::Mapping(mapping) = yaml {
            let obj = yaml_mapping_to_object(&mapping);
            assert_eq!(obj.get("a"), Some(&LiquidValue::scalar(1i64)));
            assert_eq!(obj.get("b"), Some(&LiquidValue::scalar("hello")));
        } else {
            panic!("Expected Mapping");
        }
    }

    #[test]
    fn test_round_trip_yaml_to_template() {
        // Parse YAML front matter, convert to liquid context, render template
        let yaml_str = "title: My Page\nauthor: Alice\ncount: 42";
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let mapping = yaml.as_mapping().unwrap();

        let mut ctx = Object::new();
        ctx.insert(
            "page".into(),
            LiquidValue::Object(yaml_mapping_to_object(mapping)),
        );

        let engine = crate::template::TemplateEngine::new().unwrap();
        let output = engine
            .parse_and_render(
                "{{ page.title }} by {{ page.author }} ({{ page.count }})",
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "My Page by Alice (42)");
    }

    #[test]
    fn test_build_context_with_page_and_site() {
        let page_yaml: YamlValue = serde_yaml::from_str("title: Hello\nlayout: post").unwrap();
        let site_yaml: YamlValue =
            serde_yaml::from_str("name: MySite\nurl: https://example.com").unwrap();

        let ctx = build_context(page_yaml.as_mapping(), site_yaml.as_mapping());

        if let Some(LiquidValue::Object(page)) = ctx.get("page") {
            assert_eq!(page.get("title"), Some(&LiquidValue::scalar("Hello")));
        } else {
            panic!("Expected page object");
        }

        if let Some(LiquidValue::Object(site)) = ctx.get("site") {
            assert_eq!(site.get("name"), Some(&LiquidValue::scalar("MySite")));
        } else {
            panic!("Expected site object");
        }
    }

    // ========================================================================
    // normalize_arrays tests
    // ========================================================================

    #[test]
    fn test_normalize_arrays_pads_missing_keys() {
        let arr = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("name".into(), LiquidValue::scalar("Alice"));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("name".into(), LiquidValue::scalar("Bob"));
                o.insert("role".into(), LiquidValue::scalar("Engineer"));
                o
            }),
        ]);

        let normalized = normalize_arrays(arr);
        if let LiquidValue::Array(items) = normalized {
            // Alice's object should now have "role" = Nil
            if let LiquidValue::Object(ref alice) = items[0] {
                assert_eq!(alice.get("name"), Some(&LiquidValue::scalar("Alice")));
                assert_eq!(alice.get("role"), Some(&LiquidValue::Nil));
            } else {
                panic!("Expected Object for Alice");
            }
            // Bob's object should be unchanged
            if let LiquidValue::Object(ref bob) = items[1] {
                assert_eq!(bob.get("name"), Some(&LiquidValue::scalar("Bob")));
                assert_eq!(bob.get("role"), Some(&LiquidValue::scalar("Engineer")));
            } else {
                panic!("Expected Object for Bob");
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_normalize_arrays_recursive() {
        // Object containing an array of objects
        let inner_arr = LiquidValue::Array(vec![
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("x".into(), LiquidValue::scalar(1i64));
                o
            }),
            LiquidValue::Object({
                let mut o = Object::new();
                o.insert("x".into(), LiquidValue::scalar(2i64));
                o.insert("y".into(), LiquidValue::scalar(3i64));
                o
            }),
        ]);

        let obj = LiquidValue::Object({
            let mut o = Object::new();
            o.insert("items".into(), inner_arr);
            o
        });

        let normalized = normalize_arrays(obj);
        if let LiquidValue::Object(ref root) = normalized {
            if let Some(LiquidValue::Array(ref items)) = root.get("items") {
                if let LiquidValue::Object(ref first) = items[0] {
                    assert_eq!(first.get("y"), Some(&LiquidValue::Nil));
                } else {
                    panic!("Expected Object");
                }
            } else {
                panic!("Expected Array");
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_normalize_arrays_leaves_scalars_unchanged() {
        let scalar = LiquidValue::scalar("hello");
        assert_eq!(normalize_arrays(scalar.clone()), scalar);
    }

    #[test]
    fn test_normalize_arrays_leaves_non_object_arrays_unchanged() {
        let arr = LiquidValue::Array(vec![LiquidValue::scalar(1i64), LiquidValue::scalar(2i64)]);
        let normalized = normalize_arrays(arr.clone());
        assert_eq!(normalized, arr);
    }

    // ========================================================================
    // Sexagesimal timestamp handling (Issues 101, 155, 161)
    //
    // Unquoted colon-separated values like 0:30 are kept as human-readable
    // strings (e.g. "0:30") instead of being converted to floats like Jekyll.
    // This is an intentional improvement over Jekyll.
    // ========================================================================

    #[test]
    fn test_sexagesimal_timestamp_stays_human_readable_in_liquid() {
        let yaml_str = r#"
transcript:
  - time: 0:30
    sec: 30
    who: Alexey
  - time: '1:05'
    sec: 65
    who: Guest
"#;
        let yaml: YamlValue = crate::yaml::parse_yaml_lenient(yaml_str).unwrap();
        let liquid = yaml_to_liquid(&yaml);

        if let LiquidValue::Object(root) = &liquid {
            if let Some(LiquidValue::Array(transcript)) = root.get("transcript") {
                // First entry: unquoted 0:30 stays as "0:30" (human-readable)
                if let LiquidValue::Object(ref line) = transcript[0] {
                    assert_eq!(
                        line.get("time"),
                        Some(&LiquidValue::scalar("0:30")),
                        "Unquoted 0:30 should stay as '0:30' in Liquid context"
                    );
                    assert_eq!(line.get("sec"), Some(&LiquidValue::scalar(30i64)));
                } else {
                    panic!("Expected Object for transcript[0]");
                }

                // Second entry: quoted '1:05' stays as string
                if let LiquidValue::Object(ref line) = transcript[1] {
                    assert_eq!(
                        line.get("time"),
                        Some(&LiquidValue::scalar("1:05")),
                        "Quoted '1:05' should be string '1:05' in Liquid context"
                    );
                    assert_eq!(line.get("sec"), Some(&LiquidValue::scalar(65i64)));
                } else {
                    panic!("Expected Object for transcript[1]");
                }
            } else {
                panic!("Expected transcript array");
            }
        } else {
            panic!("Expected root Object");
        }
    }

    // ========================================================================
    // Date-only string expansion (Issue 138)
    // ========================================================================

    #[test]
    fn test_expand_date_only_string_basic() {
        assert_eq!(
            expand_date_only_string("2025-11-07"),
            "2025-11-07 00:00:00 +0000"
        );
    }

    #[test]
    fn test_expand_date_only_string_leaves_full_datetime() {
        // Already has time component -- should not be modified
        assert_eq!(
            expand_date_only_string("2025-11-07 00:00:00 +0000"),
            "2025-11-07 00:00:00 +0000"
        );
    }

    #[test]
    fn test_expand_date_only_string_leaves_non_date() {
        assert_eq!(expand_date_only_string("hello"), "hello");
        assert_eq!(expand_date_only_string(""), "");
        assert_eq!(expand_date_only_string("12345"), "12345");
        assert_eq!(expand_date_only_string("not-a-date"), "not-a-date");
    }

    #[test]
    fn test_yaml_to_liquid_only_expands_date_key() {
        // Only the special "date" key should be expanded to full datetime.
        // Other date-like fields (e.g., "dateadded") should remain as plain
        // YYYY-MM-DD strings, matching Jekyll's behavior where only the `date`
        // field is converted to a Ruby Time object.
        let yaml_str = r#"
date: 2025-11-07
dateadded: 2022-02-27
title: Not a date
"#;
        let yaml: YamlValue = crate::yaml::parse_yaml_lenient(yaml_str).unwrap();
        let liquid = yaml_to_liquid(&yaml);

        if let LiquidValue::Object(obj) = &liquid {
            assert_eq!(
                obj.get("date"),
                Some(&LiquidValue::scalar("2025-11-07 00:00:00 +0000")),
                "date key should be expanded to full datetime"
            );
            assert_eq!(
                obj.get("dateadded"),
                Some(&LiquidValue::scalar("2022-02-27")),
                "dateadded should remain as plain date string (Ruby Date#to_s)"
            );
            assert_eq!(
                obj.get("title"),
                Some(&LiquidValue::scalar("Not a date")),
                "Non-date strings should remain unchanged"
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_date_expansion_only_for_date_key_in_template() {
        // Verify that only the "date" key renders as full datetime,
        // while other date-like fields render as plain date strings.
        let yaml_str = "date: 2025-11-07\ndateadded: 2022-02-27";
        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let mapping = yaml.as_mapping().unwrap();

        let mut ctx = Object::new();
        ctx.insert(
            "page".into(),
            LiquidValue::Object(yaml_mapping_to_object(mapping)),
        );

        let engine = crate::template::TemplateEngine::new().unwrap();
        let output = engine
            .parse_and_render("{{ page.date }} / {{ page.dateadded }}", &ctx)
            .unwrap();
        assert_eq!(output, "2025-11-07 00:00:00 +0000 / 2022-02-27");
    }

    // ========================================================================
    // Sexagesimal timestamp rendering (Issues 155, 161)
    // ========================================================================

    // ========================================================================
    // Issue 209: Date normalization for slash-format dates with timezone
    // ========================================================================

    #[test]
    fn test_date_normalization_slash_format_with_timezone() {
        // "2018/06/04 00:00" with Asia/Taipei -> "2018-06-04 00:00:00 +0800"
        let tz: chrono_tz::Tz = "Asia/Taipei".parse().unwrap();
        let result = expand_date_only_string_with_tz("2018/06/04 00:00", Some(tz));
        assert_eq!(result, "2018-06-04 00:00:00 +0800");
    }

    #[test]
    fn test_date_normalization_dash_format_with_time_and_timezone() {
        // "2024-02-23 19:17" with Asia/Taipei -> "2024-02-23 19:17:00 +0800"
        let tz: chrono_tz::Tz = "Asia/Taipei".parse().unwrap();
        let result = expand_date_only_string_with_tz("2024-02-23 19:17", Some(tz));
        assert_eq!(result, "2024-02-23 19:17:00 +0800");
    }

    #[test]
    fn test_date_normalization_slash_format_no_timezone() {
        // "2018/06/04 00:00" with no timezone -> "2018-06-04 00:00:00 +0000"
        let result = expand_date_only_string_with_tz("2018/06/04 00:00", None);
        assert_eq!(result, "2018-06-04 00:00:00 +0000");
    }

    #[test]
    fn test_date_normalization_slash_format_date_only() {
        // "2018/06/04" (no time) with timezone
        let tz: chrono_tz::Tz = "Asia/Taipei".parse().unwrap();
        let result = expand_date_only_string_with_tz("2018/06/04", Some(tz));
        assert_eq!(result, "2018-06-04 00:00:00 +0800");
    }

    #[test]
    fn test_date_normalization_existing_full_datetime_unchanged() {
        // Already-formatted dates should pass through unchanged
        let result = expand_date_only_string_with_tz("2018-06-04 00:00:00 +0800", None);
        assert_eq!(result, "2018-06-04 00:00:00 +0800");
    }

    // ========================================================================
    // Timezone offset format normalization (muan-blog datetime attributes)
    // Jekyll normalizes ISO 8601 dates like "2024-11-23T20:16:52+08:00" to
    // "2024-11-23 20:16:52 +0800" (space-separated, no colon in offset).
    // ========================================================================

    #[test]
    fn test_date_normalization_iso8601_with_colon_tz() {
        // RFC 3339 / ISO 8601 date with colon in timezone offset
        let result = expand_date_only_string_with_tz("2024-11-23T20:16:52+08:00", None);
        assert_eq!(result, "2024-11-23 20:16:52 +0800");
    }

    #[test]
    fn test_date_normalization_iso8601_utc_offset() {
        let result = expand_date_only_string_with_tz("2024-01-15T14:30:00+00:00", None);
        assert_eq!(result, "2024-01-15 14:30:00 +0000");
    }

    #[test]
    fn test_date_normalization_iso8601_negative_offset() {
        let result = expand_date_only_string_with_tz("2024-06-15T10:00:00-05:00", None);
        assert_eq!(result, "2024-06-15 10:00:00 -0500");
    }

    #[test]
    fn test_date_normalization_iso8601_z_suffix() {
        // Z suffix means UTC
        let result = expand_date_only_string_with_tz("2024-01-15T14:30:00Z", None);
        assert_eq!(result, "2024-01-15 14:30:00 +0000");
    }

    // ========================================================================
    // Issue 216: page.date normalization in render context with timezone
    // ========================================================================

    #[test]
    fn test_page_date_normalized_in_render_context_slash_format_with_tz() {
        // When a collection item has front matter `date: 2018/06/04 00:00` and
        // the site timezone is Asia/Taipei, rendering {{ page.date }} in a
        // template must produce "2018-06-04 00:00:00 +0800".
        use crate::template::layout::build_render_context_page_only;
        use std::collections::HashMap;

        let mut page_fm: HashMap<String, serde_yaml::Value> = HashMap::new();
        page_fm.insert(
            "date".into(),
            serde_yaml::Value::String("2018/06/04 00:00".into()),
        );
        page_fm.insert(
            "title".into(),
            serde_yaml::Value::String("Test note".into()),
        );

        // Normalize the date before building render context (simulating what
        // the generator should do)
        let tz: chrono_tz::Tz = "Asia/Taipei".parse().unwrap();
        normalize_frontmatter_date(&mut page_fm, Some(tz));

        let ctx = build_render_context_page_only("", &page_fm);
        let engine = crate::template::TemplateEngine::new().unwrap();
        let output = engine
            .parse_and_render("datetime=\"{{ page.date }}\"", &ctx)
            .unwrap();
        assert_eq!(output, "datetime=\"2018-06-04 00:00:00 +0800\"");
    }

    #[test]
    fn test_page_date_normalized_unicode_title_with_tz() {
        // Non-ASCII variant: German title with slash-format date
        use crate::template::layout::build_render_context_page_only;
        use std::collections::HashMap;

        let mut page_fm: HashMap<String, serde_yaml::Value> = HashMap::new();
        page_fm.insert(
            "date".into(),
            serde_yaml::Value::String("2023/07/11 15:27".into()),
        );
        page_fm.insert(
            "title".into(),
            serde_yaml::Value::String("Buchrezension \u{00fc}ber B\u{00fc}cher".into()),
        );

        let tz: chrono_tz::Tz = "Asia/Taipei".parse().unwrap();
        normalize_frontmatter_date(&mut page_fm, Some(tz));

        let ctx = build_render_context_page_only("", &page_fm);
        let engine = crate::template::TemplateEngine::new().unwrap();
        let output = engine
            .parse_and_render("{{ page.date }} - {{ page.title }}", &ctx)
            .unwrap();
        assert_eq!(
            output,
            "2023-07-11 15:27:00 +0800 - Buchrezension \u{00fc}ber B\u{00fc}cher"
        );
    }

    #[test]
    fn test_normalize_frontmatter_date_function() {
        // Test the normalize function itself
        use std::collections::HashMap;

        let mut fm: HashMap<String, serde_yaml::Value> = HashMap::new();
        fm.insert(
            "date".into(),
            serde_yaml::Value::String("2018/06/04 00:00".into()),
        );
        fm.insert(
            "title".into(),
            serde_yaml::Value::String("Not a date".into()),
        );

        let tz: chrono_tz::Tz = "Asia/Taipei".parse().unwrap();
        normalize_frontmatter_date(&mut fm, Some(tz));

        assert_eq!(
            fm.get("date").unwrap().as_str().unwrap(),
            "2018-06-04 00:00:00 +0800"
        );
        // title should be unchanged
        assert_eq!(fm.get("title").unwrap().as_str().unwrap(), "Not a date");
    }

    #[test]
    fn test_sexagesimal_timestamp_renders_human_readable_in_template() {
        // Unquoted 0:36 stays as "0:36" (human-readable, not "36.0" like Jekyll)
        let yaml_str = "time: 0:36\nsec: 36";
        let yaml: YamlValue = crate::yaml::parse_yaml_lenient(yaml_str).unwrap();
        let mapping = yaml.as_mapping().unwrap();

        let mut ctx = Object::new();
        ctx.insert(
            "line".into(),
            LiquidValue::Object(yaml_mapping_to_object(mapping)),
        );

        let engine = crate::template::TemplateEngine::new().unwrap();
        let output = engine.parse_and_render("{{ line.time }}", &ctx).unwrap();
        // Sexagesimal values are kept as original strings like "0:36"
        assert_eq!(output, "0:36");
    }

    #[test]
    fn test_assign_with_filter_chain() {
        // This pattern is used by just-the-docs: {% assign empty_array = "" | split: "" %}
        let engine = crate::template::TemplateEngine::new().unwrap();
        let ctx = Object::new();
        let output = engine
            .parse_and_render(
                r#"{% assign empty_array = "" | split: "" %}{{ empty_array | size }}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "0");
    }

    #[test]
    fn test_keyword_prefixed_identifiers() {
        // Variables whose names start with liquid keywords (empty, nil, null,
        // blank, true, false) must be parsed as identifiers, not as the keyword
        // literal followed by leftover characters.
        let engine = crate::template::TemplateEngine::new().unwrap();
        let ctx = Object::new();

        // "empty_items" should be an identifier, not "empty" + "_items"
        let output = engine
            .parse_and_render(
                r#"{% assign empty_items = "hello" %}{{ empty_items }}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "hello");

        // "nil_value" should be an identifier, not "nil" + "_value"
        let output = engine
            .parse_and_render(r#"{% assign nil_value = "world" %}{{ nil_value }}"#, &ctx)
            .unwrap();
        assert_eq!(output, "world");

        // "null_check" should be an identifier
        let output = engine
            .parse_and_render(r#"{% assign null_check = "ok" %}{{ null_check }}"#, &ctx)
            .unwrap();
        assert_eq!(output, "ok");

        // "blank_line" should be an identifier
        let output = engine
            .parse_and_render(r#"{% assign blank_line = "test" %}{{ blank_line }}"#, &ctx)
            .unwrap();
        assert_eq!(output, "test");

        // "true_count" should be an identifier
        let output = engine
            .parse_and_render(r#"{% assign true_count = 42 %}{{ true_count }}"#, &ctx)
            .unwrap();
        assert_eq!(output, "42");

        // "false_positive" should be an identifier
        let output = engine
            .parse_and_render(
                r#"{% assign false_positive = "nope" %}{{ false_positive }}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "nope");
    }

    #[test]
    fn test_keyword_literals_still_work() {
        // The actual keyword literals should still work correctly
        let engine = crate::template::TemplateEngine::new().unwrap();
        let ctx = Object::new();

        // "empty" as a value (in comparison) should still work
        let output = engine
            .parse_and_render(
                r#"{% assign arr = "" | split: "" %}{% if arr == empty %}yes{% endif %}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "yes");

        // "true" and "false" should still work as boolean literals
        let output = engine
            .parse_and_render(
                r#"{% assign flag = true %}{% if flag %}on{% endif %}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "on");

        let output = engine
            .parse_and_render(
                r#"{% assign flag = false %}{% if flag %}on{% else %}off{% endif %}"#,
                &ctx,
            )
            .unwrap();
        assert_eq!(output, "off");
    }

    #[test]
    fn test_yaml_to_liquid_stores_key_order_301() {
        use crate::template::filters::jsonify::KEY_ORDER_FIELD;
        use liquid::ValueView;
        let yaml_str =
            "permissions:\n  - read\nconditions:\n  - notice\nlimitations:\n  - liability\n";
        let yaml: serde_yaml::Value = serde_yaml::from_str(yaml_str).unwrap();
        let liquid_val = yaml_to_liquid(&yaml);
        if let LiquidValue::Object(obj) = &liquid_val {
            let key_order = obj.get(KEY_ORDER_FIELD).expect("should have __key_order");
            if let LiquidValue::Array(arr) = key_order {
                let keys: Vec<String> = arr.iter().map(|v| v.to_kstr().to_string()).collect();
                assert_eq!(keys, vec!["permissions", "conditions", "limitations"]);
            } else {
                panic!("__key_order should be an Array");
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_yaml_to_liquid_key_order_unicode_301() {
        use crate::template::filters::jsonify::KEY_ORDER_FIELD;
        use liquid::ValueView;
        let yaml_str = "beschreibung: H\u{00e4}llo\ntitel: W\u{00f6}rld\n";
        let yaml: serde_yaml::Value = serde_yaml::from_str(yaml_str).unwrap();
        let liquid_val = yaml_to_liquid(&yaml);
        if let LiquidValue::Object(obj) = &liquid_val {
            let key_order = obj.get(KEY_ORDER_FIELD).expect("should have __key_order");
            if let LiquidValue::Array(arr) = key_order {
                let keys: Vec<String> = arr.iter().map(|v| v.to_kstr().to_string()).collect();
                assert_eq!(keys, vec!["beschreibung", "titel"]);
            } else {
                panic!("__key_order should be an Array");
            }
        } else {
            panic!("Expected Object");
        }
    }
}
