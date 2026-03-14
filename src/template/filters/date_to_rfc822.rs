use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

use super::parse_date_string;

/// Format a date as RFC 822 (e.g., "Mon, 01 Jan 2024 00:00:00 +0000").
///
/// This matches Jekyll's `date_to_rfc822` filter used in RSS/podcast feeds.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "date_to_rfc822",
    description = "Format a date as RFC 822 (e.g., Mon, 01 Jan 2024 00:00:00 +0000).",
    parsed(DateToRfc822Filter)
)]
pub struct DateToRfc822;

#[derive(Debug, Default, Display_filter)]
#[name = "date_to_rfc822"]
struct DateToRfc822Filter;

impl Filter for DateToRfc822Filter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let input_str = input.to_kstr();
        let s = input_str.trim();
        if s.is_empty() {
            return Ok(Value::scalar(String::new()));
        }

        match parse_date_string(s) {
            Some(dt) => {
                // Format as RFC 822: "Mon, 01 Jan 2024 00:00:00 +0000"
                // NaiveDateTime has no timezone info, so we assume UTC (+0000).
                Ok(Value::scalar(
                    dt.format("%a, %d %b %Y %H:%M:%S +0000").to_string(),
                ))
            }
            None => Ok(Value::scalar(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_only() {
        let result = liquid_core::call_filter!(DateToRfc822, "2024-01-15").unwrap();
        assert_eq!(result.to_kstr(), "Mon, 15 Jan 2024 00:00:00 +0000");
    }

    #[test]
    fn test_datetime_with_space() {
        let result = liquid_core::call_filter!(DateToRfc822, "2024-03-22 10:00:00").unwrap();
        assert_eq!(result.to_kstr(), "Fri, 22 Mar 2024 10:00:00 +0000");
    }

    #[test]
    fn test_iso_datetime() {
        let result = liquid_core::call_filter!(DateToRfc822, "2024-12-01T14:30:00+00:00").unwrap();
        assert_eq!(result.to_kstr(), "Sun, 01 Dec 2024 14:30:00 +0000");
    }

    #[test]
    fn test_invalid_date_passthrough() {
        let result = liquid_core::call_filter!(DateToRfc822, "not-a-date").unwrap();
        assert_eq!(result.to_kstr(), "not-a-date");
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(DateToRfc822, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }
}
