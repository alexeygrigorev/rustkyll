//! A passthrough filter that returns its input unchanged.
//!
//! Used to gracefully handle unknown filter names at parse time. Instead of
//! crashing the build, unknown filters are registered as passthrough stubs
//! that log a warning and return the input value unchanged.

use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// A passthrough filter stub for an unknown filter name.
///
/// Implements `ParseFilter` and `FilterReflection` with a dynamic name,
/// so it can be registered in the liquid parser for any unknown filter.
#[derive(Clone, Debug)]
pub struct PassthroughFilter {
    name: String,
}

impl PassthroughFilter {
    /// Create a new passthrough filter with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl FilterReflection for PassthroughFilter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Passthrough filter for unknown filter name (returns input unchanged)"
    }

    fn positional_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &[]
    }

    fn keyword_parameters(&self) -> &'static [liquid_core::parser::ParameterReflection] {
        &[]
    }
}

impl ParseFilter for PassthroughFilter {
    fn parse(&self, _arguments: liquid_core::parser::FilterArguments) -> Result<Box<dyn Filter>> {
        Ok(Box::new(PassthroughFilterInstance {
            name: self.name.clone(),
        }))
    }

    fn reflection(&self) -> &dyn FilterReflection {
        self
    }
}

#[derive(Debug)]
struct PassthroughFilterInstance {
    name: String,
}

impl std::fmt::Display for PassthroughFilterInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Filter for PassthroughFilterInstance {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        eprintln!(
            "Warning: unknown filter '{}' used, passing through value unchanged",
            self.name
        );
        Ok(input.to_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passthrough_name() {
        let filter = PassthroughFilter::new("erl_encode");
        assert_eq!(filter.name(), "erl_encode");
    }

    #[test]
    fn test_passthrough_description() {
        let filter = PassthroughFilter::new("test_filter");
        assert_eq!(
            filter.description(),
            "Passthrough filter for unknown filter name (returns input unchanged)"
        );
    }

    #[test]
    fn test_passthrough_reflection() {
        let filter = PassthroughFilter::new("my_filter");
        let refl = filter.reflection();
        assert_eq!(refl.name(), "my_filter");
    }
}
