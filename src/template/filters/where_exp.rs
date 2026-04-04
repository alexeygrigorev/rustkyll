use std::borrow::Cow;

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

/// Pre-split dotted path for efficient repeated resolution.
/// Avoids re-splitting and re-allocating on every element evaluation.
struct ResolvedPath<'a> {
    /// Whether the first segment matches the bound variable name.
    is_bound: bool,
    /// Path segments after the variable name (for bound paths), or all segments (for runtime paths).
    segments: Vec<&'a str>,
    /// Pre-built ScalarCow keys for runtime resolution (only populated for non-bound paths).
    runtime_keys: Vec<ScalarCow<'a>>,
}

impl<'a> ResolvedPath<'a> {
    fn new(path: &'a str, var_name: &str) -> Self {
        let segments: Vec<&'a str> = path.split('.').collect();
        if segments.is_empty() {
            return ResolvedPath {
                is_bound: false,
                segments: Vec::new(),
                runtime_keys: Vec::new(),
            };
        }
        if segments[0] == var_name {
            ResolvedPath {
                is_bound: true,
                segments: segments[1..].to_vec(),
                runtime_keys: Vec::new(),
            }
        } else {
            let runtime_keys = segments.iter().map(|&p| ScalarCow::new(p)).collect();
            ResolvedPath {
                is_bound: false,
                segments,
                runtime_keys,
            }
        }
    }

    fn resolve(&self, element: &dyn ValueView, runtime: &dyn Runtime) -> Value {
        if self.is_bound {
            let mut current: &dyn ValueView = element;
            for &part in &self.segments {
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
            current.to_value()
        } else {
            self.resolve_runtime(runtime)
        }
    }

    /// Resolve a non-bound path from the runtime context only.
    /// This is used for pre-resolution of runtime variables before the element loop.
    fn resolve_runtime(&self, runtime: &dyn Runtime) -> Value {
        if self.runtime_keys.is_empty() {
            Value::Nil
        } else {
            runtime
                .try_get(&self.runtime_keys)
                .map(|v| v.to_value())
                .unwrap_or(Value::Nil)
        }
    }
}

/// The operator in a where_exp expression.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ExprOp {
    Contains,
    NotEqual,
    Equal,
    GreaterEqual,
    LessEqual,
    GreaterThan,
    LessThan,
}

/// A token in the expression: either a pre-parsed literal or a variable path.
enum ExprToken<'a> {
    Literal(Value),
    Path(ResolvedPath<'a>),
}

impl<'a> ExprToken<'a> {
    fn resolve(&self, element: &dyn ValueView, runtime: &dyn Runtime) -> Value {
        match self {
            ExprToken::Literal(v) => v.clone(),
            ExprToken::Path(p) => p.resolve(element, runtime),
        }
    }

    /// Pre-resolve tokens that don't depend on the per-element bound variable.
    /// Runtime-resolved paths (e.g., `page.short`, `site.time`) return the same
    /// value for every element, so resolving them once before the loop avoids
    /// redundant runtime lookups (e.g., 128,000+ lookups for DTC author pages).
    fn pre_resolve(self, runtime: &dyn Runtime) -> ExprToken<'a> {
        match &self {
            ExprToken::Literal(_) => self,
            ExprToken::Path(p) if !p.is_bound => {
                // Not bound to the element variable -- resolve from runtime once.
                let val = p.resolve_runtime(runtime);
                ExprToken::Literal(val)
            }
            _ => self, // Bound path -- must be resolved per element.
        }
    }
}

/// Logical connective between sub-expressions.
#[derive(Clone, Copy, Debug, PartialEq)]
enum LogicalOp {
    And,
    Or,
}

/// A pre-parsed expression that can be evaluated efficiently per element.
enum ParsedExpr<'a> {
    /// Binary expression: lhs op rhs
    Binary {
        lhs: ExprToken<'a>,
        op: ExprOp,
        rhs: ExprToken<'a>,
    },
    /// Truthiness check (no operator found)
    Truthy(ExprToken<'a>),
    /// Compound expression: sub1 and/or sub2 and/or sub3 ...
    /// Jekyll supports chaining multiple `and`/`or` in where_exp.
    Compound {
        parts: Vec<ParsedExpr<'a>>,
        ops: Vec<LogicalOp>,
    },
}

