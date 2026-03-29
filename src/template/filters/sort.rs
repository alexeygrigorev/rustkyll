use std::cmp;

use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

use super::jsonify::KEY_ORDER_FIELD;

/// Flatten one level of nested arrays, matching Ruby Liquid's `InputIterator`
/// behavior. When `map: "tags"` returns `[["A","B"],["C"]]`, downstream filters
/// like `uniq`, `sort`, `compact` need to see `["A","B","C"]`.
///
/// Non-array elements are kept as-is. This only flattens one level deep.
pub(crate) fn flatten_one_level(items: impl Iterator<Item = Value>) -> Vec<Value> {
    let mut result = Vec::new();
    for item in items {
        if let Some(arr) = item.as_array() {
            for sub in arr.values() {
                result.push(sub.to_value());
            }
        } else {
            result.push(item);
        }
    }
    result
}

#[derive(Debug, FilterParameters)]
struct SortArgs {
    #[parameter(description = "Optional property name to sort by.", arg_type = "str")]
    property: Option<Expression>,
}

/// Sort filter matching Liquid/Jekyll's stable ordering semantics.
///
/// When sorting an array of objects by a property (e.g., `| sort: 'episode'`),
/// equal sort-key values must preserve their original input order. Liquid's
/// `sort` is stable and does not inject a secondary slug/path tiebreak.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "sort",
    description = "Sorts items in an array while preserving input order for equal values.",
    parameters(SortArgs),
    parsed(SortFilter)
)]
pub struct Sort;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "sort"]
struct SortFilter {
    #[parameters]
    args: SortArgs,
}

/// Compare two Liquid values, treating Nil as less than any non-Nil value.
///
/// When both values can be parsed as numbers (integers or floats), they are
/// compared numerically. This matches Jekyll/Ruby's `<=>` operator behavior
/// where `9 <=> 23` yields -1, not 1 (which string comparison "9" vs "23"
/// would give).
fn nil_safe_compare(a: &dyn ValueView, b: &dyn ValueView) -> cmp::Ordering {
    if a.is_nil() && b.is_nil() {
        cmp::Ordering::Equal
    } else if a.is_nil() {
        cmp::Ordering::Less
    } else if b.is_nil() {
        cmp::Ordering::Greater
    } else {
        // Try numeric comparison first (matches Jekyll/Ruby <=> behavior).
        let a_str = a.to_kstr();
        let b_str = b.to_kstr();
        if let (Ok(a_num), Ok(b_num)) =
            (a_str.as_str().parse::<f64>(), b_str.as_str().parse::<f64>())
        {
            a_num.partial_cmp(&b_num).unwrap_or(cmp::Ordering::Equal)
        } else {
            // Fall back to string comparison for non-numeric values
            a_str.as_str().cmp(b_str.as_str())
        }
    }
}

/// Get a property value from an object, returning Nil if missing.
fn get_property<'a>(value: &'a Value, property: &str) -> &'a dyn ValueView {
    value
        .as_object()
        .and_then(|obj| obj.get(property))
        .unwrap_or(&Value::Nil)
}

