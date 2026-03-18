use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

use super::{format_date_to_xmlschema, get_site_timezone};

/// Format a date as ISO 8601 with timezone (e.g., "2024-01-15T00:00:00+00:00").
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "date_to_xmlschema",
    description = "Format a date as ISO 8601 with timezone.",
    parsed(DateToXmlschemaFilter)
)]
pub struct DateToXmlschema;

#[derive(Debug, Default, Display_filter)]
#[name = "date_to_xmlschema"]
struct DateToXmlschemaFilter;

impl Filter for DateToXmlschemaFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        // If input is nil, return nil so that `nil | date_to_xmlschema | jsonify`
        // produces `null` (not `""`), matching Jekyll's behavior.
        if input.is_nil() {
            return Ok(Value::Nil);
        }
        let input_str = input.to_kstr();
        let s = input_str.trim();
        if s.is_empty() {
            return Ok(Value::scalar(String::new()));
        }

        let site_tz = get_site_timezone(runtime);
        Ok(Value::scalar(format_date_to_xmlschema(s, site_tz)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_only() {
        let result = liquid_core::call_filter!(DateToXmlschema, "2024-01-15").unwrap();
        assert_eq!(result.to_kstr(), "2024-01-15T00:00:00+00:00");
    }

    #[test]
    fn test_datetime_with_space() {
        let result = liquid_core::call_filter!(DateToXmlschema, "2024-03-22 10:00:00").unwrap();
        assert!(result.to_kstr().contains("2024-03-22T10:00:00"));
    }

    #[test]
    fn test_already_iso() {
        let result =
            liquid_core::call_filter!(DateToXmlschema, "2024-01-15T14:30:00+00:00").unwrap();
        assert_eq!(result.to_kstr(), "2024-01-15T14:30:00+00:00");
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(DateToXmlschema, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_invalid_date_passthrough() {
        let result = liquid_core::call_filter!(DateToXmlschema, "not-a-date").unwrap();
        assert_eq!(result.to_kstr(), "not-a-date");
    }

    #[test]
    fn test_nil_input_returns_nil() {
        // When page.date is nil, date_to_xmlschema should return nil (not ""),
        // so that `nil | date_to_xmlschema | jsonify` produces `null`.
        let input = Value::Nil;
        let result = liquid_core::call_filter!(DateToXmlschema, input).unwrap();
        assert_eq!(
            result,
            Value::Nil,
            "nil input should produce nil output, not empty string"
        );
    }

    #[test]
    fn test_nil_date_through_jsonify_produces_null() {
        // Full pipeline: nil | date_to_xmlschema | jsonify should produce "null"
        // This matches Jekyll's behavior for pages without a date field (e.g.,
        // slack/guidelines.html which has layout: post but no date).
        use super::super::jsonify::Jsonify;
        let nil_input = Value::Nil;
        let xmlschema_result = liquid_core::call_filter!(DateToXmlschema, nil_input).unwrap();
        let jsonify_result = liquid_core::call_filter!(Jsonify, xmlschema_result).unwrap();
        assert_eq!(
            jsonify_result.to_kstr().as_str(),
            "null",
            "nil | date_to_xmlschema | jsonify should produce 'null', not '\"\"'"
        );
    }
}
