use std::collections::BTreeMap;

use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct GroupByArgs {
    #[parameter(description = "The property name to group by.", arg_type = "str")]
    property: Expression,
}

/// Group an array of objects by a property value.
///
/// Returns an array of objects, each with `name` (the property value),
/// `items` (array of matching objects), and `size` (count of items).
/// Items missing the property are grouped under an empty string name.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "group_by",
    description = "Group an array of objects by a property value.",
    parameters(GroupByArgs),
    parsed(GroupByFilter)
)]
pub struct GroupBy;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "group_by"]
struct GroupByFilter {
    #[parameters]
    args: GroupByArgs,
}

impl Filter for GroupByFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;
        let property = args.property.to_kstr();

        let array = match input.as_array() {
            Some(arr) => arr,
            None => return Ok(Value::Array(vec![])),
        };

        // Use BTreeMap to get deterministic ordering by group name
        let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();

        for item in array.values() {
            let key = if let Some(obj) = item.as_object() {
                obj.get(property.as_str())
                    .map(|v| v.render().to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };

            groups.entry(key).or_default().push(item.to_value());
        }

        let result: Vec<Value> = groups
            .into_iter()
            .map(|(name, items)| {
                let size = items.len() as i64;
                let mut obj = liquid::Object::new();
                obj.insert("name".into(), Value::scalar(name));
                obj.insert("items".into(), Value::Array(items));
                obj.insert("size".into(), Value::scalar(size));
                Value::Object(obj)
            })
            .collect();

        Ok(Value::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(props: &[(&str, &str)]) -> Value {
        let mut obj = liquid::Object::new();
        for (k, v) in props {
            obj.insert(k.to_string().into(), Value::scalar(v.to_string()));
        }
        Value::Object(obj)
    }

    #[test]
    fn test_group_by_basic() {
        let input = Value::Array(vec![
            make_item(&[("type", "a"), ("v", "1")]),
            make_item(&[("type", "b"), ("v", "2")]),
            make_item(&[("type", "a"), ("v", "3")]),
        ]);
        let result = liquid_core::call_filter!(GroupBy, input, "type").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 2);

        // Check group "a" has 2 items
        let group_a = arr.values().next().unwrap();
        let group_a_obj = group_a.as_object().unwrap();
        assert_eq!(group_a_obj.get("name").unwrap().to_kstr().as_str(), "a");
        assert_eq!(group_a_obj.get("size").unwrap().to_kstr().as_str(), "2");
        let items_a = group_a_obj.get("items").unwrap().as_array().unwrap();
        assert_eq!(items_a.size(), 2);
    }

    #[test]
    fn test_group_by_all_same_value() {
        let input = Value::Array(vec![
            make_item(&[("type", "x")]),
            make_item(&[("type", "x")]),
        ]);
        let result = liquid_core::call_filter!(GroupBy, input, "type").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }

    #[test]
    fn test_group_by_missing_property() {
        let input = Value::Array(vec![
            make_item(&[("type", "a")]),
            make_item(&[("other", "val")]),
        ]);
        let result = liquid_core::call_filter!(GroupBy, input, "type").unwrap();
        let arr = result.as_array().unwrap();
        // Two groups: "" (missing property) and "a"
        assert_eq!(arr.size(), 2);
    }

    #[test]
    fn test_group_by_empty_array() {
        let input = Value::Array(vec![]);
        let result = liquid_core::call_filter!(GroupBy, input, "type").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_group_by_non_array_input() {
        let input = Value::scalar("not an array");
        let result = liquid_core::call_filter!(GroupBy, input, "type").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_group_has_name_items_size() {
        let input = Value::Array(vec![make_item(&[("color", "red")])]);
        let result = liquid_core::call_filter!(GroupBy, input, "color").unwrap();
        let arr = result.as_array().unwrap();
        let group = arr.values().next().unwrap();
        let obj = group.as_object().unwrap();
        assert!(obj.get("name").is_some());
        assert!(obj.get("items").is_some());
        assert!(obj.get("size").is_some());
    }

    #[test]
    fn test_size_equals_items_length() {
        let input = Value::Array(vec![
            make_item(&[("k", "v")]),
            make_item(&[("k", "v")]),
            make_item(&[("k", "v")]),
        ]);
        let result = liquid_core::call_filter!(GroupBy, input, "k").unwrap();
        let arr = result.as_array().unwrap();
        let group = arr.values().next().unwrap();
        let obj = group.as_object().unwrap();
        let size: i64 = obj.get("size").unwrap().to_kstr().parse().unwrap();
        let items_len = obj.get("items").unwrap().as_array().unwrap().size() as i64;
        assert_eq!(size, items_len);
    }
}
