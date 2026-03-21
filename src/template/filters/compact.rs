use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

use super::sort::flatten_one_level;

/// Custom `compact` filter that flattens one level of nested arrays before
/// removing nil values, matching Ruby Liquid's `InputIterator` behavior.
///
/// In Jekyll, `notes | map: "tags" | compact` works because Ruby's InputIterator
/// calls `.flatten` on the input array. Without this, `map` returns an array of
/// arrays and `compact` would only remove nil elements at the top level.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "compact",
    description = "Remove nil values from an array (flattens nested arrays first).",
    parsed(CompactFilter)
)]
pub struct Compact;

#[derive(Debug, Default, Display_filter)]
#[name = "compact"]
struct CompactFilter;

impl Filter for CompactFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let array = match input.as_array() {
            Some(arr) => arr,
            None => return Ok(Value::Nil),
        };

        // Flatten one level (Ruby InputIterator compat), then remove nils.
        let flattened = flatten_one_level(array.values().map(|v| v.to_value()));
        let compacted: Vec<Value> = flattened.into_iter().filter(|v| !v.is_nil()).collect();

        Ok(Value::Array(compacted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_removes_nils() {
        let input = Value::Array(vec![Value::scalar("a"), Value::Nil, Value::scalar("b")]);
        let result = liquid_core::call_filter!(Compact, input).unwrap();
        let arr = result.as_array().unwrap();
        let vals: Vec<_> = arr.values().map(|v| v.to_kstr().to_string()).collect();
        assert_eq!(vals, vec!["a", "b"]);
    }

    #[test]
    fn test_compact_flattens_nested_arrays() {
        let input = Value::Array(vec![
            Value::Array(vec![Value::scalar("A"), Value::scalar("B")]),
            Value::Nil,
            Value::Array(vec![Value::scalar("C")]),
        ]);
        let result = liquid_core::call_filter!(Compact, input).unwrap();
        let arr = result.as_array().unwrap();
        let vals: Vec<_> = arr.values().map(|v| v.to_kstr().to_string()).collect();
        assert_eq!(vals, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_compact_non_array_returns_nil() {
        let input = Value::scalar("not an array");
        let result = liquid_core::call_filter!(Compact, input).unwrap();
        assert!(result.is_nil());
    }
}
