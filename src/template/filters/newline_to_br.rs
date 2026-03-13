use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Convert newline characters to `<br />` tags.
///
/// Matches Jekyll's `newline_to_br` behavior: each `\n` is replaced with
/// `<br />\n` (the original newline is preserved after the tag).
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "newline_to_br",
    description = "Convert newline characters to HTML <br /> tags.",
    parsed(NewlineToBrFilter)
)]
pub struct NewlineToBr;

#[derive(Debug, Default, Display_filter)]
#[name = "newline_to_br"]
struct NewlineToBrFilter;

impl Filter for NewlineToBrFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let text = input.to_kstr();
        // Normalize \r\n to \n first, then replace all \n with <br />\n
        let normalized = text.replace("\r\n", "\n");
        let result = normalized.replace('\n', "<br />\n");
        Ok(Value::scalar(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_newline() {
        let result = liquid_core::call_filter!(NewlineToBr, "hello\nworld").unwrap();
        assert_eq!(result.to_kstr(), "hello<br />\nworld");
    }

    #[test]
    fn test_no_newlines() {
        let result = liquid_core::call_filter!(NewlineToBr, "no newlines").unwrap();
        assert_eq!(result.to_kstr(), "no newlines");
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(NewlineToBr, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_multiple_newlines() {
        let result = liquid_core::call_filter!(NewlineToBr, "line1\nline2\nline3").unwrap();
        let s = result.to_kstr().to_string();
        assert_eq!(s.matches("<br />").count(), 2);
        assert_eq!(s, "line1<br />\nline2<br />\nline3");
    }

    #[test]
    fn test_windows_line_endings() {
        let result = liquid_core::call_filter!(NewlineToBr, "hello\r\nworld").unwrap();
        let s = result.to_kstr().to_string();
        assert!(s.contains("<br />"), "Should contain <br /> tag");
        // Should not double-convert
        assert_eq!(s.matches("<br />").count(), 1);
    }
}
