use std::cmp;

use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct SortArgs {
    #[parameter(description = "Optional property name to sort by.", arg_type = "str")]
    property: Option<Expression>,
}

/// Sort filter with stable tie-breaking by slug/path.
///
/// When sorting an array of objects by a property (e.g., `| sort: 'episode'`),
/// items with equal sort key values are tie-broken by `slug` (then `path` if
/// slug is also equal). This matches Jekyll's behavior where equal items
/// preserve the underlying filename/path order.
///
/// Without this, the Liquid crate's built-in sort uses `sort_by` (which is
/// stable) but the preserved input order may not match Jekyll's when the
/// collection was loaded via parallel I/O.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "sort",
    description = "Sorts items in an array with stable tie-breaking by slug.",
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

/// Tiebreak two objects by slug, then by path.
fn tiebreak(a: &Value, b: &Value) -> cmp::Ordering {
    let slug_a = get_property(a, "slug");
    let slug_b = get_property(b, "slug");
    let slug_cmp = nil_safe_compare(slug_a, slug_b);
    if slug_cmp != cmp::Ordering::Equal {
        return slug_cmp;
    }
    // Fall back to path
    let path_a = get_property(a, "path");
    let path_b = get_property(b, "path");
    nil_safe_compare(path_a, path_b)
}

impl Filter for SortFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;

        let input: Vec<_> = input
            .as_array()
            .map(|arr| arr.values().collect())
            .unwrap_or_default();

        let mut sorted: Vec<Value> = input.iter().map(|v| v.to_value()).collect();

        if let Some(property) = &args.property {
            let prop = property.to_kstr();
            sorted.sort_by(|a, b| {
                let primary = nil_safe_compare(
                    get_property(a, prop.as_str()),
                    get_property(b, prop.as_str()),
                );
                if primary != cmp::Ordering::Equal {
                    primary
                } else {
                    tiebreak(a, b)
                }
            });
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
    fn test_sort_tiebreaks_by_slug() {
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
        // episode 3 first, then episode 4 tie-broken by slug alphabetically
        assert_eq!(
            slugs,
            vec!["earlier", "data-science-interview", "data-translator"]
        );
    }

    #[test]
    fn test_sort_tiebreaks_by_path_when_slugs_equal() {
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
        assert_eq!(paths, vec!["_podcast/aaa.md", "_podcast/zzz.md"]);
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
}