impl<'a> ParsedExpr<'a> {
    /// Pre-resolve all non-element-bound tokens against the runtime.
    /// This converts runtime variable lookups to cached literal values,
    /// eliminating redundant lookups in the per-element evaluation loop.
    fn pre_resolve(self, runtime: &dyn Runtime) -> Self {
        match self {
            ParsedExpr::Binary { lhs, op, rhs } => ParsedExpr::Binary {
                lhs: lhs.pre_resolve(runtime),
                op,
                rhs: rhs.pre_resolve(runtime),
            },
            ParsedExpr::Truthy(token) => ParsedExpr::Truthy(token.pre_resolve(runtime)),
            ParsedExpr::Compound { parts, ops } => ParsedExpr::Compound {
                parts: parts.into_iter().map(|p| p.pre_resolve(runtime)).collect(),
                ops,
            },
        }
    }
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

/// Build an ExprToken from a raw string token.
fn make_token<'a>(token: &'a str, var_name: &str) -> ExprToken<'a> {
    let token = token.trim();
    if let Some(lit) = parse_literal(token) {
        ExprToken::Literal(lit)
    } else {
        ExprToken::Path(ResolvedPath::new(token, var_name))
    }
}

/// Decode HTML entities that may appear in expressions when Liquid tags are
/// processed after markdown-to-HTML conversion. For example, `>=` becomes
/// `&gt;=` and `<` becomes `&lt;` in HTML.
fn decode_html_entities(s: &str) -> Cow<'_, str> {
    // Short-circuit: if no ampersand, nothing to decode.
    if !s.contains('&') {
        return Cow::Borrowed(s);
    }
    Cow::Owned(
        s.replace("&gt;", ">")
            .replace("&lt;", "<")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'"),
    )
}

/// The known operators to search for, in order. We search for the longest
/// multi-word operators first to avoid partial matches.
const OPERATORS: &[(&str, ExprOp)] = &[
    (" contains ", ExprOp::Contains),
    (" != ", ExprOp::NotEqual),
    (" >= ", ExprOp::GreaterEqual),
    (" <= ", ExprOp::LessEqual),
    (" > ", ExprOp::GreaterThan),
    (" < ", ExprOp::LessThan),
    (" == ", ExprOp::Equal),
];

/// Parse an expression string ONCE, returning a structure that can be
/// evaluated efficiently against many elements.
/// Split an expression on ` and ` / ` or ` logical operators.
///
/// Returns None if no logical operator is found.
/// Returns the sub-expression strings and the logical operators between them.
///
/// We must be careful not to split inside quoted strings. We use a simple
/// approach: scan for ` and ` / ` or ` outside of single/double quotes.
fn split_logical_operators(expr: &str) -> Option<(Vec<&str>, Vec<LogicalOp>)> {
    let bytes = expr.as_bytes();
    let len = bytes.len();
    let mut parts = Vec::new();
    let mut ops = Vec::new();
    let mut last_split = 0;
    let mut i = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while i < len {
        let b = bytes[i];
        if b == b'\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if b == b'"' && !in_single_quote {
            in_double_quote = !in_double_quote;
        } else if !in_single_quote && !in_double_quote {
            // Check for " and " (5 chars)
            if i + 5 <= len && &expr[i..i + 5] == " and " {
                parts.push(&expr[last_split..i]);
                ops.push(LogicalOp::And);
                last_split = i + 5;
                i += 5;
                continue;
            }
            // Check for " or " (4 chars)
            if i + 4 <= len && &expr[i..i + 4] == " or " {
                parts.push(&expr[last_split..i]);
                ops.push(LogicalOp::Or);
                last_split = i + 4;
                i += 4;
                continue;
            }
        }
        i += 1;
    }

    if ops.is_empty() {
        return None;
    }

    parts.push(&expr[last_split..]);
    Some((parts, ops))
}

