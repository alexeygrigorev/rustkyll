use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Convert a Liquid value to its JSON representation.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "jsonify",
    description = "Convert a value to its JSON representation.",
    parsed(JsonifyFilter)
)]
pub struct Jsonify;

#[derive(Debug, Default, Display_filter)]
#[name = "jsonify"]
struct JsonifyFilter;

/// The key used to store YAML insertion order metadata in Liquid objects.
/// This is inserted by `yaml_to_liquid` and consumed by `liquid_to_json_string`
/// to preserve key ordering through the jsonify filter.
pub(crate) const KEY_ORDER_FIELD: &str = "__key_order";

/// Convert a Liquid value directly to a JSON string, preserving key order
/// for objects that have `__key_order` metadata.
///
/// This bypasses `serde_json::Value::Object` (which uses BTreeMap and sorts
/// keys alphabetically) by building the JSON string directly for ordered objects.
fn liquid_to_json_string(value: &Value) -> String {
    match value {
        Value::Scalar(s) => {
            // Use serde serialization to preserve the original scalar type.
            let json_val = serde_json::to_value(s)
                .unwrap_or_else(|_| serde_json::Value::String(s.to_kstr().to_string()));
            serde_json::to_string(&json_val).unwrap_or_else(|_| "null".to_string())
        }
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(liquid_to_json_string).collect();
            format!("[{}]", items.join(","))
        }
        Value::Object(obj) => liquid_object_to_json_string(obj),
        Value::Nil => "null".to_string(),
        Value::State(_) => "null".to_string(),
    }
}

/// Convert a Liquid Object to a JSON string, preserving YAML key order if available.
///
/// If the object has a `__key_order` metadata array (inserted by `yaml_to_liquid`),
/// keys are serialized in that order. The `__key_order` key itself is stripped
/// from the output. Keys not listed in `__key_order` are appended alphabetically.
///
/// If no `__key_order` is present, falls back to alphabetical key order for
/// deterministic output.
fn liquid_object_to_json_string(obj: &liquid::Object) -> String {
    // Check for __key_order metadata
    let key_order: Option<Vec<String>> = obj.get(KEY_ORDER_FIELD).and_then(|v| {
        if let Value::Array(arr) = v {
            Some(arr.iter().map(|item| item.to_kstr().to_string()).collect())
        } else {
            None
        }
    });

    let mut parts: Vec<String> = Vec::new();

    if let Some(ordered_keys) = key_order {
        let mut seen = std::collections::HashSet::new();

        // First, emit keys in the specified order
        for key in &ordered_keys {
            if key == KEY_ORDER_FIELD {
                continue;
            }
            seen.insert(key.clone());
            if let Some(val) = obj.get(key.as_str()) {
                let json_key =
                    serde_json::to_string(key).unwrap_or_else(|_| format!("\"{}\"", key));
                let json_val = liquid_to_json_string(&val.to_value());
                parts.push(format!("{}:{}", json_key, json_val));
            }
        }

        // Then, emit any remaining keys not in __key_order (sorted for determinism)
        let mut remaining: Vec<String> = obj
            .keys()
            .map(|k| k.to_string())
            .filter(|k| k != KEY_ORDER_FIELD && !seen.contains(k))
            .collect();
        remaining.sort();
        for key in &remaining {
            if let Some(val) = obj.get(key.as_str()) {
                let json_key =
                    serde_json::to_string(key).unwrap_or_else(|_| format!("\"{}\"", key));
                let json_val = liquid_to_json_string(&val.to_value());
                parts.push(format!("{}:{}", json_key, json_val));
            }
        }
    } else {
        // No key order metadata -- alphabetical order for determinism
        let mut keys: Vec<String> = obj.keys().map(|k| k.to_string()).collect();
        keys.sort();
        for key in &keys {
            if let Some(val) = obj.get(key.as_str()) {
                let json_key =
                    serde_json::to_string(key).unwrap_or_else(|_| format!("\"{}\"", key));
                let json_val = liquid_to_json_string(&val.to_value());
                parts.push(format!("{}:{}", json_key, json_val));
            }
        }
    }

    format!("{{{}}}", parts.join(","))
}

