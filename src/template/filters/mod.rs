//! Custom Liquid filters for Jekyll compatibility.
//!
//! These 6 filters are NOT provided by the `liquid` crate or `liquid-lib`'s
//! jekyll feature, but are needed by common Jekyll site templates.

mod absolute_url;
pub(crate) mod cgi_escape;
mod compact;
mod date;
mod date_to_long_string;
mod date_to_rfc822;
mod date_to_string;
mod date_to_xmlschema;
mod find_filter;
mod group_by;
mod group_by_exp;
mod join;
pub(crate) mod jsonify;
mod map;
mod markdownify;
pub(crate) mod math;
mod newline_to_br;
mod normalize_whitespace;
mod number_of_words;
pub(crate) mod passthrough;
pub(crate) mod relative_url;
pub(crate) mod render_mapping;
mod sample;
pub(crate) mod sort;
mod strip_html;
mod strip_index;
mod truncatewords;
mod uniq;
mod uri_escape;
mod url_encode;
mod where_exp;
mod where_filter;
mod xml_escape;

pub use absolute_url::AbsoluteUrl;
pub use cgi_escape::CgiEscape;
pub use compact::Compact;
pub use date::Date;
pub use date_to_long_string::DateToLongString;
pub use date_to_rfc822::DateToRfc822;
pub use date_to_string::DateToString;
pub use date_to_xmlschema::DateToXmlschema;
pub use find_filter::Find;
pub use group_by::GroupBy;
pub use group_by_exp::GroupByExp;
pub use join::Join;
pub use jsonify::Jsonify;
pub use map::Map;
pub use markdownify::Markdownify;
pub use newline_to_br::NewlineToBr;
pub use normalize_whitespace::NormalizeWhitespace;
pub use number_of_words::NumberOfWords;
pub use relative_url::RelativeUrl;
pub use render_mapping::RenderMapping;
pub use sample::Sample;
pub use sort::Sort;
pub use strip_html::StripHtml;
pub use strip_index::StripIndex;
pub use truncatewords::Truncatewords;
pub use uniq::Uniq;
pub use uri_escape::UriEscape;
pub use url_encode::UrlEncode;
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

    // Read from site.timezone in the Liquid context.
    // The system timezone fallback is applied at context build time
    // (in build_site_context), so by the time we get here, site.timezone
    // should already contain the system timezone if no explicit one was set.
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
pub fn get_system_timezone() -> Option<String> {
    iana_time_zone::get_timezone().ok()
}

/// Parse a date string trying multiple formats commonly found in Jekyll YAML.
///
/// Returns a `NaiveDateTime` on success, `None` if no format matches.
///
/// Uses `naive_local()` (not `naive_utc()`) for timezone-aware dates so that
/// the date portion is preserved as written in the front matter.  Date-only
/// strings like `2023-12-11` get expanded to `2023-12-11 00:00:00 +HHMM`
/// using the site/system timezone before reaching filters; using `naive_local()`
/// strips the offset while keeping the local date intact.
///
/// For dates WITHOUT timezone (NaiveDateTime), the datetime is returned as-is.
/// This matches Jekyll's `date` filter which formats UTC timestamps without
/// converting them. Filters that need local-timezone conversion (like
/// `date_to_string`) must call `convert_utc_naive_to_site_tz` separately.
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
    // Try "YYYY-MM-DDTHH:MM:SS" without timezone -- kept as-is (UTC per Ruby YAML)
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    // Try "YYYY-MM-DD HH:MM:SS" -- kept as-is (UTC per Ruby YAML)
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }
    // Try date-only "YYYY-MM-DD" -- kept as midnight, no conversion
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = d.and_hms_opt(0, 0, 0)?;
        return Some(dt);
    }
    // Try slash-separated dates (e.g. "2023/7/11 15:27" from muan-blog).
    // Ruby's Date.parse handles these flexibly; chrono's %m/%d/%H/%M accept
    // both 1-digit and 2-digit values when parsing.
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M:%S") {
        return Some(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M") {
        return Some(dt);
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y/%m/%d") {
        let dt = d.and_hms_opt(0, 0, 0)?;
        return Some(dt);
    }
    None
}

