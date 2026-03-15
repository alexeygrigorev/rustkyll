use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

use super::{parse_date_string, safe_chrono_format};

/// Format a date as "DD Mon YYYY" (e.g., "01 Jan 2024").
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "date_to_string",
    description = "Format a date as DD Mon YYYY.",
    parsed(DateToStringFilter)
)]
pub struct DateToString;

#[derive(Debug, Default, Display_filter)]
#[name = "date_to_string"]
struct DateToStringFilter;

impl Filter for DateToStringFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let input_str = input.to_kstr();
        let s = input_str.trim();
        if s.is_empty() {
            return Ok(Value::scalar(String::new()));
        }

        match parse_date_string(s) {
            Some(dt) => Ok(Value::scalar(
                safe_chrono_format(&dt.format("%d %b %Y")).unwrap_or_else(|| s.to_string()),
            )),
            None => Ok(Value::scalar(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_only() {
        let result = liquid_core::call_filter!(DateToString, "2024-01-15").unwrap();
        assert_eq!(result.to_kstr(), "15 Jan 2024");
    }

    #[test]
    fn test_datetime_with_space() {
        let result = liquid_core::call_filter!(DateToString, "2024-03-22 10:00:00").unwrap();
        assert_eq!(result.to_kstr(), "22 Mar 2024");
    }

    #[test]
    fn test_iso_datetime() {
        let result = liquid_core::call_filter!(DateToString, "2024-12-01T14:30:00+00:00").unwrap();
        assert_eq!(result.to_kstr(), "01 Dec 2024");
    }

    #[test]
    fn test_months() {
        assert_eq!(
            liquid_core::call_filter!(DateToString, "2024-01-01")
                .unwrap()
                .to_kstr(),
            "01 Jan 2024"
        );
        assert_eq!(
            liquid_core::call_filter!(DateToString, "2024-06-15")
                .unwrap()
                .to_kstr(),
            "15 Jun 2024"
        );
        assert_eq!(
            liquid_core::call_filter!(DateToString, "2024-09-30")
                .unwrap()
                .to_kstr(),
            "30 Sep 2024"
        );
        assert_eq!(
            liquid_core::call_filter!(DateToString, "2024-12-25")
                .unwrap()
                .to_kstr(),
            "25 Dec 2024"
        );
    }

    #[test]
    fn test_day_with_leading_zero() {
        let result = liquid_core::call_filter!(DateToString, "2024-01-01").unwrap();
        assert_eq!(result.to_kstr(), "01 Jan 2024");
    }

    #[test]
    fn test_invalid_date_passthrough() {
        let result = liquid_core::call_filter!(DateToString, "not-a-date").unwrap();
        assert_eq!(result.to_kstr(), "not-a-date");
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(DateToString, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }
}
