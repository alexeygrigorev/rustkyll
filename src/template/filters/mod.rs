//! Custom Liquid filters for Jekyll compatibility.
//!
//! These 6 filters are NOT provided by the `liquid` crate or `liquid-lib`'s
//! jekyll feature, but are needed by the DataTalks.Club site templates.

mod date_to_string;
mod date_to_xmlschema;
mod jsonify;
mod markdownify;
mod relative_url;
mod where_exp;

pub use date_to_string::DateToString;
pub use date_to_xmlschema::DateToXmlschema;
pub use jsonify::Jsonify;
pub use markdownify::Markdownify;
pub use relative_url::RelativeUrl;
pub use where_exp::WhereExp;

use chrono::NaiveDateTime;

/// Parse a date string trying multiple formats commonly found in Jekyll YAML.
///
/// Returns a `NaiveDateTime` on success, `None` if no format matches.
pub(crate) fn parse_date_string(s: &str) -> Option<NaiveDateTime> {
    // Try ISO 8601 with timezone offset (e.g. "2024-01-15T00:00:00+00:00")
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.naive_utc());
    }
    // Try "YYYY-MM-DDTHH:MM:SS" without timezone
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    // Try "YYYY-MM-DD HH:MM:SS"
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }
    // Try "YYYY-MM-DD HH:MM:SS +HHMM" (Jekyll-style with space before offset)
    if let Ok(dt) = chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S %z") {
        return Some(dt.naive_utc());
    }
    // Try date-only "YYYY-MM-DD"
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return d.and_hms_opt(0, 0, 0);
    }
    None
}
