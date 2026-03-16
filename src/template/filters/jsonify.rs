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

fn liquid_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Scalar(s) => {
            // Use serde serialization to preserve the original scalar type.
            // ScalarCow uses #[serde(transparent)] over an untagged enum, so:
            //   ScalarCowEnum::Integer(42)  -> JSON 42
            //   ScalarCowEnum::Str("42")    -> JSON "42"
            //   ScalarCowEnum::Float(1.5)   -> JSON 1.5
            //   ScalarCowEnum::Bool(true)   -> JSON true
            // This matches Jekyll's jsonify behavior where YAML strings that
            // look like numbers (e.g., tag "2024") stay as JSON strings.
            serde_json::to_value(s)
                .unwrap_or_else(|_| serde_json::Value::String(s.to_kstr().to_string()))
        }
        Value::Array(arr) => {
            let items: Vec<serde_json::Value> = arr.iter().map(liquid_to_json).collect();
            serde_json::Value::Array(items)
        }
        Value::Object(obj) => {
            let map: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.to_string(), liquid_to_json(&v.to_value())))
                .collect();
            serde_json::Value::Object(map)
        }
        Value::Nil => serde_json::Value::Null,
        Value::State(_) => serde_json::Value::Null,
    }
}

impl Filter for JsonifyFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let value = input.to_value();
        let json = liquid_to_json(&value);
        let json_str = serde_json::to_string(&json).unwrap_or_else(|_| "null".to_string());
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
        // A string that looks like a number (e.g., YAML tag "2024") must stay
        // a JSON string, not be coerced to a JSON number. This matches Jekyll's
        // jsonify behavior where `"2024".to_json` produces `"2024"`, not `2024`.
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
        // Simulates `page.tags | jsonify` where tags are ["survey", "ai", "2024"]
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
        // An actual integer (not a string) should still serialize as a number
        let input = Value::scalar(2024i64);
        let result = liquid_core::call_filter!(Jsonify, input).unwrap();
        assert_eq!(result.to_kstr(), "2024");
    }
}