/// Format a naive datetime with the correct timezone offset for the given site timezone.
pub(crate) fn format_datetime_with_tz_offset(
    dt: chrono::NaiveDateTime,
    site_tz: Option<chrono_tz::Tz>,
) -> String {
    use chrono::TimeZone;
    match site_tz {
        Some(tz) => match tz.from_local_datetime(&dt) {
            chrono::LocalResult::Single(localized) => {
                safe_chrono_format(&localized.format("%Y-%m-%dT%H:%M:%S%:z"))
                    .unwrap_or_else(|| format!("{}+00:00", dt.format("%Y-%m-%dT%H:%M:%S")))
            }
            chrono::LocalResult::Ambiguous(earliest, _) => {
                safe_chrono_format(&earliest.format("%Y-%m-%dT%H:%M:%S%:z"))
                    .unwrap_or_else(|| format!("{}+00:00", dt.format("%Y-%m-%dT%H:%M:%S")))
            }
            chrono::LocalResult::None => {
                format!("{}+00:00", dt.format("%Y-%m-%dT%H:%M:%S"))
            }
        },
        None => format!("{}+00:00", dt.format("%Y-%m-%dT%H:%M:%S")),
    }
}

/// Format a raw date string to ISO 8601 xmlschema format with the correct timezone offset.
pub(crate) fn format_date_to_xmlschema(s: &str, site_tz: Option<chrono_tz::Tz>) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return safe_chrono_format(&dt.format("%Y-%m-%dT%H:%M:%S%:z"))
            .unwrap_or_else(|| trimmed.to_string());
    }
    if let Ok(dt) = chrono::DateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S %z") {
        return safe_chrono_format(&dt.format("%Y-%m-%dT%H:%M:%S%:z"))
            .unwrap_or_else(|| trimmed.to_string());
    }
    match parse_date_string_with_tz(trimmed, site_tz) {
        Some(dt) => format_datetime_with_tz_offset(dt, site_tz),
        None => trimmed.to_string(),
    }
}

/// Check if a date string is a naive YAML timestamp (has time but no timezone).
///
/// Returns true for "YYYY-MM-DD HH:MM:SS" and "YYYY-MM-DDTHH:MM:SS" formats
/// that Ruby's YAML parser would interpret as UTC timestamps.
/// Returns false for date-only strings ("YYYY-MM-DD") and strings with
/// explicit timezone offsets ("... +0200", "...+00:00").
pub(crate) fn is_naive_yaml_timestamp(s: &str) -> bool {
    let trimmed = s.trim();
    // Must have a time component (more than just YYYY-MM-DD)
    if trimmed.len() <= 10 {
        return false;
    }
    // Must have time separator
    if !trimmed.contains('T') && trimmed.chars().filter(|c| *c == ':').count() < 2 {
        return false;
    }
    // Must NOT have an explicit timezone offset
    // Check for +HHMM, -HHMM, +HH:MM, -HH:MM, Z at end
    let bytes = trimmed.as_bytes();
    if bytes.last() == Some(&b'Z') {
        return false;
    }
    // Check for offset pattern: +/-NN:NN or +/-NNNN at the end
    if trimmed.len() > 5 {
        let tail = &trimmed[trimmed.len() - 6..];
        if tail.starts_with('+') || tail.starts_with('-') {
            return false;
        }
    }
    if trimmed.len() > 4 {
        let tail = &trimmed[trimmed.len() - 5..];
        if tail.starts_with('+') || tail.starts_with('-') {
            return false;
        }
    }
    true
}

