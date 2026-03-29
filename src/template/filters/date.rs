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
        } else if format_str.contains("%s") {
            // %s (Unix epoch seconds) is special: it requires the true UTC instant,
            // not the naive local time. parse_date_string_with_tz strips timezone
            // info via naive_local(), which would make %s compute the wrong epoch.
            // Use the timezone-aware DateTime directly when available.
            format_with_epoch_aware(s, &format_str, site_tz).unwrap_or_else(|| s.to_string())
        } else if let Some(dt) = parse_date_string_with_tz(s, site_tz) {
            safe_chrono_format(&dt.format(&format_str)).unwrap_or_else(|| s.to_string())
        } else {
            // If we can't parse it, return the input as-is (Jekyll behavior)
            s.to_string()
        };

        Ok(Value::scalar(result))
    }
}

/// Format a date string using a format that contains `%s` (Unix epoch seconds).
///
/// When the input has an explicit timezone offset, we must use the timezone-aware
/// `DateTime<FixedOffset>` so that `%s` computes the correct epoch. For naive
/// datetimes (no timezone), we treat them as UTC per Ruby YAML convention.
fn format_with_epoch_aware(
    s: &str,
    format_str: &str,
    _site_tz: Option<chrono_tz::Tz>,
) -> Option<String> {
    // Try timezone-aware formats first
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return safe_chrono_format(&dt.format(format_str));
    }
    if let Ok(dt) = chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S %z") {
        return safe_chrono_format(&dt.format(format_str));
    }
    // For naive datetimes, fall back to the standard path (treats as UTC)
    parse_date_string_with_tz(s, _site_tz).and_then(|dt| safe_chrono_format(&dt.format(format_str)))
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

    // Issue 187: Leading zero padding tests

    #[test]
    fn test_date_format_month_leading_zero() {
        // Slash-separated date with single-digit month (muan-blog style)
        let result = liquid_core::call_filter!(Date, "2023/7/11 15:27", "%m").unwrap();
        assert_eq!(result.to_kstr(), "07");
    }

    #[test]
    fn test_date_format_day_leading_zero() {
        let result = liquid_core::call_filter!(Date, "2023/7/5 15:27", "%d").unwrap();
        assert_eq!(result.to_kstr(), "05");
    }

    #[test]
    fn test_date_format_hour_leading_zero() {
        let result = liquid_core::call_filter!(Date, "2025/8/9 2:10", "%H").unwrap();
        assert_eq!(result.to_kstr(), "02");
    }

    #[test]
    fn test_date_format_combined_slash_date() {
        // Full muan-blog format: input "2023/7/11 15:27", format "%Y/%m/%d %H:%M"
        let result = liquid_core::call_filter!(Date, "2023/7/11 15:27", "%Y/%m/%d %H:%M").unwrap();
        assert_eq!(result.to_kstr(), "2023/07/11 15:27");
    }

    #[test]
    fn test_date_format_double_digit_no_change() {
        let result = liquid_core::call_filter!(Date, "2023/12/25 10:30", "%m/%d").unwrap();
        assert_eq!(result.to_kstr(), "12/25");
    }

    #[test]
    fn test_date_format_year_unchanged() {
        let result = liquid_core::call_filter!(Date, "2023/7/11 15:27", "%Y").unwrap();
        assert_eq!(result.to_kstr(), "2023");
    }

    #[test]
    fn test_date_format_textual_month() {
        let result = liquid_core::call_filter!(Date, "2023/7/11 15:27", "%B").unwrap();
        assert_eq!(result.to_kstr(), "July");
    }

    // Chirpy: %s (Unix epoch seconds) must use UTC conversion for timezone-aware dates
    #[test]
    fn test_epoch_seconds_utc_positive_offset() {
        // 2019-08-11 00:34:00 +0800 = 2019-08-10 16:34:00 UTC = 1565454840
        let result = liquid_core::call_filter!(Date, "2019-08-11 00:34:00 +0800", "%s").unwrap();
        assert_eq!(result.to_kstr(), "1565454840");
    }

    #[test]
    fn test_epoch_seconds_utc_zero_offset() {
        // 2019-08-10 16:34:00 +0000 = 1565454840
        let result = liquid_core::call_filter!(Date, "2019-08-10 16:34:00 +0000", "%s").unwrap();
        assert_eq!(result.to_kstr(), "1565454840");
    }

    #[test]
    fn test_epoch_seconds_naive_datetime() {
        // Naive datetime treated as UTC: 2019-08-10 16:34:00 = 1565454840
        let result = liquid_core::call_filter!(Date, "2019-08-10 16:34:00", "%s").unwrap();
        assert_eq!(result.to_kstr(), "1565454840");
    }
}
