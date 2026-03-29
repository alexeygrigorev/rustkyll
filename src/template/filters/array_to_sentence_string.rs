//! Nil-tolerant `array_to_sentence_string` filter.
//!
//! The upstream `liquid-lib` version errors when the input is nil (not an array).
//! Jekyll silently returns an empty string for nil inputs. This override
//! matches Jekyll's behavior.

use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Expression, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};
use std::fmt::Write;

#[derive(Clone, Debug)]
pub struct ArrayToSentenceString;

impl FilterReflection for ArrayToSentenceString {
    fn name(&self) -> &str {
        "array_to_sentence_string"
    }
    fn description(&self) -> &str {
        "Converts an array into a sentence (nil-tolerant)."
    }
    fn positional_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &[liquid_core::parser::ParameterReflection {
            name: "connector",
            description: "Word used to connect the last two items (default: 'and')",
            is_optional: true,
        }]
    }
    fn keyword_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &[]
    }
}

impl ParseFilter for ArrayToSentenceString {
    fn parse(&self, arguments: liquid_core::parser::FilterArguments) -> Result<Box<dyn Filter>> {
        let mut args = arguments;
        let connector = args.positional.next();
        Ok(Box::new(ArrayToSentenceStringFilter { connector }))
    }
    fn reflection(&self) -> &dyn FilterReflection {
        self
    }
}

#[derive(Debug)]
struct ArrayToSentenceStringFilter {
    connector: Option<Expression>,
}

impl std::fmt::Display for ArrayToSentenceStringFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "array_to_sentence_string")
    }
}

impl Filter for ArrayToSentenceStringFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        // Nil input -> empty string (matching Jekyll behavior)
        let Some(arr) = input.as_array() else {
            return Ok(Value::scalar(""));
        };

        let connector = match &self.connector {
            Some(expr) => {
                let val = expr.evaluate(runtime)?;
                val.to_kstr().to_string()
            }
            None => "and".to_string(),
        };

        let mut iter = arr.values();

        let mut sentence = iter
            .next()
            .map(|v| v.to_kstr().into_string())
            .unwrap_or_default();

        let mut peekable = iter.peekable();
        while let Some(value) = peekable.next() {
            if peekable.peek().is_some() {
                write!(sentence, ", {}", value.render()).expect("safe to write to string");
            } else {
                write!(sentence, ", {} {}", connector, value.render())
                    .expect("safe to write to string");
            }
        }

        Ok(Value::scalar(sentence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nil_input_returns_empty() {
        let result = liquid_core::call_filter!(ArrayToSentenceString, liquid_core::Value::Nil);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_kstr().as_str(), "");
    }

    #[test]
    fn test_empty_array() {
        let input = liquid_core::value!([]);
        let result = liquid_core::call_filter!(ArrayToSentenceString, input).unwrap();
        assert_eq!(result.to_kstr().as_str(), "");
    }

    #[test]
    fn test_single_element() {
        let input = liquid_core::value!(["release"]);
        let result = liquid_core::call_filter!(ArrayToSentenceString, input).unwrap();
        assert_eq!(result.to_kstr().as_str(), "release");
    }

    #[test]
    fn test_two_elements() {
        let input = liquid_core::value!(["release", "update"]);
        let result = liquid_core::call_filter!(ArrayToSentenceString, input).unwrap();
        assert_eq!(result.to_kstr().as_str(), "release, and update");
    }

    #[test]
    fn test_three_elements() {
        let input = liquid_core::value!(["a", "b", "c"]);
        let result = liquid_core::call_filter!(ArrayToSentenceString, input).unwrap();
        assert_eq!(result.to_kstr().as_str(), "a, b, and c");
    }
}
