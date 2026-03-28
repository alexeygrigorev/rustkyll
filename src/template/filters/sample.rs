use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, FilterParameters)]
struct SampleArgs {
    #[parameter(
        description = "Number of random elements to return.",
        arg_type = "integer"
    )]
    count: Option<Expression>,
}

/// Return random elements from an array.
///
/// - `array | sample` -- returns a single random element (scalar)
/// - `array | sample: N` -- returns N random elements (array, without replacement)
/// - Empty array: nil (no arg) or empty array (with arg)
/// - Nil input: nil (no arg) or empty array (with arg)
/// - Non-array input: treated as single-element array
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "sample",
    description = "Return random elements from an array.",
    parameters(SampleArgs),
    parsed(SampleFilter)
)]
pub struct Sample;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "sample"]
struct SampleFilter {
    #[parameters]
    args: SampleArgs,
}

/// Build a deterministic RNG seeded from the array content.
///
/// This ensures that `sample` produces the same output for the same input
/// across multiple builds, making static site generation reproducible.
/// The seed is derived from a hash of each element's string representation.
fn deterministic_rng(items: &[Value]) -> StdRng {
    let mut hasher = DefaultHasher::new();
    items.len().hash(&mut hasher);
    for item in items {
        item.to_kstr().as_str().hash(&mut hasher);
    }
    StdRng::seed_from_u64(hasher.finish())
}

