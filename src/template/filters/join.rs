//! Custom `join` filter that handles string input leniently.
//!
//! Jekyll (Ruby Liquid) allows `join` on strings, returning the string unchanged.
//! The `liquid` crate's built-in `join` only accepts arrays and raises an error
//! on string input. This custom filter handles both cases.
//! (Issue 328: academicpages `group-by-array` include)

use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct JoinArgs {
    #[parameter(description = "The separator string.", arg_type = "str")]
    separator: Option<Expression>,
}

/// Lenient join filter: arrays are joined with separator, strings pass through.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "join",
    description = "Join array elements with a separator. Strings pass through unchanged.",
    parameters(JoinArgs),
    parsed(JoinFilter)
)]
pub struct Join;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "join"]
struct JoinFilter {
    #[parameters]
    args: JoinArgs,
}

impl Filter for JoinFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;
        let separator = args
            .separator
            .map(|s| s.to_kstr().to_string())
            .unwrap_or_else(|| " ".to_string());

        // If input is an array, join elements with the separator.
        // Ruby's Array#join recursively flattens nested arrays, so
        // [["a", "b"], ["c"]].join(",") => "a,b,c". We match this behavior.
        if let Some(arr) = input.as_array() {
            let mut parts = Vec::new();
            flatten_into(&arr, &mut parts);
            Ok(Value::scalar(parts.join(&separator)))
        } else {
            // For non-array input (string, number, etc.), return as-is.
            // This matches Jekyll's behavior where `"hello" | join: ","` returns "hello".
            Ok(input.to_value())
        }
    }
}

/// Recursively flatten an array, collecting string representations of leaf values.
/// Matches Ruby's `Array#join` which recursively flattens nested arrays.
fn flatten_into(arr: &dyn liquid_core::model::ArrayView, out: &mut Vec<String>) {
    for v in arr.values() {
        if let Some(inner) = v.as_array() {
            flatten_into(&inner, out);
        } else {
            out.push(v.render().to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_array_with_separator() {
        let input = Value::Array(vec![
            Value::scalar("a"),
            Value::scalar("b"),
            Value::scalar("c"),
        ]);
        let result = liquid_core::call_filter!(Join, input, ",").unwrap();
        assert_eq!(result.to_kstr(), "a,b,c");
    }

    #[test]
    fn test_join_array_default_separator() {
        let input = Value::Array(vec![Value::scalar("x"), Value::scalar("y")]);
        let result = liquid_core::call_filter!(Join, input).unwrap();
        assert_eq!(result.to_kstr(), "x y");
    }

    #[test]
    fn test_join_string_passthrough() {
        // When input is a string, join should return it unchanged
        let result = liquid_core::call_filter!(Join, "hello,world", ",").unwrap();
        assert_eq!(result.to_kstr(), "hello,world");
    }

    #[test]
    fn test_join_nil_returns_empty() {
        let result = liquid_core::call_filter!(Join, Value::Nil, ",").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_join_nested_array_flattens() {
        // Ruby: [["a", "b"], ["c"]].join(",") => "a,b,c"
        let input = Value::Array(vec![
            Value::Array(vec![Value::scalar("cool posts"), Value::scalar("cat1")]),
            Value::Array(vec![Value::scalar("cool posts"), Value::scalar("cat2")]),
        ]);
        let result = liquid_core::call_filter!(Join, input, ",").unwrap();
        assert_eq!(result.to_kstr(), "cool posts,cat1,cool posts,cat2");
    }

    #[test]
    fn test_join_unicode_array() {
        let input = Value::Array(vec![
            Value::scalar("\u{4F60}\u{597D}"),
            Value::scalar("\u{4E16}\u{754C}"),
        ]);
        let result = liquid_core::call_filter!(Join, input, ",").unwrap();
        assert_eq!(result.to_kstr(), "\u{4F60}\u{597D},\u{4E16}\u{754C}");
    }
}