/// Parse a single comparison expression (no and/or).
fn parse_single_expression<'a>(expr: &'a str, var_name: &str) -> ParsedExpr<'a> {
    let expr = expr.trim();
    for &(op_str, op) in OPERATORS {
        if let Some(pos) = expr.find(op_str) {
            let lhs_str = &expr[..pos];
            let rhs_str = &expr[pos + op_str.len()..];
            return ParsedExpr::Binary {
                lhs: make_token(lhs_str, var_name),
                op,
                rhs: make_token(rhs_str, var_name),
            };
        }
    }
    // No operator found -- treat as truthiness check
    ParsedExpr::Truthy(make_token(expr, var_name))
}

fn parse_expression<'a>(expr: &'a str, var_name: &str) -> ParsedExpr<'a> {
    // First try to split on logical operators (and/or)
    if let Some((sub_exprs, logical_ops)) = split_logical_operators(expr) {
        let parts: Vec<ParsedExpr<'a>> = sub_exprs
            .into_iter()
            .map(|s| parse_single_expression(s, var_name))
            .collect();
        return ParsedExpr::Compound {
            parts,
            ops: logical_ops,
        };
    }

    parse_single_expression(expr, var_name)
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

/// Evaluate a pre-parsed expression against a single element.
fn evaluate_parsed_expr(
    parsed: &ParsedExpr<'_>,
    element: &dyn ValueView,
    runtime: &dyn Runtime,
) -> bool {
    match parsed {
        ParsedExpr::Binary { lhs, op, rhs } => {
            let lhs_val = lhs.resolve(element, runtime);
            let rhs_val = rhs.resolve(element, runtime);
            match op {
                ExprOp::Contains => array_or_string_contains(&lhs_val, &rhs_val),
                ExprOp::NotEqual => !values_equal(&lhs_val, &rhs_val),
                ExprOp::Equal => values_equal(&lhs_val, &rhs_val),
                ExprOp::GreaterEqual => compare_values(&lhs_val, &rhs_val)
                    .map(|o| o != std::cmp::Ordering::Less)
                    .unwrap_or(false),
                ExprOp::LessEqual => compare_values(&lhs_val, &rhs_val)
                    .map(|o| o != std::cmp::Ordering::Greater)
                    .unwrap_or(false),
                ExprOp::GreaterThan => compare_values(&lhs_val, &rhs_val)
                    .map(|o| o == std::cmp::Ordering::Greater)
                    .unwrap_or(false),
                ExprOp::LessThan => compare_values(&lhs_val, &rhs_val)
                    .map(|o| o == std::cmp::Ordering::Less)
                    .unwrap_or(false),
            }
        }
        ParsedExpr::Truthy(token) => {
            let val = token.resolve(element, runtime);
            is_truthy(&val)
        }
        ParsedExpr::Compound { parts, ops } => {
            // Evaluate left-to-right. Jekyll evaluates `and`/`or` without
            // precedence -- strictly left-to-right, same as Ruby Liquid.
            let mut result = evaluate_parsed_expr(&parts[0], element, runtime);
            for (i, op) in ops.iter().enumerate() {
                let next = evaluate_parsed_expr(&parts[i + 1], element, runtime);
                result = match op {
                    LogicalOp::And => result && next,
                    LogicalOp::Or => result || next,
                };
            }
            result
        }
    }
}

