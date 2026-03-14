use std::collections::BTreeMap;

use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct GroupByExpArgs {
    #[parameter(description = "The variable name for each element.", arg_type = "str")]
    var_name: Expression,
    #[parameter(description = "The Liquid expression to evaluate.", arg_type = "str")]
    expression: Expression,
}

/// Group an array of objects using a Liquid expression.
///
/// Usage: `array | group_by_exp: "item", "item.date | date: \"%Y\""`
///
/// Returns an array of objects, each with `name` (the evaluated expression result),
/// `items` (array of matching objects), and `size` (count of items).
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "group_by_exp",
    description = "Group an array of objects using a Liquid expression.",
    parameters(GroupByExpArgs),
    parsed(GroupByExpFilter)
)]
pub struct GroupByExp;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "group_by_exp"]
struct GroupByExpFilter {
    #[parameters]
    args: GroupByExpArgs,
}

/// Evaluate a Liquid expression string with a variable bound to an element.
///
/// Wraps the expression as `{{ expression }}` and renders it using a minimal
/// Liquid parser. This allows the expression to contain filter chains like
/// `item.date | date: "%Y"`.
fn evaluate_expression_as_value(
    expression: &str,
    var_name: &str,
    element: &dyn ValueView,
    runtime: &dyn Runtime,
) -> String {
    // Build a mini template: {{ expression }}
    let template_str = format!("{{{{ {} }}}}", expression);

    // Build a minimal parser with stdlib + Jekyll filters so that common filters
    // like `date` are available.
    let parser = liquid::ParserBuilder::with_stdlib()
        .filter(liquid_lib::jekyll::Slugify)
        .build();

    let parser = match parser {
        Ok(p) => p,
        Err(_) => return String::new(),
    };

    let template = match parser.parse(&template_str) {
        Ok(t) => t,
        Err(_) => return String::new(),
    };

    // Build the context: bind the variable name to the element, and include
    // all variables from the current runtime.
    let mut globals = liquid::Object::new();
    globals.insert(var_name.to_owned().into(), element.to_value());

    // Copy runtime variables into the context so expressions can reference
    // site, page, etc.
    // The Runtime trait doesn't expose iteration, so we rely on the bound
    // variable being sufficient for the expression. In practice, group_by_exp
    // expressions only reference the bound variable (e.g., "post.date | date: '%Y'").

    // Try to copy common runtime roots
    for key in &["site", "page", "paginator", "layout", "content"] {
        let keys = vec![liquid::model::ScalarCow::new(*key)];
        if let Some(val) = runtime.try_get(&keys) {
            globals.insert((*key).into(), val.to_value());
        }
    }

    match template.render(&globals) {
        Ok(output) => output.trim().to_string(),
        Err(_) => String::new(),
    }
}

