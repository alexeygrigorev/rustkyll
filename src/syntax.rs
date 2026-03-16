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
        // ── Language-specific overrides (checked first) ──
        // YAML: Rouge does not use numeric classes; numbers in flow sequences
        // are `nv` (variable value), other numbers are `s` (string).
        ("source.yaml meta.flow-sequence constant.numeric", "nv"),
        ("source.yaml constant.numeric", "s"),
        // YAML: commas in flow sequences are `pi` (punctuation indicator)
        ("source.yaml punctuation.separator.sequence", "pi"),
        // YAML: block scalar indicators (| and >) are `pi` in Rouge
        ("source.yaml keyword.control.flow.block-scalar", "pi"),
        // ── Entity names (MUST come before strings so YAML keys get `na` not `s`) ──
        ("entity.name.function", "nf"),
        ("entity.name.class", "nc"),
        ("entity.name.tag", "na"),
        ("entity.name.namespace", "nn"),
        ("entity.name.type", "nc"),
        ("entity.other.attribute-name", "na"),
        ("entity.other.inherited-class", "nb"),
        // ── Comments ──
        // comment.block.documentation covers Python docstrings (""" ... """).
        // Rouge treats these as generic strings `s`, not `sd`.
        ("comment.block.documentation", "s"),
        ("comment.line.number-sign", "c1"),
        ("comment.line.double-slash", "c1"),
        ("comment.line", "c1"),
        ("comment.block", "cm"),
        ("comment", "c"),
        // ── Strings ──
        // Python: Rouge uses generic `s` for all Python string literals,
        // not `s2`/`s1`. Other languages (Bash, YAML) use `s2`/`s1`.
        ("source.python string.quoted.double", "s"),
        ("source.python string.quoted.single", "s"),
        ("string.quoted.double", "s2"),
        ("string.quoted.single", "s1"),
        ("string.quoted.triple", "sd"),
        ("string.interpolated", "si"),
        ("string.regexp", "sr"),
        ("string.other", "sx"),
        ("string.unquoted.plain.in", "nv"),
        ("string", "s"),
        // ── Numbers ──
        ("constant.numeric.float", "mf"),
        ("constant.numeric.integer.hexadecimal", "mh"),
        ("constant.numeric.integer.octal", "mo"),
        ("constant.numeric.integer.binary", "mb"),
        ("constant.numeric.integer", "mi"),
        ("constant.numeric", "m"),
        // ── Constants ──
        ("constant.language", "kc"),
        ("constant.other", "no"),
        ("constant.character.escape", "se"),
        // ── Keywords ──
        ("keyword.control.import.as", "k"),
        ("keyword.control.import", "kn"),
        ("keyword.control", "k"),
        ("keyword.operator.logical", "ow"),
        ("keyword.operator", "o"),
        ("keyword.other", "k"),
        ("keyword.declaration", "kd"),
        ("keyword", "k"),
        // ── Storage (def, class, var, let, etc.) ──
        ("storage.type.function", "k"),
        ("storage.type.class", "k"),
        ("storage.type", "kt"),
        ("storage.modifier", "k"),
        ("storage", "k"),
        // ── Variables ──
        ("variable.parameter.option", "nt"),
        ("variable.parameter", "n"),
        ("variable.annotation", "n"),
        // Python: function names in calls are `n`; Bash: command names are plain
        ("source.python variable.function", "n"),
        ("variable.language", "bp"),
        ("variable.other", "n"),
        // ── Support (built-in functions/types) ──
        ("support.function.builtin", "nb"),
        ("support.function", "nb"),
        ("support.type", "nb"),
        ("support.class", "nb"),
        ("support.other", "n"),
        // ── Punctuation ──
        // Bash line continuation (\<newline>) is `p` (punctuation) in Rouge.
        // Note: `\\` (escaped backslash) is handled by constant.character.escape -> se.
        ("punctuation.separator.continuation.line", "p"),
        // YAML-specific punctuation: key-value separator, block sequence item,
        // and flow sequence delimiters should map to `pi` (punctuation indicator)
        // to match Rouge.
        ("punctuation.separator.key-value", "pi"),
        ("punctuation.definition.block.sequence.item", "pi"),
        ("punctuation.definition.sequence", "pi"),
        ("punctuation.definition.parameter", "nt"),
        ("punctuation.definition.annotation", "o"),
        ("punctuation.definition.comment", "c1"),
        ("punctuation.definition.string", "s"),
        ("punctuation.separator.annotation.return", "o"),
        ("punctuation.separator", "p"),
        ("punctuation.section", "p"),
        ("punctuation.accessor", "p"),
        ("punctuation", "p"),
        // ── Meta ──
        ("meta.function.decorator", "nd"),
        // Python: module names in import statements.
        // `import scipy` -> `scipy` has meta.qualified-name + meta.generic-name -> nn
        // `from X import Y` -> `X` has meta.import-name -> nn
        // `from X import Y` -> `Y` has only meta.generic-name (no meta.qualified-name) -> n
        ("meta.statement.import meta.qualified-name", "nn"),
        ("meta.import-name", "nn"),
        // Plain variables (e.g., `inputs`, `fizz_buzz` used as name)
        ("meta.generic-name", "n"),
        // ── Operators not already caught ──
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
        // Note: "dockerfile"/"docker" is intentionally NOT aliased -- syntect has
        // no Dockerfile grammar, and Rouge treats it as plaintext. Returning None
        // triggers the plaintext fallback, matching Jekyll/Rouge behavior.
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
/// Note: `"` is NOT escaped because Rouge/Jekyll do not escape it inside code spans,
/// and it is valid HTML inside `<code>` / `<span>` elements.
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
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
    let sm = scope_map();

    // First pass: collect (text, css_class) fragments per line, then merge
    // adjacent fragments with the same class before emitting HTML.
    let mut html = String::with_capacity(code.len() * 2);
    let mut parse_state = syntect::parsing::ParseState::new(syntax);
    let mut scope_stack = syntect::parsing::ScopeStack::new();

    // Accumulator for merging: (css_class_option, accumulated_text)
    let mut pending_class: Option<&'static str> = None;
    let mut pending_text = String::new();

    for line in syntect::util::LinesWithEndings::from(code) {
        let ops = parse_state.parse_line(line, ss).ok()?;
        let mut cur_pos = 0;

        for (byte_offset, op) in &ops {
            if *byte_offset > cur_pos {
                let text = &line[cur_pos..*byte_offset];
                let css_class = scope_to_css_class(sm, &scope_stack);
                accumulate_and_emit(
                    &mut html,
                    &mut pending_class,
                    &mut pending_text,
                    text,
                    css_class,
                );
            }
            cur_pos = *byte_offset;
            scope_stack.apply(op).ok()?;
        }

        if cur_pos < line.len() {
            let text = &line[cur_pos..];
            let css_class = scope_to_css_class(sm, &scope_stack);
            accumulate_and_emit(
                &mut html,
                &mut pending_class,
                &mut pending_text,
                text,
                css_class,
            );
        }

        // Flush at end of each line to prevent merging across line boundaries.
        // This ensures e.g. consecutive comment lines get separate spans
        // (matching Rouge behavior) while still merging fragments within a line.
        flush_pending(&mut html, &pending_class, &mut pending_text);
    }

    // Flush any remaining pending text
    flush_pending(&mut html, &pending_class, &mut pending_text);

    Some(html)
}

