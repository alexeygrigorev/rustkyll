use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct FindArgs {
    #[parameter(description = "The property name to find by.", arg_type = "str")]
    property: Expression,
    #[parameter(description = "The value to match.", arg_type = "str")]
    target_value: Expression,
}

/// Find the first item in an array where a property equals a value.
///
/// Usage: `array | find: "property", "value"`
///
/// Returns the first item in the array where `item.property == value`.
/// Returns `Nil` if no match is found (unlike `where` which returns an empty array).
/// Comparison is done as string rendering to match Jekyll's behavior.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "find",
    description = "Find the first item in an array where a property equals a value.",
    parameters(FindArgs),
    parsed(FindFilter)
)]
pub struct Find;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "find"]
struct FindFilter {
    #[parameters]
    args: FindArgs,
}

impl Filter for FindFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;

        let property = args.property.to_kstr();
        let target_value = args.target_value.to_kstr();

        let array = match input.as_array() {
            Some(arr) => arr,
            None => {
                return Ok(Value::Nil);
            }
        };

        for item in array.values() {
            if let Some(obj) = item.as_object() {
                if let Some(val) = obj.get(property.as_str()) {
                    let val_str = val.to_kstr();
                    if val_str.as_str() == target_value.as_str() {
                        return Ok(item.to_value());
                    }
                }
            }
        }

        Ok(Value::Nil)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_filter_basic_match() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("name".into(), Value::scalar("about.md"));
                o.insert("title".into(), Value::scalar("About"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("name".into(), Value::scalar("index.md"));
                o.insert("title".into(), Value::scalar("Home"));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(Find, input, "name", "about.md").unwrap();
        // Should return the object, not an array
        assert!(result.as_object().is_some());
        let obj = result.as_object().unwrap();
        assert_eq!(obj.get("title").unwrap().to_kstr().as_str(), "About");
    }

    #[test]
    fn test_find_filter_no_match_returns_nil() {
        let input = Value::Array(vec![Value::Object({
            let mut o = liquid::Object::new();
            o.insert("name".into(), Value::scalar("about.md"));
            o
        })]);
        let result = liquid_core::call_filter!(Find, input, "name", "nonexistent").unwrap();
        assert!(result.is_nil());
    }

    #[test]
    fn test_find_filter_multiple_matches_returns_first() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("status".into(), Value::scalar("active"));
                o.insert("name".into(), Value::scalar("first"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("status".into(), Value::scalar("active"));
                o.insert("name".into(), Value::scalar("second"));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(Find, input, "status", "active").unwrap();
        let obj = result.as_object().unwrap();
        assert_eq!(obj.get("name").unwrap().to_kstr().as_str(), "first");
    }

    #[test]
    fn test_find_filter_empty_array() {
        let input = Value::Array(vec![]);
        let result = liquid_core::call_filter!(Find, input, "name", "about.md").unwrap();
        assert!(result.is_nil());
    }

    #[test]
    fn test_find_filter_non_array_input() {
        let input = Value::scalar("not an array");
        let result = liquid_core::call_filter!(Find, input, "name", "about.md").unwrap();
        assert!(result.is_nil());
    }

    #[test]
    fn test_find_filter_missing_property_skipped() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("other".into(), Value::scalar("value"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("name".into(), Value::scalar("target.md"));
                o.insert("title".into(), Value::scalar("Target"));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(Find, input, "name", "target.md").unwrap();
        let obj = result.as_object().unwrap();
        assert_eq!(obj.get("title").unwrap().to_kstr().as_str(), "Target");
    }

    #[test]
    fn test_find_filter_unicode_values() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("name".into(), Value::scalar("hello"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("name".into(), Value::scalar("\u{00fc}ber-page.md"));
                o.insert("title".into(), Value::scalar("Das \u{00dc}ber"));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(Find, input, "name", "\u{00fc}ber-page.md").unwrap();
        let obj = result.as_object().unwrap();
        assert_eq!(
            obj.get("title").unwrap().to_kstr().as_str(),
            "Das \u{00dc}ber"
        );
    }
}
