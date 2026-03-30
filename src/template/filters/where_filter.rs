use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct WhereArgs {
    #[parameter(description = "The property name to filter by.", arg_type = "str")]
    property: Expression,
    #[parameter(description = "The value to match.", arg_type = "str")]
    target_value: Expression,
}

/// Filter an array by a property value.
///
/// Usage: `array | where: "property", "value"`
///
/// Returns all items in the array where `item.property == value`.
/// Comparison is done as string rendering to match Jekyll's behavior.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "where",
    description = "Filter an array where a property equals a value.",
    parameters(WhereArgs),
    parsed(WhereFilter)
)]
pub struct Where;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "where"]
struct WhereFilter {
    #[parameters]
    args: WhereArgs,
}

impl Filter for WhereFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;

        let property = args.property.to_kstr();
        let target_value = args.target_value.to_kstr();

        let array = match input.as_array() {
            Some(arr) => arr,
            None => {
                return Ok(Value::Array(vec![]));
            }
        };

        let mut result = Vec::new();
        for item in array.values() {
            if let Some(obj) = item.as_object() {
                if let Some(val) = obj.get(property.as_str()) {
                    // Compare as strings via to_kstr(), which coerces booleans
                    // to "true"/"false" -- matching Jekyll's where filter behavior
                    // where `pin: true` (YAML bool) matches `where: 'pin', 'true'`.
                    let val_str = val.to_kstr();
                    if val_str.as_str() == target_value.as_str() {
                        result.push(item.to_value());
                    } else if is_year_target(target_value.as_str())
                        && is_datetime_string(val_str.as_str())
                        && val_str.as_str().starts_with(target_value.as_str())
                    {
                        // When exact match fails, try year-prefix matching for
                        // datetime strings. Jekyll's where filter with dates
                        // allows matching "2018-01-15 00:00:00 +0000" against "2018".
                        result.push(item.to_value());
                    }
                }
            }
        }

        Ok(Value::Array(result))
    }
}

/// Returns true if the target string is a 4-digit year (e.g. "2018").
fn is_year_target(s: &str) -> bool {
    s.len() == 4 && s.bytes().all(|b| b.is_ascii_digit())
}