/// Convert a naive datetime (assumed UTC per Ruby YAML convention) to the
/// site timezone. If no site timezone is available, returns as-is.
///
/// This implements Jekyll's `time()` helper behavior where `date_to_string`
/// and `date_to_long_string` call `.localtime` on UTC Time objects, converting
/// them to the local timezone. The `date` filter does NOT do this conversion.
///
/// Only call this for naive datetimes that have a time component (from YAML
/// timestamps like `2025-10-10 23:59:59`). Date-only strings (`2025-10-10`)
/// are not YAML timestamps and should not be converted.
pub(crate) fn convert_utc_naive_to_site_tz(
    dt: NaiveDateTime,
    site_tz: Option<chrono_tz::Tz>,
) -> NaiveDateTime {
    use chrono::TimeZone;
    match site_tz {
        Some(tz) => {
            // Treat the naive datetime as UTC, then convert to site timezone
            let utc_dt = chrono::Utc.from_utc_datetime(&dt);
            utc_dt.with_timezone(&tz).naive_local()
        }
        None => dt, // No timezone info, keep as-is
    }
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

    // parse_date_string_with_tz returns naive datetimes as-is (no conversion).
    // Jekyll's `date` filter formats UTC timestamps without converting to local tz.
    // Only `date_to_string` / `date_to_long_string` do the local tz conversion
    // (via convert_utc_naive_to_site_tz).

    #[test]
    fn test_naive_datetime_preserved_as_is() {
        // "2020-12-18 23:59:59" -- kept as-is by parse_date_string_with_tz
        let tz = resolve_site_tz("Europe/Berlin").unwrap();
        let dt = parse_date_string_with_tz("2020-12-18 23:59:59", Some(tz));
        let dt = dt.unwrap();
        assert_eq!(dt.date().day(), 18);
        assert_eq!(dt.date().month(), 12);
        assert_eq!(dt.time().hour(), 23);
        assert_eq!(dt.time().minute(), 59);
    }

    #[test]
    fn test_naive_datetime_preserved_utc_tz() {
        let tz = resolve_site_tz("UTC").unwrap();
        let dt = parse_date_string_with_tz("2020-12-18 23:59:59", Some(tz));
        let dt = dt.unwrap();
        assert_eq!(dt.date().day(), 18);
        assert_eq!(dt.time().hour(), 23);
    }

    #[test]
    fn test_naive_datetime_preserved_no_tz() {
        let dt = parse_date_string_with_tz("2020-12-18 23:59:59", None);
        let dt = dt.unwrap();
        assert_eq!(dt.date().day(), 18);
        assert_eq!(dt.time().hour(), 23);
    }

    #[test]
    fn test_naive_date_only_preserved_as_midnight() {
        let tz = resolve_site_tz("Europe/Berlin").unwrap();
        let dt = parse_date_string_with_tz("2020-12-18", Some(tz));
        let dt = dt.unwrap();
        assert_eq!(dt.date().day(), 18);
        assert_eq!(dt.time().hour(), 0);
    }

    #[test]
    fn test_explicit_tz_not_affected_by_site_tz() {
        let tz = resolve_site_tz("America/New_York").unwrap();
        let dt = parse_date_string_with_tz("2023-10-11 00:00:00 +0200", Some(tz));
        let dt = dt.unwrap();
        assert_eq!(dt.date().day(), 11);
        assert_eq!(dt.date().month(), 10);
    }

    #[test]
    fn test_naive_t_format_preserved() {
        let tz = resolve_site_tz("Europe/Berlin").unwrap();
        let dt = parse_date_string_with_tz("2020-12-18T23:59:59", Some(tz));
        let dt = dt.unwrap();
        assert_eq!(dt.date().day(), 18);
        assert_eq!(dt.time().hour(), 23);
    }

    // convert_utc_naive_to_site_tz: used by date_to_string/date_to_long_string
    // to replicate Jekyll's Time#localtime behavior.

    #[test]
    fn test_convert_utc_to_cet_rolls_day() {
        // 23:59 UTC -> 00:59 CET next day (in December, CET = UTC+1)
        let tz = resolve_site_tz("Europe/Berlin").unwrap();
        let dt = chrono::NaiveDate::from_ymd_opt(2020, 12, 18)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();
        let result = convert_utc_naive_to_site_tz(dt, Some(tz));
        assert_eq!(result.date().day(), 19);
        assert_eq!(result.time().hour(), 0);
        assert_eq!(result.time().minute(), 59);
    }

    #[test]
    fn test_convert_utc_to_utc_no_change() {
        let tz = resolve_site_tz("UTC").unwrap();
        let dt = chrono::NaiveDate::from_ymd_opt(2020, 12, 18)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();
        let result = convert_utc_naive_to_site_tz(dt, Some(tz));
        assert_eq!(result.date().day(), 18);
        assert_eq!(result.time().hour(), 23);
    }

    #[test]
    fn test_convert_utc_midnight_stays_same_day_in_cest() {
        // 00:00 UTC -> 02:00 CEST (UTC+2 in September)
        let tz = resolve_site_tz("Europe/Berlin").unwrap();
        let dt = chrono::NaiveDate::from_ymd_opt(2025, 9, 6)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let result = convert_utc_naive_to_site_tz(dt, Some(tz));
        assert_eq!(result.date().day(), 6);
        assert_eq!(result.time().hour(), 2);
    }

    #[test]
    fn test_convert_none_tz_preserves() {
        let dt = chrono::NaiveDate::from_ymd_opt(2020, 12, 18)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();
        let result = convert_utc_naive_to_site_tz(dt, None);
        assert_eq!(result.date().day(), 18);
        assert_eq!(result.time().hour(), 23);
    }

    #[test]
    fn test_is_naive_yaml_timestamp() {
        // Naive datetimes (should be treated as UTC by YAML parser)
        assert!(is_naive_yaml_timestamp("2020-12-18 23:59:59"));
        assert!(is_naive_yaml_timestamp("2020-12-18T23:59:59"));
        assert!(is_naive_yaml_timestamp("2020-12-18 00:00:00"));
        // Date-only (not a timestamp)
        assert!(!is_naive_yaml_timestamp("2020-12-18"));
        assert!(!is_naive_yaml_timestamp("2020-12"));
        // Explicit timezone (not naive)
        assert!(!is_naive_yaml_timestamp("2020-12-18 23:59:59 +0200"));
        assert!(!is_naive_yaml_timestamp("2020-12-18T23:59:59+00:00"));
        assert!(!is_naive_yaml_timestamp("2020-12-18T23:59:59Z"));
    }

    #[test]
    fn test_format_datetime_tz_berlin_winter() {
        let dt = chrono::NaiveDate::from_ymd_opt(2023, 12, 11)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let tz = resolve_site_tz("Europe/Berlin").unwrap();
        assert_eq!(
            format_datetime_with_tz_offset(dt, Some(tz)),
            "2023-12-11T00:00:00+01:00"
        );
    }

    #[test]
    fn test_format_datetime_tz_berlin_summer() {
        let dt = chrono::NaiveDate::from_ymd_opt(2023, 7, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let tz = resolve_site_tz("Europe/Berlin").unwrap();
        assert_eq!(
            format_datetime_with_tz_offset(dt, Some(tz)),
            "2023-07-15T00:00:00+02:00"
        );
    }

    #[test]
    fn test_format_datetime_tz_none_defaults_utc() {
        let dt = chrono::NaiveDate::from_ymd_opt(2023, 12, 11)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert_eq!(
            format_datetime_with_tz_offset(dt, None),
            "2023-12-11T00:00:00+00:00"
        );
    }

    #[test]
    fn test_format_datetime_tz_utc() {
        let dt = chrono::NaiveDate::from_ymd_opt(2023, 12, 11)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let tz = resolve_site_tz("UTC").unwrap();
        assert_eq!(
            format_datetime_with_tz_offset(dt, Some(tz)),
            "2023-12-11T00:00:00+00:00"
        );
    }

    #[test]
    fn test_xmlschema_berlin_winter() {
        let tz = resolve_site_tz("Europe/Berlin");
        assert_eq!(
            format_date_to_xmlschema("2023-12-11", tz),
            "2023-12-11T00:00:00+01:00"
        );
    }

    #[test]
    fn test_xmlschema_berlin_summer() {
        let tz = resolve_site_tz("Europe/Berlin");
        assert_eq!(
            format_date_to_xmlschema("2023-07-15", tz),
            "2023-07-15T00:00:00+02:00"
        );
    }

    #[test]
    fn test_xmlschema_no_tz_defaults_utc() {
        assert_eq!(
            format_date_to_xmlschema("2023-12-11", None),
            "2023-12-11T00:00:00+00:00"
        );
    }

    #[test]
    fn test_xmlschema_with_explicit_time() {
        let tz = resolve_site_tz("Europe/Berlin");
        assert_eq!(
            format_date_to_xmlschema("2023-12-11T14:30:00", tz),
            "2023-12-11T14:30:00+01:00"
        );
    }

    #[test]
    fn test_xmlschema_preserves_rfc3339_tz() {
        let tz = resolve_site_tz("Europe/Berlin");
        assert_eq!(
            format_date_to_xmlschema("2024-01-15T14:30:00+00:00", tz),
            "2024-01-15T14:30:00+00:00"
        );
    }

    #[test]
    fn test_xmlschema_preserves_jekyll_style_tz() {
        let tz = resolve_site_tz("Europe/Berlin");
        assert_eq!(
            format_date_to_xmlschema("2023-10-11 00:00:00 +0200", tz),
            "2023-10-11T00:00:00+02:00"
        );
    }

    #[test]
    fn test_xmlschema_empty_string() {
        assert_eq!(format_date_to_xmlschema("", None), "");
    }

    #[test]
    fn test_xmlschema_invalid_passthrough() {
        assert_eq!(format_date_to_xmlschema("not-a-date", None), "not-a-date");
    }

    #[test]
    fn test_xmlschema_event_dates_berlin() {
        let tz = resolve_site_tz("Europe/Berlin");
        assert_eq!(
            format_date_to_xmlschema("2024-01-15", tz),
            "2024-01-15T00:00:00+01:00"
        );
        assert_eq!(
            format_date_to_xmlschema("2024-01-19", tz),
            "2024-01-19T00:00:00+01:00"
        );
    }

    #[test]
    fn test_xmlschema_preserves_explicit_utc_offset() {
        let tz = resolve_site_tz("Europe/Berlin");
        assert_eq!(
            format_date_to_xmlschema("2023-12-11 00:00:00 +0000", tz),
            "2023-12-11T00:00:00+00:00"
        );
    }
}