impl Filter for SampleFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;
        let has_count = args.count.is_some();

        // Build the source array
        let items: Vec<Value> = if input.is_nil() {
            vec![]
        } else if let Some(arr) = input.as_array() {
            arr.values().map(|v| v.to_value()).collect()
        } else {
            // Non-array, non-nil: wrap in a single-element array
            vec![input.to_value()]
        };

        if let Some(n) = args.count {
            // sample: N  -- return an array of up to N elements
            let n = if n < 0 { 0usize } else { n as usize };
            if items.is_empty() || n == 0 {
                return Ok(Value::Array(vec![]));
            }
            let take = std::cmp::min(n, items.len());
            let mut rng = deterministic_rng(&items);
            let mut shuffled = items;
            shuffled.shuffle(&mut rng);
            shuffled.truncate(take);
            Ok(Value::Array(shuffled))
        } else {
            // sample (no arg) -- return a single element as a scalar
            if items.is_empty() {
                if has_count {
                    return Ok(Value::Array(vec![]));
                }
                return Ok(Value::Nil);
            }
            let mut rng = deterministic_rng(&items);
            let chosen = items.choose(&mut rng).unwrap().clone();
            Ok(chosen)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: run the sample filter with no argument, return the Value
    fn sample_no_arg(input: Value) -> Value {
        liquid_core::call_filter!(Sample, input).unwrap()
    }

    // Helper: run the sample filter with a count argument, return the Value
    fn sample_with_n(input: Value, n: i64) -> Value {
        liquid_core::call_filter!(Sample, input, n).unwrap()
    }

    // ---- sample with no argument ----

    #[test]
    fn test_sample_no_arg_single_element() {
        let input = Value::Array(vec![Value::scalar("one")]);
        let result = sample_no_arg(input);
        assert_eq!(result.to_kstr().as_str(), "one");
    }

    #[test]
    fn test_sample_no_arg_multiple_elements() {
        let input = Value::Array(vec![
            Value::scalar("apple"),
            Value::scalar("banana"),
            Value::scalar("cherry"),
        ]);
        let result = sample_no_arg(input);
        let s = result.to_kstr().to_string();
        assert!(
            s == "apple" || s == "banana" || s == "cherry",
            "Expected one of the input elements, got: {s}"
        );
    }

    #[test]
    fn test_sample_no_arg_returns_scalar_not_array() {
        let input = Value::Array(vec![Value::scalar("only")]);
        let result = sample_no_arg(input);
        // Should be a scalar, not a 1-element array
        assert!(result.as_array().is_none(), "Expected scalar, got array");
    }

    #[test]
    fn test_sample_no_arg_empty_array_returns_nil() {
        let input = Value::Array(vec![]);
        let result = sample_no_arg(input);
        assert!(result.is_nil(), "Expected Nil for empty array");
    }

    #[test]
    fn test_sample_no_arg_nil_input_returns_nil() {
        let result = sample_no_arg(Value::Nil);
        assert!(result.is_nil(), "Expected Nil for nil input");
    }

    // ---- sample with N argument ----

    #[test]
    fn test_sample_n_returns_correct_count() {
        let input = Value::Array(vec![
            Value::scalar("a"),
            Value::scalar("b"),
            Value::scalar("c"),
            Value::scalar("d"),
        ]);
        let result = sample_with_n(input, 2);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 2);
    }

    #[test]
    fn test_sample_n_elements_from_input() {
        let input = Value::Array(vec![
            Value::scalar("a"),
            Value::scalar("b"),
            Value::scalar("c"),
            Value::scalar("d"),
        ]);
        let result = sample_with_n(input, 2);
        let arr = result.as_array().unwrap();
        let valid = ["a", "b", "c", "d"];
        for v in arr.values() {
            let s = v.to_kstr().to_string();
            assert!(valid.contains(&s.as_str()), "Unexpected element: {s}");
        }
    }

    #[test]
    fn test_sample_n_no_duplicates() {
        // With 4 elements and sample: 4, all should appear exactly once
        let input = Value::Array(vec![
            Value::scalar("a"),
            Value::scalar("b"),
            Value::scalar("c"),
            Value::scalar("d"),
        ]);
        let result = sample_with_n(input, 4);
        let arr = result.as_array().unwrap();
        let mut vals: Vec<String> = arr.values().map(|v| v.to_kstr().to_string()).collect();
        vals.sort();
        assert_eq!(vals, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_sample_n_greater_than_size_caps_at_size() {
        let input = Value::Array(vec![
            Value::scalar("a"),
            Value::scalar("b"),
            Value::scalar("c"),
        ]);
        let result = sample_with_n(input, 5);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 3);
        let mut vals: Vec<String> = arr.values().map(|v| v.to_kstr().to_string()).collect();
        vals.sort();
        assert_eq!(vals, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_sample_n_zero_returns_empty() {
        let input = Value::Array(vec![
            Value::scalar("x"),
            Value::scalar("y"),
            Value::scalar("z"),
        ]);
        let result = sample_with_n(input, 0);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_sample_n_empty_array_returns_empty() {
        let input = Value::Array(vec![]);
        let result = sample_with_n(input, 3);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_sample_n_nil_input_returns_empty() {
        let result = sample_with_n(Value::Nil, 3);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    // ---- non-array input ----

    #[test]
    fn test_sample_no_arg_scalar_string() {
        let result = sample_no_arg(Value::scalar("hello"));
        assert_eq!(result.to_kstr().as_str(), "hello");
    }

    #[test]
    fn test_sample_n_scalar_integer() {
        let result = sample_with_n(Value::scalar(42), 1);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
        assert_eq!(arr.values().next().unwrap().to_kstr().as_str(), "42");
    }

    // ---- Unicode / non-ASCII content ----

    #[test]
    fn test_sample_unicode_single() {
        // French accented string
        let input = Value::Array(vec![Value::scalar("\u{00e9}t\u{00e9}")]);
        let result = sample_no_arg(input);
        assert_eq!(result.to_kstr().as_str(), "\u{00e9}t\u{00e9}");
    }

    #[test]
    fn test_sample_unicode_multiple() {
        let items = vec![
            Value::scalar("\u{00e9}t\u{00e9}"), // French: ete with accents
            Value::scalar("\u{00fc}\u{00f6}\u{00e4}"), // German umlauts
            Value::scalar("\u{6771}\u{4eac}"),  // CJK: Tokyo
            Value::scalar("\u{1f31f}"),         // Emoji: star
        ];
        let input = Value::Array(items.clone());
        let result = sample_with_n(input, 2);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 2);

        let valid: Vec<String> = items.iter().map(|v| v.to_kstr().to_string()).collect();
        for v in arr.values() {
            let s = v.to_kstr().to_string();
            assert!(
                valid.contains(&s),
                "Expected one of the Unicode input elements, got: {s}"
            );
        }
    }

    #[test]
    fn test_sample_unicode_all_elements_preserved() {
        // sample: N with N >= size should return all elements
        let items = vec![
            Value::scalar("\u{00e9}t\u{00e9}"),
            Value::scalar("\u{00fc}\u{00f6}\u{00e4}"),
            Value::scalar("\u{6771}\u{4eac}"),
            Value::scalar("\u{1f31f}"),
        ];
        let input = Value::Array(items.clone());
        let result = sample_with_n(input, 10);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 4);

        let mut got: Vec<String> = arr.values().map(|v| v.to_kstr().to_string()).collect();
        got.sort();
        let mut expected: Vec<String> = items.iter().map(|v| v.to_kstr().to_string()).collect();
        expected.sort();
        assert_eq!(got, expected);
    }

    // ---- determinism ----

    #[test]
    fn test_sample_deterministic_across_calls() {
        // The sample filter must produce the same output for the same input
        // across multiple calls, ensuring reproducible builds.
        let items = vec![
            Value::scalar("alpha"),
            Value::scalar("bravo"),
            Value::scalar("charlie"),
            Value::scalar("delta"),
            Value::scalar("echo"),
            Value::scalar("foxtrot"),
        ];

        let result1 = sample_with_n(Value::Array(items.clone()), 4);
        let result2 = sample_with_n(Value::Array(items.clone()), 4);

        let order1: Vec<String> = result1
            .as_array()
            .unwrap()
            .values()
            .map(|v| v.to_kstr().to_string())
            .collect();
        let order2: Vec<String> = result2
            .as_array()
            .unwrap()
            .values()
            .map(|v| v.to_kstr().to_string())
            .collect();

        assert_eq!(
            order1, order2,
            "sample must be deterministic: got {order1:?} vs {order2:?}"
        );
    }

    #[test]
    fn test_sample_no_arg_deterministic() {
        // Even sample with no argument should be deterministic for the same input
        let items = vec![
            Value::scalar("one"),
            Value::scalar("two"),
            Value::scalar("three"),
        ];

        let result1 = sample_no_arg(Value::Array(items.clone()));
        let result2 = sample_no_arg(Value::Array(items.clone()));

        assert_eq!(
            result1.to_kstr().to_string(),
            result2.to_kstr().to_string(),
            "sample (no arg) must be deterministic"
        );
    }
}
