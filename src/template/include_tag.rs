//! Custom Jekyll-compatible include tag with lenient parameter access.
//!
//! This replaces `liquid_lib::jekyll::IncludeTag` with a version that:
//! - Returns Nil for missing include parameters instead of erroring
//! - Supports all parameter types (string, numeric, boolean, variable references)
//! - Supports both dot notation (`include.param`) and bracket notation (`include["param"]`)

use std::collections::HashMap;
use std::io::Write;

use liquid_core::error::ResultLiquidExt;
use liquid_core::model::{DisplayCow, KString, KStringCow, ObjectView, State, Value, ValueView};
use liquid_core::parser::FilterChain;
use liquid_core::parser::TryMatchToken;
use liquid_core::runtime::StackFrame;
use liquid_core::{Error, Result};
use liquid_core::{
    Expression, Language, ParseTag, Renderable, Runtime, TagReflection, TagTokenIter,
};

/// Sentinel filename used in preprocessed dynamic include tags.
/// When the parser sees this as the include filename, it knows to read the
/// next token(s) as a Liquid expression for the real path.
const DYNAMIC_INCLUDE_SENTINEL: &str = "__DYNAMIC_INCLUDE__";

/// A custom include tag that supports lenient parameter access.
#[derive(Copy, Clone, Debug, Default)]
pub struct LenientIncludeTag;

impl TagReflection for LenientIncludeTag {
    fn tag(&self) -> &'static str {
        "include"
    }

    fn description(&self) -> &'static str {
        "Jekyll-compatible include with lenient parameter access"
    }
}

impl ParseTag for LenientIncludeTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        _options: &Language,
    ) -> Result<Box<dyn Renderable>> {
        let name = arguments.expect_next("Identifier or literal expected.")?;

        let name = match name.expect_identifier() {
            TryMatchToken::Matches(name) => name.to_kstr().to_string(),
            TryMatchToken::Fails(name) => {
                // Handle quoted strings (from pre-processing of paths with `/`).
                // Strip surrounding quotes if present.
                let s = name.as_str();
                if (s.starts_with('"') && s.ends_with('"'))
                    || (s.starts_with('\'') && s.ends_with('\''))
                {
                    s[1..s.len() - 1].to_owned()
                } else {
                    s.to_owned()
                }
            }
        };

        // Check if this is a dynamic include (preprocessed from {% include {{ expr }} %}).
        // The sentinel value __DYNAMIC_INCLUDE__ means the actual path comes from
        // the next token which is a Liquid expression (variable + optional filters).
        let partial = if name == DYNAMIC_INCLUDE_SENTINEL {
            let path_token = arguments.expect_next("Expected dynamic include path expression.")?;
            match path_token.expect_filter_chain(_options) {
                TryMatchToken::Matches(chain) => IncludePath::Dynamic(chain),
                TryMatchToken::Fails(token) => {
                    return Err(Error::with_msg(format!(
                        "Invalid dynamic include path expression: {}",
                        token.as_str()
                    )));
                }
            }
        } else {
            IncludePath::Literal(Expression::with_literal(name))
        };

        let mut vars: Vec<(KString, Expression)> = Vec::new();
        while let Ok(next) = arguments.expect_next("") {
            let id = next.expect_identifier().into_result()?.to_owned();

            arguments
                .expect_next("\"=\" expected.")?
                .expect_str("=")
                .into_result_custom_msg("expected \"=\" to be used for the assignment")?;

            vars.push((
                id.into(),
                arguments
                    .expect_next("expected value")?
                    .expect_value()
                    .into_result()?,
            ));
        }

        arguments.expect_nothing()?;

        Ok(Box::new(LenientInclude { partial, vars }))
    }

    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

/// The include path, which can be a literal filename or a dynamic expression
/// (variable reference with optional filters).
#[derive(Debug)]
enum IncludePath {
    /// A static literal filename (e.g., `header.html`).
    Literal(Expression),
    /// A dynamic expression (e.g., `page.form | append: '.html'`).
    Dynamic(FilterChain),
}

