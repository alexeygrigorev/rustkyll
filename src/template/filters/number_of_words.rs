use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Count the number of words in a string.
///
/// Splits on whitespace and counts non-empty segments, matching Jekyll's
/// `number_of_words` filter behavior.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "number_of_words",
    description = "Count the number of words in a string.",
    parsed(NumberOfWordsFilter)
)]
pub struct NumberOfWords;

#[derive(Debug, Default, Display_filter)]
#[name = "number_of_words"]
struct NumberOfWordsFilter;

impl Filter for NumberOfWordsFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let text = input.to_kstr();
        let count = text.split_whitespace().count() as i64;
        Ok(Value::scalar(count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_words() {
        let result = liquid_core::call_filter!(NumberOfWords, "Hello world").unwrap();
        assert_eq!(result.to_kstr(), "2");
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(NumberOfWords, "").unwrap();
        assert_eq!(result.to_kstr(), "0");
    }

    #[test]
    fn test_extra_whitespace() {
        let result =
            liquid_core::call_filter!(NumberOfWords, "  spaces  between  words  ").unwrap();
        assert_eq!(result.to_kstr(), "3");
    }

    #[test]
    fn test_single_word() {
        let result = liquid_core::call_filter!(NumberOfWords, "single").unwrap();
        assert_eq!(result.to_kstr(), "1");
    }

    #[test]
    fn test_tabs_and_newlines() {
        let result = liquid_core::call_filter!(NumberOfWords, "one\ntwo\tthree").unwrap();
        assert_eq!(result.to_kstr(), "3");
    }
}
