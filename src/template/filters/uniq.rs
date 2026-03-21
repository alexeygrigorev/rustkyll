use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

use super::sort::flatten_one_level;

/// Custom `uniq` filter that flattens one level of nested arrays before
/// deduplicating, matching Ruby Liquid's `InputIterator` behavior.
///
/// In Jekyll, `notes | map: "tags" | uniq` works because Ruby's InputIterator
/// calls `.flatten` on the input array. Without this, `map` returns an array
/// of arrays and `uniq` would deduplicate whole sub-arrays instead of
/// individual tag strings.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "uniq",
    description = "Remove duplicate elements from an array (flattens nested arrays first).",
    parsed(UniqFilter)
)]
pub struct Uniq;

#[derive(Debug, Default, Display_filter)]
#[name = "uniq"]
struct UniqFilter;

impl Filter for UniqFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let array = match input.as_array() {
            Some(arr) => arr,
            None => return Ok(Value::Nil),
        };

        // Flatten one level (Ruby InputIterator compat), then deduplicate.
        let flattened = flatten_one_level(array.values().map(|v| v.to_value()));

        let mut deduped: Vec<Value> = Vec::with_capacity(flattened.len());
        for item in flattened {
            let dominated = deduped.iter().any(|existing| {
                liquid_core::model::ValueViewCmp::new(existing.as_view())
                    == liquid_core::model::ValueViewCmp::new(item.as_view())
            });
            if !dominated {
                deduped.push(item);
            }
        }

        Ok(Value::Array(deduped))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniq_scalars() {
        let input = Value::Array(vec![
            Value::scalar("a"),
            Value::scalar("b"),
            Value::scalar("a"),
        ]);
        let result = liquid_core::call_filter!(Uniq, input).unwrap();
        let arr = result.as_array().unwrap();
        let vals: Vec<_> = arr.values().map(|v| v.to_kstr().to_string()).collect();
        assert_eq!(vals, vec!["a", "b"]);
    }

    #[test]
    fn test_uniq_flattens_nested_arrays() {
        let input = Value::Array(vec![
            Value::Array(vec![Value::scalar("Book"), Value::scalar("Mental health")]),
            Value::Array(vec![Value::scalar("Life"), Value::scalar("Book")]),
            Value::Array(vec![]),
        ]);
        let result = liquid_core::call_filter!(Uniq, input).unwrap();
        let arr = result.as_array().unwrap();
        let vals: Vec<_> = arr.values().map(|v| v.to_kstr().to_string()).collect();
        assert_eq!(vals, vec!["Book", "Mental health", "Life"]);
    }

    #[test]
    fn test_uniq_non_array_returns_nil() {
        let input = Value::scalar("not an array");
        let result = liquid_core::call_filter!(Uniq, input).unwrap();
        assert!(result.is_nil());
    }

    #[test]
    fn test_uniq_unicode() {
        let input = Value::Array(vec![
            Value::Array(vec![
                Value::scalar("\u{53f0}\u{7063}"),
                Value::scalar("Design"),
            ]),
            Value::Array(vec![
                Value::scalar("Design"),
                Value::scalar("\u{53f0}\u{7063}"),
            ]),
        ]);
        let result = liquid_core::call_filter!(Uniq, input).unwrap();
        let arr = result.as_array().unwrap();
        let vals: Vec<_> = arr.values().map(|v| v.to_kstr().to_string()).collect();
        assert_eq!(vals, vec!["\u{53f0}\u{7063}", "Design"]);
    }
}