impl std::fmt::Display for IncludePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncludePath::Literal(expr) => write!(f, "{}", expr),
            IncludePath::Dynamic(chain) => write!(f, "{{{{ {} }}}}", chain),
        }
    }
}

#[derive(Debug)]
struct LenientInclude {
    partial: IncludePath,
    vars: Vec<(KString, Expression)>,
}

/// A wrapper around include parameters that returns Nil for missing keys.
#[derive(Debug)]
struct LenientIncludeParams {
    params: HashMap<String, Value>,
    nil: Value,
}

impl LenientIncludeParams {
    fn new(params: HashMap<String, Value>) -> Self {
        Self {
            params,
            nil: Value::Nil,
        }
    }
}

impl std::fmt::Display for LenientIncludeParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (i, (k, v)) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", k, v.render())?;
        }
        write!(f, "}}")
    }
}

impl ValueView for LenientIncludeParams {
    fn as_debug(&self) -> &dyn std::fmt::Debug {
        self
    }

    fn render(&self) -> DisplayCow<'_> {
        DisplayCow::Owned(Box::new(self.to_string()))
    }

    fn source(&self) -> DisplayCow<'_> {
        DisplayCow::Owned(Box::new(self.to_string()))
    }

    fn type_name(&self) -> &'static str {
        "object"
    }

    fn query_state(&self, state: State) -> bool {
        match state {
            State::Truthy => true,
            State::DefaultValue | State::Empty | State::Blank => self.params.is_empty(),
        }
    }

    fn to_kstr(&self) -> KStringCow<'_> {
        KStringCow::from(self.to_string())
    }

    fn to_value(&self) -> Value {
        let mut obj = liquid_core::Object::new();
        for (k, v) in &self.params {
            obj.insert(k.clone().into(), v.clone());
        }
        Value::Object(obj)
    }

    fn as_object(&self) -> Option<&dyn ObjectView> {
        Some(self)
    }
}

impl ObjectView for LenientIncludeParams {
    fn as_value(&self) -> &dyn ValueView {
        self
    }

    fn size(&self) -> i64 {
        self.params.len() as i64
    }

    fn keys<'k>(&'k self) -> Box<dyn Iterator<Item = KStringCow<'k>> + 'k> {
        Box::new(self.params.keys().map(|k| KStringCow::from(k.as_str())))
    }

    fn values<'k>(&'k self) -> Box<dyn Iterator<Item = &'k dyn ValueView> + 'k> {
        Box::new(self.params.values().map(|v| v as &dyn ValueView))
    }

    fn iter<'k>(&'k self) -> Box<dyn Iterator<Item = (KStringCow<'k>, &'k dyn ValueView)> + 'k> {
        Box::new(
            self.params
                .iter()
                .map(|(k, v)| (KStringCow::from(k.as_str()), v as &dyn ValueView)),
        )
    }

    fn contains_key(&self, _index: &str) -> bool {
        // Always return true so the runtime doesn't error on missing keys
        true
    }

    fn get<'s>(&'s self, index: &str) -> Option<&'s dyn ValueView> {
        self.params
            .get(index)
            .map(|v| v as &dyn ValueView)
            .or(Some(&self.nil as &dyn ValueView))
    }
}

impl Renderable for LenientInclude {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> Result<()> {
        let name = match &self.partial {
            IncludePath::Literal(expr) => expr.evaluate(runtime)?.render().to_string(),
            IncludePath::Dynamic(chain) => {
                let value = chain.evaluate(runtime)?;
                if value.is_nil() {
                    return Err(Error::with_msg(
                        "Dynamic include path resolved to nil. \
                         The variable used in {% include {{ ... }} %} is not set."
                            .to_string(),
                    ));
                }
                let rendered = value.render().to_string();
                if rendered.is_empty() {
                    return Err(Error::with_msg(
                        "Dynamic include path resolved to empty string. \
                         The variable used in {% include {{ ... }} %} is empty."
                            .to_string(),
                    ));
                }
                rendered
            }
        };

