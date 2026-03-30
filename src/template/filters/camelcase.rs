use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Passthrough filter matching Jekyll's behavior for the `camelcase` filter.
///
/// Jekyll's Liquid does not define a `camelcase` filter. Some themes (like
/// Mediumish) reference it, but in practice it passes through the value
/// unchanged. We register it as a named filter so it doesn't trigger
/// unknown-filter warnings.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "camelcase",
    description = "Passthrough filter (camelcase is not a standard Jekyll filter).",
    parsed(CamelcaseFilter)
)]
pub struct Camelcase;

#[derive(Debug, Default, Display_filter)]
#[name = "camelcase"]
struct CamelcaseFilter;

impl Filter for CamelcaseFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        Ok(input.to_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passthrough_snake_case() {
        // camelcase is a passthrough in Jekyll -- input is returned unchanged
        let result = liquid_core::call_filter!(Camelcase, "hello_world").unwrap();
        assert_eq!(result.to_kstr().as_str(), "hello_world");
    }

    #[test]
    fn test_passthrough_single_word() {
        let result = liquid_core::call_filter!(Camelcase, "tutorial").unwrap();
        assert_eq!(result.to_kstr().as_str(), "tutorial");
    }

    #[test]
    fn test_passthrough_capitalized() {
        let result = liquid_core::call_filter!(Camelcase, "Jekyll").unwrap();
        assert_eq!(result.to_kstr().as_str(), "Jekyll");
    }

    #[test]
    fn test_passthrough_empty() {
        let result = liquid_core::call_filter!(Camelcase, "").unwrap();
        assert_eq!(result.to_kstr().as_str(), "");
    }

    #[test]
    fn test_passthrough_with_spaces() {
        let result = liquid_core::call_filter!(Camelcase, "web development").unwrap();
        assert_eq!(result.to_kstr().as_str(), "web development");
    }
}
