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
            let obj = yaml_mapping_to_object(map);
            LiquidValue::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => {
            // Strip the tag and convert the inner value
            yaml_to_liquid(&tagged.value)
        }
    }
}

/// Convert a `serde_yaml::Mapping` to a `liquid::Object`.
///
/// Keys are converted to strings (non-string keys use their debug representation).
/// Values are recursively converted via `yaml_to_liquid`.
pub fn yaml_mapping_to_object(mapping: &serde_yaml::Mapping) -> Object {
    let mut obj = Object::new();
    for (key, value) in mapping {
        let key_str = match key {
            serde_yaml::Value::String(s) => s.clone(),
            other => format!("{other:?}"),
        };
        obj.insert(key_str.into(), yaml_to_liquid(value));
    }
    obj
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
}
