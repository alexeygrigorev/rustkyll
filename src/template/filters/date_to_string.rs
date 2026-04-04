use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

use super::date_to_long_string::ordinal_suffix;
use super::{
    convert_utc_naive_to_site_tz, get_site_timezone, is_naive_yaml_timestamp,
    parse_date_string_with_tz, safe_chrono_format,
};

#[derive(Debug, FilterParameters)]
struct DateToStringArgs {
    #[parameter(
        description = "Format type: 'ordinal' for ordinal day suffix.",
        arg_type = "str"
    )]
    format_type: Option<Expression>,
    #[parameter(
        description = "Date style: 'US' for month-first format.",
        arg_type = "str"
    )]
    style: Option<Expression>,
}

/// Format a date as "DD Mon YYYY" (e.g., "01 Jan 2024").
/// With `"ordinal"` argument: "15th Jan 2024".
/// With `"ordinal", "US"` arguments: "Jan 15th, 2024".
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "date_to_string",
    description = "Format a date as DD Mon YYYY.",
    parameters(DateToStringArgs),
    parsed(DateToStringFilter)
)]
pub struct DateToString;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "date_to_string"]
struct DateToStringFilter {
    #[parameters]
    args: DateToStringArgs,
}

impl Filter for DateToStringFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;

        let input_str = input.to_kstr();
        let s = input_str.trim();
        if s.is_empty() {
            return Ok(Value::scalar(String::new()));
        }

        let is_ordinal = args
            .format_type
            .as_ref()
            .map(|v| v.to_kstr().as_ref() == "ordinal")
            .unwrap_or(false);

        let is_us = args
            .style
            .as_ref()
            .map(|v| v.to_kstr().as_ref() == "US")
            .unwrap_or(false);

        let site_tz = get_site_timezone(runtime);
        match parse_date_string_with_tz(s, site_tz) {
            Some(dt) => {
                // Jekyll's date_to_string calls Time#localtime which converts
                // UTC timestamps to the local timezone. Only apply this for
                // naive YAML timestamps (no explicit timezone offset).
                let dt = if is_naive_yaml_timestamp(s) {
                    convert_utc_naive_to_site_tz(dt, site_tz)
                } else {
                    dt
                };
                let day = safe_chrono_format(&dt.format("%e"))
                    .unwrap_or_default()
                    .trim()
                    .parse::<u32>()
                    .unwrap_or(0);
                let month_abbr =
                    safe_chrono_format(&dt.format("%b")).unwrap_or_else(|| "???".to_string());
                let year =
                    safe_chrono_format(&dt.format("%Y")).unwrap_or_else(|| "????".to_string());

                if is_ordinal && is_us {
                    let suffix = ordinal_suffix(day);
                    Ok(Value::scalar(format!(
                        "{} {}{}, {}",
                        month_abbr, day, suffix, year
                    )))
                } else if is_ordinal {
                    let suffix = ordinal_suffix(day);
                    Ok(Value::scalar(format!(
                        "{}{} {} {}",
                        day, suffix, month_abbr, year
                    )))
                } else {
                    Ok(Value::scalar(
                        safe_chrono_format(&dt.format("%d %b %Y")).unwrap_or_else(|| s.to_string()),
                    ))
                }
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

    // ========================================================================
    // Ordinal and US-style formatting
    // ========================================================================

    #[test]
    fn test_date_to_string_ordinal() {
        let result = liquid_core::call_filter!(DateToString, "2024-01-15", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "15th Jan 2024");
    }

    #[test]
    fn test_date_to_string_ordinal_1st() {
        let result = liquid_core::call_filter!(DateToString, "2024-01-01", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "1st Jan 2024");
    }

    #[test]
    fn test_date_to_string_ordinal_2nd() {
        let result = liquid_core::call_filter!(DateToString, "2024-01-02", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "2nd Jan 2024");
    }

    #[test]
    fn test_date_to_string_ordinal_3rd() {
        let result = liquid_core::call_filter!(DateToString, "2024-01-03", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "3rd Jan 2024");
    }

    #[test]
    fn test_date_to_string_ordinal_11th() {
        let result = liquid_core::call_filter!(DateToString, "2024-01-11", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "11th Jan 2024");
    }

    #[test]
    fn test_date_to_string_ordinal_us() {
        let result =
            liquid_core::call_filter!(DateToString, "2024-01-15", "ordinal", "US").unwrap();
        assert_eq!(result.to_kstr(), "Jan 15th, 2024");
    }

    #[test]
    fn test_date_to_string_ordinal_us_1st() {
        let result =
            liquid_core::call_filter!(DateToString, "2024-01-01", "ordinal", "US").unwrap();
        assert_eq!(result.to_kstr(), "Jan 1st, 2024");
    }

    #[test]
    fn test_date_to_string_ordinal_us_31st() {
        let result =
            liquid_core::call_filter!(DateToString, "2024-01-31", "ordinal", "US").unwrap();
        assert_eq!(result.to_kstr(), "Jan 31st, 2024");
    }

    // ========================================================================
    // D10: Timezone handling -- use naive_local, not naive_utc
    // ========================================================================

    #[test]
    fn test_d10_positive_offset_preserves_date() {
        // "2023-10-11 00:00:00 +0200" should remain Oct 11, not become Oct 10 (UTC)
        let result = liquid_core::call_filter!(DateToString, "2023-10-11 00:00:00 +0200").unwrap();
        assert_eq!(result.to_kstr(), "11 Oct 2023");
    }

    #[test]
    fn test_d10_negative_offset_preserves_date() {
        // "2024-06-15 23:30:00 -0500" should remain Jun 15, not become Jun 16 (UTC)
        let result = liquid_core::call_filter!(DateToString, "2024-06-15 23:30:00 -0500").unwrap();
        assert_eq!(result.to_kstr(), "15 Jun 2024");
    }

    #[test]
    fn test_d10_utc_offset_unchanged() {
        let result = liquid_core::call_filter!(DateToString, "2024-12-31 23:00:00 +0000").unwrap();
        assert_eq!(result.to_kstr(), "31 Dec 2024");
    }

    #[test]
    fn test_d10_rfc3339_positive_offset() {
        let result = liquid_core::call_filter!(DateToString, "2024-03-22T10:00:00+00:00").unwrap();
        assert_eq!(result.to_kstr(), "22 Mar 2024");
    }

    #[test]
    fn test_d10_date_only_no_tz() {
        let result = liquid_core::call_filter!(DateToString, "2024-01-15").unwrap();
        assert_eq!(result.to_kstr(), "15 Jan 2024");
    }
}
