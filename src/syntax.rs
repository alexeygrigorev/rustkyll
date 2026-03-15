//! Syntax highlighting using syntect, producing Rouge/Pygments-compatible CSS class spans.
//!
//! Rouge (used by Jekyll) and Pygments use the same CSS class names for tokens.
//! Syntect uses TextMate grammar scopes. This module maps TextMate scopes to the
//! corresponding Rouge/Pygments CSS class names so that existing `syntax.css`
//! stylesheets work without modification.

use std::sync::OnceLock;

use syntect::highlighting::ScopeSelectors;
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// A single scope-to-CSS-class mapping rule.
struct ScopeMapping {
    selectors: ScopeSelectors,
    css_class: &'static str,
}

/// Build the scope-to-CSS-class mapping table.
///
/// The mapping covers the standard Pygments/Rouge token types that appear in
/// the `syntax.css` file used by the DataTalks.Club site (and most Jekyll themes).
///
/// Order matters: more specific selectors should come first so that the first
/// match wins.
fn build_scope_map() -> Vec<ScopeMapping> {
    let rules: &[(&str, &str)] = &[
        // Comments
        ("comment.line.number-sign", "c1"),
        ("comment.line.double-slash", "c1"),
        ("comment.line", "c1"),
        ("comment.block.documentation", "sd"),
        ("comment.block", "cm"),
        ("comment", "c"),
        // Strings
        ("string.quoted.double", "s2"),
        ("string.quoted.single", "s1"),
        ("string.quoted.triple", "sd"),
        ("string.interpolated", "si"),
        ("string.regexp", "sr"),
        ("string.other", "sx"),
        ("string", "s"),
        // Numbers
        ("constant.numeric.float", "mf"),
        ("constant.numeric.integer.hexadecimal", "mh"),
        ("constant.numeric.integer.octal", "mo"),
        ("constant.numeric.integer.binary", "mb"),
        ("constant.numeric.integer", "mi"),
        ("constant.numeric", "m"),
        // Constants
        ("constant.language", "kc"),
        ("constant.other", "no"),
        ("constant.character.escape", "se"),
        // Keywords
        ("keyword.control.import", "kn"),
        ("keyword.control", "k"),
        ("keyword.operator.logical", "ow"),
        ("keyword.operator", "o"),
        ("keyword.other", "k"),
        ("keyword.declaration", "kd"),
        ("keyword", "k"),
        // Storage (def, class, var, let, etc.)
        ("storage.type.function", "k"),
        ("storage.type.class", "k"),
        ("storage.type", "kt"),
        ("storage.modifier", "k"),
        ("storage", "k"),
        // Entity names
        ("entity.name.function", "nf"),
        ("entity.name.class", "nc"),
        ("entity.name.tag", "nt"),
        ("entity.name.namespace", "nn"),
        ("entity.name.type", "nc"),
        ("entity.other.attribute-name", "na"),
        ("entity.other.inherited-class", "nb"),
        // Variables
        ("variable.parameter", "nv"),
        ("variable.language", "bp"),
        ("variable.other", "n"),
        // Support (built-in functions/types)
        ("support.function.builtin", "nb"),
        ("support.function", "nb"),
        ("support.type", "nb"),
        ("support.class", "nb"),
        ("support.other", "n"),
        // Punctuation
        ("punctuation.definition.string", "s"),
        ("punctuation.separator", "p"),
        ("punctuation.section", "p"),
        ("punctuation.accessor", "o"),
        ("punctuation", "p"),
        // Meta (decorators)
        ("meta.function.decorator", "nd"),
        ("meta.function-call.arguments", "n"),
        // Operators not already caught
        ("keyword.operator.assignment", "o"),
    ];

    rules
        .iter()
        .filter_map(|(scope_str, css)| {
            let selectors: ScopeSelectors = scope_str.parse().ok()?;
            Some(ScopeMapping {
                selectors,
                css_class: css,
            })
        })
        .collect()
}

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Look up the syntect `SyntaxReference` for a language name.
///
/// Tries the name as-is first, then common aliases.
fn find_syntax(lang: &str) -> Option<&'static SyntaxReference> {
    let ss = get_syntax_set();

    // Try exact name match first
    if let Some(syn) = ss.find_syntax_by_token(lang) {
        return Some(syn);
    }

    // Common aliases
    let alias = match lang {
        "yml" => "yaml",
        "js" => "javascript",
        "ts" => "typescript",
        "sh" | "shell" | "console" => "bash",
        "dockerfile" => "Dockerfile",
        "rb" => "ruby",
        "py" => "python",
        _ => return None,
    };
    ss.find_syntax_by_token(alias)
}

/// Map a syntect scope stack to a Rouge/Pygments CSS class.
///
/// Returns `None` if the scope is plain text (no highlighting needed).
fn scope_to_css_class(
    scope_map: &[ScopeMapping],
    scope: &syntect::parsing::ScopeStack,
) -> Option<&'static str> {
    for mapping in scope_map {
        if mapping.selectors.does_match(scope.as_slice()).is_some() {
            return Some(mapping.css_class);
        }
    }
    None
}

