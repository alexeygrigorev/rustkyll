use liquid_core::model::ScalarCow;
use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct WhereExpArgs {
    #[parameter(description = "The variable name for each element.", arg_type = "str")]
    var_name: Expression,
    #[parameter(description = "The Liquid expression to evaluate.", arg_type = "str")]
    expression: Expression,
}

/// Filter an array using a Liquid expression.
///
/// Usage: `array | where_exp: "item", "item.field op value"`
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "where_exp",
    description = "Filter an array using a Liquid expression.",
    parameters(WhereExpArgs),
    parsed(WhereExpFilter)
)]
pub struct WhereExp;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "where_exp"]
struct WhereExpFilter {
    #[parameters]
    args: WhereExpArgs,
}

/// Resolve a dotted path like "item.field.subfield" against
/// a context where `var_name` is bound to `element`, and other
/// variables come from the runtime.
fn resolve_value(
    path: &str,
    var_name: &str,
    element: &dyn ValueView,
    runtime: &dyn Runtime,
) -> Value {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.is_empty() {
        return Value::Nil;
    }

    // Check if the path starts with our bound variable
    if parts[0] == var_name {
        // Navigate from the element
        let mut current: &dyn ValueView = element;
        for &part in &parts[1..] {
            if let Some(obj) = current.as_object() {
                if let Some(val) = obj.get(part) {
                    current = val;
                } else {
                    return Value::Nil;
                }
            } else {
                return Value::Nil;
            }
        }
        return current.to_value();
    }

    // Try resolving from the runtime context
    let keys: Vec<ScalarCow<'_>> = parts.iter().map(|&p| ScalarCow::new(p)).collect();
    runtime
        .try_get(&keys)
        .map(|v| v.to_value())
        .unwrap_or(Value::Nil)
}

/// Parse a literal value from the expression (string literal, true, false, number).
fn parse_literal(s: &str) -> Option<Value> {
    let s = s.trim();
    if s == "true" {
        return Some(Value::scalar(true));
    }
    if s == "false" {
        return Some(Value::scalar(false));
    }
    if s == "nil" || s == "null" {
        return Some(Value::Nil);
    }
    // Quoted string
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        let inner = &s[1..s.len() - 1];
        return Some(Value::scalar(inner.to_string()));
    }
    // Integer
    if let Ok(i) = s.parse::<i64>() {
        return Some(Value::scalar(i));
    }
    // Float
    if let Ok(f) = s.parse::<f64>() {
        return Some(Value::scalar(f));
    }
    None
}

/// Get a value from the expression token -- either a literal or a variable path.
fn get_value(token: &str, var_name: &str, element: &dyn ValueView, runtime: &dyn Runtime) -> Value {
    let token = token.trim();
    if let Some(lit) = parse_literal(token) {
        lit
    } else {
        resolve_value(token, var_name, element, runtime)
    }
}

/// Compare two liquid values as strings for ordering.
fn compare_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    // Try numeric comparison first
    if let (Some(a_int), Some(b_int)) = (to_integer(a), to_integer(b)) {
        return Some(a_int.cmp(&b_int));
    }
    if let (Some(a_f), Some(b_f)) = (to_float(a), to_float(b)) {
        return a_f.partial_cmp(&b_f);
    }
    // Fall back to string comparison
    let a_str = a.render().to_string();
    let b_str = b.render().to_string();
    Some(a_str.cmp(&b_str))
}

fn to_integer(v: &Value) -> Option<i64> {
    match v {
        Value::Scalar(s) => s.to_integer(),
        _ => None,
    }
}

fn to_float(v: &Value) -> Option<f64> {
    match v {
        Value::Scalar(s) => s.to_float(),
        _ => None,
    }
}

fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Nil => false,
        Value::Scalar(s) => {
            // Non-nil non-false scalars are truthy in Liquid
            s.to_bool().unwrap_or(true)
        }
        _ => true,
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    // Direct comparison
    if a == b {
        return true;
    }
    // Try numeric comparison (integer vs float)
    if let (Some(af), Some(bf)) = (to_float(a), to_float(b)) {
        return (af - bf).abs() < f64::EPSILON;
    }
    // String comparison fallback
    a.render().to_string() == b.render().to_string()
}

