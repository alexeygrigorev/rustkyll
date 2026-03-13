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
            if let Some(b) = s.to_bool() {
                serde_json::Value::Bool(b)
            } else if let Some(i) = s.to_integer() {
                serde_json::Value::Number(serde_json::Number::from(i))
            } else if let Some(f) = s.to_float() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::String(s.to_kstr().to_string())
            }
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
}