/// Accumulate text into the pending buffer if it has the same CSS class,
/// or flush the old pending and start a new one.
fn accumulate_and_emit(
    html: &mut String,
    pending_class: &mut Option<&'static str>,
    pending_text: &mut String,
    text: &str,
    css_class: Option<&'static str>,
) {
    if css_class == *pending_class {
        // Same class: just accumulate
        pending_text.push_str(text);
    } else {
        // Different class: flush old, start new
        flush_pending(html, pending_class, pending_text);
        *pending_class = css_class;
        pending_text.push_str(text);
    }
}

/// Flush pending accumulated text to HTML.
///
/// Rouge does not wrap leading/trailing whitespace inside `<span>` elements.
/// This function strips leading whitespace (spaces/tabs) and emits it as
/// plain text before the span, matching Rouge behavior. Newlines inside
/// spans are preserved (Rouge does not split spans at line boundaries).
fn flush_pending(
    html: &mut String,
    pending_class: &Option<&'static str>,
    pending_text: &mut String,
) {
    if pending_text.is_empty() {
        return;
    }
    if let Some(css_class) = pending_class {
        // Strip leading whitespace (spaces/tabs only, not newlines) from the token
        // and emit it as plain text before the span, matching Rouge behavior.
        let leading_ws_end = pending_text
            .find(|c: char| c != ' ' && c != '\t')
            .unwrap_or(pending_text.len());
        let leading_ws = &pending_text[..leading_ws_end];
        let content = &pending_text[leading_ws_end..];

        if !leading_ws.is_empty() {
            html.push_str(leading_ws);
        }

        if !content.is_empty() {
            let escaped = html_escape(content);
            html.push_str("<span class=\"");
            html.push_str(css_class);
            html.push_str("\">");
            html.push_str(&escaped);
            html.push_str("</span>");
        }
    } else {
        let escaped = html_escape(pending_text);
        html.push_str(&escaped);
    }
    pending_text.clear();
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
        // Rouge uses generic `s` for Python strings (not s2/s1)
        assert!(
            html.contains("class=\"s\""),
            "python string should be highlighted as s: {html}"
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

    // ── YAML token mapping tests ──

    #[test]
    fn test_yaml_keys_are_na() {
        let html = highlight_code("yaml", "name: CI\n").unwrap();
        assert!(
            html.contains("<span class=\"na\">name</span>"),
            "YAML keys should map to na: {html}"
        );
    }

    #[test]
    fn test_yaml_colon_is_pi() {
        let html = highlight_code("yaml", "name: CI\n").unwrap();
        assert!(
            html.contains("<span class=\"pi\">:</span>"),
            "YAML colon should map to pi: {html}"
        );
    }

    #[test]
    fn test_yaml_string_value_is_s() {
        let html = highlight_code("yaml", "name: CI\n").unwrap();
        assert!(
            html.contains("<span class=\"s\">CI</span>"),
            "YAML string value should map to s: {html}"
        );
    }

    #[test]
    fn test_yaml_comment_is_c1() {
        let html = highlight_code("yaml", "# this is a comment\n").unwrap();
        assert!(
            html.contains("<span class=\"c1\"># this is a comment"),
            "YAML comments should map to c1: {html}"
        );
    }

    #[test]
    fn test_yaml_comment_merged() {
        // The # and comment text should be merged into one span
        let html = highlight_code("yaml", "# comment\n").unwrap();
        assert!(
            !html.contains("<span class=\"c1\">#</span><span class=\"c1\">"),
            "YAML comment # and text should be merged: {html}"
        );
    }

    #[test]
    fn test_yaml_boolean_is_kc() {
        let html = highlight_code("yaml", "fail-fast: false\n").unwrap();
        assert!(
            html.contains("<span class=\"kc\">false</span>"),
            "YAML booleans should map to kc: {html}"
        );
    }

    #[test]
    fn test_yaml_dash_is_pi() {
        let html = highlight_code("yaml", "- repo: example\n").unwrap();
        assert!(
            html.contains("<span class=\"pi\">-</span>"),
            "YAML dash should map to pi: {html}"
        );
    }

    #[test]
    fn test_yaml_flow_brackets_are_pi() {
        let html = highlight_code("yaml", "branches: [ main ]\n").unwrap();
        assert!(
            html.contains("<span class=\"pi\">[</span>"),
            "YAML flow sequence bracket should map to pi: {html}"
        );
        assert!(
            html.contains("<span class=\"pi\">]</span>"),
            "YAML flow sequence bracket should map to pi: {html}"
        );
    }

    #[test]
    fn test_yaml_flow_value_is_nv() {
        let html = highlight_code("yaml", "branches: [ main ]\n").unwrap();
        assert!(
            html.contains("<span class=\"nv\">main</span>"),
            "YAML values in flow sequences should map to nv: {html}"
        );
    }

    #[test]
    fn test_yaml_pipe_is_pi() {
        let html = highlight_code("yaml", "key: |\n  value\n").unwrap();
        assert!(
            html.contains("<span class=\"pi\">|</span>"),
            "YAML block scalar pipe should map to pi: {html}"
        );
    }

    #[test]
    fn test_yaml_version_number_is_s() {
        let html = highlight_code("yaml", "rev: 3.7.9\n").unwrap();
        assert!(
            html.contains("<span class=\"s\">3.7.9</span>"),
            "YAML version-like numbers should map to s: {html}"
        );
    }

    // ── Python token mapping tests ──

    #[test]
    fn test_python_import_module_is_nn() {
        let html = highlight_code("python", "import scipy\n").unwrap();
        assert!(
            html.contains("<span class=\"kn\">import</span>"),
            "import keyword should be kn: {html}"
        );
        assert!(
            html.contains("<span class=\"nn\">scipy</span>"),
            "module name after import should be nn: {html}"
        );
    }

    #[test]
    fn test_python_from_import_classes() {
        let html = highlight_code("python", "from matplotlib import pyplot as plt\n").unwrap();
        assert!(
            html.contains("<span class=\"nn\">matplotlib</span>"),
            "from module should be nn: {html}"
        );
        assert!(
            html.contains("<span class=\"n\">pyplot</span>"),
            "imported name should be n: {html}"
        );
        assert!(
            html.contains("<span class=\"k\">as</span>"),
            "as keyword should be k: {html}"
        );
        assert!(
            html.contains("<span class=\"n\">plt</span>"),
            "alias should be n: {html}"
        );
    }

    #[test]
    fn test_python_def_and_function_name() {
        let html = highlight_code("python", "def fizz_buzz(n: int) -> str:\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">def</span>"),
            "def should be k: {html}"
        );
        assert!(
            html.contains("<span class=\"nf\">fizz_buzz</span>"),
            "function name should be nf: {html}"
        );
    }

    #[test]
    fn test_python_parameter_is_n() {
        let html = highlight_code("python", "def fizz_buzz(n: int) -> str:\n").unwrap();
        assert!(
            html.contains("<span class=\"n\">n</span>"),
            "parameter should be n: {html}"
        );
    }

    #[test]
    fn test_python_return_arrow_is_o() {
        let html = highlight_code("python", "def fizz_buzz(n: int) -> str:\n").unwrap();
        assert!(
            html.contains("<span class=\"o\">-&gt;</span>"),
            "return arrow -> should be o: {html}"
        );
    }

    #[test]
    fn test_python_docstring_is_s() {
        let html = highlight_code(
            "python",
            "\"\"\"Function to solve the fizzbuzz problem.\"\"\"\n",
        )
        .unwrap();
        // Rouge uses `s` for Python docstrings (triple-quoted strings)
        assert!(
            html.contains("<span class=\"s\">\"\"\"Function"),
            "docstring should be s: {html}"
        );
    }

    #[test]
    fn test_python_builtin_is_nb() {
        let html = highlight_code("python", "print(\"hello\")\n").unwrap();
        assert!(
            html.contains("<span class=\"nb\">print</span>"),
            "built-in function should be nb: {html}"
        );
    }

    #[test]
    fn test_python_variable_is_n() {
        let html = highlight_code("python", "inputs = [3, 5]\n").unwrap();
        assert!(
            html.contains("<span class=\"n\">inputs</span>"),
            "variable should be n: {html}"
        );
    }

    #[test]
    fn test_python_string_is_s() {
        let html = highlight_code("python", "x = \"hello\"\n").unwrap();
        // Rouge uses generic `s` for all Python strings
        assert!(
            html.contains("class=\"s\""),
            "python string should be s (not s2): {html}"
        );
    }

    #[test]
    fn test_python_function_call_is_n() {
        let html = highlight_code("python", "assert fizz_buzz(inp) == out\n").unwrap();
        assert!(
            html.contains("<span class=\"n\">fizz_buzz</span>"),
            "function name in call should be n: {html}"
        );
    }

    #[test]
    fn test_python_decorator_at_is_o() {
        let html = highlight_code("python", "@pytest.mark.parametrize\n").unwrap();
        assert!(
            html.contains("<span class=\"o\">@</span>"),
            "@ in decorator should be o: {html}"
        );
    }

    #[test]
    fn test_python_dot_accessor_is_p() {
        let html = highlight_code("python", "@pytest.mark.parametrize\n").unwrap();
        assert!(
            html.contains("<span class=\"p\">.</span>"),
            ". accessor should be p: {html}"
        );
    }

    // ── Bash token mapping tests ──

    #[test]
    fn test_bash_command_is_plain() {
        let html = highlight_code("bash", "git checkout -b dev\n").unwrap();
        // Command name (git) should not be wrapped in a span
        assert!(
            html.starts_with("git checkout"),
            "bash command and args should be plain text: {html}"
        );
    }

    #[test]
    fn test_bash_flag_is_nt() {
        let html = highlight_code("bash", "git checkout -b dev\n").unwrap();
        assert!(
            html.contains("<span class=\"nt\">-b</span>"),
            "bash flag should be nt: {html}"
        );
    }

    #[test]
    fn test_bash_string_is_s2() {
        let html = highlight_code("bash", "git commit -m \"<commit message>\"\n").unwrap();
        assert!(
            html.contains("class=\"s2\""),
            "bash double-quoted string should be s2: {html}"
        );
    }

    #[test]
    fn test_bash_comment_is_c1() {
        let html = highlight_code("bash", "# comment\n").unwrap();
        assert!(
            html.contains("<span class=\"c1\"># comment"),
            "bash comment should be c1: {html}"
        );
    }

    // ── Span merging tests ──

    #[test]
    fn test_comment_fragments_merged() {
        // In syntect, the # and comment text may be separate tokens, both c1.
        // They should be merged into a single span.
        let html = highlight_code("python", "# hello world\n").unwrap();
        // Should NOT have two adjacent c1 spans
        assert!(
            !html.contains("</span><span class=\"c1\">"),
            "adjacent c1 fragments should be merged: {html}"
        );
        // Should have one c1 span containing the whole comment
        assert!(
            html.contains("<span class=\"c1\"># hello world"),
            "comment should be one merged span: {html}"
        );
    }

    #[test]
    fn test_whitespace_only_not_wrapped() {
        let html = highlight_code("yaml", "name: CI\n").unwrap();
        // The space between : and CI should not be inside a span
        assert!(
            !html.contains("<span class=\"s\"> </span>"),
            "whitespace-only text should not be wrapped in span: {html}"
        );
    }

    #[test]
    fn test_quotes_not_html_escaped() {
        let html = highlight_code("python", "x = \"hello\"\n").unwrap();
        // Quotes should NOT be escaped as &quot; (matching Rouge/Jekyll)
        assert!(
            !html.contains("&quot;"),
            "quotes should not be html-escaped: {html}"
        );
        assert!(
            html.contains("\"hello\""),
            "quotes should be literal: {html}"
        );
    }

    // ── SQL token mapping tests ──
    // Rouge maps: SELECT/FROM/WHERE/JOIN/ON/AS/AND/NULL -> k (keyword),
    // COUNT -> nb (built-in), column refs -> no, operators -> o

    #[test]
    fn test_sql_supported() {
        let code = "SELECT 1\n";
        let html = highlight_code("sql", code);
        assert!(html.is_some(), "sql should be a known language");
    }

    #[test]
    fn test_sql_select_is_k() {
        let html = highlight_code("sql", "SELECT COUNT(c.nickname) AS number_nickname\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">SELECT</span>"),
            "SQL SELECT should map to k: {html}"
        );
    }

    #[test]
    fn test_sql_count_is_nb() {
        let html = highlight_code("sql", "SELECT COUNT(c.nickname) AS number_nickname\n").unwrap();
        assert!(
            html.contains("<span class=\"nb\">COUNT</span>"),
            "SQL COUNT should map to nb: {html}"
        );
    }

    #[test]
    fn test_sql_from_is_k() {
        let html = highlight_code("sql", "SELECT 1\nFROM clients c\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">FROM</span>"),
            "SQL FROM should map to k: {html}"
        );
    }

    #[test]
    fn test_sql_left_join_is_k() {
        let html =
            highlight_code("sql", "SELECT 1\nFROM t1\nLEFT JOIN t2 ON t1.id=t2.id\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">LEFT JOIN</span>")
                || (html.contains("<span class=\"k\">LEFT</span>")
                    && html.contains("<span class=\"k\">JOIN</span>")),
            "SQL LEFT JOIN should map to k: {html}"
        );
    }

    #[test]
    fn test_sql_where_is_k() {
        let html = highlight_code("sql", "SELECT 1\nFROM t\nWHERE t.id IS NULL\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">WHERE</span>"),
            "SQL WHERE should map to k: {html}"
        );
    }

    #[test]
    fn test_sql_null_is_k() {
        let html = highlight_code("sql", "SELECT 1\nFROM t\nWHERE t.id IS NULL\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">NULL</span>")
                || html.contains("<span class=\"kc\">NULL</span>"),
            "SQL NULL should map to k or kc: {html}"
        );
    }

    #[test]
    fn test_sql_as_is_k() {
        let html = highlight_code("sql", "SELECT COUNT(1) AS cnt\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">AS</span>"),
            "SQL AS should map to k: {html}"
        );
    }

    #[test]
    fn test_sql_number_is_m() {
        let html =
            highlight_code("sql", "SELECT MONTH(current_date) - 1 AS previous_month\n").unwrap();
        assert!(
            html.contains("<span class=\"mi\">1</span>")
                || html.contains("<span class=\"m\">1</span>"),
            "SQL number should map to mi or m: {html}"
        );
    }

    #[test]
    fn test_sql_operator_is_o() {
        let html =
            highlight_code("sql", "SELECT MONTH(current_date) - 1 AS previous_month\n").unwrap();
        assert!(
            html.contains("<span class=\"o\">-</span>"),
            "SQL minus operator should map to o: {html}"
        );
    }

    // ── Docker (Dockerfile) tests ──
    // Rouge treats "docker" language as plaintext (no syntax highlighting spans).
    // Syntect's default set does not include Dockerfile syntax. Our highlighter
    // correctly returns None for unknown languages, matching Rouge behavior.

    #[test]
    fn test_docker_returns_none() {
        // "docker" is not a recognized language in syntect defaults, and Rouge
        // treats it as plaintext, so returning None is correct behavior.
        assert!(
            highlight_code("docker", "FROM ubuntu:latest\n").is_none(),
            "docker should return None (plaintext fallback, matching Rouge)"
        );
        assert!(
            highlight_code("dockerfile", "FROM ubuntu:latest\n").is_none(),
            "dockerfile should return None since syntect has no Dockerfile grammar"
        );
    }

    // ── Regression tests: real DTC site code blocks ──
    // These verify that our output matches Rouge for actual code from the
    // DataTalks.Club blog posts.

    #[test]
    fn test_regression_bash_docker_volume() {
        // From blog/how-to-run-postgresql-and-pgadmin-with-docker.html
        let code = "docker volume create --name postgres_volume_local -d local\n";
        let html = highlight_code("bash", code).unwrap();
        // Rouge output: docker volume create <span class="nt">--name</span> ... <span class="nt">-d</span> local
        assert!(
            html.contains("<span class=\"nt\">--name</span>"),
            "bash --name flag should be nt: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">-d</span>"),
            "bash -d flag should be nt: {html}"
        );
    }

    #[test]
    fn test_regression_bash_escaped_backslash_is_se() {
        // From blog/how-to-run-postgresql-and-pgadmin-with-docker.html
        // The markdown source has \\ (double backslash = literal backslash),
        // which Rouge highlights as `se` (string escape).
        let code = "docker run -it \\\\\n  --rm --name postgresql \\\\\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nt\">-it</span>"),
            "bash -it flag should be nt: {html}"
        );
        assert!(
            html.contains("<span class=\"se\">\\\\</span>"),
            "bash escaped backslash (\\\\) should be se: {html}"
        );
    }

    #[test]
    fn test_regression_bash_line_continuation_is_p() {
        // From blog/ml-deployment-lambda.html
        // Single backslash + newline (line continuation) is `p` in Rouge.
        let code = "curl -XPOST http://example.com \\\n    -d '{\"data\":\".10\"}'\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"p\">\\"),
            "bash line continuation (single \\) should be p: {html}"
        );
    }

    #[test]
    fn test_regression_bash_env_var_string() {
        // From blog/how-to-run-postgresql-and-pgadmin-with-docker.html
        let code = "docker run -e POSTGRES_USER=\"root\"\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("class=\"s2\""),
            "bash double-quoted env value should contain s2: {html}"
        );
    }

    #[test]
    fn test_regression_yaml_services() {
        // From blog/how-to-run-postgresql-and-pgadmin-with-docker.html
        let code = "services:\n pgdatabase:\n   image: postgres:13\n";
        let html = highlight_code("yaml", code).unwrap();
        assert!(
            html.contains("<span class=\"na\">services</span>"),
            "YAML services key should be na: {html}"
        );
        assert!(
            html.contains("<span class=\"na\">image</span>"),
            "YAML image key should be na: {html}"
        );
        assert!(
            html.contains("<span class=\"s\">postgres:13</span>"),
            "YAML image value should be s: {html}"
        );
    }

    #[test]
    fn test_regression_yaml_boolean_true() {
        // From blog/how-to-run-postgresql-and-pgadmin-with-docker.html
        let html = highlight_code("yaml", "external: true\n").unwrap();
        assert!(
            html.contains("<span class=\"kc\">true</span>"),
            "YAML true should be kc: {html}"
        );
    }

    #[test]
    fn test_regression_python_conditional() {
        // From blog/do-you-know-golden-rules-while-working-with-data.html
        let code = "if City_Name == 'Lisboa' OR City_Name == 'Lisbon':\n";
        let html = highlight_code("python", code).unwrap();
        assert!(
            html.contains("<span class=\"k\">if</span>"),
            "Python if should be k: {html}"
        );
        assert!(
            html.contains("<span class=\"o\">==</span>"),
            "Python == should be o: {html}"
        );
        assert!(
            html.contains("<span class=\"s\">'Lisboa'</span>"),
            "Python single-quoted string should be s: {html}"
        );
    }

    #[test]
    fn test_regression_python_simple_condition() {
        // From blog/naming-variables-in-machine-learning.html
        let code = "if e > 10:\n    break\n";
        let html = highlight_code("python", code).unwrap();
        assert!(
            html.contains("<span class=\"k\">if</span>"),
            "Python if should be k: {html}"
        );
        assert!(
            html.contains("<span class=\"mi\">10</span>"),
            "Python integer literal should be mi: {html}"
        );
        assert!(
            html.contains("<span class=\"k\">break</span>"),
            "Python break should be k: {html}"
        );
    }

    #[test]
    fn test_regression_sql_select_count_join() {
        // From blog/important-sql-fact-that-everyone-should-know.html
        let code = "SELECT COUNT(c.nickname) AS number_nickname\nFROM clients c\nLEFT JOIN client_invoice ci ON c.id=ci.user_id\nWHERE ci.id IS NULL\n";
        let html = highlight_code("sql", code).unwrap();
        assert!(
            html.contains("<span class=\"k\">SELECT</span>"),
            "SQL SELECT should be k: {html}"
        );
        assert!(
            html.contains("<span class=\"nb\">COUNT</span>"),
            "SQL COUNT should be nb: {html}"
        );
        assert!(
            html.contains("<span class=\"k\">FROM</span>"),
            "SQL FROM should be k: {html}"
        );
        assert!(
            html.contains("<span class=\"k\">WHERE</span>"),
            "SQL WHERE should be k: {html}"
        );
    }

    #[test]
    fn test_regression_sql_month_function() {
        // From blog/do-you-know-golden-rules-while-working-with-data.html
        let code = "SELECT MONTH(current_date) - 1 AS previous_month\nFROM table\n";
        let html = highlight_code("sql", code).unwrap();
        assert!(
            html.contains("<span class=\"k\">SELECT</span>"),
            "SQL SELECT should be k: {html}"
        );
        assert!(
            html.contains("<span class=\"k\">AS</span>"),
            "SQL AS should be k: {html}"
        );
        assert!(
            html.contains("<span class=\"o\">-</span>"),
            "SQL minus operator should be o: {html}"
        );
    }
}