        {
            // Always create a lenient include object, even when there are no vars.
            // This way, accessing include.missing_param returns Nil instead of erroring.
            let mut params = HashMap::new();
            for (id, val) in &self.vars {
                let value = val
                    .try_evaluate(runtime)
                    .ok_or_else(|| Error::with_msg("failed to evaluate value"))?
                    .into_owned();
                params.insert(id.to_string(), value);
            }

            let lenient_params = LenientIncludeParams::new(params);

            let mut pass_through =
                HashMap::<liquid_core::model::KStringRef<'_>, &dyn ValueView>::new();
            pass_through.insert("include".into(), &lenient_params);

            let scope = StackFrame::new(runtime, &pass_through);
            let partial = scope
                .partials()
                .get(&name)
                .trace_with(|| format!("{{% include {} %}}", self.partial).into())?;

            partial
                .render_to(writer, &scope)
                .trace_with(|| format!("{{% include {} %}}", self.partial).into())
                .context_key_with(|| self.partial.to_string().into())
                .value_with(|| name.clone().into())?;
        }

        Ok(())
    }
}

/// Pre-process a template string to quote include paths containing `/`.
///
/// The Liquid parser's pest grammar does not recognize `/` as a valid token
/// inside tag arguments. Jekyll `{% include subdir/file.html %}` uses
/// unquoted paths with directory separators, which causes a parse error.
///
/// This function finds `{% include path/to/file.html ... %}` patterns and
/// wraps the path in double quotes so the Liquid parser can handle it as a
/// string literal: `{% include "path/to/file.html" ... %}`.
///
/// Paths without `/` are left unchanged.
pub fn preprocess_include_paths(template: &str) -> String {
    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("{%") {
        // Copy everything up to this tag
        result.push_str(&remaining[..start]);

        // Find the closing %}
        let after_open = &remaining[start + 2..];
        if let Some(end_offset) = after_open.find("%}") {
            let tag_inner = &after_open[..end_offset];
            let tag_end = start + 2 + end_offset + 2;

            // Check if this is an include tag with a path containing /
            let trimmed = tag_inner.trim();

            // Handle whitespace-control variants (e.g., {%- include -%})
            let trimmed = trimmed.strip_prefix('-').unwrap_or(trimmed).trim();
            let trimmed_end = trimmed.strip_suffix('-').unwrap_or(trimmed).trim();

            if let Some(after_include) = trimmed_end
                .strip_prefix("include")
                .filter(|rest| rest.starts_with(' ') || rest.starts_with('\t'))
            {
                let after_include = after_include.trim_start();

                // Handle dynamic include paths: {% include {{ expr }} ... %}
                // Replace with sentinel + expression so the parser can
                // distinguish dynamic paths from literal filenames.
                if after_include.starts_with("{{") {
                    if let Some(close_pos) = after_include.find("}}") {
                        let inner_expr = after_include[2..close_pos].trim();
                        let rest_after_braces = after_include[close_pos + 2..].trim_start();
                        // Preserve original whitespace-control markers
                        let orig_tag = &remaining[start..tag_end];
                        let open_marker = if orig_tag.starts_with("{%-") {
                            "{%-"
                        } else {
                            "{%"
                        };
                        let close_marker = if orig_tag.ends_with("-%}") {
                            "-%}"
                        } else {
                            "%}"
                        };
                        result.push_str(open_marker);
                        result.push_str(" include ");
                        result.push_str(DYNAMIC_INCLUDE_SENTINEL);
                        result.push(' ');
                        result.push_str(inner_expr);
                        if !rest_after_braces.is_empty() {
                            result.push(' ');
                            result.push_str(rest_after_braces);
                        }
                        result.push(' ');
                        result.push_str(close_marker);
                        remaining = &remaining[tag_end..];
                        continue;
                    }
                }

                // Check if the first "word" (up to space or %}) contains a /
                let path_end = after_include
                    .find([' ', '\t'])
                    .unwrap_or(after_include.len());
                let path = &after_include[..path_end];

                if path.contains('/') && !path.starts_with('"') && !path.starts_with('\'') {
                    // Reconstruct the tag with quoted path
                    let rest_after_path = &after_include[path_end..];
                    // Preserve original whitespace-control markers
                    let orig_tag = &remaining[start..tag_end];
                    let open_marker = if orig_tag.starts_with("{%-") {
                        "{%-"
                    } else {
                        "{%"
                    };
                    let close_marker = if orig_tag.ends_with("-%}") {
                        "-%}"
                    } else {
                        "%}"
                    };
                    result.push_str(open_marker);
                    result.push_str(" include \"");
                    result.push_str(path);
                    result.push('"');
                    result.push_str(rest_after_path);
                    result.push(' ');
                    result.push_str(close_marker);
                    remaining = &remaining[tag_end..];
                    continue;
                }
            }

            // Not an include with / -- copy as-is
            result.push_str(&remaining[start..tag_end]);
            remaining = &remaining[tag_end..];
        } else {
            // No closing %} -- copy rest as-is
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }

    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_simple_include_unchanged() {
        let input = "{% include simple.html %}";
        assert_eq!(preprocess_include_paths(input), input);
    }

    #[test]
    fn test_preprocess_subdirectory_include() {
        let input = "{% include subdir/file.html %}";
        let output = preprocess_include_paths(input);
        assert!(
            output.contains("\"subdir/file.html\""),
            "Expected quoted path, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_deeply_nested_include() {
        let input = "{% include a/b/c.html %}";
        let output = preprocess_include_paths(input);
        assert!(
            output.contains("\"a/b/c.html\""),
            "Expected quoted path, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_include_with_params() {
        let input = r#"{% include subdir/file.html param="value" %}"#;
        let output = preprocess_include_paths(input);
        assert!(
            output.contains("\"subdir/file.html\""),
            "Expected quoted path, got: {}",
            output
        );
        assert!(
            output.contains("param=\"value\""),
            "Expected params preserved, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_no_double_quote_already_quoted() {
        let input = r#"{% include "subdir/file.html" %}"#;
        let output = preprocess_include_paths(input);
        // Should remain unchanged since it's already quoted
        assert_eq!(output, input);
    }

    #[test]
    fn test_preprocess_preserves_non_include_tags() {
        let input = "{% if true %}hello{% endif %}";
        assert_eq!(preprocess_include_paths(input), input);
    }

    #[test]
    fn test_preprocess_mixed_content() {
        let input = "<p>{% include subdir/file.html %}</p>{% if true %}yes{% endif %}";
        let output = preprocess_include_paths(input);
        assert!(output.contains("\"subdir/file.html\""));
        assert!(output.contains("{% if true %}"));
    }

    // ========================================================================
    // Issue 41: Dynamic include paths
    // ========================================================================

    #[test]
    fn test_preprocess_dynamic_include_strips_braces() {
        let input = "{% include {{ page.form | append: '.html' }} %}";
        let output = preprocess_include_paths(input);
        assert!(
            !output.contains("{{"),
            "Should strip {{ wrapper, got: {}",
            output
        );
        assert!(
            !output.contains("}}"),
            "Should strip }} wrapper, got: {}",
            output
        );
        assert!(
            output.contains("page.form"),
            "Should preserve inner expression, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_dynamic_include_simple_var() {
        let input = "{% include {{ var }} %}";
        let output = preprocess_include_paths(input);
        assert!(
            !output.contains("{{"),
            "Should strip {{ wrapper, got: {}",
            output
        );
        assert!(
            output.contains("var"),
            "Should preserve variable name, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_dynamic_and_static_mixed() {
        let input = "{% include {{ var }} %}\n{% include subdir/file.html %}";
        let output = preprocess_include_paths(input);
        // Dynamic include should have {{ stripped
        assert!(
            !output.contains("{{"),
            "Dynamic include should have {{ stripped, got: {}",
            output
        );
        // Static subdirectory include should be quoted
        assert!(
            output.contains("\"subdir/file.html\""),
            "Static include should be quoted, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_dynamic_include_with_params() {
        let input = r#"{% include {{ page.form }} param1="value" %}"#;
        let output = preprocess_include_paths(input);
        assert!(
            !output.contains("{{"),
            "Should strip {{ wrapper, got: {}",
            output
        );
        assert!(
            output.contains("page.form"),
            "Should preserve variable, got: {}",
            output
        );
        assert!(
            output.contains(r#"param1="value""#),
            "Should preserve params, got: {}",
            output
        );
    }
}