/// Check if an array contains a value, or a string contains a substring.
fn array_or_string_contains(lhs: &Value, rhs: &Value) -> bool {
    if let Value::Array(arr) = lhs {
        arr.iter().any(|item| values_equal(item, rhs))
    } else {
        let lhs_str = lhs.render().to_string();
        let rhs_str = rhs.render().to_string();
        lhs_str.contains(&rhs_str)
    }
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

        // Decode HTML entities ONCE for the entire filter invocation,
        // then parse the expression ONCE. This avoids re-parsing per element.
        let decoded = decode_html_entities(&expression);
        let decoded_trimmed = decoded.trim();
        let parsed = parse_expression(decoded_trimmed, &var_name);
        // Pre-resolve non-bound tokens (e.g., page.short, site.time) against
        // the runtime ONCE, avoiding redundant lookups per element.
        let parsed = parsed.pre_resolve(runtime);

        let mut result = Vec::new();
        for item in array.values() {
            if evaluate_parsed_expr(&parsed, item, runtime) {
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

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(decode_html_entities("a &gt;= b").as_ref(), "a >= b");
        assert_eq!(decode_html_entities("a &lt; b").as_ref(), "a < b");
        assert_eq!(decode_html_entities("a &gt; b").as_ref(), "a > b");
        assert_eq!(decode_html_entities("a &lt;= b").as_ref(), "a <= b");
        assert_eq!(decode_html_entities("a &amp; b").as_ref(), "a & b");
        assert_eq!(
            decode_html_entities("no entities here").as_ref(),
            "no entities here"
        );
    }

    #[test]
    fn test_decode_html_entities_short_circuit() {
        // Verify that strings without '&' are returned as borrowed (no allocation)
        let input = "item.field contains value";
        let result = decode_html_entities(input);
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_where_exp_date_string_comparison_less_than() {
        // Simulates: track.date < site.time where dates are strings
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("date".into(), Value::scalar("2021-02-05 20:00:00"));
                o.insert("name".into(), Value::scalar("Track 1"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("date".into(), Value::scalar("2099-12-31 23:59:59"));
                o.insert("name".into(), Value::scalar("Track 2"));
                o
            }),
        ]);
        // All 2021 dates are less than 2026 site.time
        let result = liquid_core::call_filter!(
            WhereExp,
            input,
            "item",
            "item.date < \"2026-03-15 00:00:00\""
        )
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }

    #[test]
    fn test_where_exp_date_string_comparison_greater_equal() {
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("date".into(), Value::scalar("2021-02-05 20:00:00"));
                o.insert("name".into(), Value::scalar("Past"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("date".into(), Value::scalar("2099-12-31 23:59:59"));
                o.insert("name".into(), Value::scalar("Future"));
                o
            }),
        ]);
        let result = liquid_core::call_filter!(
            WhereExp,
            input,
            "item",
            "item.date >= \"2026-03-15 00:00:00\""
        )
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }

    #[test]
    fn test_where_exp_html_encoded_greater_equal() {
        // When markdown converts >= to &gt;= inside Liquid expressions
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
        // Expression with HTML-encoded >= (&gt;=)
        let result =
            liquid_core::call_filter!(WhereExp, input, "item", "item.time &gt;= 15").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 2);
    }

    #[test]
    fn test_where_exp_html_encoded_less_than() {
        // When markdown converts < to &lt; inside Liquid expressions
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
        // Expression with HTML-encoded < (&lt;)
        let result =
            liquid_core::call_filter!(WhereExp, input, "item", "item.time &lt; 15").unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }

    #[test]
    fn test_where_exp_html_encoded_date_comparison() {
        // Real-world scenario: track.date &gt;= site.time (HTML-encoded from markdown)
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("date".into(), Value::scalar("2021-02-05 20:00:00"));
                o.insert("name".into(), Value::scalar("Past Track"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("date".into(), Value::scalar("2099-12-31 23:59:59"));
                o.insert("name".into(), Value::scalar("Future Track"));
                o
            }),
        ]);

        // HTML-encoded >= -- should filter to only the future track
        let result = liquid_core::call_filter!(
            WhereExp,
            input.clone(),
            "track",
            "track.date &gt;= \"2026-03-15 00:00:00\""
        )
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);

        // HTML-encoded < -- should filter to only the past track
        let result = liquid_core::call_filter!(
            WhereExp,
            input,
            "track",
            "track.date &lt; \"2026-03-15 00:00:00\""
        )
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1);
    }

    #[test]
    fn test_parse_expression_contains() {
        let parsed = parse_expression("item.authors contains \"alice\"", "item");
        match &parsed {
            ParsedExpr::Binary { op, .. } => assert_eq!(*op, ExprOp::Contains),
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn test_parse_expression_truthy() {
        let parsed = parse_expression("item.active", "item");
        assert!(matches!(parsed, ParsedExpr::Truthy(_)));
    }

    #[test]
    fn test_resolved_path_bound() {
        let path = ResolvedPath::new("item.field.sub", "item");
        assert!(path.is_bound);
        assert_eq!(path.segments, vec!["field", "sub"]);
    }

    #[test]
    fn test_resolved_path_unbound() {
        let path = ResolvedPath::new("site.posts", "item");
        assert!(!path.is_bound);
        assert_eq!(path.segments, vec!["site", "posts"]);
        assert_eq!(path.runtime_keys.len(), 2);
    }

    #[test]
    fn test_where_exp_and_operator() {
        // Chirpy uses: where_exp: 'item', 'item.pin != true and item.hidden != true'
        // This requires `and` logical operator support.
        let input = Value::Array(vec![
            // pinned, not hidden -> should be EXCLUDED (pin != true is false)
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Pinned Post"));
                o.insert("pin".into(), Value::scalar(true));
                o
            }),
            // not pinned, not hidden -> should be INCLUDED
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Normal Post"));
                o
            }),
            // not pinned, hidden -> should be EXCLUDED (hidden != true is false)
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Hidden Post"));
                o.insert("hidden".into(), Value::scalar(true));
                o
            }),
            // pinned and hidden -> should be EXCLUDED (both conditions fail)
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Pinned Hidden"));
                o.insert("pin".into(), Value::scalar(true));
                o.insert("hidden".into(), Value::scalar(true));
                o
            }),
        ]);

        let result = liquid_core::call_filter!(
            WhereExp,
            input,
            "item",
            "item.pin != true and item.hidden != true"
        )
        .unwrap();
        let arr = result.as_array().unwrap();
        // Only "Normal Post" should pass both conditions
        assert_eq!(
            arr.size(),
            1,
            "Expected 1 result (Normal Post only), got {}",
            arr.size()
        );
        let first = arr.values().next().unwrap();
        let title = first.as_object().unwrap().get("title").unwrap();
        assert_eq!(title.to_kstr().as_str(), "Normal Post");
    }

    #[test]
    fn test_where_exp_or_operator() {
        // Jekyll docs example: where_exp: "item", "item.sub_genre == 'MCU' or item.sub_genre == 'DCEU'"
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("MCU Movie"));
                o.insert("sub_genre".into(), Value::scalar("MCU"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("DCEU Movie"));
                o.insert("sub_genre".into(), Value::scalar("DCEU"));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Indie Movie"));
                o.insert("sub_genre".into(), Value::scalar("indie"));
                o
            }),
        ]);

        let result = liquid_core::call_filter!(
            WhereExp,
            input,
            "item",
            "item.sub_genre == 'MCU' or item.sub_genre == 'DCEU'"
        )
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(
            arr.size(),
            2,
            "Expected MCU and DCEU movies, got {}",
            arr.size()
        );
    }

    #[test]
    fn test_where_exp_and_with_unicode_values() {
        // Test and operator with non-ASCII content
        let input = Value::Array(vec![
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("Статья о Rust"));
                o.insert("lang".into(), Value::scalar("ru"));
                o.insert("published".into(), Value::scalar(true));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("記事"));
                o.insert("lang".into(), Value::scalar("ja"));
                o.insert("published".into(), Value::scalar(false));
                o
            }),
            Value::Object({
                let mut o = liquid::Object::new();
                o.insert("title".into(), Value::scalar("مقالة"));
                o.insert("lang".into(), Value::scalar("ar"));
                o.insert("published".into(), Value::scalar(true));
                o
            }),
        ]);

        let result = liquid_core::call_filter!(
            WhereExp,
            input,
            "item",
            "item.published == true and item.lang != 'ar'"
        )
        .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.size(), 1, "Expected only the Russian article");
        let first = arr.values().next().unwrap();
        let title = first.as_object().unwrap().get("title").unwrap();
        assert_eq!(title.to_kstr().as_str(), "Статья о Rust");
    }
}