/// HTML-escape a string (only the characters that matter inside a `<code>` block).
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Highlight `code` using the given language, returning HTML with
/// Rouge/Pygments-compatible `<span class="xx">` tokens.
///
/// If the language is unknown or is `"plaintext"`, returns `None` so the
/// caller can fall back to plain HTML-escaped code.
pub fn highlight_code(lang: &str, code: &str) -> Option<String> {
    if lang.is_empty() || lang == "plaintext" {
        return None;
    }

    let syntax = find_syntax(lang)?;
    let ss = get_syntax_set();
    let scope_map = scope_map();

    let mut html = String::with_capacity(code.len() * 2);
    let mut parse_state = syntect::parsing::ParseState::new(syntax);
    let mut scope_stack = syntect::parsing::ScopeStack::new();

    for line in syntect::util::LinesWithEndings::from(code) {
        let ops = parse_state.parse_line(line, ss).ok()?;
        let mut cur_pos = 0;

        for (byte_offset, op) in ops {
            // Emit text before this operation
            if byte_offset > cur_pos {
                let text = &line[cur_pos..byte_offset];
                emit_text(&mut html, text, &scope_stack, scope_map);
            }
            cur_pos = byte_offset;
            scope_stack.apply(&op).ok()?;
        }

        // Emit remaining text on this line
        if cur_pos < line.len() {
            let text = &line[cur_pos..];
            emit_text(&mut html, text, &scope_stack, scope_map);
        }
    }

    Some(html)
}

/// Emit a text fragment, wrapped in a `<span>` if it has a CSS class.
fn emit_text(
    html: &mut String,
    text: &str,
    scope_stack: &syntect::parsing::ScopeStack,
    scope_map: &[ScopeMapping],
) {
    if text.is_empty() {
        return;
    }

    let escaped = html_escape(text);

    if let Some(css_class) = scope_to_css_class(scope_map, scope_stack) {
        // Split by newlines to avoid wrapping newlines inside spans
        // (Rouge outputs spans that don't cross line boundaries)
        let mut first = true;
        for part in escaped.split('\n') {
            if !first {
                html.push('\n');
            }
            first = false;
            if !part.is_empty() {
                html.push_str("<span class=\"");
                html.push_str(css_class);
                html.push_str("\">");
                html.push_str(part);
                html.push_str("</span>");
            }
        }
    } else {
        html.push_str(&escaped);
    }
}

static SCOPE_MAP: OnceLock<Vec<ScopeMapping>> = OnceLock::new();

fn scope_map() -> &'static Vec<ScopeMapping> {
    SCOPE_MAP.get_or_init(build_scope_map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_python_produces_spans() {
        let code = "import os\n";
        let result = highlight_code("python", code);
        assert!(result.is_some(), "python should be a known language");
        let html = result.unwrap();
        assert!(
            html.contains("<span class=\""),
            "highlighted output should contain span elements: {html}"
        );
        // 'import' should be keyword.namespace -> kn
        assert!(
            html.contains("<span class=\"kn\">import</span>"),
            "import should be highlighted as kn: {html}"
        );
    }

    #[test]
    fn test_highlight_plaintext_returns_none() {
        assert!(highlight_code("plaintext", "hello").is_none());
        assert!(highlight_code("", "hello").is_none());
    }

    #[test]
    fn test_highlight_unknown_lang_returns_none() {
        assert!(highlight_code("nonexistent_language_xyz", "hello").is_none());
    }

    #[test]
    fn test_highlight_python_string() {
        let code = "x = \"hello\"\n";
        let html = highlight_code("python", code).unwrap();
        // The string should be highlighted with an s-family class
        assert!(
            html.contains("<span class=\"s2\">&quot;hello&quot;</span>")
                || html.contains("<span class=\"s\">")
                || html.contains("class=\"s1\"")
                || html.contains("class=\"s2\""),
            "string should be highlighted: {html}"
        );
    }

    #[test]
    fn test_highlight_python_comment() {
        let code = "# comment\n";
        let html = highlight_code("python", code).unwrap();
        assert!(
            html.contains("class=\"c1\"") || html.contains("class=\"c\""),
            "comment should be highlighted: {html}"
        );
    }

    #[test]
    fn test_highlight_yaml() {
        let code = "name: CI\n";
        let html = highlight_code("yaml", code).unwrap();
        assert!(
            html.contains("<span class=\""),
            "yaml should produce spans: {html}"
        );
    }

    #[test]
    fn test_highlight_bash() {
        let code = "git checkout -b dev\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\""),
            "bash should produce spans: {html}"
        );
    }

    #[test]
    fn test_highlight_html_escaping() {
        let code = "x = 1 < 2 && y > 0\n";
        let html = highlight_code("python", code).unwrap();
        assert!(
            !html.contains(" < ") && !html.contains(" > "),
            "angle brackets should be escaped: {html}"
        );
        assert!(
            html.contains("&lt;") && html.contains("&gt;"),
            "angle brackets should be html entities: {html}"
        );
    }

    #[test]
    fn test_highlight_multiline() {
        let code = "def foo():\n    return 1\n";
        let html = highlight_code("python", code).unwrap();
        assert!(
            html.contains('\n'),
            "multiline code should preserve newlines: {html}"
        );
        // 'def' should get a keyword-related class
        assert!(
            html.contains("class=\"k\"")
                || html.contains("class=\"kd\"")
                || html.contains("class=\"nf\""),
            "def should be highlighted: {html}"
        );
    }

    #[test]
    fn test_alias_js() {
        let code = "var x = 1;\n";
        let html = highlight_code("js", code);
        assert!(html.is_some(), "js should resolve to javascript");
    }

    #[test]
    fn test_alias_sh() {
        let code = "echo hello\n";
        let html = highlight_code("sh", code);
        assert!(html.is_some(), "sh should resolve to bash");
    }
}
