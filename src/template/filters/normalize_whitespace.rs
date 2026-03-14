use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Collapse multiple whitespace characters into a single space and trim.
///
/// Matches Jekyll's `normalize_whitespace` filter: replaces all runs of
/// whitespace (spaces, tabs, newlines, carriage returns) with a single space,
/// and trims leading/trailing whitespace.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "normalize_whitespace",
    description = "Collapse multiple whitespace characters into a single space and trim.",
    parsed(NormalizeWhitespaceFilter)
)]
pub struct NormalizeWhitespace;

#[derive(Debug, Default, Display_filter)]
#[name = "normalize_whitespace"]
struct NormalizeWhitespaceFilter;

impl Filter for NormalizeWhitespaceFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let text = input.to_kstr();
        // Split on whitespace (handles spaces, tabs, newlines, carriage returns)
        // and rejoin with single spaces. This also trims leading/trailing whitespace.
        let result: String = text.split_whitespace().collect::<Vec<&str>>().join(" ");
        Ok(Value::scalar(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiple_spaces_between_words() {
        let result = liquid_core::call_filter!(NormalizeWhitespace, "hello   world").unwrap();
        assert_eq!(result.to_kstr(), "hello world");
    }

    #[test]
    fn test_tabs_and_newlines() {
        let result =
            liquid_core::call_filter!(NormalizeWhitespace, "hello\t\tworld\n\nfoo").unwrap();
        assert_eq!(result.to_kstr(), "hello world foo");
    }

    #[test]
    fn test_leading_trailing_whitespace_trimmed() {
        let result = liquid_core::call_filter!(NormalizeWhitespace, "  hello   world  ").unwrap();
        assert_eq!(result.to_kstr(), "hello world");
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(NormalizeWhitespace, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_already_clean() {
        let result = liquid_core::call_filter!(NormalizeWhitespace, "already clean").unwrap();
        assert_eq!(result.to_kstr(), "already clean");
    }

    #[test]
    fn test_only_whitespace() {
        let result = liquid_core::call_filter!(NormalizeWhitespace, "   \t\n  ").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_mixed_whitespace() {
        let result =
            liquid_core::call_filter!(NormalizeWhitespace, "  hello   world\n\t foo  ").unwrap();
        assert_eq!(result.to_kstr(), "hello world foo");
    }

    #[test]
    fn test_carriage_return() {
        let result = liquid_core::call_filter!(NormalizeWhitespace, "hello\r\nworld").unwrap();
        assert_eq!(result.to_kstr(), "hello world");
    }
}
