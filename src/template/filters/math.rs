//! Lenient math filters matching Ruby Liquid behavior.
//!
//! Ruby Liquid coerces non-numeric strings to 0 in math operations:
//! `"<" | times: 1` returns `0` (because `"<".to_i == 0` in Ruby).
//!
//! The stdlib `times`, `plus`, `minus` filters error on non-numeric inputs.
//! We override them with lenient versions that coerce non-numeric inputs
//! to 0, matching Ruby's behavior. This is needed by jekyll-toc.html which
//! uses `| times: 1` to coerce heading-level chars to integers.

use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Coerce a Liquid value to an integer, falling back to 0.
/// Matches Ruby's `String#to_i` where `"<".to_i == 0`.
fn coerce_to_i64(v: &dyn ValueView) -> i64 {
    if let Some(s) = v.as_scalar() {
        if let Some(i) = s.to_integer() {
            return i;
        }
        if let Some(f) = s.to_float() {
            return f as i64;
        }
    }
    0
}

fn coerce_to_f64(v: &dyn ValueView) -> f64 {
    if let Some(s) = v.as_scalar() {
        if let Some(f) = s.to_float() {
            return f;
        }
        if let Some(i) = s.to_integer() {
            return i as f64;
        }
    }
    0.0
}

fn is_float_value(v: &dyn ValueView) -> bool {
    if let Some(s) = v.as_scalar() {
        s.to_float().is_some() && s.to_integer().is_none()
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// times
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Times;

impl FilterReflection for Times {
    fn name(&self) -> &str {
        "times"
    }
    fn description(&self) -> &str {
        "Multiplies a number by the operand (lenient: non-numeric coerces to 0)"
    }
    fn positional_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &[liquid_core::parser::ParameterReflection {
            name: "operand",
            description: "The operand to multiply by",
            is_optional: false,
        }]
    }
    fn keyword_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &[]
    }
}

impl ParseFilter for Times {
    fn parse(&self, arguments: liquid_core::parser::FilterArguments) -> Result<Box<dyn Filter>> {
        let mut args = arguments;
        let operand = args
            .positional
            .next()
            .ok_or_else(|| liquid_core::Error::with_msg("times requires an argument"))?;
        Ok(Box::new(TimesFilter { operand }))
    }
    fn reflection(&self) -> &dyn FilterReflection {
        self
    }
}

#[derive(Debug)]
struct TimesFilter {
    operand: liquid_core::Expression,
}

impl std::fmt::Display for TimesFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "times")
    }
}

impl Filter for TimesFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let operand = self.operand.evaluate(runtime)?;
        if is_float_value(input) || is_float_value(operand.as_view()) {
            Ok(Value::scalar(
                coerce_to_f64(input) * coerce_to_f64(operand.as_view()),
            ))
        } else {
            Ok(Value::scalar(
                coerce_to_i64(input) * coerce_to_i64(operand.as_view()),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// plus
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Plus;

impl FilterReflection for Plus {
    fn name(&self) -> &str {
        "plus"
    }
    fn description(&self) -> &str {
        "Adds a number (lenient: non-numeric coerces to 0)"
    }
    fn positional_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &[liquid_core::parser::ParameterReflection {
            name: "operand",
            description: "The operand to add",
            is_optional: false,
        }]
    }
    fn keyword_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &[]
    }
}

impl ParseFilter for Plus {
    fn parse(&self, arguments: liquid_core::parser::FilterArguments) -> Result<Box<dyn Filter>> {
        let mut args = arguments;
        let operand = args
            .positional
            .next()
            .ok_or_else(|| liquid_core::Error::with_msg("plus requires an argument"))?;
        Ok(Box::new(PlusFilter { operand }))
    }
    fn reflection(&self) -> &dyn FilterReflection {
        self
    }
}

#[derive(Debug)]
struct PlusFilter {
    operand: liquid_core::Expression,
}

impl std::fmt::Display for PlusFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "plus")
    }
}

impl Filter for PlusFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let operand = self.operand.evaluate(runtime)?;
        if is_float_value(input) || is_float_value(operand.as_view()) {
            Ok(Value::scalar(
                coerce_to_f64(input) + coerce_to_f64(operand.as_view()),
            ))
        } else {
            Ok(Value::scalar(
                coerce_to_i64(input) + coerce_to_i64(operand.as_view()),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// minus
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Minus;

impl FilterReflection for Minus {
    fn name(&self) -> &str {
        "minus"
    }
    fn description(&self) -> &str {
        "Subtracts a number (lenient: non-numeric coerces to 0)"
    }
    fn positional_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &[liquid_core::parser::ParameterReflection {
            name: "operand",
            description: "The operand to subtract",
            is_optional: false,
        }]
    }
    fn keyword_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &[]
    }
}

impl ParseFilter for Minus {
    fn parse(&self, arguments: liquid_core::parser::FilterArguments) -> Result<Box<dyn Filter>> {
        let mut args = arguments;
        let operand = args
            .positional
            .next()
            .ok_or_else(|| liquid_core::Error::with_msg("minus requires an argument"))?;
        Ok(Box::new(MinusFilter { operand }))
    }
    fn reflection(&self) -> &dyn FilterReflection {
        self
    }
}

#[derive(Debug)]
struct MinusFilter {
    operand: liquid_core::Expression,
}

impl std::fmt::Display for MinusFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "minus")
    }
}

impl Filter for MinusFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let operand = self.operand.evaluate(runtime)?;
        if is_float_value(input) || is_float_value(operand.as_view()) {
            Ok(Value::scalar(
                coerce_to_f64(input) - coerce_to_f64(operand.as_view()),
            ))
        } else {
            Ok(Value::scalar(
                coerce_to_i64(input) - coerce_to_i64(operand.as_view()),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::engine::TemplateEngine;
    use liquid::Object;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    #[test]
    fn test_times_numeric() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), liquid::model::Value::scalar(3i64));
        let out = eng.parse_and_render("{{ x | times: 4 }}", &ctx).unwrap();
        assert_eq!(out, "12");
    }

    #[test]
    fn test_times_non_numeric_coerces_to_zero() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), liquid::model::Value::scalar("<"));
        let out = eng.parse_and_render("{{ x | times: 1 }}", &ctx).unwrap();
        assert_eq!(out, "0", "non-numeric string times 1 should be 0");
    }

    #[test]
    fn test_times_string_number() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), liquid::model::Value::scalar("3"));
        let out = eng.parse_and_render("{{ x | times: 2 }}", &ctx).unwrap();
        assert_eq!(out, "6");
    }

    #[test]
    fn test_plus_non_numeric_coerces_to_zero() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), liquid::model::Value::scalar("abc"));
        let out = eng.parse_and_render("{{ x | plus: 5 }}", &ctx).unwrap();
        assert_eq!(out, "5");
    }

    #[test]
    fn test_minus_non_numeric_coerces_to_zero() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), liquid::model::Value::scalar("abc"));
        let out = eng.parse_and_render("{{ x | minus: 3 }}", &ctx).unwrap();
        assert_eq!(out, "-3");
    }

    #[test]
    fn test_times_float() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert("x".into(), liquid::model::Value::scalar(2.5f64));
        let out = eng.parse_and_render("{{ x | times: 4 }}", &ctx).unwrap();
        assert_eq!(out, "10");
    }
}