impl Filter for JsonifyFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let value = input.to_value();
        let json_str = liquid_to_json_string(&value);
        Ok(Value::scalar(json_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonify_string() {
        let result = liquid_core::call_filter!(Jsonify, "hello").unwrap();
        assert_eq!(result.to_kstr(), "\"hello\"");
    }

    #[test]
    fn test_jsonify_string_with_special_chars() {
        let result = liquid_core::call_filter!(Jsonify, "He said \"hi\"").unwrap();
        let s = result.to_kstr();
        assert!(s.contains("\\\""));
        assert!(s.starts_with('"'));
        assert!(s.ends_with('"'));
    }

    #[test]
    fn test_jsonify_integer() {
        let input = Value::scalar(42i64);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        assert_eq!(result.to_kstr(), "42");
    }

    #[test]
    fn test_jsonify_float() {
        let input = Value::scalar(2.750f64);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        assert_eq!(result.to_kstr(), "2.75");
    }

    #[test]
    fn test_jsonify_boolean() {
        let input = Value::scalar(true);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        assert_eq!(result.to_kstr(), "true");
    }

    #[test]
    fn test_jsonify_nil() {
        let input = Value::Nil;
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        assert_eq!(result.to_kstr(), "null");
    }

    #[test]
    fn test_jsonify_array() {
        let input = Value::Array(vec![Value::scalar("a"), Value::scalar("b")]);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        assert_eq!(result.to_kstr(), "[\"a\",\"b\"]");
    }

    #[test]
    fn test_jsonify_object() {
        let mut obj = liquid::Object::new();
        obj.insert("name".into(), Value::scalar("Alice"));
        let input = Value::Object(obj);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        assert_eq!(result.to_kstr(), "{\"name\":\"Alice\"}");
    }

    #[test]
    fn test_jsonify_empty_string() {
        let result = liquid_core::call_filter!(Jsonify, "").unwrap();
        assert_eq!(result.to_kstr(), "\"\"");
    }

    #[test]
    fn test_jsonify_numeric_string_stays_string() {
        let input = Value::scalar("2024");
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        assert_eq!(
            result.to_kstr(),
            "\"2024\"",
            "String '2024' should serialize as JSON string, not number"
        );
    }

    #[test]
    fn test_jsonify_array_with_numeric_string() {
        let input = Value::Array(vec![
            Value::scalar("survey"),
            Value::scalar("ai"),
            Value::scalar("2024"),
        ]);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        assert_eq!(
            result.to_kstr(),
            "[\"survey\",\"ai\",\"2024\"]",
            "All tags should be JSON strings, including numeric-looking ones"
        );
    }

    #[test]
    fn test_jsonify_actual_integer_stays_number() {
        let input = Value::scalar(2024i64);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        assert_eq!(result.to_kstr(), "2024");
    }

    #[test]
    fn test_jsonify_preserves_yaml_key_order() {
        let mut obj = liquid::Object::new();
        obj.insert("permissions".into(), Value::scalar("p"));
        obj.insert("conditions".into(), Value::scalar("c"));
        obj.insert("limitations".into(), Value::scalar("l"));
        obj.insert(
            "__key_order".into(),
            Value::Array(vec![
                Value::scalar("permissions"),
                Value::scalar("conditions"),
                Value::scalar("limitations"),
            ]),
        );
        let input = Value::Object(obj);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        let json_str = result.to_kstr().to_string();
        let perm_pos = json_str
            .find("\"permissions\"")
            .expect("should have permissions");
        let cond_pos = json_str
            .find("\"conditions\"")
            .expect("should have conditions");
        let lim_pos = json_str
            .find("\"limitations\"")
            .expect("should have limitations");
        assert!(
            perm_pos < cond_pos && cond_pos < lim_pos,
            "Keys should be in YAML order: permissions < conditions < limitations. Got: {}",
            json_str
        );
        assert!(
            !json_str.contains("__key_order"),
            "__key_order metadata should be stripped from jsonify output"
        );
    }

    #[test]
    fn test_jsonify_nested_objects_preserve_key_order() {
        let mut inner = liquid::Object::new();
        inner.insert("z_field".into(), Value::scalar("z"));
        inner.insert("a_field".into(), Value::scalar("a"));
        inner.insert(
            "__key_order".into(),
            Value::Array(vec![Value::scalar("z_field"), Value::scalar("a_field")]),
        );
        let mut outer = liquid::Object::new();
        outer.insert("items".into(), Value::Object(inner));
        outer.insert("name".into(), Value::scalar("test"));
        outer.insert(
            "__key_order".into(),
            Value::Array(vec![Value::scalar("items"), Value::scalar("name")]),
        );
        let input = Value::Object(outer);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        let json_str = result.to_kstr().to_string();
        let items_pos = json_str.find("\"items\"").expect("should have items");
        let name_pos = json_str.find("\"name\"").expect("should have name");
        assert!(
            items_pos < name_pos,
            "Outer keys should be in order: items < name. Got: {}",
            json_str
        );
        let z_pos = json_str.find("\"z_field\"").expect("should have z_field");
        let a_pos = json_str.find("\"a_field\"").expect("should have a_field");
        assert!(
            z_pos < a_pos,
            "Inner keys should be in order: z_field < a_field. Got: {}",
            json_str
        );
    }

    #[test]
    fn test_jsonify_object_without_key_order_still_works() {
        let mut obj = liquid::Object::new();
        obj.insert("name".into(), Value::scalar("Alice"));
        obj.insert("age".into(), Value::scalar(30i64));
        let input = Value::Object(obj);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        let json_str = result.to_kstr().to_string();
        assert!(json_str.contains("\"name\":\"Alice\""));
        assert!(json_str.contains("\"age\":30"));
    }

    #[test]
    fn test_jsonify_key_order_with_unicode_keys() {
        let mut obj = liquid::Object::new();
        obj.insert("beschreibung".into(), Value::scalar("Hallo W\u{00f6}rld"));
        obj.insert("titel".into(), Value::scalar("F\u{00fc}r dich"));
        obj.insert(
            "__key_order".into(),
            Value::Array(vec![Value::scalar("beschreibung"), Value::scalar("titel")]),
        );
        let input = Value::Object(obj);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        let json_str = result.to_kstr().to_string();
        let besch_pos = json_str
            .find("\"beschreibung\"")
            .expect("should have beschreibung");
        let titel_pos = json_str.find("\"titel\"").expect("should have titel");
        assert!(
            besch_pos < titel_pos,
            "Unicode keys should be in order. Got: {}",
            json_str
        );
    }
}
