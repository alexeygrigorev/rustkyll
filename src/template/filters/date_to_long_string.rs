//! The `date_to_long_string` filter.
//!
//! Formats a date as "DD Month YYYY" (e.g., "27 March 2013").
//! With `"ordinal"` argument: "27th March 2013".

use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

use super::{
    convert_utc_naive_to_site_tz, get_site_timezone, is_naive_yaml_timestamp,
    parse_date_string_with_tz, safe_chrono_format,
};

#[derive(Debug, FilterParameters)]
struct DateToLongStringArgs {
    #[parameter(
        description = "Format type: 'ordinal' for ordinal day suffix.",
        arg_type = "str"
    )]
    format_type: Option<Expression>,
}

/// Format a date as "DD Month YYYY" (e.g., "27 March 2013").
/// With `"ordinal"` argument: "27th March 2013".
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "date_to_long_string",
    description = "Format a date as DD Month YYYY.",
    parameters(DateToLongStringArgs),
    parsed(DateToLongStringFilter)
)]
pub struct DateToLongString;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "date_to_long_string"]
struct DateToLongStringFilter {
    #[parameters]
    args: DateToLongStringArgs,
}

/// Return the English ordinal suffix for a day number.
fn ordinal_suffix(day: u32) -> &'static str {
    match day {
        11..=13 => "th",
        _ => match day % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    }
}

impl Filter for DateToLongStringFilter {
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

        let site_tz = get_site_timezone(runtime);
        match parse_date_string_with_tz(s, site_tz) {
            Some(dt) => {
                // Jekyll's date_to_long_string calls Time#localtime which converts
                // UTC timestamps to the local timezone. Only for naive YAML timestamps.
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
                let month =
                    safe_chrono_format(&dt.format("%B")).unwrap_or_else(|| "???".to_string());
                let year =
                    safe_chrono_format(&dt.format("%Y")).unwrap_or_else(|| "????".to_string());
                if is_ordinal {
                    let suffix = ordinal_suffix(day);
                    Ok(Value::scalar(format!(
                        "{}{} {} {}",
                        day, suffix, month, year
                    )))
                } else {
                    // Jekyll uses %d which zero-pads single-digit days (e.g., "07 November")
                    Ok(Value::scalar(format!("{:02} {} {}", day, month, year)))
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
    fn test_date_to_long_string_basic() {
        let result = liquid_core::call_filter!(DateToLongString, "2013-03-27").unwrap();
        assert_eq!(result.to_kstr(), "27 March 2013");
    }

    #[test]
    fn test_date_to_long_string_september() {
        let result = liquid_core::call_filter!(DateToLongString, "2001-09-11").unwrap();
        assert_eq!(result.to_kstr(), "11 September 2001");
    }

    #[test]
    fn test_date_to_long_string_january() {
        // Jekyll's %d produces zero-padded days
        let result = liquid_core::call_filter!(DateToLongString, "2024-01-01").unwrap();
        assert_eq!(result.to_kstr(), "01 January 2024");
    }

    #[test]
    fn test_date_to_long_string_december() {
        let result = liquid_core::call_filter!(DateToLongString, "2024-12-25").unwrap();
        assert_eq!(result.to_kstr(), "25 December 2024");
    }

    #[test]
    fn test_date_to_long_string_iso_datetime() {
        let result =
            liquid_core::call_filter!(DateToLongString, "2024-06-15T10:30:00+00:00").unwrap();
        assert_eq!(result.to_kstr(), "15 June 2024");
    }

    #[test]
    fn test_date_to_long_string_empty() {
        let result = liquid_core::call_filter!(DateToLongString, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_date_to_long_string_invalid_passthrough() {
        let result = liquid_core::call_filter!(DateToLongString, "not-a-date").unwrap();
        assert_eq!(result.to_kstr(), "not-a-date");
    }

    #[test]
    fn test_date_to_long_string_ordinal() {
        let result = liquid_core::call_filter!(DateToLongString, "2013-03-27", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "27th March 2013");
    }

    #[test]
    fn test_date_to_long_string_ordinal_1st() {
        let result = liquid_core::call_filter!(DateToLongString, "2024-01-01", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "1st January 2024");
    }

    #[test]
    fn test_date_to_long_string_ordinal_2nd() {
        let result = liquid_core::call_filter!(DateToLongString, "2024-01-02", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "2nd January 2024");
    }

    #[test]
    fn test_date_to_long_string_ordinal_3rd() {
        let result = liquid_core::call_filter!(DateToLongString, "2024-01-03", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "3rd January 2024");
    }

    #[test]
    fn test_date_to_long_string_ordinal_11th() {
        let result = liquid_core::call_filter!(DateToLongString, "2024-01-11", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "11th January 2024");
    }

    #[test]
    fn test_date_to_long_string_ordinal_12th() {
        let result = liquid_core::call_filter!(DateToLongString, "2024-01-12", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "12th January 2024");
    }

    #[test]
    fn test_date_to_long_string_ordinal_13th() {
        let result = liquid_core::call_filter!(DateToLongString, "2024-01-13", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "13th January 2024");
    }

    #[test]
    fn test_date_to_long_string_ordinal_21st() {
        let result = liquid_core::call_filter!(DateToLongString, "2024-01-21", "ordinal").unwrap();
        assert_eq!(result.to_kstr(), "21st January 2024");
    }

    #[test]
    fn test_ordinal_suffix() {
        assert_eq!(ordinal_suffix(1), "st");
        assert_eq!(ordinal_suffix(2), "nd");
        assert_eq!(ordinal_suffix(3), "rd");
        assert_eq!(ordinal_suffix(4), "th");
        assert_eq!(ordinal_suffix(11), "th");
        assert_eq!(ordinal_suffix(12), "th");
        assert_eq!(ordinal_suffix(13), "th");
        assert_eq!(ordinal_suffix(21), "st");
        assert_eq!(ordinal_suffix(22), "nd");
        assert_eq!(ordinal_suffix(23), "rd");
        assert_eq!(ordinal_suffix(31), "st");
    }
}
