use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Escape XML special characters in a string.
///
/// Escapes `&`, `<`, `>`, `"`, and `'` to their XML entity equivalents.
/// Matches Jekyll's `xml_escape` filter behavior.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "xml_escape",
    description = "Escape XML special characters.",
    parsed(XmlEscapeFilter)
)]
pub struct XmlEscape;

#[derive(Debug, Default, Display_filter)]
#[name = "xml_escape"]
struct XmlEscapeFilter;

impl Filter for XmlEscapeFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        let text = input.to_kstr();
        // Important: & must be escaped first to avoid double-escaping
        let result = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;");
        Ok(Value::scalar(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_angle_brackets() {
        let result = liquid_core::call_filter!(XmlEscape, "<p>Hello</p>").unwrap();
        assert_eq!(result.to_kstr(), "&lt;p&gt;Hello&lt;/p&gt;");
    }

    #[test]
    fn test_ampersand() {
        let result = liquid_core::call_filter!(XmlEscape, "Tom & Jerry").unwrap();
        assert_eq!(result.to_kstr(), "Tom &amp; Jerry");
    }

    #[test]
    fn test_double_quotes() {
        let result = liquid_core::call_filter!(XmlEscape, "She said \"hi\"").unwrap();
        assert_eq!(result.to_kstr(), "She said &quot;hi&quot;");
    }

    #[test]
    fn test_single_quotes() {
        let result = liquid_core::call_filter!(XmlEscape, "it's").unwrap();
        assert_eq!(result.to_kstr(), "it&apos;s");
    }

    #[test]
    fn test_no_special_chars() {
        let result = liquid_core::call_filter!(XmlEscape, "no special chars").unwrap();
        assert_eq!(result.to_kstr(), "no special chars");
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(XmlEscape, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_already_escaped_ampersand() {
        let result = liquid_core::call_filter!(XmlEscape, "&amp;").unwrap();
        assert_eq!(result.to_kstr(), "&amp;amp;");
    }

    #[test]
    fn test_all_five_special_chars() {
        let result = liquid_core::call_filter!(XmlEscape, "<b>Tom & Jerry's \"show\"</b>").unwrap();
        assert_eq!(
            result.to_kstr(),
            "&lt;b&gt;Tom &amp; Jerry&apos;s &quot;show&quot;&lt;/b&gt;"
        );
    }
}
