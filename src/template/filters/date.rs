//! Custom `date` filter that handles date-only strings (`YYYY-MM-DD`).
//!
//! The liquid crate's built-in `date` filter does not parse `YYYY-MM-DD` strings
//! (it expects timestamps or RFC-style datetime). Jekyll's `date` filter uses
//! Ruby's `Date.parse` which handles bare dates.
//!
//! This filter replaces the built-in one to match Jekyll's behavior.

use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

use super::{get_site_timezone, parse_date_string_with_tz, safe_chrono_format};

#[derive(Debug, FilterParameters)]
struct DateArgs {
    #[parameter(description = "strftime format string", arg_type = "str")]
    format: Expression,
}

/// Custom `date` filter matching Jekyll's behavior for date-only strings.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "date",
    description = "Format a date using strftime format string.",
    parameters(DateArgs),
    parsed(DateFilter)
)]
pub struct Date;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "date"]
struct DateFilter {
    #[parameters]
    args: DateArgs,
}

impl Filter for DateFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;
        let format_str = args.format.to_kstr();

        let input_str = input.to_kstr();
        let s = input_str.trim();

        if s.is_empty() {
            return Ok(Value::scalar(String::new()));
        }

        let site_tz = get_site_timezone(runtime);

        // Handle "now" / "today" like Jekyll
        let result = if s.eq_ignore_ascii_case("now") || s.eq_ignore_ascii_case("today") {
            safe_chrono_format(&chrono::Local::now().naive_local().format(&format_str))
                .unwrap_or_else(|| s.to_string())
        } else if let Some(dt) = parse_date_string_with_tz(s, site_tz) {
            safe_chrono_format(&dt.format(&format_str)).unwrap_or_else(|| s.to_string())
        } else {
            // If we can't parse it, return the input as-is (Jekyll behavior)
            s.to_string()
        };

        Ok(Value::scalar(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_format_day_month_year() {
        let result = liquid_core::call_filter!(Date, "2024-07-24", "%d.%m.%Y").unwrap();
        assert_eq!(result.to_kstr(), "24.07.2024");
    }

    #[test]
    fn test_date_format_year_only() {
        let result = liquid_core::call_filter!(Date, "2024-07-24", "%Y").unwrap();
        assert_eq!(result.to_kstr(), "2024");
    }

    #[test]
    fn test_date_format_full_datetime() {
        let result =
            liquid_core::call_filter!(Date, "2024-01-15 10:30:00", "%d.%m.%Y %H:%M").unwrap();
        assert_eq!(result.to_kstr(), "15.01.2024 10:30");
    }

    #[test]
    fn test_date_format_iso_datetime() {
        let result =
            liquid_core::call_filter!(Date, "2024-12-01T14:30:00+00:00", "%d %b %Y").unwrap();
        assert_eq!(result.to_kstr(), "01 Dec 2024");
    }

    #[test]
    fn test_date_format_empty_string() {
        let result = liquid_core::call_filter!(Date, "", "%d.%m.%Y").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_date_format_invalid_passthrough() {
        let result = liquid_core::call_filter!(Date, "not-a-date", "%d.%m.%Y").unwrap();
        assert_eq!(result.to_kstr(), "not-a-date");
    }

    #[test]
    fn test_date_format_now_does_not_panic() {
        // "now" with a normal format should produce something (exact value depends on time)
        let result = liquid_core::call_filter!(Date, "now", "%Y-%m-%d").unwrap();
        assert!(!result.to_kstr().is_empty());
    }

    #[test]
    fn test_date_format_problematic_specifier_no_panic() {
        // %Z on NaiveDateTime has no timezone info -- chrono's Display can error.
        // The filter must NOT panic; it should return the input as-is.
        let result = liquid_core::call_filter!(Date, "2024-07-24", "%Z").unwrap();
        // Either a valid result or fallback to input -- but no panic
        let s = result.to_kstr();
        assert!(!s.is_empty());
    }

    #[test]
    fn test_date_format_timezone_specifiers_no_panic() {
        // Multiple timezone-related specifiers that can fail on NaiveDateTime
        for fmt in &["%z", "%:z", "%Z", "%+"] {
            let result = liquid_core::call_filter!(Date, "2024-01-15", *fmt).unwrap();
            let s = result.to_kstr();
            // Should not panic; either formatted or fallback
            assert!(!s.is_empty(), "Format {} produced empty result", fmt);
        }
    }
}
