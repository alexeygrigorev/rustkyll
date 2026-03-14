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
pub use truncatewords::Truncatewords;
pub use where_exp::WhereExp;
pub use where_filter::Where;
pub use xml_escape::XmlEscape;

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