/// Returns true if the string looks like a datetime (starts with YYYY-MM-DD pattern).
/// Matches formats like "2018-01-15", "2018-01-15 00:00:00 +0000", etc.
fn is_datetime_string(s: &str) -> bool {
    // Must be at least "YYYY-MM-DD" (10 chars) and match the pattern
    if s.len() < 10 {
        return false;
    }
    let bytes = s.as_bytes();
    // Check YYYY-MM-DD pattern
    bytes[0..4].iter().all(|b| b.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_where_filter_basic_match() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("short".into(), Value::scalar("alice"));
                o.insert("title".into(), Value::scalar("Alice Smith"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("short".into(), Value::scalar("bob"));
                o.insert("title".into(), Value::scalar("Bob Jones"));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(Where, input, "short", "alice").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }

    #[test]
    fn test_where_filter_no_match() {
        let input = Value::Array(vec![Value::Object({
            let mut o = liquid::Object::new();
            o.insert("short".into(), Value::scalar("alice"));
            o
        })]);
        let result = liquid_core::call_filter!(Where, input, "short", "nobody").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_where_filter_empty_array() {
        let input = Value::Array(vec![]);
        let result = liquid_core::call_filter!(Where, input, "short", "alice").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_where_filter_non_array_input() {
        let input = Value::scalar("not an array");
        let result = liquid_core::call_filter!(Where, input, "short", "alice").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_where_filter_multiple_matches() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("status".into(), Value::scalar("active"));
                o.insert("name".into(), Value::scalar("a"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("status".into(), Value::scalar("inactive"));
                o.insert("name".into(), Value::scalar("b"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("status".into(), Value::scalar("active"));
                o.insert("name".into(), Value::scalar("c"));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(Where, input, "status", "active").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 2);
    }

    #[test]
    fn test_where_filter_bool_true_matches_string_true() {
        // TDD: Jekyll coerces booleans to strings in `where` filter.
        // `pin: true` (bool) should match `where: 'pin', 'true'` (string target).
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Pinned Post"));
                o.insert("pin".into(), Value::scalar(true));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Normal Post"));
                o.insert("pin".into(), Value::scalar(false));
                o
            }),
        ]);
        // Verify boolean to_kstr works as expected
        let bool_val = Value::scalar(true);
        assert_eq!(
            bool_val.to_kstr().as_str(),
            "true",
            "to_kstr on bool true must return 'true'"
        );
        let bool_val = Value::scalar(false);
        assert_eq!(
            bool_val.to_kstr().as_str(),
            "false",
            "to_kstr on bool false must return 'false'"
        );

        let result = liquid_core::call_filter!(Where, input, "pin", "true").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1, "Boolean true should match string 'true'");
    }

    #[test]
    fn test_where_filter_bool_false_matches_string_false() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Post A"));
                o.insert("hidden".into(), Value::scalar(false));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Post B"));
                o.insert("hidden".into(), Value::scalar(true));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(Where, input, "hidden", "false").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1, "Boolean false should match string 'false'");
    }

    #[test]
    fn test_where_filter_bool_true_does_not_match_string_false() {
        let input = Value::Array(vec![Value::Object({
            let mut o = liquid::Object::new();
            o.insert("pin".into(), Value::scalar(true));
            o
        })]);
        let result = liquid_core::call_filter!(Where, input, "pin", "false").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(
            arr.size(),
            0,
            "Boolean true should NOT match string 'false'"
        );
    }

    #[test]
    fn test_where_filter_string_comparison_still_works() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("category".into(), Value::scalar("blog"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("category".into(), Value::scalar("news"));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(Where, input, "category", "blog").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(
            arr.size(),
            1,
            "String-to-string comparison should still work"
        );
    }

    #[test]
    fn test_where_filter_integer_matches_string() {
        // Jekyll coerces numbers too: `priority: 1` (int) matches `where: 'priority', '1'`.
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("priority".into(), Value::scalar(1i64));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("priority".into(), Value::scalar(2i64));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(Where, input, "priority", "1").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1, "Integer 1 should match string '1'");
    }

    #[test]
    fn test_where_filter_mixed_bool_and_nil() {
        // Posts without the `pin` field should not match `where: 'pin', 'true'`.
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Pinned"));
                o.insert("pin".into(), Value::scalar(true));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("No Pin Field"));
                // pin field is absent
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Nil Pin"));
                o.insert("pin".into(), Value::Nil);
                o
            }),
        ]);
        let result = liquid_core::call_filter!(Where, input, "pin", "true").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(
            arr.size(),
            1,
            "Only the post with pin=true should match; absent and nil should not"
        );
    }

    #[test]
    fn test_where_filter_datetime_matches_year() {
        // TDD: datetime string "2018-01-15 00:00:00 +0000" should match target "2018"
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Post A"));
                o.insert("date".into(), Value::scalar("2018-01-15 00:00:00 +0000"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Post B"));
                o.insert("date".into(), Value::scalar("2019-03-20 00:00:00 +0000"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Post C"));
                o.insert("date".into(), Value::scalar("2018-06-30 12:00:00 +0000"));
                o
            }),
        ]);

        // Filter for 2018 -- should get 2 posts
        let result = liquid_core::call_filter!(Where, input.clone(), "date", "2018").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 2, "Should match 2 posts with year 2018");

        // Filter for 2019 -- should get 1 post
        let result = liquid_core::call_filter!(Where, input.clone(), "date", "2019").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1, "Should match 1 post with year 2019");

        // Filter for 2020 -- should get 0 posts
        let result = liquid_core::call_filter!(Where, input, "date", "2020").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0, "Should match 0 posts with year 2020");
    }

    #[test]
    fn test_where_filter_datetime_with_negative_timezone() {
        // Date with negative timezone offset should still match year
        let input = Value::Array(vec![Value::Object({
            let mut o = liquid::Object::new();
            o.insert("title".into(), Value::scalar("Post"));
            o.insert("date".into(), Value::scalar("2018-12-31 23:00:00 -0500"));
            o
        })]);
        let result = liquid_core::call_filter!(Where, input, "date", "2018").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(
            arr.size(),
            1,
            "Datetime with -0500 offset should match year 2018"
        );
    }

    #[test]
    fn test_where_filter_bare_date_matches_year() {
        // Bare date "2018-06-15" should also match year "2018" via prefix
        let input = Value::Array(vec![Value::Object({
            let mut o = liquid::Object::new();
            o.insert("title".into(), Value::scalar("Post"));
            o.insert("date".into(), Value::scalar("2018-06-15"));
            o
        })]);
        let result = liquid_core::call_filter!(Where, input, "date", "2018").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1, "Bare date should match year 2018");
    }

    #[test]
    fn test_where_filter_exact_match_takes_priority() {
        // If date field literally contains "2018", exact match works
        let input = Value::Array(vec![Value::Object({
            let mut o = liquid::Object::new();
            o.insert("date".into(), Value::scalar("2018"));
            o
        })]);
        let result = liquid_core::call_filter!(Where, input, "date", "2018").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1, "Exact match should still work");
    }

    #[test]
    fn test_where_filter_non_date_string_no_spurious_match() {
        // "2018abc" is not a datetime pattern, should NOT match "2018"
        let input = Value::Array(vec![Value::Object({
            let mut o = liquid::Object::new();
            o.insert("val".into(), Value::scalar("2018abc"));
            o
        })]);
        let result = liquid_core::call_filter!(Where, input, "val", "2018").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(
            arr.size(),
            0,
            "Non-date string starting with 2018 should NOT match"
        );
    }

    #[test]
    fn test_where_filter_datetime_unicode_content_no_regression() {
        // Non-ASCII content in other fields must not cause issues
        let input = Value::Array(vec![Value::Object({
            let mut o = liquid::Object::new();
            o.insert("title".into(), Value::scalar("Привет мир 日本語"));
            o.insert("date".into(), Value::scalar("2018-01-15 00:00:00 +0000"));
            o
        })]);
        let result = liquid_core::call_filter!(Where, input, "date", "2018").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(
            arr.size(),
            1,
            "Unicode content should not interfere with datetime matching"
        );
    }

    #[test]
    fn test_where_filter_missing_property_on_item() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("short".into(), Value::scalar("alice"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("other".into(), Value::scalar("value"));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(Where, input, "short", "alice").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }
}
