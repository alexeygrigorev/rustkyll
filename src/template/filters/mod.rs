//! Custom Liquid filters for Jekyll compatibility.
//!
//! These 6 filters are NOT provided by the `liquid` crate or `liquid-lib`'s
//! jekyll feature, but are needed by common Jekyll site templates.

mod absolute_url;
mod date;
mod date_to_long_string;
mod date_to_rfc822;
mod date_to_string;
mod date_to_xmlschema;
mod group_by;
mod group_by_exp;
mod jsonify;
mod markdownify;
mod newline_to_br;
mod normalize_whitespace;
mod number_of_words;
pub(crate) mod passthrough;
mod relative_url;
mod sort;
mod truncatewords;
mod where_exp;
mod where_filter;
mod xml_escape;

pub use absolute_url::AbsoluteUrl;
pub use date::Date;
pub use date_to_long_string::DateToLongString;
pub use date_to_rfc822::DateToRfc822;
pub use date_to_string::DateToString;
pub use date_to_xmlschema::DateToXmlschema;
pub use group_by::GroupBy;
pub use group_by_exp::GroupByExp;
pub use jsonify::Jsonify;
pub use markdownify::Markdownify;
pub use newline_to_br::NewlineToBr;
pub use normalize_whitespace::NormalizeWhitespace;
pub use number_of_words::NumberOfWords;
pub use relative_url::RelativeUrl;
pub use sort::Sort;
pub use truncatewords::Truncatewords;
pub use where_exp::WhereExp;
pub use where_filter::Where;
pub use xml_escape::XmlEscape;

use chrono::NaiveDateTime;

/// Safely format a chrono `DelayedFormat` value to a string.
///
/// `chrono::DelayedFormat::fmt()` can return `Err` for certain format specifiers
/// (e.g., `%Z` on `NaiveDateTime`). The standard `.to_string()` method panics in
/// that case because `format!("{}", ...)` panics on `Display::fmt` errors.
///
/// This function catches the error and returns `None`, allowing callers to fall
/// back gracefully (e.g., returning the input as-is, matching Jekyll behavior).
pub(crate) fn safe_chrono_format(formatted: &impl std::fmt::Display) -> Option<String> {
    use std::fmt::Write;
    let mut buf = String::new();
    match write!(buf, "{}", formatted) {
        Ok(()) => Some(buf),
        Err(_) => None,
    }
}

/// Resolve the site timezone from an IANA name (e.g. "Europe/Berlin").
///
/// Returns `None` if the name is not a valid IANA timezone.
pub(crate) fn resolve_site_tz(name: &str) -> Option<chrono_tz::Tz> {
    name.parse::<chrono_tz::Tz>().ok()
}

/// Extract the site timezone name from the Liquid runtime context.
///
/// Looks up `site.timezone` which is set from the `timezone` key in `_config.yml`.
pub(crate) fn get_site_timezone(runtime: &dyn liquid_core::Runtime) -> Option<chrono_tz::Tz> {
    use liquid_core::model::ScalarCow;
    use liquid_core::ValueView;
    let tz_str = runtime
        .try_get(&[ScalarCow::new("site"), ScalarCow::new("timezone")])
        .map(|v| v.to_kstr().to_string())?;
    if tz_str.is_empty() {
        return None;
    }
    resolve_site_tz(&tz_str)
}

/// Get the system's local timezone as an IANA timezone name.
///
/// Uses the `iana-time-zone` crate which reads the system timezone from
/// OS-specific sources (e.g., /etc/timezone on Linux, registry on Windows).
/// Returns `None` if the system timezone cannot be determined or is not a
/// valid IANA timezone.
pub(crate) fn get_system_timezone() -> Option<String> {
    iana_time_zone::get_timezone().ok()
}