impl Filter for GroupByExpFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;
        let var_name = args.var_name.to_kstr();
        let expression = args.expression.to_kstr();

        let array = match input.as_array() {
            Some(arr) => arr,
            None => return Ok(Value::Array(vec![])),
        };

        // Use BTreeMap to get deterministic ordering by group name
        // but also track insertion order for Jekyll compatibility
        let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
        let mut key_indices: BTreeMap<String, usize> = BTreeMap::new();

        for item in array.values() {
            let key = evaluate_expression_as_value(&expression, &var_name, item, runtime);

            if let Some(&idx) = key_indices.get(&key) {
                groups[idx].1.push(item.to_value());
            } else {
                let idx = groups.len();
                key_indices.insert(key.clone(), idx);
                groups.push((key, vec![item.to_value()]));
            }
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
    fn test_group_by_exp_simple_property() {
        let input = Value::Array(vec![
            make_item(&[("category", "tech"), ("title", "Post 1")]),
            make_item(&[("category", "life"), ("title", "Post 2")]),
            make_item(&[("category", "tech"), ("title", "Post 3")]),
        ]);
        let result = liquid_core::call_filter!(GroupByExp, input, "item", "item.category").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 2);

        // First group should be "tech" (first encountered)
        let group0 = arr.values().next().unwrap();
        let obj0 = group0.as_object().unwrap();
        assert_eq!(obj0.get("name").unwrap().to_kstr().as_str(), "tech");
        assert_eq!(obj0.get("items").unwrap().as_array().unwrap().size(), 2);
        assert_eq!(obj0.get("size").unwrap().to_kstr().as_str(), "2");

        // Second group should be "life"
        let group1 = arr.values().nth(1).unwrap();
        let obj1 = group1.as_object().unwrap();
        assert_eq!(obj1.get("name").unwrap().to_kstr().as_str(), "life");
        assert_eq!(obj1.get("items").unwrap().as_array().unwrap().size(), 1);
        assert_eq!(obj1.get("size").unwrap().to_kstr().as_str(), "1");
    }

    #[test]
    fn test_group_by_exp_with_filter_chain() {
        // Test expression with a filter pipe: item.name | downcase
        let input = Value::Array(vec![
            make_item(&[("name", "Alice")]),
            make_item(&[("name", "BOB")]),
            make_item(&[("name", "alice")]),
        ]);
        let result =
            liquid_core::call_filter!(GroupByExp, input, "item", "item.name | downcase").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 2);

        // "alice" group should have 2 items
        let group0 = arr.values().next().unwrap();
        let obj0 = group0.as_object().unwrap();
        assert_eq!(obj0.get("name").unwrap().to_kstr().as_str(), "alice");
        assert_eq!(obj0.get("items").unwrap().as_array().unwrap().size(), 2);
    }

    #[test]
    fn test_group_by_exp_all_same_group() {
        let input = Value::Array(vec![
            make_item(&[("type", "x")]),
            make_item(&[("type", "x")]),
            make_item(&[("type", "x")]),
        ]);
        let result = liquid_core::call_filter!(GroupByExp, input, "item", "item.type").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
        let group = arr.values().next().unwrap();
        let obj = group.as_object().unwrap();
        assert_eq!(obj.get("size").unwrap().to_kstr().as_str(), "3");
    }

    #[test]
    fn test_group_by_exp_empty_array() {
        let input = Value::Array(vec![]);
        let result = liquid_core::call_filter!(GroupByExp, input, "item", "item.category").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_group_by_exp_non_array_input() {
        let input = Value::scalar("not an array");
        let result = liquid_core::call_filter!(GroupByExp, input, "item", "item.category").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_group_by_exp_missing_property() {
        let input = Value::Array(vec![
            make_item(&[("category", "tech")]),
            make_item(&[("other", "val")]),
        ]);
        let result = liquid_core::call_filter!(GroupByExp, input, "item", "item.category").unwrap();
        let arr = result.as_array().unwrap();
        // Two groups: "tech" and "" (missing property renders as empty string)
        assert_eq!(arr.size(), 2);
    }

    #[test]
    fn test_group_by_exp_multiple_filter_pipes() {
        // Test chained filters: item.name | downcase | strip
        let input = Value::Array(vec![
            make_item(&[("name", "  Alice  ")]),
            make_item(&[("name", "ALICE")]),
        ]);
        let result =
            liquid_core::call_filter!(GroupByExp, input, "item", "item.name | downcase | strip")
                .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
        let group = arr.values().next().unwrap();
        let obj = group.as_object().unwrap();
        assert_eq!(obj.get("name").unwrap().to_kstr().as_str(), "alice");
    }

    #[test]
    fn test_group_by_exp_has_name_items_size() {
        let input = Value::Array(vec![make_item(&[("color", "red")])]);
        let result = liquid_core::call_filter!(GroupByExp, input, "item", "item.color").unwrap();
        let arr = result.as_array().unwrap();
        let group = arr.values().next().unwrap();
        let obj = group.as_object().unwrap();
        assert!(obj.get("name").is_some());
        assert!(obj.get("items").is_some());
        assert!(obj.get("size").is_some());
    }

    #[test]
    fn test_group_by_exp_truncate_filter() {
        // Test grouping by year using truncate to extract first 4 chars
        // This validates that filter arguments with colons work
        let input = Value::Array(vec![
            make_item(&[("date", "2024-01-15")]),
            make_item(&[("date", "2024-06-20")]),
            make_item(&[("date", "2023-03-10")]),
        ]);
        let result =
            liquid_core::call_filter!(GroupByExp, input, "post", "post.date | truncate: 4, \"\"")
                .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 2);

        // Check we have groups for 2024 and 2023
        let names: Vec<String> = arr
            .values()
            .map(|g| {
                g.as_object()
                    .unwrap()
                    .get("name")
                    .unwrap()
                    .to_kstr()
                    .to_string()
            })
            .collect();
        assert!(names.contains(&"2024".to_string()));
        assert!(names.contains(&"2023".to_string()));

        // 2024 should have 2 items
        for g in arr.values() {
            let obj = g.as_object().unwrap();
            if obj.get("name").unwrap().to_kstr().as_str() == "2024" {
                assert_eq!(obj.get("size").unwrap().to_kstr().as_str(), "2");
            }
        }
    }
}
