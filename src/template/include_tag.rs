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

/// A variant of `LenientIncludeTag` that registers under the name `include_cached`.
///
/// The `jekyll-include-cache` plugin provides this tag. It is functionally
/// identical to `include` but caches the rendered output. Since rustkyll does
/// not do incremental re-rendering, we simply treat it as an alias.
#[derive(Copy, Clone, Debug, Default)]
pub struct LenientIncludeCachedTag;

impl TagReflection for LenientIncludeCachedTag {
    fn tag(&self) -> &'static str {
        "include_cached"
    }

    fn description(&self) -> &'static str {
        "jekyll-include-cache: alias for include with lenient parameter access"
    }
}

impl ParseTag for LenientIncludeCachedTag {
    fn parse(
        &self,
        arguments: TagTokenIter<'_>,
        options: &Language,
    ) -> Result<Box<dyn Renderable>> {
        // Delegate to the same parsing logic as LenientIncludeTag
        LenientIncludeTag.parse(arguments, options)
    }

    fn reflection(&self) -> &dyn TagReflection {
        self
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

/// Replace backslash-escaped quotes inside double-quoted and single-quoted
/// parameter values of an include tag.
///
/// Jekyll supports `\"` inside double-quoted include parameter values and `\'`
/// inside single-quoted values. After parsing, Jekyll unescapes them. The
/// Liquid parser's pest grammar does not support backslash escapes, so we
/// replace `\"` with `&quot;` and `\'` with `&#39;` before the Liquid parser
/// sees the template. Since include parameter values typically end up in HTML
/// output, using HTML entities is semantically correct.
fn unescape_include_params(params: &str) -> String {
    let mut result = String::with_capacity(params.len());
    let mut chars = params.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            // Start of a double-quoted value -- copy everything inside,
            // replacing `\"` with `&quot;`
            result.push('"');
            loop {
                match chars.next() {
                    Some('\\') if chars.peek() == Some(&'"') => {
                        chars.next(); // consume the escaped quote
                        result.push_str("&quot;");
                    }
                    Some('"') => {
                        result.push('"');
                        break;
                    }
                    Some(c) => result.push(c),
                    None => break, // unterminated string -- stop gracefully
                }
            }
        } else if ch == '\'' {
            // Start of a single-quoted value -- copy everything inside,
            // replacing `\'` with `&#39;`
            result.push('\'');
            loop {
                match chars.next() {
                    Some('\\') if chars.peek() == Some(&'\'') => {
                        chars.next();
                        result.push_str("&#39;");
                    }
                    Some('\'') => {
                        result.push('\'');
                        break;
                    }
                    Some(c) => result.push(c),
                    None => break,
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Pre-process a template string to handle Jekyll include tag quirks.
///
/// The Liquid parser's pest grammar does not recognize:
/// 1. `/` as a valid token inside tag arguments (unquoted subdirectory paths)
/// 2. `\"` as an escape sequence inside double-quoted strings
///
/// This function:
/// - Wraps unquoted paths containing `/` in double quotes
/// - Replaces `\"` inside double-quoted parameter values with `&quot;`
/// - Replaces `\'` inside single-quoted parameter values with `&#39;`
/// - Handles dynamic includes (`{% include {{ expr }} %}`)
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

            // Check if this is an include tag
            let trimmed = tag_inner.trim();

            // Handle whitespace-control variants (e.g., {%- include -%})
            let trimmed = trimmed.strip_prefix('-').unwrap_or(trimmed).trim();
            let trimmed_end = trimmed.strip_suffix('-').unwrap_or(trimmed).trim();

            // Match both `include` and `include_cached` tags.
            // Try `include_cached` first (longer prefix) to avoid partial match.
            let include_match = trimmed_end
                .strip_prefix("include_cached")
                .filter(|rest| rest.starts_with(' ') || rest.starts_with('\t'))
                .map(|rest| ("include_cached", rest))
                .or_else(|| {
                    trimmed_end
                        .strip_prefix("include")
                        .filter(|rest| rest.starts_with(' ') || rest.starts_with('\t'))
                        .map(|rest| ("include", rest))
                });

            if let Some((tag_name, after_tag_name)) = include_match {
                let after_include = after_tag_name.trim_start();

                // Handle dynamic/interpolated include paths that contain {{ expr }}.
                // This covers three patterns:
                //   1. Fully dynamic:    {% include {{ expr }} %}
                //   2. Suffix only:      {% include {{ expr }}.html %}
                //   3. Interpolated:     {% include prefix/{{ expr }}.suffix %}
                // All are rewritten to use the DYNAMIC_INCLUDE_SENTINEL with
                // appropriate prepend/append filters for any literal prefix/suffix.
                if let Some(open_pos) = after_include.find("{{") {
                    if let Some(close_pos) = after_include.find("}}") {
                        let prefix = &after_include[..open_pos];
                        let inner_expr = after_include[open_pos + 2..close_pos].trim();
                        let after_braces = &after_include[close_pos + 2..];

                        // The suffix is the literal text immediately after }}
                        // up to the first whitespace (or end of string).
                        // Everything after the suffix is params / rest.
                        let suffix_end =
                            after_braces.find([' ', '\t']).unwrap_or(after_braces.len());
                        let suffix = &after_braces[..suffix_end];
                        let rest_after_path = after_braces[suffix_end..].trim_start();

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

                        // Build the rewritten expression: inner_expr with
                        // optional prepend (for prefix) and append (for suffix).
                        let mut expr = inner_expr.to_string();
                        if !prefix.is_empty() {
                            expr.push_str(&format!(r#" | prepend: "{}""#, prefix));
                        }
                        if !suffix.is_empty() {
                            expr.push_str(&format!(r#" | append: "{}""#, suffix));
                        }

                        result.push_str(open_marker);
                        result.push(' ');
                        result.push_str(tag_name);
                        result.push(' ');
                        result.push_str(DYNAMIC_INCLUDE_SENTINEL);
                        result.push(' ');
                        result.push_str(&expr);
                        if !rest_after_path.is_empty() {
                            result.push(' ');
                            result.push_str(rest_after_path);
                        }
                        result.push(' ');
                        result.push_str(close_marker);
                        remaining = &remaining[tag_end..];
                        continue;
                    }
                }

                // Extract the path (first "word" up to space or end)
                let path_end = after_include
                    .find([' ', '\t'])
                    .unwrap_or(after_include.len());
                let path = &after_include[..path_end];
                let rest_after_path = &after_include[path_end..];

                let needs_path_quoting =
                    path.contains('/') && !path.starts_with('"') && !path.starts_with('\'');
                let has_escaped_quotes =
                    rest_after_path.contains("\\\"") || rest_after_path.contains("\\'");

                if needs_path_quoting || has_escaped_quotes {
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
                    result.push(' ');
                    result.push_str(tag_name);
                    result.push(' ');
                    if needs_path_quoting {
                        result.push('"');
                        result.push_str(path);
                        result.push('"');
                    } else {
                        result.push_str(path);
                    }
                    if has_escaped_quotes {
                        result.push_str(&unescape_include_params(rest_after_path));
                    } else {
                        result.push_str(rest_after_path);
                    }
                    result.push(' ');
                    result.push_str(close_marker);
                    remaining = &remaining[tag_end..];
                    continue;
                }
            }

            // Not an include with special handling -- copy as-is
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

    // ========================================================================
    // Issue 50: Escaped quotes in include parameters
    // ========================================================================

    #[test]
    fn test_unescape_include_params_double_quotes() {
        let input = r#" param="value with \"escaped\" quotes""#;
        let output = unescape_include_params(input);
        assert_eq!(output, r#" param="value with &quot;escaped&quot; quotes""#);
    }

    #[test]
    fn test_unescape_include_params_single_quotes() {
        let input = r" param='value with \'escaped\' quotes'";
        let output = unescape_include_params(input);
        assert_eq!(output, " param='value with &#39;escaped&#39; quotes'");
    }

    #[test]
    fn test_unescape_include_params_no_escapes() {
        let input = r#" param="normal value" other="also normal""#;
        let output = unescape_include_params(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_unescape_include_params_mixed() {
        let input = r#" normal="ok" escaped="has \"quotes\"" also="fine""#;
        let output = unescape_include_params(input);
        assert_eq!(
            output,
            r#" normal="ok" escaped="has &quot;quotes&quot;" also="fine""#
        );
    }

    #[test]
    fn test_preprocess_escaped_quotes_simple() {
        let input = r#"{% include file.html param="value with \"escaped\" quotes" %}"#;
        let output = preprocess_include_paths(input);
        assert!(
            !output.contains("\\\""),
            "Should not contain backslash-escaped quotes, got: {}",
            output
        );
        assert!(
            output.contains("&quot;"),
            "Should contain HTML entity quotes, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_escaped_quotes_anchor_headings() {
        // The exact failing line from DataTalksClub/docs default.html
        let input = r##"{% include vendor/anchor_headings.html html=content beforeHeading="true" anchorBody="<svg viewBox=\"0 0 16 16\" aria-hidden=\"true\"><use xlink:href=\"#svg-link\"></use></svg>" anchorClass="anchor-heading" anchorAttrs="aria-labelledby=\"%html_id%\"" %}"##;
        let output = preprocess_include_paths(input);
        // Path should be quoted (contains /)
        assert!(
            output.contains(r#""vendor/anchor_headings.html""#),
            "Path should be quoted, got: {}",
            output
        );
        // Escaped quotes should be replaced with &quot;
        assert!(
            !output.contains("\\\""),
            "Should not contain backslash-escaped quotes, got: {}",
            output
        );
        assert!(
            output.contains("viewBox=&quot;0 0 16 16&quot;"),
            "Should have HTML entity quotes in viewBox, got: {}",
            output
        );
        assert!(
            output.contains("aria-hidden=&quot;true&quot;"),
            "Should have HTML entity quotes in aria-hidden, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_escaped_quotes_no_path_slash() {
        // Include without / in path but with escaped quotes in params
        let input = r#"{% include file.html param="has \"escaped\" quotes" normal="ok" %}"#;
        let output = preprocess_include_paths(input);
        assert!(
            !output.contains("\\\""),
            "Should not contain backslash-escaped quotes, got: {}",
            output
        );
        assert!(
            output.contains("&quot;escaped&quot;"),
            "Should replace with HTML entities, got: {}",
            output
        );
        assert!(
            output.contains("normal=\"ok\""),
            "Non-escaped params should be preserved, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_escaped_quotes_preserves_other_tags() {
        let input = r#"{% if true %}{% include file.html param="\"escaped\"" %}{% endif %}"#;
        let output = preprocess_include_paths(input);
        assert!(output.contains("{% if true %}"));
        assert!(output.contains("{% endif %}"));
        assert!(
            !output.contains("\\\""),
            "Escaped quotes should be replaced, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_no_escaped_quotes_unchanged() {
        // Regression: include without escaped quotes should be identical
        let input = r#"{% include file.html param="normal" %}"#;
        assert_eq!(preprocess_include_paths(input), input);
    }

    #[test]
    fn test_preprocess_whitespace_control_with_escaped_quotes() {
        let input = r#"{%- include file.html param="\"escaped\"" -%}"#;
        let output = preprocess_include_paths(input);
        assert!(
            output.starts_with("{%-"),
            "Should preserve open whitespace control, got: {}",
            output
        );
        assert!(
            output.ends_with("-%}"),
            "Should preserve close whitespace control, got: {}",
            output
        );
        assert!(
            !output.contains("\\\""),
            "Should replace escaped quotes, got: {}",
            output
        );
    }

    // ========================================================================
    // Issue 53: include_cached tag support
    // ========================================================================

    #[test]
    fn test_preprocess_include_cached_simple_unchanged() {
        let input = "{% include_cached foo.html %}";
        assert_eq!(preprocess_include_paths(input), input);
    }

    #[test]
    fn test_preprocess_include_cached_subdirectory() {
        let input = "{% include_cached subdir/foo.html %}";
        let output = preprocess_include_paths(input);
        assert!(
            output.contains("\"subdir/foo.html\""),
            "Expected quoted path, got: {}",
            output
        );
        assert!(
            output.contains("include_cached"),
            "Should preserve include_cached tag name, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_include_cached_with_params_unchanged() {
        let input = r#"{% include_cached foo.html param="val" %}"#;
        assert_eq!(preprocess_include_paths(input), input);
    }

    #[test]
    fn test_preprocess_include_cached_whitespace_control() {
        let input = "{%- include_cached foo.html -%}";
        assert_eq!(preprocess_include_paths(input), input);
    }

    #[test]
    fn test_preprocess_include_cached_whitespace_control_subdir() {
        let input = "{%- include_cached subdir/foo.html -%}";
        let output = preprocess_include_paths(input);
        assert!(
            output.starts_with("{%-"),
            "Should preserve open whitespace control, got: {}",
            output
        );
        assert!(
            output.ends_with("-%}"),
            "Should preserve close whitespace control, got: {}",
            output
        );
        assert!(
            output.contains("\"subdir/foo.html\""),
            "Expected quoted path, got: {}",
            output
        );
        assert!(
            output.contains("include_cached"),
            "Should preserve include_cached tag name, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_mixed_include_and_include_cached() {
        let input = "{% include subdir/a.html %}\n{% include_cached subdir/b.html %}";
        let output = preprocess_include_paths(input);
        assert!(
            output.contains("include \"subdir/a.html\""),
            "include tag should be processed, got: {}",
            output
        );
        assert!(
            output.contains("include_cached \"subdir/b.html\""),
            "include_cached tag should be processed, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_include_cached_dynamic() {
        let input = "{% include_cached {{ page.partial }} %}";
        let output = preprocess_include_paths(input);
        assert!(
            !output.contains("{{"),
            "Should strip {{ wrapper, got: {}",
            output
        );
        assert!(
            output.contains("include_cached"),
            "Should preserve include_cached tag name, got: {}",
            output
        );
        assert!(
            output.contains("page.partial"),
            "Should preserve variable name, got: {}",
            output
        );
        assert!(
            output.contains(DYNAMIC_INCLUDE_SENTINEL),
            "Should contain dynamic include sentinel, got: {}",
            output
        );
    }

    // ========================================================================
    // Issue 256: Interpolated variable paths in include tags
    // ========================================================================

    #[test]
    fn test_preprocess_interpolated_path_basic() {
        // analytics/{{ platform }}.html -> dynamic include with prepend/append
        let input = "{% include analytics/{{ platform }}.html %}";
        let output = preprocess_include_paths(input);
        assert!(
            output.contains(DYNAMIC_INCLUDE_SENTINEL),
            "Should contain dynamic include sentinel, got: {}",
            output
        );
        assert!(
            !output.contains("{{"),
            "Should strip {{ braces, got: {}",
            output
        );
        assert!(
            output.contains("platform"),
            "Should preserve variable name, got: {}",
            output
        );
        // Should have prepend for the prefix and append for the suffix
        assert!(
            output.contains(r#"prepend: "analytics/""#),
            "Should prepend the prefix, got: {}",
            output
        );
        assert!(
            output.contains(r#"append: ".html""#),
            "Should append the suffix, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_interpolated_path_prefix_only() {
        // dir/{{ var }} -> dynamic include with only prepend
        let input = "{% include dir/{{ var }} %}";
        let output = preprocess_include_paths(input);
        assert!(
            output.contains(DYNAMIC_INCLUDE_SENTINEL),
            "Should contain dynamic include sentinel, got: {}",
            output
        );
        assert!(
            output.contains(r#"prepend: "dir/""#),
            "Should prepend the prefix, got: {}",
            output
        );
        // No append needed since there's no suffix
        assert!(
            !output.contains("append:"),
            "Should not have append when there's no suffix, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_interpolated_path_suffix_only() {
        // {{ var }}.html -> dynamic include with only append
        let input = "{% include {{ var }}.html %}";
        let output = preprocess_include_paths(input);
        assert!(
            output.contains(DYNAMIC_INCLUDE_SENTINEL),
            "Should contain dynamic include sentinel, got: {}",
            output
        );
        assert!(
            output.contains(r#"append: ".html""#),
            "Should append the suffix, got: {}",
            output
        );
        // No prepend needed since there's no prefix
        assert!(
            !output.contains("prepend:"),
            "Should not have prepend when there's no prefix, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_interpolated_path_with_filters() {
        // analytics/{{ platform | downcase }}.html -> preserve filters, add prepend/append
        let input = "{% include analytics/{{ platform | downcase }}.html %}";
        let output = preprocess_include_paths(input);
        assert!(
            output.contains(DYNAMIC_INCLUDE_SENTINEL),
            "Should contain dynamic include sentinel, got: {}",
            output
        );
        assert!(
            output.contains("platform | downcase"),
            "Should preserve existing filters, got: {}",
            output
        );
        assert!(
            output.contains(r#"prepend: "analytics/""#),
            "Should prepend the prefix, got: {}",
            output
        );
        assert!(
            output.contains(r#"append: ".html""#),
            "Should append the suffix, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_interpolated_path_with_params() {
        // analytics/{{ var }}.html param="value" -> preserve params
        let input = r#"{% include analytics/{{ var }}.html param="value" %}"#;
        let output = preprocess_include_paths(input);
        assert!(
            output.contains(DYNAMIC_INCLUDE_SENTINEL),
            "Should contain dynamic include sentinel, got: {}",
            output
        );
        assert!(
            output.contains(r#"param="value""#),
            "Should preserve params after the path, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_interpolated_path_include_cached() {
        let input = "{% include_cached analytics/{{ var }}.html %}";
        let output = preprocess_include_paths(input);
        assert!(
            output.contains(DYNAMIC_INCLUDE_SENTINEL),
            "Should contain dynamic include sentinel, got: {}",
            output
        );
        assert!(
            output.contains("include_cached"),
            "Should preserve include_cached tag name, got: {}",
            output
        );
        assert!(
            output.contains(r#"prepend: "analytics/""#),
            "Should prepend the prefix, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_interpolated_path_whitespace_control() {
        let input = "{%- include analytics/{{ var }}.html -%}";
        let output = preprocess_include_paths(input);
        assert!(
            output.starts_with("{%-"),
            "Should preserve open whitespace control, got: {}",
            output
        );
        assert!(
            output.ends_with("-%}"),
            "Should preserve close whitespace control, got: {}",
            output
        );
        assert!(
            output.contains(DYNAMIC_INCLUDE_SENTINEL),
            "Should contain dynamic include sentinel, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_fully_dynamic_still_works() {
        // Regression: fully dynamic path (no prefix/suffix) should still work
        let input = "{% include {{ var }} %}";
        let output = preprocess_include_paths(input);
        assert!(
            output.contains(DYNAMIC_INCLUDE_SENTINEL),
            "Should contain dynamic include sentinel, got: {}",
            output
        );
        assert!(
            output.contains("var"),
            "Should preserve variable name, got: {}",
            output
        );
        // Fully dynamic: no prepend/append needed
        assert!(
            !output.contains("prepend:"),
            "Should not have prepend for fully dynamic, got: {}",
            output
        );
        assert!(
            !output.contains("append:"),
            "Should not have append for fully dynamic, got: {}",
            output
        );
    }

    #[test]
    fn test_preprocess_no_interpolation_unchanged() {
        // Regression: literal paths should remain unchanged
        let input = "{% include simple.html %}";
        assert_eq!(preprocess_include_paths(input), input);

        let input2 = "{% include subdir/file.html %}";
        let output2 = preprocess_include_paths(input2);
        // Should be quoted (existing behavior), not treated as dynamic
        assert!(
            output2.contains("\"subdir/file.html\""),
            "Should quote path with /, got: {}",
            output2
        );
        assert!(
            !output2.contains(DYNAMIC_INCLUDE_SENTINEL),
            "Should NOT contain dynamic sentinel for literal paths, got: {}",
            output2
        );
    }
}
