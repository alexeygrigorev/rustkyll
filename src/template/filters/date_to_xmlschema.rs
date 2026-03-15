use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

use super::{get_site_timezone, parse_date_string_with_tz, safe_chrono_format};

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
        let input_str = input.to_kstr();
        let s = input_str.trim();
        if s.is_empty() {
            return Ok(Value::scalar(String::new()));
        }

        // If the input already has timezone info, try to preserve it
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Ok(Value::scalar(
                safe_chrono_format(&dt.format("%Y-%m-%dT%H:%M:%S%:z"))
                    .unwrap_or_else(|| s.to_string()),
            ));
        }

        let site_tz = get_site_timezone(runtime);
        match parse_date_string_with_tz(s, site_tz) {
            Some(dt) => {
                // Default timezone is UTC (+00:00)
                let formatted = safe_chrono_format(&dt.format("%Y-%m-%dT%H:%M:%S"))
                    .map(|f| format!("{}+00:00", f))
                    .unwrap_or_else(|| s.to_string());
                Ok(Value::scalar(formatted))
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
}
