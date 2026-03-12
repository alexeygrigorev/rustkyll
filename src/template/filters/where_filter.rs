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
                    let rendered = val.render().to_string();
                    if rendered == target_value.as_str() {
                        result.push(item.to_value());
                    }
                }
            }
        }

        Ok(Value::Array(result))
    }
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