impl Filter for SortFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;

        // Jekyll's `sort` on a Hash/Mapping returns an array of [key, value] pairs,
        // sorted by key. This matches Ruby's `Hash#sort` behavior.
        if let Some(obj) = input.as_object() {
            if args.property.is_none() {
                let mut pairs: Vec<Value> = obj
                    .iter()
                    .filter(|(k, _)| k.as_str() != KEY_ORDER_FIELD)
                    .map(|(k, v)| Value::Array(vec![Value::scalar(k.to_string()), v.to_value()]))
                    .collect();
                // Sort by key (first element of each pair)
                pairs.sort_by(|a, b| {
                    let a_key = a.as_array().and_then(|arr| arr.get(0));
                    let b_key = b.as_array().and_then(|arr| arr.get(0));
                    match (a_key, b_key) {
                        (Some(ak), Some(bk)) => nil_safe_compare(ak, bk),
                        _ => cmp::Ordering::Equal,
                    }
                });
                return Ok(Value::Array(pairs));
            }
        }

        let input: Vec<_> = input
            .as_array()
            .map(|arr| arr.values().collect())
            .unwrap_or_default();

        // Flatten one level of nested arrays, matching Ruby Liquid's InputIterator
        // behavior. This is needed for patterns like `map: "tags" | sort` where
        // each element may be a sub-array of tag strings.
        let mut sorted: Vec<Value> = flatten_one_level(input.iter().map(|v| v.to_value()));

        if let Some(property) = &args.property {
            let prop = property.to_kstr();
            // Check if any item actually has this property.
            // If all lookups return Nil (e.g., sort:0 on strings), fall back
            // to sorting by value -- matching Ruby's behavior where
            // Array#sort_by with a non-existent key degrades to value sort.
            let any_has_property = sorted.iter().any(|v| {
                v.as_object()
                    .and_then(|obj| obj.get(prop.as_str()))
                    .map(|val| !val.is_nil())
                    .unwrap_or(false)
            });
            if any_has_property {
                sorted.sort_by(|a, b| {
                    nil_safe_compare(
                        get_property(a, prop.as_str()),
                        get_property(b, prop.as_str()),
                    )
                });
            }
            // When no items have the property, preserve original order
            // (matching Jekyll/Ruby's sort_by behavior with nil keys)
        } else {
            // No property -- sort scalars directly, no tiebreak needed
            sorted.sort_by(|a, b| nil_safe_compare(a, b));
        }

        Ok(Value::Array(sorted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_obj(props: Vec<(&str, Value)>) -> Value {
        let mut o = liquid::Object::new();
        for (k, v) in props {
            o.insert(k.to_owned().into(), v);
        }
        Value::Object(o)
    }

    #[test]
    fn test_sort_by_property_basic() {
        let input = Value::Array(vec![
            make_obj(vec![
                ("episode", Value::scalar(3)),
                ("slug", Value::scalar("charlie")),
            ]),
            make_obj(vec![
                ("episode", Value::scalar(1)),
                ("slug", Value::scalar("alpha")),
            ]),
            make_obj(vec![
                ("episode", Value::scalar(2)),
                ("slug", Value::scalar("bravo")),
            ]),
        ]);
        let result = liquid_core::call_filter!(Sort, input, "episode").unwrap();
        let arr = result.as_array().unwrap();
        // Should be sorted by episode: 1, 2, 3
        let slugs: Vec<_> = arr
            .values()
            .map(|v| {
                v.as_object()
                    .unwrap()
                    .get("slug")
                    .unwrap()
                    .to_kstr()
                    .to_string()
            })
            .collect();
        assert_eq!(slugs, vec!["alpha", "bravo", "charlie"]);
    }

    #[test]
    fn test_sort_preserves_input_order_for_equal_property_values() {
        let input = Value::Array(vec![
            make_obj(vec![
                ("episode", Value::scalar(4)),
                ("slug", Value::scalar("data-translator")),
            ]),
            make_obj(vec![
                ("episode", Value::scalar(4)),
                ("slug", Value::scalar("data-science-interview")),
            ]),
            make_obj(vec![
                ("episode", Value::scalar(3)),
                ("slug", Value::scalar("earlier")),
            ]),
        ]);
        let result = liquid_core::call_filter!(Sort, input, "episode").unwrap();
        let arr = result.as_array().unwrap();
        let slugs: Vec<_> = arr
            .values()
            .map(|v| {
                v.as_object()
                    .unwrap()
                    .get("slug")
                    .unwrap()
                    .to_kstr()
                    .to_string()
            })
            .collect();
        assert_eq!(
            slugs,
            vec!["earlier", "data-translator", "data-science-interview"]
        );
    }

    #[test]
    fn test_sort_preserves_input_order_for_equal_property_values_with_paths() {
        let input = Value::Array(vec![
            make_obj(vec![
                ("episode", Value::scalar(4)),
                ("slug", Value::scalar("same")),
                ("path", Value::scalar("_podcast/zzz.md")),
            ]),
            make_obj(vec![
                ("episode", Value::scalar(4)),
                ("slug", Value::scalar("same")),
                ("path", Value::scalar("_podcast/aaa.md")),
            ]),
        ]);
        let result = liquid_core::call_filter!(Sort, input, "episode").unwrap();
        let arr = result.as_array().unwrap();
        let paths: Vec<_> = arr
            .values()
            .map(|v| {
                v.as_object()
                    .unwrap()
                    .get("path")
                    .unwrap()
                    .to_kstr()
                    .to_string()
            })
            .collect();
        assert_eq!(paths, vec!["_podcast/zzz.md", "_podcast/aaa.md"]);
    }

    #[test]
    fn test_sort_preserves_input_order_for_equal_property_values_existing_case() {
        let input = Value::Array(vec![
            make_obj(vec![
                ("episode", Value::scalar(4)),
                ("slug", Value::scalar("data-translator")),
            ]),
            make_obj(vec![
                ("episode", Value::scalar(4)),
                ("slug", Value::scalar("data-science-interview")),
            ]),
            make_obj(vec![
                ("episode", Value::scalar(3)),
                ("slug", Value::scalar("earlier")),
            ]),
        ]);
        let result = liquid_core::call_filter!(Sort, input, "episode").unwrap();
        let arr = result.as_array().unwrap();
        let slugs: Vec<_> = arr
            .values()
            .map(|v| {
                v.as_object()
                    .unwrap()
                    .get("slug")
                    .unwrap()
                    .to_kstr()
                    .to_string()
            })
            .collect();
        // episode 3 first, then episode 4 items stay in input order
        assert_eq!(
            slugs,
            vec!["earlier", "data-translator", "data-science-interview"]
        );
    }

    #[test]
    fn test_sort_preserves_input_order_when_slugs_equal() {
        let input = Value::Array(vec![
            make_obj(vec![
                ("episode", Value::scalar(4)),
                ("slug", Value::scalar("same")),
                ("path", Value::scalar("_podcast/zzz.md")),
            ]),
            make_obj(vec![
                ("episode", Value::scalar(4)),
                ("slug", Value::scalar("same")),
                ("path", Value::scalar("_podcast/aaa.md")),
            ]),
        ]);
        let result = liquid_core::call_filter!(Sort, input, "episode").unwrap();
        let arr = result.as_array().unwrap();
        let paths: Vec<_> = arr
            .values()
            .map(|v| {
                v.as_object()
                    .unwrap()
                    .get("path")
                    .unwrap()
                    .to_kstr()
                    .to_string()
            })
            .collect();
        assert_eq!(paths, vec!["_podcast/zzz.md", "_podcast/aaa.md"]);
    }

    #[test]
    fn test_sort_scalars_without_property() {
        let input = Value::Array(vec![
            Value::scalar("cherry"),
            Value::scalar("apple"),
            Value::scalar("banana"),
        ]);
        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        let vals: Vec<_> = arr.values().map(|v| v.to_kstr().to_string()).collect();
        assert_eq!(vals, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_sort_empty_array() {
        let input = Value::Array(vec![]);
        let result = liquid_core::call_filter!(Sort, input, "x").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_sort_nil_values_first() {
        let input = Value::Array(vec![
            make_obj(vec![
                ("val", Value::scalar(2)),
                ("slug", Value::scalar("b")),
            ]),
            make_obj(vec![("slug", Value::scalar("a"))]), // no "val" key
            make_obj(vec![
                ("val", Value::scalar(1)),
                ("slug", Value::scalar("c")),
            ]),
        ]);
        let result = liquid_core::call_filter!(Sort, input, "val").unwrap();
        let arr = result.as_array().unwrap();
        let slugs: Vec<_> = arr
            .values()
            .map(|v| {
                v.as_object()
                    .unwrap()
                    .get("slug")
                    .unwrap()
                    .to_kstr()
                    .to_string()
            })
            .collect();
        // nil sorts first, then 1, then 2
        assert_eq!(slugs, vec!["a", "c", "b"]);
    }

    #[test]
    fn test_sort_non_array_input() {
        let input = Value::scalar("not an array");
        let result = liquid_core::call_filter!(Sort, input, "x").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_sort_integers_numerically() {
        // When sorting by a property that has integer values,
        // comparison is done numerically (matching Jekyll/Ruby behavior).
        let input = Value::Array(vec![
            make_obj(vec![
                ("num", Value::scalar(10)),
                ("slug", Value::scalar("ten")),
            ]),
            make_obj(vec![
                ("num", Value::scalar(2)),
                ("slug", Value::scalar("two")),
            ]),
            make_obj(vec![
                ("num", Value::scalar(1)),
                ("slug", Value::scalar("one")),
            ]),
        ]);
        let result = liquid_core::call_filter!(Sort, input, "num").unwrap();
        let arr = result.as_array().unwrap();
        let slugs: Vec<_> = arr
            .values()
            .map(|v| {
                v.as_object()
                    .unwrap()
                    .get("slug")
                    .unwrap()
                    .to_kstr()
                    .to_string()
            })
            .collect();
        // Numeric comparison: 1 < 2 < 10
        assert_eq!(slugs, vec!["one", "two", "ten"]);
    }

    #[test]
    fn test_sort_scalars_numerically() {
        // Sorting bare numeric scalars should also use numeric comparison.
        let input = Value::Array(vec![Value::scalar(23), Value::scalar(9), Value::scalar(3)]);
        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        let vals: Vec<_> = arr.values().map(|v| v.to_kstr().to_string()).collect();
        // Numeric: 3 < 9 < 23 (not string: "23" < "3" < "9")
        assert_eq!(vals, vec!["3", "9", "23"]);
    }

    #[test]
    fn test_sort_object_returns_key_value_pairs() {
        // Jekyll: `hash | sort` returns [[key, value], ...] sorted by key.
        let mut obj = liquid::Object::new();
        obj.insert("es".to_owned().into(), Value::scalar("Spanish"));
        obj.insert("ar".to_owned().into(), Value::scalar("Arabic"));
        obj.insert("en".to_owned().into(), Value::scalar("English"));
        let input = Value::Object(obj);

        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 3);

        // Each element is a [key, value] array, sorted alphabetically by key.
        let pairs: Vec<(String, String)> = arr
            .values()
            .map(|v| {
                let pair = v.as_array().unwrap();
                let key = pair.get(0).unwrap().to_kstr().to_string();
                let val = pair.get(1).unwrap().to_kstr().to_string();
                (key, val)
            })
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("ar".to_string(), "Arabic".to_string()),
                ("en".to_string(), "English".to_string()),
                ("es".to_string(), "Spanish".to_string()),
            ]
        );
    }

    #[test]
    fn test_sort_object_index_access_returns_key() {
        // After sorting, `sorted[0][0]` should be the first key alphabetically.
        let mut obj = liquid::Object::new();
        obj.insert("es".to_owned().into(), Value::scalar("Spanish"));
        obj.insert("ar".to_owned().into(), Value::scalar("Arabic"));
        obj.insert("en".to_owned().into(), Value::scalar("English"));
        let input = Value::Object(obj);

        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();

        // First pair should be "ar" (alphabetically first)
        let first_pair = arr.get(0).unwrap();
        let first_key = first_pair.as_array().unwrap().get(0).unwrap();
        assert_eq!(first_key.to_kstr().as_str(), "ar");
    }

    #[test]
    fn test_sort_object_with_28_locale_keys() {
        // Simulates the opensource-guide _data/locales/ with 28 locale keys.
        let locale_keys = vec![
            "ar", "bg", "bn", "de", "el", "en", "es", "fa", "fr", "he", "hi", "hu", "id", "it",
            "ja", "ko", "mr", "nl", "pl", "pt-br", "ro", "ru", "sw", "th", "tr", "uk", "zh-hans",
            "zh-hant",
        ];
        let mut obj = liquid::Object::new();
        for key in &locale_keys {
            let mut inner = liquid::Object::new();
            inner.insert(
                "name".to_owned().into(),
                Value::scalar(format!("Locale {key}")),
            );
            obj.insert(key.to_string().into(), Value::Object(inner));
        }
        let input = Value::Object(obj);

        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 28);

        // Verify the keys are alphabetically sorted
        let keys: Vec<String> = arr
            .values()
            .map(|v| v.as_array().unwrap().get(0).unwrap().to_kstr().to_string())
            .collect();
        let mut expected = locale_keys
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(keys, expected);
    }

    #[test]
    fn test_sort_object_unicode_values() {
        // Test with non-ASCII locale names (Arabic, Chinese, Hindi).
        let mut obj = liquid::Object::new();
        let mut ar_obj = liquid::Object::new();
        ar_obj.insert("name".to_owned().into(), Value::scalar("العربية"));
        obj.insert("ar".to_owned().into(), Value::Object(ar_obj));

        let mut zh_obj = liquid::Object::new();
        zh_obj.insert("name".to_owned().into(), Value::scalar("简体中文"));
        obj.insert("zh-hans".to_owned().into(), Value::Object(zh_obj));

        let mut hi_obj = liquid::Object::new();
        hi_obj.insert("name".to_owned().into(), Value::scalar("हिन्दी"));
        obj.insert("hi".to_owned().into(), Value::Object(hi_obj));

        let input = Value::Object(obj);
        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 3);

        // Sorted by key: ar, hi, zh-hans
        let keys: Vec<String> = arr
            .values()
            .map(|v| v.as_array().unwrap().get(0).unwrap().to_kstr().to_string())
            .collect();
        assert_eq!(keys, vec!["ar", "hi", "zh-hans"]);

        // Verify unicode values are preserved
        let first_value = arr
            .get(0)
            .unwrap()
            .as_array()
            .unwrap()
            .get(1)
            .unwrap()
            .to_value();
        let ar_name = first_value
            .as_object()
            .unwrap()
            .get("name")
            .unwrap()
            .to_kstr()
            .to_string();
        assert_eq!(ar_name, "العربية");
    }

    #[test]
    fn test_sort_object_empty() {
        let obj = liquid::Object::new();
        let input = Value::Object(obj);
        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_sort_object_single_key() {
        let mut obj = liquid::Object::new();
        obj.insert("only".to_owned().into(), Value::scalar("value"));
        let input = Value::Object(obj);
        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);

        let pair = arr.get(0).unwrap().as_array().unwrap();
        assert_eq!(pair.get(0).unwrap().to_kstr().as_str(), "only");
        assert_eq!(pair.get(1).unwrap().to_kstr().as_str(), "value");
    }

    #[test]
    fn test_sort_object_with_nested_objects() {
        // Values can be complex objects, not just scalars.
        let mut obj = liquid::Object::new();
        let mut nested = liquid::Object::new();
        nested.insert("x".to_owned().into(), Value::scalar(42));
        obj.insert("beta".to_owned().into(), Value::Object(nested));
        obj.insert("alpha".to_owned().into(), Value::scalar("simple"));
        let input = Value::Object(obj);

        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 2);

        // First pair: alpha
        let first_key = arr
            .get(0)
            .unwrap()
            .as_array()
            .unwrap()
            .get(0)
            .unwrap()
            .to_kstr()
            .to_string();
        assert_eq!(first_key, "alpha");

        // Second pair: beta (with nested object preserved)
        let second_val = arr
            .get(1)
            .unwrap()
            .as_array()
            .unwrap()
            .get(1)
            .unwrap()
            .to_value();
        let x = second_val.as_object().unwrap().get("x").unwrap();
        assert_eq!(x.to_kstr().as_str(), "42");
    }

    #[test]
    fn test_sort_object_size_preserved() {
        // .size on the sorted result should match the original object key count.
        let mut obj = liquid::Object::new();
        for i in 0..5 {
            obj.insert(format!("key{i}").into(), Value::scalar(i));
        }
        let input = Value::Object(obj);
        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 5);
    }

    #[test]
    fn test_sort_object_unicode_keys() {
        // Keys themselves can contain non-ASCII characters.
        let mut obj = liquid::Object::new();
        obj.insert("日本語".to_owned().into(), Value::scalar("Japanese"));
        obj.insert("中文".to_owned().into(), Value::scalar("Chinese"));
        obj.insert("한국어".to_owned().into(), Value::scalar("Korean"));
        let input = Value::Object(obj);

        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 3);

        // Keys sorted by Unicode codepoint order
        let keys: Vec<String> = arr
            .values()
            .map(|v| v.as_array().unwrap().get(0).unwrap().to_kstr().to_string())
            .collect();
        let mut expected = vec!["日本語", "中文", "한국어"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(keys, expected);
    }

    #[test]
    fn test_sort_object_excludes_key_order_metadata() {
        // Objects with __key_order metadata should not leak it as a real key
        // when sorted. This caused phantom languages in opensource-guide.
        let mut obj = liquid::Object::new();
        obj.insert("es".to_owned().into(), Value::scalar("Spanish"));
        obj.insert("ar".to_owned().into(), Value::scalar("Arabic"));
        obj.insert("en".to_owned().into(), Value::scalar("English"));
        obj.insert(
            KEY_ORDER_FIELD.to_owned().into(),
            Value::Array(vec![
                Value::scalar("ar"),
                Value::scalar("en"),
                Value::scalar("es"),
            ]),
        );
        let input = Value::Object(obj);

        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        // Should have 3 pairs, not 4 (no __key_order)
        assert_eq!(arr.size(), 3);

        let keys: Vec<String> = arr
            .values()
            .map(|v| v.as_array().unwrap().get(0).unwrap().to_kstr().to_string())
            .collect();
        assert_eq!(keys, vec!["ar", "en", "es"]);
        assert!(
            !keys.contains(&KEY_ORDER_FIELD.to_string()),
            "__key_order should not appear in sorted output"
        );
    }

    #[test]
    fn test_sort_object_with_key_order_unicode_locales() {
        // Simulates the opensource-guide locales with __key_order metadata.
        let mut obj = liquid::Object::new();
        let locale_keys = vec!["ar", "en", "zh-hans", "\u{65E5}\u{672C}\u{8A9E}"];
        for key in &locale_keys {
            obj.insert(
                key.to_string().into(),
                Value::scalar(format!("Locale {key}")),
            );
        }
        obj.insert(
            KEY_ORDER_FIELD.to_owned().into(),
            Value::Array(locale_keys.iter().map(|k| Value::scalar(*k)).collect()),
        );
        let input = Value::Object(obj);

        let result = liquid_core::call_filter!(Sort, input).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 4, "Should have 4 locales, not 5");

        let keys: Vec<String> = arr
            .values()
            .map(|v| v.as_array().unwrap().get(0).unwrap().to_kstr().to_string())
            .collect();
        assert!(
            !keys.contains(&KEY_ORDER_FIELD.to_string()),
            "__key_order must not leak into sorted locale list"
        );
    }
}
