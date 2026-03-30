use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Passthrough filter for `url_escape`.
///
/// Jekyll does not define a `url_escape` filter (it has `url_encode`,
/// `cgi_escape`, and `uri_escape` instead). Some themes reference
/// `url_escape`, but in practice it passes through the value unchanged.
/// We register it as a named filter to avoid unknown-filter warnings.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "url_escape",
    description = "Passthrough filter (url_escape is not a standard Jekyll filter).",
    parsed(UrlEscapeFilter)
)]
pub struct UrlEscape;

#[derive(Debug, Default, Display_filter)]
#[name = "url_escape"]
struct UrlEscapeFilter;

impl Filter for UrlEscapeFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        Ok(input.to_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passthrough_spaces() {
        // url_escape is a passthrough -- spaces are NOT encoded
        let result = liquid_core::call_filter!(UrlEscape, "web development").unwrap();
        assert_eq!(result.to_kstr().as_str(), "web development");
    }

    #[test]
    fn test_passthrough_simple() {
        let result = liquid_core::call_filter!(UrlEscape, "Jekyll").unwrap();
        assert_eq!(result.to_kstr().as_str(), "Jekyll");
    }

    #[test]
    fn test_passthrough_empty() {
        let result = liquid_core::call_filter!(UrlEscape, "").unwrap();
        assert_eq!(result.to_kstr().as_str(), "");
    }
}
