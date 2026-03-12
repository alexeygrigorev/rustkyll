use liquid_core::model::ScalarCow;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Prepend the site's baseurl to a path.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "relative_url",
    description = "Prepend the site baseurl to a path.",
    parsed(RelativeUrlFilter)
)]
pub struct RelativeUrl;

#[derive(Debug, Default, Display_filter)]
#[name = "relative_url"]
struct RelativeUrlFilter;

impl Filter for RelativeUrlFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let path = input.to_kstr();

        // Try to get site.baseurl from the runtime context
        let baseurl = runtime
            .try_get(&[ScalarCow::new("site"), ScalarCow::new("baseurl")])
            .and_then(|v| {
                let s = v.to_kstr().to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            });

        match baseurl {
            Some(base) => {
                let base = base.trim_end_matches('/');
                if path.starts_with('/') {
                    Ok(Value::scalar(format!("{base}{path}")))
                } else {
                    Ok(Value::scalar(format!("{base}/{path}")))
                }
            }
            None => {
                // No baseurl set: ensure path starts with /
                if path.is_empty() || path.starts_with('/') {
                    Ok(Value::scalar(path.to_string()))
                } else {
                    Ok(Value::scalar(format!("/{path}")))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_without_leading_slash_no_baseurl() {
        let result = liquid_core::call_filter!(RelativeUrl, "images/photo.jpg").unwrap();
        assert_eq!(result.to_kstr(), "/images/photo.jpg");
    }

    #[test]
    fn test_path_with_leading_slash_no_baseurl() {
        let result = liquid_core::call_filter!(RelativeUrl, "/images/photo.jpg").unwrap();
        assert_eq!(result.to_kstr(), "/images/photo.jpg");
    }

    #[test]
    fn test_empty_string_no_baseurl() {
        let result = liquid_core::call_filter!(RelativeUrl, "").unwrap();
        assert_eq!(result.to_kstr(), "");
    }
}