/// Parse a date string trying multiple formats commonly found in Jekyll YAML.
///
/// Returns a `NaiveDateTime` on success, `None` if no format matches.
///
/// Uses `naive_local()` (not `naive_utc()`) for timezone-aware dates so that
/// the date portion is preserved as written in the front matter. Jekyll's
/// `date_to_string` uses the date as-is without converting to UTC, so a date
/// like `2023-10-11 00:00:00 +0200` should remain Oct 11, not become Oct 10.
///
/// For dates WITHOUT timezone (NaiveDateTime), Jekyll treats them as local
/// time via Ruby's `Time.parse`, which interprets naive datetimes in the
/// process timezone. The datetime is returned as-is without conversion.
/// The `site_tz` parameter is accepted for API compatibility but is not
/// used for naive dates (only timezone-aware dates use it).
pub(crate) fn parse_date_string_with_tz(
    s: &str,
    _site_tz: Option<chrono_tz::Tz>,
) -> Option<NaiveDateTime> {
    // Try ISO 8601 with timezone offset (e.g. "2024-01-15T00:00:00+00:00")
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.naive_local());
    }
    // Try "YYYY-MM-DD HH:MM:SS +HHMM" (Jekyll-style with space before offset)
    // Check this BEFORE the bare "YYYY-MM-DD HH:MM:SS" to avoid partial matches.
    if let Ok(dt) = chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S %z") {
        return Some(dt.naive_local());
    }
    // Try "YYYY-MM-DDTHH:MM:SS" without timezone -- treat as local time (match Jekyll)
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    // Try "YYYY-MM-DD HH:MM:SS" -- treat as local time (match Jekyll)
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }
    // Try date-only "YYYY-MM-DD" -- treat as local time at midnight
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = d.and_hms_opt(0, 0, 0)?;
        return Some(dt);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_safe_chrono_format_valid() {
        let dt = chrono::NaiveDate::from_ymd_opt(2024, 7, 24)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let result = safe_chrono_format(&dt.format("%Y-%m-%d"));
        assert_eq!(result, Some("2024-07-24".to_string()));
    }

    #[test]
    fn test_safe_chrono_format_month_name() {
        let dt = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let result = safe_chrono_format(&dt.format("%B %Y"));
        assert_eq!(result, Some("March 2024".to_string()));
    }

    #[test]
    fn test_safe_chrono_format_does_not_panic_on_tz_specifier() {
        // %Z on NaiveDateTime -- chrono may return an error from Display::fmt
        let dt = chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        // This should NOT panic regardless of whether chrono returns Ok or Err
        let _result = safe_chrono_format(&dt.format("%Z"));
        // We don't assert the value -- just that it didn't panic
    }

    #[test]
    fn test_safe_chrono_format_tz_offset_specifier() {
        let dt = chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        // %z, %:z on NaiveDateTime may fail
        let _r1 = safe_chrono_format(&dt.format("%z"));
        let _r2 = safe_chrono_format(&dt.format("%:z"));
        let _r3 = safe_chrono_format(&dt.format("%+"));
        // No panic is the test
    }

    #[test]
    fn test_resolve_site_tz_valid() {
        let tz = resolve_site_tz("Europe/Berlin");
        assert!(tz.is_some());
    }

    #[test]
    fn test_resolve_site_tz_invalid() {
        let tz = resolve_site_tz("Not/A/Timezone");
        assert!(tz.is_none());
    }

    #[test]
    fn test_resolve_site_tz_utc() {
        let tz = resolve_site_tz("UTC");
        assert!(tz.is_some());
    }

    // Issue 109: NaiveDateTime without timezone should be treated as UTC
    // and converted to the site timezone.

    #[test]
    fn test_naive_datetime_not_converted_by_site_tz() {
        // Naive datetimes (no timezone) are kept as-is, matching Jekyll's
        // Time.parse which treats naive dates as local time. The site timezone
        // does NOT cause conversion of naive datetimes.
        let tz = resolve_site_tz("Europe/Berlin").unwrap();
        let dt = parse_date_string_with_tz("2020-12-18 23:59:59", Some(tz));
        let dt = dt.unwrap();
        assert_eq!(dt.date().day(), 18);
        assert_eq!(dt.date().month(), 12);
        assert_eq!(dt.date().year(), 2020);
        assert_eq!(dt.time().hour(), 23);
        assert_eq!(dt.time().minute(), 59);
    }

    #[test]
    fn test_naive_datetime_preserved_as_is() {
        // "2020-12-18 23:59:59" -- naive datetime should be returned as-is
        // (matching Jekyll's Time.parse which treats naive dates as local time)
        let tz = resolve_site_tz("UTC").unwrap();
        let dt = parse_date_string_with_tz("2020-12-18 23:59:59", Some(tz));
        let dt = dt.unwrap();
        assert_eq!(dt.date().day(), 18);
        assert_eq!(dt.date().month(), 12);
        assert_eq!(dt.time().hour(), 23);
        assert_eq!(dt.time().minute(), 59);
    }

    #[test]
    fn test_naive_date_only_preserved_as_midnight() {
        // "2020-12-18" -- should become midnight, no timezone conversion
        let tz = resolve_site_tz("Europe/Berlin").unwrap();
        let dt = parse_date_string_with_tz("2020-12-18", Some(tz));
        let dt = dt.unwrap();
        assert_eq!(dt.date().day(), 18);
        assert_eq!(dt.time().hour(), 0);
    }

    #[test]
    fn test_explicit_tz_not_affected_by_site_tz() {
        // Dates with explicit timezone should use naive_local() regardless of site_tz
        let tz = resolve_site_tz("America/New_York").unwrap();
        let dt = parse_date_string_with_tz("2023-10-11 00:00:00 +0200", Some(tz));
        let dt = dt.unwrap();
        // Should remain Oct 11 (local time of +0200), NOT converted to New York
        assert_eq!(dt.date().day(), 11);
        assert_eq!(dt.date().month(), 10);
    }

    #[test]
    fn test_naive_t_format_preserved_as_is() {
        // "2020-12-18T23:59:59" (ISO without tz) -- should be preserved as-is
        let tz = resolve_site_tz("Europe/Berlin").unwrap();
        let dt = parse_date_string_with_tz("2020-12-18T23:59:59", Some(tz));
        let dt = dt.unwrap();
        assert_eq!(dt.date().day(), 18);
        assert_eq!(dt.time().hour(), 23);
    }
}
