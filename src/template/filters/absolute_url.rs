use liquid_core::model::ScalarCow;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Prepend the site's url and baseurl to a path, producing an absolute URL.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "absolute_url",
    description = "Prepend the site url and baseurl to a path.",
    parsed(AbsoluteUrlFilter)
)]
pub struct AbsoluteUrl;

#[derive(Debug, Default, Display_filter)]
#[name = "absolute_url"]
struct AbsoluteUrlFilter;

impl Filter for AbsoluteUrlFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let path = input.to_kstr().to_string();

        // Get site.url from context
        let site_url = runtime
            .try_get(&[ScalarCow::new("site"), ScalarCow::new("url")])
            .map(|v| v.to_kstr().to_string())
            .unwrap_or_default();

        // Get site.baseurl from context
        let baseurl = runtime
            .try_get(&[ScalarCow::new("site"), ScalarCow::new("baseurl")])
            .map(|v| v.to_kstr().to_string())
            .unwrap_or_default();

        // Strip trailing slashes from url and baseurl
        let site_url = site_url.trim_end_matches('/');
        let baseurl = baseurl.trim_end_matches('/');

        // Ensure path has a leading slash (unless empty)
        let path = if !path.is_empty() && !path.starts_with('/') {
            format!("/{path}")
        } else {
            path
        };

        // Build the result: url + baseurl + path
        // Avoid double slashes between components
        let result = if site_url.is_empty() && baseurl.is_empty() {
            path
        } else if baseurl.is_empty() {
            if path.is_empty() {
                site_url.to_string()
            } else {
                format!("{site_url}{path}")
            }
        } else if site_url.is_empty() {
            if path.is_empty() {
                baseurl.to_string()
            } else {
                format!("{baseurl}{path}")
            }
        } else if path.is_empty() {
            format!("{site_url}{baseurl}")
        } else {
            format!("{site_url}{baseurl}{path}")
        };

        Ok(Value::scalar(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_context_returns_path() {
        let result = liquid_core::call_filter!(AbsoluteUrl, "/about.html").unwrap();
        assert_eq!(result.to_kstr(), "/about.html");
    }
}