/// Evaluate a simple expression like "item.field op value" or "item.field contains value".
fn evaluate_expression(
    expr: &str,
    var_name: &str,
    element: &dyn ValueView,
    runtime: &dyn Runtime,
) -> bool {
    let expr = expr.trim();

    // Try to find a known operator
    let operators = [" contains ", " != ", " >= ", " <= ", " > ", " < ", " == "];

    for op in &operators {
        if let Some(pos) = expr.find(op) {
            let lhs_str = &expr[..pos];
            let rhs_str = &expr[pos + op.len()..];
            let lhs = get_value(lhs_str, var_name, element, runtime);
            let rhs = get_value(rhs_str, var_name, element, runtime);

            return match op.trim() {
                "contains" => {
                    // Array contains element, or string contains substring
                    if let Value::Array(arr) = &lhs {
                        arr.iter().any(|item| values_equal(item, &rhs))
                    } else {
                        let lhs_str = lhs.render().to_string();
                        let rhs_str = rhs.render().to_string();
                        lhs_str.contains(&rhs_str)
                    }
                }
                "!=" => !values_equal(&lhs, &rhs),
                "==" => values_equal(&lhs, &rhs),
                ">=" => compare_values(&lhs, &rhs)
                    .map(|o| o != std::cmp::Ordering::Less)
                    .unwrap_or(false),
                "<=" => compare_values(&lhs, &rhs)
                    .map(|o| o != std::cmp::Ordering::Greater)
                    .unwrap_or(false),
                ">" => compare_values(&lhs, &rhs)
                    .map(|o| o == std::cmp::Ordering::Greater)
                    .unwrap_or(false),
                "<" => compare_values(&lhs, &rhs)
                    .map(|o| o == std::cmp::Ordering::Less)
                    .unwrap_or(false),
                _ => false,
            };
        }
    }

    // No operator found -- treat expression as a truthiness check
    let val = get_value(expr, var_name, element, runtime);
    is_truthy(&val)
}

impl Filter for WhereExpFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;

        let var_name = args.var_name.to_kstr();
        let expression = args.expression.to_kstr();

        let array = match input.as_array() {
            Some(arr) => arr,
            None => {
                // Non-array input: return empty array
                return Ok(Value::Array(vec![]));
            }
        };

        let mut result = Vec::new();
        for item in array.values() {
            if evaluate_expression(&expression, &var_name, item, runtime) {
                result.push(item.to_value());
            }
        }

        Ok(Value::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_where_exp_not_equal_true() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("draft".into(), Value::scalar(true));
                o.insert("name".into(), Value::scalar("a"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("draft".into(), Value::scalar(false));
                o.insert("name".into(), Value::scalar("b"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("name".into(), Value::scalar("c"));
                o
            }),
        ]);
        let result =
            liquid_core::call_filter!(WhereExp, input, "item", "item.draft != true").unwrap();
        let arr = result.as_array().unwrap();
        // The second item (draft=false) and third item (draft missing=nil) should pass
        assert_eq!(arr.size(), 2);
    }

    #[test]
    fn test_where_exp_contains() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert(
                    "authors".into(),
                    Value::Array(vec![Value::scalar("alice"), Value::scalar("bob")]),
                );
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("authors".into(), Value::Array(vec![Value::scalar("carol")]));
                o
            }),
        ]);
        let result =
            liquid_core::call_filter!(WhereExp, input, "item", "item.authors contains \"alice\"")
                .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }

    #[test]
    fn test_where_exp_greater_than() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("time".into(), Value::scalar(10i64));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("time".into(), Value::scalar(20i64));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(WhereExp, input, "item", "item.time > 15").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }

    #[test]
    fn test_where_exp_greater_equal() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("time".into(), Value::scalar(15i64));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("time".into(), Value::scalar(20i64));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("time".into(), Value::scalar(10i64));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(WhereExp, input, "item", "item.time >= 15").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 2);
    }

    #[test]
    fn test_where_exp_less_than() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("time".into(), Value::scalar(10i64));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("time".into(), Value::scalar(20i64));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(WhereExp, input, "item", "item.time < 15").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }

    #[test]
    fn test_where_exp_less_equal() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("time".into(), Value::scalar(15i64));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("time".into(), Value::scalar(20i64));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(WhereExp, input, "item", "item.time <= 15").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }

    #[test]
    fn test_where_exp_empty_array() {
        let input = Value::Array(vec![]);
        let result = liquid_core::call_filter!(WhereExp, input, "item", "item.x != true").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_where_exp_non_array_input() {
        let input = Value::scalar("not an array");
        let result = liquid_core::call_filter!(WhereExp, input, "item", "item.x != true").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 0);
    }

    #[test]
    fn test_where_exp_equal() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("status".into(), Value::scalar("active"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("status".into(), Value::scalar("inactive"));
                o
            }),
        ]);
        let result =
            liquid_core::call_filter!(WhereExp, input, "item", "item.status == \"active\"")
                .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }
}
