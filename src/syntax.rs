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
        // Ruby: Rouge classifies built-in constants (Class, String, etc.) as `no` (name constant).
        // Syntect scopes these as support.class.ruby -> nb. Override for Ruby.
        ("source.ruby support.class", "no"),
        // Ruby: Rouge classifies special methods (new, initialize, etc.) as `nf` (function name).
        // Syntect scopes these as keyword.other.special-method.ruby -> k. Override for Ruby.
        ("source.ruby keyword.other.special-method", "nf"),
        // Ruby: block parameter delimiters (|...|) are `o` (operator) in Rouge.
        ("source.ruby punctuation.definition.parameters", "o"),
        // Ruby: string interpolation markers (#{...}) are `si` in Rouge.
        ("source.ruby punctuation.section.embedded", "si"),
        // Ruby: bare identifiers inside string interpolation (#{...}) are `n` (Name).
        // Without this rule, they match `string.quoted.double` -> `s2` since the
        // scope stack still contains the enclosing string scope.
        ("source.ruby.embedded.source", "n"),
        // Ruby: string quote delimiters should match their enclosing string type so
        // they merge during accumulate_and_emit. Double-quoted -> s2, single-quoted -> s1.
        (
            "source.ruby string.quoted.double punctuation.definition.string",
            "s2",
        ),
        (
            "source.ruby string.quoted.single punctuation.definition.string",
            "s1",
        ),
        // PHP: Rouge classifies $variable as `nv` (Name.Variable).
        // Syntect scopes PHP variables as variable.other -> n. Override for PHP.
        ("source.php variable.other", "nv"),
        // PHP: Rouge classifies class names (after `new`, in extends/implements, etc.) as `nc` (Name.Class).
        // Syntect scopes these as support.class.php -> nb. Override for PHP.
        ("source.php support.class", "nc"),
        // JSON: Rouge renders string delimiters (quotes) as part of the string token.
        // Syntect emits punctuation.definition.string.{begin,end} as separate scopes.
        // Map them to `s2` (same as string.quoted.double) so they merge with the
        // string content in accumulate_and_emit, producing a single <span> per string.
        ("source.json punctuation.definition.string", "s2"),
        // YAML: Rouge treats the opening double-quote as `s2` and the closing
        // double-quote as part of the string content (`s`). Syntect emits
        // `punctuation.definition.string.begin` for the opening quote and
        // `punctuation.definition.string.end` for the closing quote. We only
        // map the `.begin` scope to `s2`; the `.end` scope falls through to
        // the `string.quoted.double` -> `s` rule below, so the closing quote
        // merges with the string content -- matching Rouge output exactly.
        ("source.yaml punctuation.definition.string.begin", "s2"),
        ("source.yaml string.quoted.double", "s"),
        // YAML: Rouge does not use numeric classes; numbers in flow sequences
        // are `nv` (variable value), other numbers are `s` (string).
        ("source.yaml meta.flow-sequence constant.numeric", "nv"),
        ("source.yaml constant.numeric", "s"),
        // YAML: commas in flow sequences are `pi` (punctuation indicator)
        ("source.yaml punctuation.separator.sequence", "pi"),
        // YAML: block scalar indicators (| and >) are `pi` in Rouge
        ("source.yaml keyword.control.flow.block-scalar", "pi"),
        // JavaScript: Rouge uses `kd` (keyword.declaration) for `var`, `function`, etc.
        // Syntect scopes these as storage.type / storage.type.function which would
        // map to `kt` / `k` generically. Override for JS specifically.
        ("source.js storage.type.function", "kd"),
        ("source.js storage.type", "kd"),
        // JavaScript: Rouge uses `nx` (name.other) for most identifiers.
        // Syntect uses entity.name.function (-> nf), variable.other (-> n),
        // variable.parameter (-> n), variable.function (no match), and
        // meta.property.object (no match). Override all to `nx` for JS.
        ("source.js variable.parameter", "nx"),
        ("source.js variable.other", "nx"),
        ("source.js variable.function", "nf"),
        ("source.js meta.property.object", "nx"),
        // JavaScript: `new` is handled via post-processing (keyword.operator
        // scope covers both `new` and `=`, so we can't override at scope level).
        // JavaScript: Rouge maps class/constructor names (e.g. Function, Date)
        // to `nc` (name.class). Syntect uses support.class -> `nb`.
        ("source.js support.class", "nc"),
        // JavaScript: Rouge maps integer literals to `mi` (number.integer).
        // Syntect uses constant.numeric (general) for JS numbers -> `m`.
        ("source.js constant.numeric", "mi"),
        // SQL: Rouge treats aggregate/builtin functions (COUNT, SUM, etc.) as keywords (k),
        // not builtins (nb). Override the generic support.function -> nb mapping.
        ("source.sql support.function", "k"),
        // SQL: Rouge maps database-name and table-name identifiers to n (name),
        // not no (name other / constant.other). Override generic constant.other -> no.
        ("source.sql constant.other.database-name", "n"),
        ("source.sql constant.other.table-name", "n"),
        ("source.sql constant.other", "n"),
        // SQL: Rouge maps all SQL numbers to mi (integer), not m (generic numeric).
        // Syntect uses constant.numeric without .integer suffix for SQL.
        ("source.sql constant.numeric", "mi"),
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
        // Python dict colons are "p" in Rouge, not "pi". Must come before YAML rule.
        ("source.python punctuation.separator.key-value", "p"),
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
    let is_python = lang == "python" || lang == "py";
    let is_yaml = lang == "yaml" || lang == "yml";
    let lines: Vec<&str> = syntect::util::LinesWithEndings::from(code).collect();
    let mut previous_python_indent = 0usize;
    let mut previous_python_had_unclosed_delimiter = false;

    // Accumulator for merging: (css_class_option, accumulated_text)
    let mut pending_class: Option<&'static str> = None;
    let mut pending_text = String::new();

    for line in lines {
        if is_python
            && previous_python_had_unclosed_delimiter
            && python_should_reset_after_unclosed_delimiter(line, previous_python_indent)
        {
            parse_state = syntect::parsing::ParseState::new(syntax);
            scope_stack = syntect::parsing::ScopeStack::new();
        }

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
        // Python triple-quoted strings are the exception: Rouge keeps them as a
        // single span across embedded newlines, so we leave those open and let
        // the next line continue the same token.
        let keep_open_for_python_string = matches!(
            (lang, pending_class),
            ("python", Some("s" | "s1" | "s2" | "sd" | "sr" | "sx"))
                | ("py", Some("s" | "s1" | "s2" | "sd" | "sr" | "sx"))
        );
        if !keep_open_for_python_string {
            flush_pending(&mut html, &pending_class, &mut pending_text);
        }

        if is_python {
            previous_python_indent = leading_indent_width(line);
            previous_python_had_unclosed_delimiter = python_has_unclosed_delimiter(line);
        }
    }

    // Flush any remaining pending text
    flush_pending(&mut html, &pending_class, &mut pending_text);

    // Python post-processing
    if is_python {
        // Merge dotted module names in import statements.
        // Syntect splits "arize.otel" into separate tokens (arize, ., otel) but
        // Rouge keeps the whole qualified name in one <span class="nn"> span.
        html = merge_python_dotted_modules(&html);

        // Rouge classifies `print` as `k` (keyword, Python 2 legacy) while
        // syntect classifies it as `nb` (builtin). Match Rouge.
        html = html.replace(
            "<span class=\"nb\">print</span>",
            "<span class=\"k\">print</span>",
        );

        // Rouge classifies `input` as a builtin (`nb`) when used as an
        // identifier/parameter name. Syntect uses `n` (generic name).
        // Match Rouge for `input` used as parameter name.
        html = html.replace(
            "<span class=\"n\">input</span>",
            "<span class=\"nb\">input</span>",
        );

        // Note: Python string delimiter split (sh+s+sh) and method call
        // reclassification (n->nf) are disabled because the DTC site's Rouge
        // version keeps strings as single 's' spans and methods as 'n'.

        // Rouge classifies `not`, `in` as `ow` (Operator.Word).
        html = html.replace(
            "<span class=\"k\">not</span>",
            "<span class=\"ow\">not</span>",
        );
        html = html.replace(
            "<span class=\"k\">in</span>",
            "<span class=\"ow\">in</span>",
        );

        html = postprocess_python_builtin_calls(&html);
        html = html.replace("</span>]", "</span><span class=\"p\">]</span>");
        html = html.replace("<span class=\"p\">]</span>)", "<span class=\"p\">])</span>");
    }

    // YAML post-processing: syntect classifies `on` as constant.language (kc)
    // since it's a YAML boolean, but Rouge treats it as `na` when used as a mapping key.
    if is_yaml {
        html = html.replace(
            "<span class=\"kc\">on</span><span class=\"pi\">:</span>",
            "<span class=\"na\">on</span><span class=\"pi\">:</span>",
        );
        html = html.replace(
            "<span class=\"kc\">true</span>",
            "<span class=\"no\">true</span>",
        );
        html = html.replace(
            "<span class=\"kc\">false</span>",
            "<span class=\"no\">false</span>",
        );
        html = postprocess_yaml_flow_mappings(&html);
    }

    // Bash post-processing: Rouge classifies `install` as builtin (`nb`).
    if lang == "bash" || lang == "sh" || lang == "shell" {
        html = postprocess_bash_prompt_lines(&html);
        html = postprocess_bash_install(&html);
        html = postprocess_bash_local(&html);
        html = postprocess_bash_export(&html);
        html = postprocess_bash_split_merged_nt_flags(&html);
        html = postprocess_bash_wrap_bare_flags_after_continuation(&html);
        html = postprocess_bash_env_var_assignments(&html);
        html = postprocess_bash_flag_argument_scope(&html);
        html = postprocess_bash_n_to_nv_uppercase(&html);
        html = postprocess_bash_var_substitution(&html);
        html = postprocess_bash_line_continuation_se(&html);
        html = postprocess_bash_var_eq_unwrap_s(&html);
        html = postprocess_bash_angle_bracket_placeholders(&html);
        html = postprocess_bash_json_braces(&html);
        html = postprocess_bash_json_string_escapes(&html);
        html = postprocess_bash_bracket_and_pipe(&html);
    }

    // SQL post-processing: Rouge wraps every token in SQL in a span.
    // Syntect's SQL grammar leaves many tokens bare (punctuation, identifiers,
    // some keywords like IS/IN/NOT). Post-process to wrap them.
    if lang == "sql" {
        html = postprocess_sql_highlighting(&html);
        html = reclassify_sql_name_to_keyword(&html);
    }

    // XML/HTML post-processing: Rouge treats `<tagname>` as a single `nt`
    // (name.tag) token, while syntect splits it into `p` (<) + `na` (tagname)
    // + `p` (>). Merge them to match Rouge output.
    if is_xml_like_language(lang) {
        html = postprocess_xml_processing_instructions(&html);
        html = postprocess_xml_tag_tokens(&html);
    }

    // JavaScript post-processing: Rouge classifies `new` as `k` (keyword).
    // Syntect scopes it as keyword.operator (same as `=`), so we can't
    // distinguish via scope rules. Fix with a targeted string replacement.
    if lang == "javascript" || lang == "js" {
        html = html.replace(
            "<span class=\"o\">new</span>",
            "<span class=\"k\">new</span>",
        );
    }

    // Java post-processing
    if lang == "java" {
        html = postprocess_java_new_class_names(&html);
        html = postprocess_java_punctuation_to_operator(&html);
        html = postprocess_java_annotations(&html);
        html = html.replace(
            "<span class=\"kt\">class</span>",
            "<span class=\"kd\">class</span>",
        );
        html = html.replace(
            "<span class=\"kt\">interface</span>",
            "<span class=\"kd\">interface</span>",
        );
    }

    // JS/Ruby post-processing: Rouge splits quoted strings into
    // dl (delimiter) + s1/s2 (content) + dl (delimiter).
    // Syntect emits the whole quoted string as a single s1/s2 span.
    if is_dl_split_language(lang) {
        html = postprocess_string_delimiter_split(&html);
    }

    // Ruby post-processing: wrap bare identifiers in <span class="n">
    // Syntect assigns only `source.ruby` to bare identifiers, so they get no span.
    // Rouge classifies them as `n` (Name).
    if lang == "ruby" || lang == "rb" {
        html = postprocess_ruby_bare_identifiers(&html);
        // Rouge classifies `::` as `o` (operator). Syntect uses the same
        // punctuation.accessor scope for both `.` and `::`, so override via post-processing.
        html = html.replace("<span class=\"p\">::</span>", "<span class=\"o\">::</span>");
        // Rouge classifies identifiers after `.` as `nf` (method calls).
        html = html.replace(
            "<span class=\"p\">.</span><span class=\"n\">",
            "<span class=\"p\">.</span><span class=\"nf\">",
        );
        // Rouge classifies special-method keywords (gem, require) as `n` when
        // used as arguments after `(` or `,`.
        html = html.replace(
            "<span class=\"p\">(</span><span class=\"nf\">",
            "<span class=\"p\">(</span><span class=\"n\">",
        );
        html = html.replace(
            "<span class=\"p\">,</span> <span class=\"nf\">",
            "<span class=\"p\">,</span> <span class=\"n\">",
        );
    }

    Some(html)
}

fn postprocess_python_builtin_calls(html: &str) -> String {
    let mut out = html.to_string();
    for builtin in ["min", "max", "sum"] {
        for suffix in [
            "<span class=\"p\">()</span>",
            "<span class=\"p\">().</span>",
            "<span class=\"p\">(</span>",
        ] {
            let from = format!("<span class=\"n\">{builtin}</span>{suffix}");
            let to = format!("<span class=\"nb\">{builtin}</span>{suffix}");
            out = out.replace(&from, &to);
        }
    }
    out
}

fn postprocess_yaml_flow_mappings(html: &str) -> String {
    let mut out = String::with_capacity(html.len());

    for line in html.split_inclusive('\n') {
        if line.contains("<span class=\"p\">{</span>")
            || line.contains("<span class=\"p\">}</span>")
        {
            let mut line = line
                .replace("<span class=\"p\">{</span>", "<span class=\"pi\">{</span>")
                .replace("<span class=\"p\">}</span>", "<span class=\"pi\">}</span>");
            line = line.replace(
                "<span class=\"pi\">{</span><span class=\"na\">",
                "<span class=\"pi\">{</span><span class=\"nv\">",
            );
            line = line.replace(
                "<span class=\"pi\">,</span> <span class=\"na\">",
                "<span class=\"pi\">,</span> <span class=\"nv\">",
            );
            out.push_str(&split_yaml_flow_double_quoted_spaces(&line));
        } else {
            out.push_str(line);
        }
    }

    out
}

fn split_yaml_flow_double_quoted_spaces(line: &str) -> String {
    let prefix = "<span class=\"s2\">\"</span><span class=\"s\">";
    let suffix = "</span>";
    let mut out = String::with_capacity(line.len());
    let mut rest = line;

    while let Some(start) = rest.find(prefix) {
        let (before, after_start) = rest.split_at(start);
        out.push_str(before);
        out.push_str(prefix);

        let content_start = &after_start[prefix.len()..];
        let Some(content_end) = content_start.find(suffix) else {
            out.push_str(content_start);
            return out;
        };

        let content = &content_start[..content_end];
        if content.contains(' ') {
            let mut parts = content.split(' ').peekable();
            while let Some(part) = parts.next() {
                out.push_str(part);
                if parts.peek().is_some() {
                    out.push_str("</span><span class=\"nv\"> </span><span class=\"s\">");
                }
            }
        } else {
            out.push_str(content);
        }
        out.push_str(suffix);
        rest = &content_start[content_end + suffix.len()..];
    }

    out.push_str(rest);
    out
}

fn leading_indent_width(line: &str) -> usize {
    line.chars()
        .take_while(|ch| matches!(ch, ' ' | '\t'))
        .count()
}

fn python_should_reset_after_unclosed_delimiter(line: &str, previous_indent: usize) -> bool {
    let trimmed = line.trim_start_matches([' ', '\t']);
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return false;
    }

    if leading_indent_width(line) > previous_indent {
        return false;
    }

    !matches!(
        trimmed.chars().next(),
        Some(')' | ']' | '}' | ',' | '.' | ':' | '+' | '-' | '*' | '/' | '%' | '&' | '|')
    )
}

fn python_has_unclosed_delimiter(line: &str) -> bool {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        if in_single {
            if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        if in_double {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }

        match ch {
            '#' => break,
            '\'' => in_single = true,
            '"' => in_double = true,
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }
    }

    paren_depth > 0 || bracket_depth > 0 || brace_depth > 0
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
            // Keep trailing newlines inside spans to match Rouge/Jekyll output.
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

/// Merge dotted Python module names in import statements.
///
/// Syntect tokenizes `arize.otel` as three tokens: `arize`, `.`, `otel`, producing:
/// `<span class="nn">arize</span><span class="p">.</span><span class="nn">otel</span>`
///
/// Rouge keeps the whole qualified name together:
/// `<span class="nn">arize.otel</span>`
///
/// This function merges `<span class="nn">X</span><span class="p">.</span><span class="nn">Y</span>`
/// into `<span class="nn">X.Y</span>`, repeatedly, to handle multi-level dotted names.
fn merge_python_dotted_modules(html: &str) -> String {
    let pattern = "</span><span class=\"p\">.</span><span class=\"nn\">";
    let mut result = html.to_string();

    // Repeatedly merge until no more patterns found
    while result.contains(pattern) {
        // Find: <span class="nn">X</span><span class="p">.</span><span class="nn">Y</span>
        // Replace the separator, keeping X and Y as part of one span
        let nn_open = "<span class=\"nn\">";
        let mut new_result = String::with_capacity(result.len());
        let mut remaining = result.as_str();

        while !remaining.is_empty() {
            if let Some(pos) = remaining.find(nn_open) {
                new_result.push_str(&remaining[..pos + nn_open.len()]);
                remaining = &remaining[pos + nn_open.len()..];

                // Find the closing </span> for this nn span
                if let Some(close_pos) = remaining.find("</span>") {
                    let name = &remaining[..close_pos];
                    let after_close = &remaining[close_pos + 7..]; // skip </span>

                    // Check if followed by <span class="p">.</span><span class="nn">
                    if let Some(after_pattern) =
                        after_close.strip_prefix("<span class=\"p\">.</span><span class=\"nn\">")
                    {
                        // Find the closing </span> for the next nn span
                        if let Some(next_close) = after_pattern.find("</span>") {
                            let next_name = &after_pattern[..next_close];
                            // Merge: write "X.Y" and continue after the second </span>
                            new_result.push_str(name);
                            new_result.push('.');
                            new_result.push_str(next_name);
                            new_result.push_str("</span>");
                            remaining = &after_pattern[next_close + 7..];
                            continue;
                        }
                    }

                    // No merge possible -- write name and closing tag normally
                    new_result.push_str(name);
                    new_result.push_str("</span>");
                    remaining = after_close;
                } else {
                    // No closing tag, copy rest
                    new_result.push_str(remaining);
                    break;
                }
            } else {
                new_result.push_str(remaining);
                break;
            }
        }

        if new_result == result {
            break; // No more merges possible
        }
        result = new_result;
    }

    result
}

/// SQL tokens that syntect scopes as names (`n`) but Rouge classifies as keywords (`k`).
const SQL_NAME_TO_KEYWORD: &[&str] = &[
    "OR",
    "AND",
    "MONTH",
    "YEAR",
    "DAY",
    "HOUR",
    "MINUTE",
    "SECOND",
    "DATE",
    "TIME",
    "TIMESTAMP",
    "INTERVAL",
    "TABLE",
    "table",
    // Rouge classifies single-letter identifiers as keywords in SQL.
    // These are typically table aliases (e.g., FROM clients c).
    "c",
];

/// SQL keywords that syntect's grammar does not assign scopes to.
/// Rouge treats these as keywords (class `k`).
const SQL_EXTRA_KEYWORDS: &[&str] = &[
    "IS",
    "IN",
    "NOT",
    "BETWEEN",
    "LIKE",
    "ILIKE",
    "EXISTS",
    "ANY",
    "ALL",
    "SOME",
    "CASE",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "CAST",
    "COALESCE",
    "NULLIF",
    "TRUE",
    "FALSE",
    "DISTINCT",
    "ASC",
    "DESC",
    "OFFSET",
    "FETCH",
    "FIRST",
    "NEXT",
    "ROWS",
    "ONLY",
    "UNION",
    "INTERSECT",
    "EXCEPT",
    "WITH",
    "RECURSIVE",
    "AS",
    "INTO",
    "VALUES",
    "SET",
    "DEFAULT",
    "CHECK",
    "CONSTRAINT",
    "REFERENCES",
    "PRIMARY",
    "FOREIGN",
    "KEY",
    "INDEX",
    "UNIQUE",
    "CASCADE",
    "RESTRICT",
    "NO",
    "ACTION",
    "DEFERRABLE",
    "INITIALLY",
    "DEFERRED",
    "IMMEDIATE",
    "CROSS",
    "NATURAL",
    "USING",
    "LATERAL",
    "TABLESAMPLE",
    "UNNEST",
    "GRANT",
    "REVOKE",
    "PRIVILEGES",
    "PUBLIC",
    "ROLE",
    "SESSION",
    "BEGIN",
    "COMMIT",
    "ROLLBACK",
    "SAVEPOINT",
    "RELEASE",
    "TRIGGER",
    "BEFORE",
    "AFTER",
    "FOR",
    "EACH",
    "ROW",
    "EXECUTE",
    "PROCEDURE",
    "FUNCTION",
    "RETURNS",
    "LANGUAGE",
    "SECURITY",
    "DEFINER",
    "INVOKER",
    "SCHEMA",
    "DATABASE",
    "TABLESPACE",
    "EXTENSION",
    "SEQUENCE",
    "VIEW",
    "MATERIALIZED",
    "REFRESH",
    "CONCURRENTLY",
    "ANALYZE",
    "EXPLAIN",
    "VERBOSE",
    "COSTS",
    "BUFFERS",
    "TIMING",
    "SUMMARY",
    "VACUUM",
    "REINDEX",
    "CLUSTER",
    "COPY",
    "DELIMITER",
    "CSV",
    "HEADER",
    "TEMPORARY",
    "TEMP",
    "UNLOGGED",
    "IF",
    "REPLACE",
    "OWNER",
    "TO",
    "RENAME",
    "COLUMN",
    "TYPE",
    "ADD",
    "DROP",
    "ENABLE",
    "DISABLE",
    "ALWAYS",
    "IDENTITY",
    "GENERATED",
    "BY",
    "PARTITION",
    "RANGE",
    "LIST",
    "HASH",
    "INCLUDING",
    "EXCLUDING",
    "PARALLEL",
    "SAFE",
    "UNSAFE",
];

/// Post-process Java highlighted HTML: Rouge classifies class/type names
/// after `new` as `nc` (name.class). Syntect maps them to `nb` (support.class).
/// Replace `<span class="k">new</span> <span class="nb">X</span>` with
/// `<span class="k">new</span> <span class="nc">X</span>`.
fn postprocess_java_new_class_names(html: &str) -> String {
    html.replace(
        "<span class=\"k\">new</span> <span class=\"nb\">",
        "<span class=\"k\">new</span> <span class=\"nc\">",
    )
}

/// Post-process Java: Rouge classifies `{`, `}`, `(`, `)`, `;`, `<`, `>` as `o`.
fn postprocess_java_punctuation_to_operator(html: &str) -> String {
    let mut result = html.to_string();
    for ch in &["{", "}", "(", ")", ";"] {
        let from = format!("<span class=\"p\">{ch}</span>");
        let to = format!("<span class=\"o\">{ch}</span>");
        result = result.replace(&from, &to);
    }
    result = result.replace(
        "<span class=\"p\">&lt;</span>",
        "<span class=\"o\">&lt;</span>",
    );
    result = result.replace(
        "<span class=\"p\">&gt;</span>",
        "<span class=\"o\">&gt;</span>",
    );
    result
}

/// Reclassify SQL tokens that syntect scopes as names (`n`) but Rouge treats as keywords (`k`).
fn reclassify_sql_name_to_keyword(html: &str) -> String {
    use std::collections::HashSet;
    use std::sync::OnceLock;
    static KW_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    let keywords = KW_SET.get_or_init(|| SQL_NAME_TO_KEYWORD.iter().copied().collect());

    let mut result = String::with_capacity(html.len());
    let mut remaining = html;
    let prefix = "<span class=\"n\">";
    let suffix = "</span>";

    while !remaining.is_empty() {
        if let Some(pos) = remaining.find(prefix) {
            result.push_str(&remaining[..pos]);
            remaining = &remaining[pos + prefix.len()..];
            if let Some(end) = remaining.find(suffix) {
                let token = &remaining[..end];
                if keywords.contains(token) {
                    result.push_str("<span class=\"k\">");
                } else {
                    result.push_str(prefix);
                }
                result.push_str(token);
                result.push_str(suffix);
                remaining = &remaining[end + suffix.len()..];
            } else {
                result.push_str(prefix);
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }
    result
}

/// Post-process Java: Rouge classifies `@Annotation` as `nd` (Name.Decorator).
fn postprocess_java_annotations(html: &str) -> String {
    let pattern = "<span class=\"o\">@</span><span class=\"n\">";
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;
    while !remaining.is_empty() {
        if let Some(pos) = remaining.find(pattern) {
            result.push_str(&remaining[..pos]);
            let after = &remaining[pos + pattern.len()..];
            if let Some(close) = after.find("</span>") {
                let name = &after[..close];
                result.push_str("<span class=\"nd\">@");
                result.push_str(name);
                result.push_str("</span>");
                remaining = &after[close + 7..];
            } else {
                result.push_str(pattern);
                remaining = after;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }
    result
}

// Note: postprocess_python_string_delimiter_split and postprocess_python_method_calls
// were removed because the DTC site's Rouge version (3.x) keeps Python strings as
// single 's' spans and method names as 'n' (not 'nf').

/// Post-process Bash highlighted HTML to wrap `install` as a builtin (`nb`).
fn postprocess_bash_install(html: &str) -> String {
    html.replace(" install ", " <span class=\"nb\">install </span>")
}

/// Post-process Bash highlighted HTML to classify `local` as a builtin (`nb`).
/// Syntect may classify it as keyword (`k`) or leave it bare.
fn postprocess_bash_local(html: &str) -> String {
    // Case 1: syntect already wrapped it as keyword `k` -- remap to `nb`
    let result = html.replace(
        "<span class=\"k\">local</span>",
        "<span class=\"nb\">local</span>",
    );
    // Case 2: bare `local` not inside a span, as a whole word only.
    // Walk through the HTML, skipping <span>...</span> regions, and replace
    // whole-word `local` in bare text segments.
    let mut out = String::with_capacity(result.len() + 64);
    let mut rest = result.as_str();
    while !rest.is_empty() {
        if let Some(span_start) = rest.find("<span") {
            let before_span = &rest[..span_start];
            out.push_str(&replace_bare_local(before_span));
            if let Some(close) = rest[span_start..].find("</span>") {
                let end = span_start + close + "</span>".len();
                out.push_str(&rest[span_start..end]);
                rest = &rest[end..];
            } else {
                out.push_str(&rest[span_start..]);
                break;
            }
        } else {
            out.push_str(&replace_bare_local(rest));
            break;
        }
    }
    out
}

/// Replace whole-word `local` in a bare text segment (no HTML tags).
fn replace_bare_local(text: &str) -> String {
    let keyword = "local";
    let replacement = "<span class=\"nb\">local</span>";
    let mut out = String::with_capacity(text.len() + 64);
    let mut search_from = 0;
    while let Some(pos) = text[search_from..].find(keyword) {
        let abs_pos = search_from + pos;
        let end_pos = abs_pos + keyword.len();
        // Check word boundaries
        let before_ok = abs_pos == 0
            || !text.as_bytes()[abs_pos - 1].is_ascii_alphanumeric()
                && text.as_bytes()[abs_pos - 1] != b'_';
        let after_ok = end_pos == text.len()
            || !text.as_bytes()[end_pos].is_ascii_alphanumeric()
                && text.as_bytes()[end_pos] != b'_';
        out.push_str(&text[search_from..abs_pos]);
        if before_ok && after_ok {
            out.push_str(replacement);
        } else {
            out.push_str(keyword);
        }
        search_from = end_pos;
    }
    out.push_str(&text[search_from..]);
    out
}

/// Post-process Bash highlighted HTML to classify `export` as a builtin (`nb`).
/// Syntect classifies it as keyword (`k`). Also remaps the variable name span
/// from `n` (name) to `nv` (name.variable) and unwraps the value span `s` (string)
/// so the output matches Rouge: `<span class="nb">export</span> <span class="nv">VAR</span><span class="o">=</span>value`
fn postprocess_bash_export(html: &str) -> String {
    // Step 1: remap <span class="k">export</span> to <span class="nb">export</span>
    let html = html.replace(
        "<span class=\"k\">export</span>",
        "<span class=\"nb\">export</span>",
    );

    // Step 2: After `<span class="nb">export</span> `, remap <span class="n">VAR</span>
    // to <span class="nv">VAR</span> and unwrap <span class="s">value</span> after =
    let export_prefix = "<span class=\"nb\">export</span> ";
    let mut result = String::with_capacity(html.len() + 64);
    let mut rest = html.as_str();

    while let Some(pos) = rest.find(export_prefix) {
        result.push_str(&rest[..pos + export_prefix.len()]);
        rest = &rest[pos + export_prefix.len()..];

        // Check if followed by <span class="n">VARNAME</span>
        let n_open = "<span class=\"n\">";
        if rest.starts_with(n_open) {
            // Remap n -> nv
            result.push_str("<span class=\"nv\">");
            rest = &rest[n_open.len()..];
            // Copy through to </span>
            if let Some(close_pos) = rest.find("</span>") {
                let end = close_pos + "</span>".len();
                result.push_str(&rest[..end]);
                rest = &rest[end..];

                // Check if followed by <span class="o">=</span><span class="s">value</span>
                let eq_span = "<span class=\"o\">=</span>";
                let s_open = "<span class=\"s\">";
                let eq_s = format!("{}{}", eq_span, s_open);
                if rest.starts_with(&eq_s) {
                    result.push_str(eq_span);
                    rest = &rest[eq_s.len()..];
                    // Unwrap the <span class="s"> -- copy contents, skip closing </span>
                    if let Some(close_pos) = rest.find("</span>") {
                        result.push_str(&rest[..close_pos]);
                        rest = &rest[close_pos + "</span>".len()..];
                    }
                }
            }
        }
    }
    result.push_str(rest);
    result
}

/// Post-process bare Bash prompt lines to match Rouge's prompt tokenization.
fn postprocess_bash_prompt_lines(html: &str) -> String {
    let mut out = String::with_capacity(html.len() + 32);

    for line in html.split_inclusive('\n') {
        let (content, trailing_newline) = match line.strip_suffix('\n') {
            Some(content) => (content, "\n"),
            None => (line, ""),
        };

        // Find "$ " at line start, optionally preceded by whitespace.
        // Skip if already wrapped in a span (don't double-wrap).
        let trimmed = content.trim_start();
        let leading_ws = &content[..content.len() - trimmed.len()];

        if trimmed.starts_with("$ ") && !trimmed.starts_with("<span") {
            let rest = &trimmed[2..]; // after "$ "
            out.push_str(leading_ws);
            out.push_str("<span class=\"nv\">$ </span>");
            if let Some(tail) = rest.strip_prefix("promptfoo eval ") {
                out.push_str("promptfoo <span class=\"nb\">eval </span>");
                out.push_str(tail);
            } else {
                out.push_str(rest);
            }
            out.push_str(trailing_newline);
        } else {
            out.push_str(line);
        }
    }

    out
}

/// Split merged `<span class="nt">` spans that contain multiple space-separated flags.
/// E.g. `<span class="nt">-it --rm</span>` -> `<span class="nt">-it</span> <span class="nt">--rm</span>`
fn postprocess_bash_split_merged_nt_flags(html: &str) -> String {
    let open_tag = "<span class=\"nt\">";
    let close_tag = "</span>";
    let mut result = String::with_capacity(html.len() + 64);
    let mut rest = html;

    while let Some(start) = rest.find(open_tag) {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + open_tag.len()..];
        if let Some(close_pos) = after_open.find(close_tag) {
            let content = &after_open[..close_pos];
            // Only split if content has spaces and all parts look like flags (-x or --word)
            let parts: Vec<&str> = content.split(' ').collect();
            if parts.len() > 1 && parts.iter().all(|p| p.starts_with('-')) {
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        result.push(' ');
                    }
                    result.push_str(open_tag);
                    result.push_str(part);
                    result.push_str(close_tag);
                }
            } else {
                // Single flag or non-flag content: keep as-is
                result.push_str(open_tag);
                result.push_str(content);
                result.push_str(close_tag);
            }
            rest = &after_open[close_pos + close_tag.len()..];
        } else {
            // Unclosed span, push the rest and break
            result.push_str(&rest[start..]);
            rest = "";
        }
    }
    result.push_str(rest);
    result
}

/// Wrap bare flags (`-x` or `--word`) that appear after line continuation markers.
/// Handles both `<span class="se">\\</span>\n` (escaped backslash) and
/// `<span class="p">\n</span>` (line continuation) patterns.
fn postprocess_bash_wrap_bare_flags_after_continuation(html: &str) -> String {
    // Pattern: after a line continuation + newline, bare flags on the next line
    // need to be wrapped in <span class="nt">...</span>.
    // We look for the escaped-backslash continuation: <span class="se">\\</span>\n
    let se_marker = "<span class=\"se\">\\\\</span>\n";

    let mut result = String::with_capacity(html.len() + 128);
    let mut rest = html;

    while let Some(pos) = rest.find(se_marker) {
        let marker_end = pos + se_marker.len();
        result.push_str(&rest[..marker_end]);
        rest = &rest[marker_end..];

        // Now process the text segment after the continuation until the next tag or newline.
        // We need to wrap bare flags in this segment.
        rest = wrap_bare_flags_in_segment(rest, &mut result);
    }
    result.push_str(rest);
    result
}

/// Given text starting after a continuation marker, wrap bare flags in the leading
/// text (before any existing span) with `<span class="nt">...</span>`.
/// Returns the remaining unconsumed portion of the input.
fn wrap_bare_flags_in_segment<'a>(input: &'a str, out: &mut String) -> &'a str {
    let rest = input;

    // Find the next span tag or end of line
    let next_tag = rest.find("<span");
    let next_newline = rest.find('\n');

    // Determine the end of the bare text region
    let bare_end = match (next_tag, next_newline) {
        (Some(t), Some(n)) => t.min(n),
        (Some(t), None) => t,
        (None, Some(n)) => n,
        (None, None) => rest.len(),
    };

    if bare_end == 0 {
        // No bare text to process
        return rest;
    }

    let bare_text = &rest[..bare_end];
    // Wrap any bare flags in this text segment
    out.push_str(&wrap_flags_in_text(bare_text));
    &rest[bare_end..]
}

/// Wrap bare flags (tokens starting with `-`) in a text segment.
/// Flags are words matching `-[a-zA-Z]` or `--[a-zA-Z]`.
/// Already-wrapped flags (preceded by `class="nt">`) are skipped.
fn wrap_flags_in_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 64);
    let mut chars = text.char_indices().peekable();

    while let Some(&(i, _)) = chars.peek() {
        // Check if current position starts a flag pattern
        if text[i..].starts_with("--")
            || (text[i..].starts_with('-')
                && text.len() > i + 1
                && text.as_bytes()[i + 1].is_ascii_alphabetic())
        {
            // Check it's at a word boundary (start of string or preceded by whitespace)
            let at_boundary = i == 0 || text.as_bytes()[i - 1].is_ascii_whitespace();
            if at_boundary {
                // Find end of flag (until space, newline, <, or end)
                let flag_start = i;
                let mut flag_end = i;
                for (j, c) in chars.clone() {
                    if c == ' ' || c == '\n' || c == '<' || c == '=' {
                        flag_end = j;
                        break;
                    }
                    flag_end = j + c.len_utf8();
                }
                let flag = &text[flag_start..flag_end];
                result.push_str("<span class=\"nt\">");
                result.push_str(flag);
                result.push_str("</span>");
                // Advance chars past the flag
                while let Some(&(j, _)) = chars.peek() {
                    if j >= flag_end {
                        break;
                    }
                    chars.next();
                }
                continue;
            }
        }
        result.push(text[i..].chars().next().unwrap());
        chars.next();
    }
    result
}

/// Post-process Bash highlighted HTML to fix flag argument scope.
///
/// 1. Remap `<span class="n">--flag</span>` to `<span class="nt">--flag</span>` when the
///    content starts with `--` (syntect gives these class `n` but Rouge uses `nt`).
/// 2. Unwrap `<span class="s">VALUE</span>` to bare `VALUE` when it immediately follows
///    a `</span><span class="o">=</span>` sequence preceded by a flag (class `nt`),
///    and the value is a single word (no spaces). This matches Rouge/Jekyll behavior.
fn postprocess_bash_flag_argument_scope(html: &str) -> String {
    // Pass 1: remap <span class="n">--xxx</span> to <span class="nt">--xxx</span>
    let n_open = "<span class=\"n\">";
    let nt_open = "<span class=\"nt\">";
    let close_tag = "</span>";

    let mut result = String::with_capacity(html.len() + 64);
    let mut rest = html;

    while let Some(pos) = rest.find(n_open) {
        result.push_str(&rest[..pos]);
        let after_open = &rest[pos + n_open.len()..];
        if let Some(close_pos) = after_open.find(close_tag) {
            let content = &after_open[..close_pos];
            if content.starts_with("--") && !content.contains(' ') {
                // Remap n -> nt for flag-like content
                result.push_str(nt_open);
            } else {
                result.push_str(n_open);
            }
            result.push_str(content);
            result.push_str(close_tag);
            rest = &after_open[close_pos + close_tag.len()..];
        } else {
            result.push_str(&rest[pos..]);
            rest = "";
        }
    }
    result.push_str(rest);

    // Pass 2: unwrap <span class="s">VALUE</span> after flag=
    // Pattern: <span class="nt">--FLAG</span><span class="o">=</span><span class="s">VALUE</span>
    let flag_eq_prefix = r#"</span><span class="o">=</span><span class="s">"#;
    let mut pass2 = String::with_capacity(result.len());
    let mut rest2 = result.as_str();

    while let Some(pos) = rest2.find(flag_eq_prefix) {
        // Check that the preceding span was class "nt" (a flag)
        let before = &rest2[..pos];
        let is_after_flag = before.ends_with(|_: char| false)
            || before
                .rfind("<span class=\"nt\">")
                .map(|nt_pos| {
                    // Make sure the nt span ends right at `pos` (i.e., pos is where </span> starts
                    // for the nt span). The `pos` here points to `</span><span class="o">=...`
                    // so the nt span's closing </span> starts at `pos`.
                    let after_nt_open = &before[nt_pos + "<span class=\"nt\">".len()..];
                    // Check content between nt open and pos ends with the content (no other spans)
                    !after_nt_open.contains("</span>") && after_nt_open.starts_with('-')
                })
                .unwrap_or(false);

        if is_after_flag {
            // Find the value inside <span class="s">VALUE</span>
            let after_prefix = &rest2[pos + flag_eq_prefix.len()..];
            if let Some(close_pos) = after_prefix.find(close_tag) {
                let value = &after_prefix[..close_pos];
                // Only unwrap single-word values (no spaces)
                if !value.contains(' ') {
                    // Write everything up to the flag_eq_prefix, but replace s span with bare text
                    result_push_unwrapped(&mut pass2, &rest2[..pos], value);
                    rest2 = &after_prefix[close_pos + close_tag.len()..];
                    continue;
                }
            }
        }

        // No match or multi-word value: copy up to and including the prefix
        pass2.push_str(&rest2[..pos + flag_eq_prefix.len()]);
        rest2 = &rest2[pos + flag_eq_prefix.len()..];
    }
    pass2.push_str(rest2);
    pass2
}

/// Helper: push the before-flag part + </span><span class="o">=</span> + bare value
fn result_push_unwrapped(out: &mut String, before: &str, value: &str) {
    out.push_str(before);
    out.push_str(r#"</span><span class="o">=</span>"#);
    out.push_str(value);
}

/// Post-process Bash highlighted HTML to wrap bare `UPPER_CASE_VAR=` patterns
/// with `<span class="nv">VAR</span><span class="o">=</span>`, and bare `-e` flags
/// preceding them with `<span class="nt">-e</span>`.
///
/// This matches Rouge/Jekyll behavior for `docker run -e VAR="val"` patterns.
fn postprocess_bash_env_var_assignments(html: &str) -> String {
    let mut result = String::with_capacity(html.len() + 128);
    let mut rest = html;

    while !rest.is_empty() {
        // Skip over existing <span>...</span> regions
        if rest.starts_with("<span") {
            if let Some(close) = rest.find("</span>") {
                let end = close + "</span>".len();
                result.push_str(&rest[..end]);
                rest = &rest[end..];
                continue;
            } else {
                // Unclosed span, push everything
                result.push_str(rest);
                break;
            }
        }

        // Find the next span tag
        let next_span = rest.find("<span");
        let bare_end = next_span.unwrap_or(rest.len());
        let bare_text = &rest[..bare_end];

        // Process bare text for VAR= patterns (with optional -e prefix)
        result.push_str(&wrap_env_var_assignments(bare_text));

        rest = &rest[bare_end..];
    }

    result
}

/// In a bare text segment (no HTML tags), find and wrap:
/// - `-e VAR=` -> `<span class="nt">-e</span> <span class="nv">VAR</span><span class="o">=</span>`
/// - standalone `VAR=` -> `<span class="nv">VAR</span><span class="o">=</span>`
///
/// Only matches uppercase variable names: `[A-Z][A-Z0-9_]*=`
fn wrap_env_var_assignments(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 64);
    let mut search_from = 0;
    let bytes = text.as_bytes();

    while search_from < bytes.len() {
        // Look for uppercase letter that could start a variable name
        if let Some(rel_pos) = find_uppercase_start(&bytes[search_from..]) {
            let var_start = search_from + rel_pos;

            // Find the end of the uppercase var name (must end with =)
            if let Some(var_end) = find_var_end(&bytes[var_start..]) {
                let abs_var_end = var_start + var_end;
                // Verify it ends with = (the byte at var_end is =)
                if abs_var_end < bytes.len() && bytes[abs_var_end] == b'=' {
                    let var_name = &text[var_start..abs_var_end];

                    // Check if preceded by "-e " and wrap that too
                    let emit_start = if var_start >= 3 && &text[var_start - 3..var_start] == "-e " {
                        // Check the -e is not already inside a span (look for > before -e)
                        let e_start = var_start - 3;
                        let before_e = &text[search_from..e_start];
                        result.push_str(before_e);
                        result.push_str("<span class=\"nt\">-e</span> ");
                        abs_var_end
                    } else {
                        result.push_str(&text[search_from..var_start]);
                        abs_var_end
                    };

                    result.push_str("<span class=\"nv\">");
                    result.push_str(var_name);
                    result.push_str("</span><span class=\"o\">=</span>");
                    search_from = emit_start + 1; // skip past the =
                    let _ = emit_start; // suppress warning
                    continue;
                }
            }
        }

        // No more patterns found in remaining text
        result.push_str(&text[search_from..]);
        break;
    }

    result
}

/// Find the first uppercase ASCII letter in a byte slice, returning its offset.
/// Only matches positions that are at a word boundary (preceded by whitespace,
/// start of string, or `>`).
fn find_uppercase_start(bytes: &[u8]) -> Option<usize> {
    for (i, &b) in bytes.iter().enumerate() {
        if b.is_ascii_uppercase() {
            // Check word boundary
            let at_boundary = i == 0
                || bytes[i - 1] == b' '
                || bytes[i - 1] == b'\n'
                || bytes[i - 1] == b'\t'
                || bytes[i - 1] == b'>';
            if at_boundary {
                return Some(i);
            }
        }
    }
    None
}

/// Starting from an uppercase letter, find the end of a `[A-Z][A-Z0-9_]*` sequence.
/// Returns the offset of the first non-matching character. The caller checks if
/// that character is `=`.
fn find_var_end(bytes: &[u8]) -> Option<usize> {
    if bytes.is_empty() || !bytes[0].is_ascii_uppercase() {
        return None;
    }
    for (i, &b) in bytes.iter().enumerate().skip(1) {
        if !(b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') {
            return Some(i);
        }
    }
    Some(bytes.len())
}

/// Post-process Bash highlighted HTML to remap `<span class="n">UPPER_CASE</span>`
/// to `<span class="nv">UPPER_CASE</span>` for all-uppercase variable names.
/// Only matches names that are all uppercase letters, digits, and underscores.
fn postprocess_bash_n_to_nv_uppercase(html: &str) -> String {
    let n_open = "<span class=\"n\">";
    let nv_open = "<span class=\"nv\">";
    let close_tag = "</span>";

    let mut result = String::with_capacity(html.len() + 64);
    let mut rest = html;

    while let Some(pos) = rest.find(n_open) {
        result.push_str(&rest[..pos]);
        let after_open = &rest[pos + n_open.len()..];
        if let Some(close_pos) = after_open.find(close_tag) {
            let content = &after_open[..close_pos];
            // Check if content is all uppercase + digits + underscores, starting with uppercase
            if !content.is_empty()
                && content.as_bytes()[0].is_ascii_uppercase()
                && content
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            {
                result.push_str(nv_open);
            } else {
                result.push_str(n_open);
            }
            result.push_str(content);
            result.push_str(close_tag);
            rest = &after_open[close_pos + close_tag.len()..];
        } else {
            result.push_str(&rest[pos..]);
            rest = "";
        }
    }
    result.push_str(rest);
    result
}

/// Post-process Bash highlighted HTML to remap `${` and `}` braces in
/// variable substitutions from class `p` to class `k`, matching Rouge.
/// Pattern: `<span class="p">${</span>...<span class="p">}</span>`
fn postprocess_bash_var_substitution(html: &str) -> String {
    // Remap <span class="p">${</span> to <span class="k">${</span>
    let html = html.replace("<span class=\"p\">${</span>", "<span class=\"k\">${</span>");
    // Remap the closing <span class="p">}</span> ONLY when it follows a </span>
    // that ends a variable name span (to avoid remapping arbitrary `}` punctuation).
    // Pattern: </span><span class="p">}</span>  ->  </span><span class="k">}</span>
    html.replace(
        "</span><span class=\"p\">}</span>",
        "</span><span class=\"k\">}</span>",
    )
}

/// Post-process Bash highlighted HTML to remap line continuation
/// `<span class="p">\<newline></span>` to `<span class="se">\</span><newline>`.
/// Rouge classifies the backslash as `se` (string escape) and keeps the newline
/// outside the span.
fn postprocess_bash_line_continuation_se(html: &str) -> String {
    html.replace(
        "<span class=\"p\">\\\n</span>",
        "<span class=\"se\">\\</span>\n",
    )
}

/// Post-process Bash highlighted HTML to unwrap `<span class="s">VALUE</span>`
/// after `<span class="o">=</span>` when the value is a single unquoted word.
/// This matches Rouge/Jekyll behavior where `VAR=value` leaves the value as bare text.
///
/// Only unwraps when:
/// - The span class is exactly `s` (not `s1`, `s2`, etc.)
/// - The value is a single word (no spaces)
/// - The value does NOT start with a quote character (`"`, `'`, or `&quot;`)
fn postprocess_bash_var_eq_unwrap_s(html: &str) -> String {
    let eq_span = r#"<span class="o">=</span>"#;
    let s_open = r#"<span class="s">"#;
    let close_tag = "</span>";
    let pattern = format!("{}{}", eq_span, s_open);

    let mut result = String::with_capacity(html.len());
    let mut rest = html;

    while let Some(pos) = rest.find(&pattern) {
        let after_pattern = &rest[pos + pattern.len()..];
        if let Some(close_pos) = after_pattern.find(close_tag) {
            let value = &after_pattern[..close_pos];
            // Only unwrap single-word, unquoted values
            if !value.contains(' ')
                && !value.starts_with('"')
                && !value.starts_with('\'')
                && !value.starts_with("&quot;")
            {
                // Write everything up to the pattern, then eq_span + bare value
                result.push_str(&rest[..pos]);
                result.push_str(eq_span);
                result.push_str(value);
                rest = &after_pattern[close_pos + close_tag.len()..];
                continue;
            }
        }
        // No match: copy up to and including the pattern
        result.push_str(&rest[..pos + pattern.len()]);
        rest = &rest[pos + pattern.len()..];
    }
    result.push_str(rest);
    result
}

/// Post-process bash highlighted HTML to unwrap angle-bracket placeholders.
/// Syntect wraps `<` and `>` as `<span class="o">&lt;</span>` and
/// `<span class="o">&gt;</span>`, but Jekyll/Rouge leaves them as plain
/// `&lt;placeholder-name&gt;`.  We match the pattern where a `<` operator span
/// is followed by word/hyphen/underscore text and then a `>` operator span,
/// and collapse the three pieces into a single HTML-entity sequence.
fn postprocess_bash_angle_bracket_placeholders(html: &str) -> String {
    let lt_span = r#"<span class="o">&lt;</span>"#;
    let gt_span = r#"<span class="o">&gt;</span>"#;

    let mut result = String::with_capacity(html.len());
    let mut rest = html;

    while let Some(lt_pos) = rest.find(lt_span) {
        let after_lt = &rest[lt_pos + lt_span.len()..];
        // Check if the text between < and > is a simple placeholder name
        // (word chars, hyphens, underscores, dots, slashes -- no spaces or HTML tags)
        if let Some(gt_pos) = after_lt.find(gt_span) {
            let between = &after_lt[..gt_pos];
            let is_placeholder = !between.is_empty()
                && !between.contains('<')
                && !between.contains(' ')
                && between
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/');
            if is_placeholder {
                result.push_str(&rest[..lt_pos]);
                result.push_str("&lt;");
                result.push_str(between);
                result.push_str("&gt;");
                rest = &after_lt[gt_pos + gt_span.len()..];
                continue;
            }
        }
        // Not a placeholder pattern -- copy past this lt_span and continue
        result.push_str(&rest[..lt_pos + lt_span.len()]);
        rest = &rest[lt_pos + lt_span.len()..];
    }
    result.push_str(rest);
    result
}

/// Post-process bash highlighted HTML to wrap bare `{` and `}` as
/// `<span class="o">` (operator), matching Rouge/Jekyll behavior for JSON
/// output in bash code blocks.
///
/// Only wraps braces that appear as bare text (not already inside a span,
/// not preceded by `$` which would indicate `${VAR}` expansion).
fn postprocess_bash_json_braces(html: &str) -> String {
    // Walk the HTML character by character, tracking span depth.
    // Only wrap { and } that appear at span depth 0 (bare text, not inside
    // any <span> element) and not inside an HTML tag.
    let mut result = String::with_capacity(html.len() + 64);
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut span_depth: i32 = 0;

    while i < len {
        // Check for <span or </span> tag starts
        if bytes[i] == b'<' && i + 5 < len {
            if html[i..].starts_with("<span ") || html[i..].starts_with("<span>") {
                // Opening span tag -- find its end and copy it through
                span_depth += 1;
                if let Some(gt) = html[i..].find('>') {
                    result.push_str(&html[i..i + gt + 1]);
                    i += gt + 1;
                } else {
                    result.push('<');
                    i += 1;
                }
                continue;
            } else if html[i..].starts_with("</span>") {
                span_depth -= 1;
                result.push_str("</span>");
                i += 7;
                continue;
            }
        }

        // Check for other HTML tags (like <code>, etc.) -- pass through
        if bytes[i] == b'<' {
            if let Some(gt) = html[i..].find('>') {
                result.push_str(&html[i..i + gt + 1]);
                i += gt + 1;
                continue;
            }
        }

        // Now we're in text content
        if (bytes[i] == b'{' || bytes[i] == b'}') && span_depth == 0 {
            result.push_str("<span class=\"o\">");
            result.push(bytes[i] as char);
            result.push_str("</span>");
            i += 1;
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

/// Post-process bash s2 spans to split `\"` escape sequences into separate
/// `<span class="se">` spans, matching Jekyll/Rouge tokenization.
///
/// Rouge breaks `"{\"prediction\": \"1\"}"` into alternating spans:
///   `<span class="s2">"{</span><span class="se">\"</span><span class="s2">prediction</span>...`
///
/// Syntect keeps the entire string as one `<span class="s2">` span.
/// This function splits any `s2` span containing `\"` into the Rouge pattern.
fn postprocess_bash_json_string_escapes(html: &str) -> String {
    let s2_open = "<span class=\"s2\">";
    let span_close = "</span>";
    let mut result = String::with_capacity(html.len() + html.len() / 8);
    let mut remaining = html;

    while !remaining.is_empty() {
        // Find next s2 span
        if let Some(start) = remaining.find(s2_open) {
            // Copy everything before this span
            result.push_str(&remaining[..start]);
            let after_open = &remaining[start + s2_open.len()..];

            // Find the closing </span>
            if let Some(close_pos) = after_open.find(span_close) {
                let content = &after_open[..close_pos];

                // Only process if content contains \"
                if content.contains("\\\"") {
                    // Split content at each \" and emit alternating s2/se spans
                    let parts: Vec<&str> = content.split("\\\"").collect();
                    for (i, part) in parts.iter().enumerate() {
                        if !part.is_empty() {
                            result.push_str(s2_open);
                            result.push_str(part);
                            result.push_str(span_close);
                        }
                        // Emit se span for \" between parts (not after last)
                        if i < parts.len() - 1 {
                            result.push_str("<span class=\"se\">\\\"</span>");
                        }
                    }
                } else {
                    // No escapes; emit unchanged
                    result.push_str(s2_open);
                    result.push_str(content);
                    result.push_str(span_close);
                }

                remaining = &after_open[close_pos + span_close.len()..];
            } else {
                // No closing tag found; emit as-is
                result.push_str(&remaining[start..]);
                break;
            }
        } else {
            // No more s2 spans
            result.push_str(remaining);
            break;
        }
    }
    result
}

/// Post-process bash highlighted HTML to fix bracket and pipe operator classes.
///
/// Rouge/Jekyll classifies `[` as operator (`o`) and leaves `]` and `|` as bare
/// text in bash blocks. Syntect classifies `[`/`]` as keyword (`k`) and `|` as
/// operator-word (`ow`). This function remaps to match Jekyll output.
fn postprocess_bash_bracket_and_pipe(html: &str) -> String {
    html.replace(r#"<span class="k">[</span>"#, r#"<span class="o">[</span>"#)
        .replace(r#"<span class="k">]</span>"#, "]")
        .replace(r#"<span class="ow">|</span>"#, "|")
}

/// Post-process SQL highlighted HTML to wrap bare tokens in spans,
/// matching Rouge's behavior where every token gets a span class.
fn postprocess_sql_highlighting(html: &str) -> String {
    use std::collections::HashSet;
    use std::sync::OnceLock;

    static KW_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    let keywords = KW_SET.get_or_init(|| SQL_EXTRA_KEYWORDS.iter().copied().collect());

    let mut result = String::with_capacity(html.len() + html.len() / 4);
    let mut remaining = html;

    while !remaining.is_empty() {
        if remaining.starts_with("<span class=\"") {
            if let Some(end) = remaining.find("</span>") {
                let span_end = end + "</span>".len();
                let span_html = &remaining[..span_end];
                // Split multi-word keyword spans (e.g. "LEFT JOIN" -> "LEFT" + "JOIN")
                // Rouge outputs each SQL keyword as a separate span.
                if span_html.starts_with("<span class=\"k\">") {
                    let content_start = "<span class=\"k\">".len();
                    let content_end = span_html.len() - "</span>".len();
                    let content = &span_html[content_start..content_end];
                    if content.contains(' ') {
                        let words: Vec<&str> = content.split(' ').collect();
                        for (i, word) in words.iter().enumerate() {
                            if i > 0 {
                                result.push(' ');
                            }
                            if !word.is_empty() {
                                result.push_str("<span class=\"k\">");
                                result.push_str(word);
                                result.push_str("</span>");
                            }
                        }
                    } else {
                        result.push_str(span_html);
                    }
                } else {
                    result.push_str(span_html);
                }
                remaining = &remaining[span_end..];
            } else {
                result.push_str(remaining);
                break;
            }
        } else if remaining.starts_with('<') {
            if let Some(end) = remaining.find('>') {
                result.push_str(&remaining[..=end]);
                remaining = &remaining[end + 1..];
            } else {
                result.push_str(remaining);
                break;
            }
        } else {
            let text_end = remaining.find('<').unwrap_or(remaining.len());
            let bare_text = &remaining[..text_end];
            remaining = &remaining[text_end..];
            wrap_sql_bare_tokens(bare_text, keywords, &mut result);
        }
    }

    result
}

/// Wrap individual bare tokens in SQL highlighted output with appropriate spans.
fn wrap_sql_bare_tokens(
    bare_text: &str,
    keywords: &std::collections::HashSet<&str>,
    result: &mut String,
) {
    let mut chars = bare_text.char_indices().peekable();

    while let Some(&(i, c)) = chars.peek() {
        if c == '\n' || c == ' ' || c == '\t' {
            result.push(c);
            chars.next();
        } else if is_sql_punctuation(c) {
            result.push_str("<span class=\"p\">");
            result.push(c);
            result.push_str("</span>");
            chars.next();
        } else if c == '*' {
            result.push_str("<span class=\"o\">*</span>");
            chars.next();
        } else if c.is_alphanumeric() || c == '_' {
            let start = i;
            let mut end = i;
            while let Some(&(j, ch)) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' {
                    end = j + ch.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            let word = &bare_text[start..end];
            let upper = word.to_uppercase();

            if keywords.contains(upper.as_str()) {
                result.push_str("<span class=\"k\">");
                result.push_str(word);
                result.push_str("</span>");
            } else {
                result.push_str("<span class=\"n\">");
                result.push_str(word);
                result.push_str("</span>");
            }
        } else if c == '&' {
            // HTML entity -- copy through and classify
            let start = i;
            chars.next();
            let mut entity_end = start + 1;
            while let Some(&(j, ch)) = chars.peek() {
                entity_end = j + ch.len_utf8();
                chars.next();
                if ch == ';' {
                    break;
                }
            }
            let entity = &bare_text[start..entity_end];
            if entity == "&amp;" || entity == "&lt;" || entity == "&gt;" {
                result.push_str("<span class=\"o\">");
                result.push_str(entity);
                result.push_str("</span>");
            } else {
                result.push_str(entity);
            }
        } else {
            result.push(c);
            chars.next();
        }
    }
}

fn is_sql_punctuation(c: char) -> bool {
    matches!(c, '(' | ')' | '.' | ',' | ';' | ':')
}

/// Check if a language name corresponds to an XML/HTML-like syntax.
fn is_xml_like_language(lang: &str) -> bool {
    matches!(
        lang,
        "xml" | "html" | "htm" | "xhtml" | "svg" | "xsd" | "xslt" | "rss" | "opml"
    )
}

/// Merge XML processing instructions (`<?...?>`) into a single `cp` span.
fn postprocess_xml_processing_instructions(html: &str) -> String {
    let pi_open = "<span class=\"p\">&lt;?</span>";
    let pi_close = "<span class=\"p\">?&gt;</span>";
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;
    while !remaining.is_empty() {
        if let Some(open_pos) = remaining.find(pi_open) {
            result.push_str(&remaining[..open_pos]);
            let after_open = &remaining[open_pos + pi_open.len()..];
            if let Some(close_pos) = after_open.find(pi_close) {
                let inner_html = &after_open[..close_pos];
                let mut inner_text = String::new();
                let mut inner_rem = inner_html;
                while !inner_rem.is_empty() {
                    if inner_rem.starts_with("<span ") {
                        if let Some(gt) = inner_rem.find('>') {
                            inner_rem = &inner_rem[gt + 1..];
                        } else {
                            break;
                        }
                    } else if inner_rem.starts_with("</span>") {
                        inner_rem = &inner_rem[7..];
                    } else {
                        let next_tag = inner_rem.find('<').unwrap_or(inner_rem.len());
                        inner_text.push_str(&inner_rem[..next_tag]);
                        inner_rem = &inner_rem[next_tag..];
                    }
                }
                result.push_str("<span class=\"cp\">&lt;?");
                result.push_str(&inner_text);
                result.push_str("?&gt;</span>");
                remaining = &after_open[close_pos + pi_close.len()..];
            } else {
                result.push_str(pi_open);
                remaining = after_open;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }
    result
}

/// Post-process XML/HTML highlighted output to merge tag punctuation with tag names,
/// matching Rouge's behavior where `<tagname>` is a single `nt` (name.tag) token.
///
/// Syntect produces: `<span class="p">&lt;</span><span class="na">tag</span><span class="p">&gt;</span>`
/// Rouge produces:   `<span class="nt">&lt;tag&gt;</span>`
///
/// For tags with attributes:
/// Syntect: `<span class="p">&lt;</span><span class="na">tag</span> <span class="na">attr</span>...`
/// Rouge:   `<span class="nt">&lt;tag</span> <span class="na">attr=</span>...`
fn postprocess_xml_tag_tokens(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    // Patterns we need to match and transform:
    let open_lt = "<span class=\"p\">&lt;</span>";
    let close_lt = "<span class=\"p\">&lt;/</span>";
    let close_gt = "<span class=\"p\">&gt;</span>";
    let na_open = "<span class=\"na\">";
    let span_close = "</span>";

    while !remaining.is_empty() {
        // Look for either opening `<` or closing `</` tag pattern
        let close_pos = remaining.find(close_lt);
        let open_pos = remaining.find(open_lt);

        // Find the earliest match
        let (pos, is_closing_tag) = match (open_pos, close_pos) {
            (Some(o), Some(c)) => {
                if c < o {
                    (c, true)
                } else {
                    (o, false)
                }
            }
            (Some(o), None) => (o, false),
            (None, Some(c)) => (c, true),
            (None, None) => {
                result.push_str(remaining);
                break;
            }
        };

        // Copy everything before this match
        result.push_str(&remaining[..pos]);

        let lt_pattern = if is_closing_tag { close_lt } else { open_lt };
        let after_lt = &remaining[pos + lt_pattern.len()..];

        // Check if followed by <span class="na">tagname</span>
        if let Some(after_na_open) = after_lt.strip_prefix(na_open) {
            if let Some(name_end) = after_na_open.find(span_close) {
                let tag_name = &after_na_open[..name_end];
                let after_name = &after_na_open[name_end + span_close.len()..];

                // Check what follows the tag name:
                if let Some(after_gt) = after_name.strip_prefix(close_gt) {
                    // Pattern: <tagname> or </tagname> -- merge into single nt span
                    result.push_str("<span class=\"nt\">&lt;");
                    if is_closing_tag {
                        result.push('/');
                    }
                    result.push_str(tag_name);
                    result.push_str("&gt;</span>");
                    remaining = after_gt;
                } else {
                    // Tag has attributes or other content after the name.
                    // Convert to: <span class="nt">&lt;tagname</span>
                    // and we need to also convert the eventual closing > from p to nt.
                    result.push_str("<span class=\"nt\">&lt;");
                    if is_closing_tag {
                        result.push('/');
                    }
                    result.push_str(tag_name);
                    result.push_str("</span>");
                    remaining = after_name;
                }
            } else {
                // No closing </span> found -- just copy the pattern as-is
                result.push_str(lt_pattern);
                remaining = after_lt;
            }
        } else {
            // Not followed by <span class="na"> -- copy as-is
            result.push_str(lt_pattern);
            remaining = after_lt;
        }
    }

    // Second pass: convert remaining <span class="p">&gt;</span> that close tags
    // with attributes to <span class="nt">&gt;</span>.
    // These are the `>` that come after attribute values, closing the tag.
    // We detect them by looking for `<span class="p">&gt;</span>` that is NOT
    // preceded by other punctuation context.
    // Simple approach: in XML/HTML, `<span class="p">&gt;</span>` at the end of
    // tag declarations should become nt. Since we've already converted the opening
    // `<` to nt, any remaining `<span class="p">&gt;</span>` is the closing `>`.
    result = result.replace(
        "<span class=\"p\">&gt;</span>",
        "<span class=\"nt\">&gt;</span>",
    );

    // Also merge attribute name + equals sign:
    // Syntect: <span class="na">attr</span><span class="pi">=</span>
    // Rouge:   <span class="na">attr=</span>
    let eq_pattern = "</span><span class=\"pi\">=</span>";
    if result.contains(eq_pattern) {
        result = result.replace(eq_pattern, "=</span>");
    }

    // Also normalize string class for XML/HTML attributes:
    // Syntect uses s2 for double-quoted strings, Rouge uses plain s
    result = result.replace("class=\"s2\"", "class=\"s\"");

    result
}

/// Check if a language uses `dl` (delimiter) splitting for string literals in Rouge.
/// JavaScript and Ruby split quoted strings into dl + s1/s2 + dl.
/// JSON, Python, and most other languages do NOT.
fn is_dl_split_language(lang: &str) -> bool {
    matches!(lang, "javascript" | "js" | "ruby" | "rb")
}

/// Post-process Ruby highlighted HTML to wrap bare identifiers in `<span class="n">`.
///
/// Syntect assigns only `source.ruby` to bare identifiers like local variables,
/// so they get no span. Rouge classifies them as `n` (Name).
/// This function finds bare word-character sequences outside of span tags and
/// wraps them in `<span class="n">`.
fn postprocess_ruby_bare_identifiers(html: &str) -> String {
    let mut result = String::with_capacity(html.len() + html.len() / 4);
    let mut remaining = html;

    while !remaining.is_empty() {
        if remaining.starts_with("<span ") {
            // Inside a span tag - copy the entire span
            if let Some(end) = remaining.find("</span>") {
                let span_end = end + "</span>".len();
                result.push_str(&remaining[..span_end]);
                remaining = &remaining[span_end..];
            } else {
                result.push_str(remaining);
                break;
            }
        } else {
            // Bare text - find the next span or end
            let text_end = remaining.find("<span ").unwrap_or(remaining.len());
            let bare_text = &remaining[..text_end];
            remaining = &remaining[text_end..];

            // Wrap word-character sequences (identifiers) in <span class="n">
            let mut chars = bare_text.chars().peekable();
            while chars.peek().is_some() {
                let c = *chars.peek().unwrap_or(&' ');
                if c.is_alphanumeric() || c == '_' {
                    // Collect the identifier
                    let mut ident = String::new();
                    while chars
                        .peek()
                        .is_some_and(|ch| ch.is_alphanumeric() || *ch == '_')
                    {
                        ident.push(chars.next().unwrap_or(' '));
                    }
                    result.push_str("<span class=\"n\">");
                    result.push_str(&ident);
                    result.push_str("</span>");
                } else {
                    result.push(chars.next().unwrap_or(' '));
                }
            }
        }
    }

    result
}

/// Post-process highlighted HTML to split quoted string spans into
/// dl (delimiter) + s1/s2 (content) + dl (delimiter), matching Rouge behavior.
///
/// Transforms:
///   `<span class="s1">'content'</span>` -> `<span class="dl">'</span><span class="s1">content</span><span class="dl">'</span>`
///   `<span class="s2">"content"</span>` -> `<span class="dl">"</span><span class="s2">content</span><span class="dl">"</span>`
fn postprocess_string_delimiter_split(html: &str) -> String {
    let mut result = String::with_capacity(html.len() + html.len() / 8);
    let mut remaining = html;

    while !remaining.is_empty() {
        // Look for <span class="s1"> or <span class="s2">
        let s1_pos = remaining.find("<span class=\"s1\">");
        let s2_pos = remaining.find("<span class=\"s2\">");

        let (pos, class_tag, string_class) = match (s1_pos, s2_pos) {
            (Some(a), Some(b)) => {
                if a < b {
                    (a, "<span class=\"s1\">", "s1")
                } else {
                    (b, "<span class=\"s2\">", "s2")
                }
            }
            (Some(a), None) => (a, "<span class=\"s1\">", "s1"),
            (None, Some(b)) => (b, "<span class=\"s2\">", "s2"),
            (None, None) => {
                result.push_str(remaining);
                break;
            }
        };

        // Copy everything before this span
        result.push_str(&remaining[..pos]);
        let after_open = &remaining[pos + class_tag.len()..];

        // Find the closing </span>
        if let Some(close_pos) = after_open.find("</span>") {
            let content = &after_open[..close_pos];
            let after_span = &after_open[close_pos + "</span>".len()..];

            // Check if content starts and ends with matching quote
            let bytes = content.as_bytes();
            if bytes.len() >= 2 {
                let first = bytes[0];
                let last = bytes[bytes.len() - 1];
                let is_single = first == b'\'' && last == b'\'';
                let is_double = first == b'"' && last == b'"';

                if is_single || is_double {
                    let quote = if is_single { "'" } else { "\"" };
                    let inner = &content[1..content.len() - 1];

                    // Emit: <span class="dl">QUOTE</span><span class="s1/s2">INNER</span><span class="dl">QUOTE</span>
                    result.push_str("<span class=\"dl\">");
                    result.push_str(quote);
                    result.push_str("</span>");
                    if !inner.is_empty() {
                        result.push_str("<span class=\"");
                        result.push_str(string_class);
                        result.push_str("\">");
                        result.push_str(inner);
                        result.push_str("</span>");
                    }
                    result.push_str("<span class=\"dl\">");
                    result.push_str(quote);
                    result.push_str("</span>");
                    remaining = after_span;
                    continue;
                }
            }

            // Not a complete quoted string -- emit as-is
            result.push_str(class_tag);
            result.push_str(content);
            result.push_str("</span>");
            remaining = after_span;
        } else {
            // No closing </span> -- copy rest
            result.push_str(class_tag);
            result.push_str(after_open);
            break;
        }
    }

    result
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
    fn test_yaml_boolean_is_no() {
        let html = highlight_code("yaml", "fail-fast: false\n").unwrap();
        assert!(
            html.contains("<span class=\"no\">false</span>"),
            "YAML booleans should map to no: {html}"
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
    fn test_python_multiline_docstring_is_one_span() {
        let code = "\"\"\"This is my great and neat function to solve the famous\nFizz Buzz problem.\n:param num: That's the number which we want the answer for\n:return: fizz, buzz, fizzbuzz or the number itself\n\"\"\"\n";
        let html = highlight_code("python", code).unwrap();
        assert!(
            html.contains("<span class=\"s\">\"\"\"This is my great and neat function to solve the famous\nFizz Buzz problem.\n:param num: That's the number which we want the answer for\n:return: fizz, buzz, fizzbuzz or the number itself\n\"\"\"</span>"),
            "multiline Python docstrings should stay in one span like Rouge: {html}"
        );
    }

    #[test]
    fn test_python_builtin_is_nb() {
        // Rouge classifies `print` as `k` (keyword, Python 2 legacy).
        // Our post-processing maps `nb` -> `k` for `print` to match Rouge.
        let html = highlight_code("python", "print(\"hello\")\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">print</span>"),
            "print should be k (matching Rouge): {html}"
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
        // Python strings should be a single span with class "s"
        assert!(
            html.contains("<span class=\"s\">"),
            "Python string should contain s class: {html}"
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
    fn test_sql_count_is_k() {
        // Issue 160: Rouge treats SQL aggregate functions as keywords (k), not builtins (nb)
        let html = highlight_code("sql", "SELECT COUNT(c.nickname) AS number_nickname\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">COUNT</span>"),
            "SQL COUNT should map to k (matching Rouge): {html}"
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

    /// Issue 325: Rouge classifies 'c' (table alias) as keyword in SQL.
    #[test]
    fn test_325_sql_table_alias_c_is_k() {
        let html = highlight_code("sql", "SELECT COUNT(c.nickname) AS number_nickname\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">c</span>"),
            "SQL 'c' table alias should map to k (matching Rouge). Got: {html}"
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
    fn test_regression_bash_line_continuation_is_se() {
        // From blog/ml-deployment-lambda.html
        // Single backslash + newline (line continuation) is `se` in Rouge/Jekyll.
        let code = "curl -XPOST http://example.com \\\n    -d '{\"data\":\".10\"}'\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"se\">\\</span>"),
            "bash line continuation (single \\) should be se: {html}"
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
            html.contains("<span class=\"no\">true</span>"),
            "YAML true should be no: {html}"
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
        // Python strings should be a single s span
        assert!(
            html.contains("<span class=\"s\">'Lisboa'</span>"),
            "Python single-quoted string should be single s span: {html}"
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
        // Rouge wraps every SQL token: keywords=k, functions=k, names=n, punct=p
        let code = "SELECT COUNT(c.nickname) AS number_nickname\nFROM clients c\nLEFT JOIN client_invoice ci ON c.id=ci.user_id\nWHERE ci.id IS NULL\n";
        let html = highlight_code("sql", code).unwrap();
        assert!(
            html.contains("<span class=\"k\">SELECT</span>"),
            "SQL SELECT should be k: {html}"
        );
        // Issue 160: Rouge maps SQL aggregate functions (COUNT, SUM) to k (keyword)
        assert!(
            html.contains("<span class=\"k\">COUNT</span>"),
            "SQL COUNT should be k (matching Rouge): {html}"
        );
        assert!(
            html.contains("<span class=\"k\">FROM</span>"),
            "SQL FROM should be k: {html}"
        );
        assert!(
            html.contains("<span class=\"k\">WHERE</span>"),
            "SQL WHERE should be k: {html}"
        );
        // Issue 160: Rouge maps SQL identifiers to n
        assert!(
            html.contains("<span class=\"n\">nickname</span>"),
            "SQL column name should be n: {html}"
        );
        assert!(
            html.contains("<span class=\"n\">clients</span>"),
            "SQL table name should be n: {html}"
        );
        // Issue 160: Rouge wraps SQL punctuation in p spans
        assert!(
            html.contains("<span class=\"p\">(</span>"),
            "SQL open paren should be p: {html}"
        );
        assert!(
            html.contains("<span class=\"p\">.</span>"),
            "SQL dot should be p: {html}"
        );
        // Issue 160: IS should be keyword
        assert!(
            html.contains("<span class=\"k\">IS</span>"),
            "SQL IS should be k: {html}"
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
        // MONTH and table should be reclassified from 'n' to 'k' to match Rouge
        assert!(
            html.contains("<span class=\"k\">MONTH</span>"),
            "SQL MONTH should be keyword class k: {html}"
        );
        assert!(
            html.contains("<span class=\"k\">table</span>"),
            "SQL table should be keyword class k: {html}"
        );
    }

    #[test]
    fn test_sql_or_classified_as_keyword() {
        let code = "SELECT * FROM t WHERE a = 1 OR b = 2\n";
        let html = highlight_code("sql", code).unwrap();
        assert!(
            html.contains("<span class=\"k\">OR</span>"),
            "SQL OR should be keyword class k: {html}"
        );
    }

    // ========================================================================
    // Issue 158: Python dict colon should be "p" not "pi"
    // ========================================================================

    #[test]
    fn test_issue158_python_dict_colon_is_p() {
        // Python dict: {"role": "user"} -- colon should be "p" (punctuation),
        // not "pi" (punctuation indicator, which is YAML-specific).
        // Jekyll/Rouge outputs <span class="p">:</span> for Python dict colons.
        let code = "{\"role\": \"user\"}\n";
        let html = highlight_code("python", code).unwrap();
        // The colon should NOT be "pi"
        assert!(
            !html.contains("<span class=\"pi\">:</span>"),
            "Python dict colon should NOT be pi (YAML-only). Got: {html}"
        );
    }

    // ========================================================================
    // Issue 158: Python module dot should be part of module name span
    // ========================================================================

    #[test]
    fn test_issue158_python_import_module_dot() {
        // "from arize.otel import register" -- Jekyll/Rouge outputs:
        // <span class="nn">arize.otel</span>
        // NOT: <span class="nn">arize</span><span class="p">.</span><span class="nn">otel</span>
        let code = "from arize.otel import register\n";
        let html = highlight_code("python", code).unwrap();
        assert!(
            html.contains("<span class=\"nn\">arize.otel</span>"),
            "Python dotted module name should keep dot inside nn span. Got: {html}"
        );
    }

    #[test]
    fn test_issue165_python_comment_trailing_newline_inside_span() {
        // Rouge/Jekyll keeps trailing newlines INSIDE comment spans for Python.
        let code = "import trax # Our Main Library\nfrom trax import layers as tl\n";
        let html = highlight_code("python", code).unwrap();
        assert!(
            html.contains("<span class=\"c1\"># Our Main Library\n</span>"),
            "Python comment span should include trailing newline (matching Rouge). Got:\n{html}"
        );
    }

    #[test]
    fn test_issue163_yaml_comment_trailing_newline_inside_span() {
        // Rouge/Jekyll keeps trailing newlines INSIDE spans.
        let code = "# This is a comment\nkey: value\n";
        let html = highlight_code("yaml", code).unwrap();
        assert!(
            html.contains("<span class=\"c1\"># This is a comment\n</span>"),
            "YAML comment span should include trailing newline (matching Rouge). Got: {html}"
        );
    }

    #[test]
    fn test_issue163_yaml_on_keyword_is_na() {
        let code = "on:\n  push:\n";
        let html = highlight_code("yaml", code).unwrap();
        assert!(
            html.contains("<span class=\"na\">on</span>"),
            "YAML `on` key should be `na`, not `kc`. Got: {html}"
        );
    }

    #[test]
    fn test_issue163_bash_install_is_nb() {
        let code = "pip install pre-commit\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nb\">install </span>"),
            "Bash `install` should be classified as `nb` (builtin). Got: {html}"
        );
    }

    // ========================================================================
    // Issue 404: Bash `local` builtin should use class `nb`
    // ========================================================================

    #[test]
    fn test_issue404_bash_local_builtin_is_nb() {
        let code = "local var=\"hello\"\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nb\">local</span>"),
            "Bash `local` should be classified as `nb` (builtin). Got: {html}"
        );
    }

    #[test]
    fn test_issue404_bash_local_trailing_is_nb() {
        let code = "docker volume create -d local\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nb\">local</span>"),
            "Trailing `local` should be classified as `nb`. Got: {html}"
        );
    }

    #[test]
    fn test_issue404_bash_local_not_in_identifier() {
        let code = "echo postgres_volume_local\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            !html.contains("<span class=\"nb\">local</span>"),
            "`local` inside identifier should NOT be wrapped. Got: {html}"
        );
    }

    #[test]
    fn test_issue404_bash_local_not_double_wrapped() {
        // If `local` is already inside a span, it should not be double-wrapped
        let code = "local var=\"hello\"\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            !html.contains("<span class=\"nb\"><span class=\"nb\">local</span></span>"),
            "`local` should not be double-wrapped. Got: {html}"
        );
    }

    // ========================================================================
    // Issue 410: Bash `export` builtin classification
    // ========================================================================

    #[test]
    fn test_issue410_bash_export_builtin_is_nb() {
        let code = "export FOO=bar\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nb\">export</span>"),
            "Bash `export` should be classified as `nb` (builtin). Got: {html}"
        );
        assert!(
            !html.contains("<span class=\"k\">export</span>"),
            "Bash `export` should NOT be classified as `k` (keyword). Got: {html}"
        );
    }

    #[test]
    fn test_issue410_bash_export_var_assignment_wrapped() {
        let code = "export FOO=bar\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nv\">FOO</span><span class=\"o\">=</span>"),
            "Variable name after `export` should be wrapped with nv/o. Got: {html}"
        );
    }

    #[test]
    fn test_issue410_bash_export_full_line() {
        let code = "export AWS_REGION=eu-central-1\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nb\">export</span> <span class=\"nv\">AWS_REGION</span><span class=\"o\">=</span>eu-central-1"),
            "Full `export VAR=val` line should match Rouge output. Got: {html}"
        );
    }

    #[test]
    fn test_issue410_bash_export_multiple_lines() {
        let code = "export AWS_REGION=eu-central-1\nexport AWS_ACCOUNT=PUT_VALUE_HERE\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nb\">export</span> <span class=\"nv\">AWS_REGION</span><span class=\"o\">=</span>eu-central-1"),
            "First export line should be correct. Got: {html}"
        );
        assert!(
            html.contains("<span class=\"nb\">export</span> <span class=\"nv\">AWS_ACCOUNT</span><span class=\"o\">=</span>PUT_VALUE_HERE"),
            "Second export line should be correct. Got: {html}"
        );
    }

    #[test]
    fn test_issue410_bash_export_not_in_identifier() {
        let code = "exported_data=1\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            !html.contains("<span class=\"nb\">export</span>"),
            "`export` inside identifier should NOT be wrapped. Got: {html}"
        );
    }

    #[test]
    fn test_issue410_bash_export_not_double_wrapped() {
        let code = "export VAR=1\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            !html.contains("<span class=\"nb\"><span class=\"nb\">export</span></span>"),
            "`export` should not be double-wrapped. Got: {html}"
        );
    }

    // ========================================================================
    // Issue 406: Bash continuation flag splitting
    // ========================================================================

    #[test]
    fn test_issue406_split_merged_nt_spans_two_flags() {
        // When syntect emits `-it --rm` in a single nt span, we should split them
        let code = "docker run -it --rm\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nt\">-it</span> <span class=\"nt\">--rm</span>"),
            "merged nt spans should be split into separate flags. Got: {html}"
        );
    }

    #[test]
    fn test_issue406_split_merged_nt_spans_three_flags() {
        // Three flags in a single nt span
        let code = "docker run -e -v -p\n";
        let html = highlight_code("bash", code).unwrap();
        // Each flag should be individually wrapped
        assert!(
            html.contains("<span class=\"nt\">-e</span>"),
            "flag -e should be individually wrapped. Got: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">-v</span>"),
            "flag -v should be individually wrapped. Got: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">-p</span>"),
            "flag -p should be individually wrapped. Got: {html}"
        );
    }

    #[test]
    fn test_issue406_single_flag_nt_unchanged() {
        // A single flag in an nt span should remain unchanged
        let code = "docker run --name foo\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nt\">--name</span>"),
            "single flag nt span should remain unchanged. Got: {html}"
        );
    }

    #[test]
    fn test_issue406_bare_flags_after_escaped_backslash_continuation() {
        // Flags on continuation lines after `\\` should be wrapped in nt spans
        let code = "docker run -it \\\\\n  --rm --name postgresql \\\\\n  -e POSTGRES_USER \\\\\n  -v path:/path \\\\\n  -p 5432:5432\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nt\">--rm</span>"),
            "bare --rm after continuation should be wrapped in nt. Got: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">--name</span>"),
            "bare --name after continuation should be wrapped in nt. Got: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">-e</span>"),
            "bare -e after continuation should be wrapped in nt. Got: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">-v</span>"),
            "bare -v after continuation should be wrapped in nt. Got: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">-p</span>"),
            "bare -p after continuation should be wrapped in nt. Got: {html}"
        );
    }

    #[test]
    fn test_issue406_bare_flags_after_single_backslash_continuation() {
        // Flags on continuation lines after single `\` (line continuation)
        let code = "docker run -it \\\n  --rm --name postgresql \\\n  -e POSTGRES_USER\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nt\">--rm</span>"),
            "bare --rm after single \\ continuation should be wrapped in nt. Got: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">--name</span>"),
            "bare --name after single \\ continuation should be wrapped in nt. Got: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">-e</span>"),
            "bare -e after single \\ continuation should be wrapped in nt. Got: {html}"
        );
    }

    #[test]
    fn test_issue406_no_wrap_non_flag_bare_words() {
        // Non-flag words should not be wrapped, only flags starting with -
        let code = "docker run -it \\\\\n  postgresql --name foo\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nt\">--name</span>"),
            "--name should be wrapped. Got: {html}"
        );
        // postgresql should NOT be wrapped in nt
        assert!(
            !html.contains("<span class=\"nt\">postgresql</span>"),
            "non-flag word 'postgresql' should not be wrapped in nt. Got: {html}"
        );
    }

    #[test]
    fn test_issue406_no_double_wrap_already_wrapped_flags() {
        // Flags already wrapped should not be double-wrapped
        let code = "docker run --name foo\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            !html.contains("<span class=\"nt\"><span class=\"nt\">--name</span></span>"),
            "already-wrapped flags should not be double-wrapped. Got: {html}"
        );
    }

    // ========================================================================
    // Issue 408: YAML boolean `true`/`false` should use class `no`
    // ========================================================================

    #[test]
    fn test_issue408_yaml_true_is_no() {
        let code = "enabled: true\n";
        let html = highlight_code("yaml", code).unwrap();
        assert!(
            html.contains("<span class=\"no\">true</span>"),
            "YAML `true` should be `no`, not `kc`. Got: {html}"
        );
    }

    #[test]
    fn test_issue408_yaml_false_is_no() {
        let code = "published: false\n";
        let html = highlight_code("yaml", code).unwrap();
        assert!(
            html.contains("<span class=\"no\">false</span>"),
            "YAML `false` should be `no`, not `kc`. Got: {html}"
        );
    }

    #[test]
    fn test_issue408_json_true_unchanged() {
        // JSON booleans should NOT be affected -- they should stay `kc`
        let code = "{\"active\": true}\n";
        let html = highlight_code("json", code).unwrap();
        assert!(
            html.contains("<span class=\"kc\">true</span>"),
            "JSON `true` should remain `kc`. Got: {html}"
        );
    }

    // ========================================================================
    // Issue 177: XML/HTML tag names should be `nt` (name.tag)
    // ========================================================================

    // ========================================================================
    // Issue 177: XML/HTML tag names should be `nt` (name.tag), not `p` + `na`
    // ========================================================================

    #[test]
    fn test_issue177_xml_simple_tag_is_nt() {
        // Rouge: <span class="nt">&lt;dependencies&gt;</span>
        let code = "<dependencies>\n</dependencies>\n";
        let html = highlight_code("xml", code).unwrap();
        assert!(
            html.contains("<span class=\"nt\">&lt;dependencies&gt;</span>"),
            "XML opening tag should be nt: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">&lt;/dependencies&gt;</span>"),
            "XML closing tag should be nt: {html}"
        );
    }

    #[test]
    fn test_issue177_xml_maven_pom_tags() {
        // From mlwiki.org ANTLR4_Maven.html
        let code = "<dependencies>\n  <dependency>\n    <groupId>org.antlr</groupId>\n  </dependency>\n</dependencies>\n";
        let html = highlight_code("xml", code).unwrap();
        assert!(
            html.contains("<span class=\"nt\">&lt;dependencies&gt;</span>"),
            "XML <dependencies> should be nt: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">&lt;groupId&gt;</span>"),
            "XML <groupId> should be nt: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">&lt;/groupId&gt;</span>"),
            "XML </groupId> should be nt: {html}"
        );
    }

    #[test]
    fn test_issue177_xml_tag_with_attributes() {
        // Rouge: <span class="nt">&lt;salutation</span> <span class="na">color=</span><span class="s">"blue"</span><span class="nt">&gt;</span>
        let code = "<salutation color=\"blue\">\n</salutation>\n";
        let html = highlight_code("xml", code).unwrap();
        assert!(
            html.contains("<span class=\"nt\">&lt;salutation</span>"),
            "XML tag with attrs: opening should be nt: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">&gt;</span>"),
            "XML tag with attrs: closing > should be nt: {html}"
        );
    }

    #[test]
    fn test_issue177_html_simple_tags() {
        let code = "<div>\n  <span>hello</span>\n</div>\n";
        let html = highlight_code("html", code).unwrap();
        assert!(
            html.contains("<span class=\"nt\">&lt;span&gt;</span>"),
            "HTML <span> should be nt: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">&lt;/span&gt;</span>"),
            "HTML </span> should be nt: {html}"
        );
    }

    #[test]
    fn test_issue177_html_tag_with_class_attribute() {
        let code = "<div class=\"container\">\n</div>\n";
        let html = highlight_code("html", code).unwrap();
        assert!(
            html.contains("<span class=\"nt\">&lt;div</span>"),
            "HTML <div with attrs: tag name should be nt: {html}"
        );
        assert!(
            html.contains("<span class=\"nt\">&gt;</span>"),
            "HTML tag closing > should be nt: {html}"
        );
    }

    #[test]
    fn test_issue177_no_p_for_xml_angle_brackets() {
        // After fix, XML angle brackets around tags should NOT be class "p"
        let code = "<dependency>\n</dependency>\n";
        let html = highlight_code("xml", code).unwrap();
        assert!(
            !html.contains("<span class=\"p\">&lt;</span><span class=\"na\">"),
            "XML should NOT have p+na pattern for tags: {html}"
        );
    }

    // ── Issue 180: JSON string token tests ──

    #[test]
    fn test_json_string_single_span() {
        // JSON string values should be rendered as a single <span class="s2">"example"</span>
        // matching Rouge/Jekyll output, not split into separate spans for delimiters and content.
        let code = "{\"name\": \"example\"}\n";
        let html = highlight_code("json", code).unwrap();
        assert!(
            html.contains("<span class=\"s2\">\"example\"</span>"),
            "JSON string value should be a single s2 span: {html}"
        );
        // Should NOT have separate spans for quote delimiters
        assert!(
            !html.contains("<span class=\"s2\">\"</span><span class=\"s2\">example</span>"),
            "JSON string should not be split into separate spans: {html}"
        );
    }

    #[test]
    fn test_json_key_single_span() {
        // JSON keys should also be rendered as single spans
        let code = "{\"name\": \"value\"}\n";
        let html = highlight_code("json", code).unwrap();
        assert!(
            html.contains("<span class=\"s2\">\"name\"</span>"),
            "JSON key should be a single s2 span: {html}"
        );
    }

    #[test]
    fn test_json_string_with_special_chars() {
        // JSON strings with special characters (slashes, spaces, colons) should be single spans
        let code = "{\"path\": \"/foo/bar\"}\n";
        let html = highlight_code("json", code).unwrap();
        assert!(
            html.contains("<span class=\"s2\">\"/foo/bar\"</span>"),
            "JSON string with slashes should be a single s2 span: {html}"
        );
    }

    #[test]
    fn test_json_empty_string() {
        // Empty JSON strings should be rendered as a single span with just the quotes
        let code = "{\"key\": \"\"}\n";
        let html = highlight_code("json", code).unwrap();
        assert!(
            html.contains("<span class=\"s2\">\"\"</span>"),
            "JSON empty string should be a single s2 span: {html}"
        );
    }

    #[test]
    fn test_json_string_with_url() {
        // JSON strings containing URLs should be single spans
        let code = "{\"url\": \"https://example.com/path\"}\n";
        let html = highlight_code("json", code).unwrap();
        assert!(
            html.contains("<span class=\"s2\">\"https://example.com/path\"</span>"),
            "JSON URL string should be a single s2 span: {html}"
        );
    }

    #[test]
    fn test_json_non_string_tokens_unchanged() {
        // JSON numbers, booleans, and null should not be affected by string merging
        let code = "{\"count\": 42, \"active\": true, \"data\": null}\n";
        let html = highlight_code("json", code).unwrap();
        assert!(
            html.contains("<span class=\"m\">42</span>")
                || html.contains("<span class=\"mi\">42</span>"),
            "JSON integer should be m or mi: {html}"
        );
        assert!(
            html.contains("<span class=\"kc\">true</span>"),
            "JSON boolean should be kc: {html}"
        );
        assert!(
            html.contains("<span class=\"kc\">null</span>"),
            "JSON null should be kc: {html}"
        );
    }

    #[test]
    fn test_json_multiline_object() {
        // Multi-line JSON should have properly merged strings on each line
        let code = "{\n  \"name\": \"example\",\n  \"version\": \"1.0.0\"\n}\n";
        let html = highlight_code("json", code).unwrap();
        assert!(
            html.contains("<span class=\"s2\">\"name\"</span>"),
            "JSON key on separate line should be single span: {html}"
        );
        assert!(
            html.contains("<span class=\"s2\">\"example\"</span>"),
            "JSON value on separate line should be single span: {html}"
        );
        assert!(
            html.contains("<span class=\"s2\">\"version\"</span>"),
            "Second JSON key should be single span: {html}"
        );
        assert!(
            html.contains("<span class=\"s2\">\"1.0.0\"</span>"),
            "Second JSON value should be single span: {html}"
        );
    }

    #[test]
    fn test_python_highlighting_unchanged_by_json_fix() {
        // Ensure Python string highlighting is not affected by JSON-specific logic
        let code = "x = \"hello\"\n";
        let html = highlight_code("python", code).unwrap();
        // Python strings should still use class "s" for content (not s2)
        assert!(
            html.contains("class=\"s\""),
            "Python string should still be s: {html}"
        );
        // Python strings should be a single s span
        assert!(
            html.contains("<span class=\"s\">\"hello\"</span>"),
            "Python string should be single s span: {html}"
        );
    }

    // ── Issue 193: YAML double-quoted string token merging ──

    #[test]
    fn test_yaml_double_quoted_string_split_spans() {
        // YAML double-quoted string values: Rouge splits the opening quote into
        // a separate <span class="s2"> and the content (+ closing quote) into
        // <span class="s">. This is the exact pattern from large-docs-site pages
        // (e.g. page-301.md): setting2: "example"
        let code = "setting2: \"example\"\n";
        let html = highlight_code("yaml", code).unwrap();
        // Rouge output: <span class="s2">"</span><span class="s">example"</span>
        // The opening quote should be in its own s2 span
        assert!(
            html.contains("<span class=\"s2\">\"</span>"),
            "YAML opening quote should be a separate s2 span: {html}"
        );
        // The content should be in an s span (not s2), preventing merging
        assert!(
            html.contains("<span class=\"s\">"),
            "YAML string content should use class s: {html}"
        );
        // Should NOT have a single merged span with quotes and content
        assert!(
            !html.contains("<span class=\"s2\">\"example\"</span>"),
            "YAML should not merge quotes with content into s2: {html}"
        );
    }

    #[test]
    fn test_yaml_config_block_with_quoted_string() {
        // Full YAML config block from large-docs-site, matching the exact content
        // that causes 500 page diffs
        let code = "api-reference:\n  enabled: true\n  option_301: value\n  nested:\n    setting1: true\n    setting2: \"example\"\n";
        let html = highlight_code("yaml", code).unwrap();
        // The opening quote should be in its own span (s2), separate from content
        assert!(
            html.contains("<span class=\"s2\">\"</span>"),
            "YAML config block: opening quote should be separate s2 span: {html}"
        );
    }

    #[test]
    fn test_yaml_quoted_string_matches_rouge() {
        // Exact code block from large-docs-site integrations pages.
        // Rouge/Jekyll output for `setting2: "example"` is:
        //   <span class="s2">"</span><span class="s">example"</span>
        // The opening quote gets its own `s2` span, but the closing quote
        // merges with the string content into the `s` span.
        let code = "integrations:\n  enabled: true\n  option_549: value\n  nested:\n    setting1: true\n    setting2: \"example\"\n";
        let html = highlight_code("yaml", code).unwrap();
        let expected = "<span class=\"s2\">\"</span><span class=\"s\">example\"</span>";
        assert!(
            html.contains(expected),
            "YAML quoted string should match Rouge pattern.\nExpected to contain: {}\nActual: {}",
            expected,
            html
        );
    }

    #[test]
    fn test_issue340_yaml_flow_mapping_matches_promptfoo_snippet() {
        let code = "# Example YAML config (pseudo)\n\
prompts:\n\
  - prompt1.txt\n\
providers:\n\
  - openai:gpt-4\n\
tests:\n\
  - vars: {question: \"What is 2+2?\"}\n\
    assert:\n\
      - type: contains\n\
        value: \"4\"\n";
        let html = highlight_code("yaml", code).unwrap();
        let expected = concat!(
            "<span class=\"na\">vars</span><span class=\"pi\">:</span> ",
            "<span class=\"pi\">{</span><span class=\"nv\">question</span>",
            "<span class=\"pi\">:</span> <span class=\"s2\">\"</span>",
            "<span class=\"s\">What</span><span class=\"nv\"> </span>",
            "<span class=\"s\">is</span><span class=\"nv\"> </span>",
            "<span class=\"s\">2+2?\"</span><span class=\"pi\">}</span>"
        );
        assert!(
            html.contains(expected),
            "YAML flow mapping should match Rouge/Jekyll output.\nExpected to contain: {expected}\nActual: {html}"
        );
    }

    #[test]
    fn test_ruby_class_new() {
        let html = highlight_code("ruby", "x = Class.new\n").unwrap();
        assert!(
            html.contains("<span class=\"n\">x</span>"),
            "Ruby variable x should be n: {html}"
        );
        assert!(
            html.contains("<span class=\"no\">Class</span>"),
            "Ruby constant Class should be no: {html}"
        );
        assert!(
            html.contains("<span class=\"nf\">new</span>"),
            "Ruby method new should be nf: {html}"
        );
    }

    // ── Issue 290: Ruby token mapping fixes for theme sites ──

    #[test]
    fn test_ruby_double_colon_is_o() {
        let html = highlight_code("ruby", "GitHubPages::Dependencies\n").unwrap();
        assert!(
            html.contains("<span class=\"o\">::</span>"),
            "Ruby :: should be o (operator), not p: {html}"
        );
    }

    #[test]
    fn test_ruby_method_after_dot_is_nf() {
        let html = highlight_code("ruby", "obj.gems.each do\nend\n").unwrap();
        assert!(
            html.contains("<span class=\"nf\">gems</span>"),
            "Ruby method after . should be nf: {html}"
        );
        assert!(
            html.contains("<span class=\"nf\">each</span>"),
            "Ruby method after . should be nf: {html}"
        );
    }

    #[test]
    fn test_ruby_block_pipe_is_o() {
        let html = highlight_code("ruby", "items.each do |item|\nend\n").unwrap();
        assert!(
            html.contains("<span class=\"o\">|</span>"),
            "Ruby block | should be o (operator): {html}"
        );
    }

    #[test]
    fn test_ruby_string_interpolation_si() {
        let code = "x = \"hello \x23{name}\"\n";
        let html = highlight_code("ruby", code).unwrap();
        assert!(
            html.contains("<span class=\"si\">"),
            "Ruby string interpolation should contain si spans: {html}"
        );
        assert!(
            html.contains("<span class=\"s2\">"),
            "Ruby interpolated string should use s2: {html}"
        );
    }

    #[test]
    fn test_ruby_gem_as_param_is_n() {
        let code = "s.add_dependency(gem, \"test\")\n";
        let html = highlight_code("ruby", code).unwrap();
        assert!(
            html.contains("<span class=\"n\">gem</span>"),
            "Ruby 'gem' as argument should be n: {html}"
        );
    }

    #[test]
    fn test_ruby_theme_site_code_block_full() {
        let code = concat!(
            "# Ruby code with syntax highlighting\n",
            "GitHubPages::Dependencies.gems.each do |gem, version|\n",
            "  s.add_dependency(gem, \"= #",
            "{version}\")\n",
            "end\n"
        );
        let html = highlight_code("ruby", code).unwrap();
        assert!(
            html.contains("<span class=\"c1\"># Ruby code with syntax highlighting"),
            "Ruby comment should be c1: {html}"
        );
        assert!(
            html.contains("<span class=\"no\">GitHubPages</span>"),
            "Ruby constant should be no: {html}"
        );
        assert!(
            html.contains("<span class=\"no\">Dependencies</span>"),
            "Ruby constant should be no: {html}"
        );
        assert!(
            html.contains("<span class=\"o\">::</span>"),
            "Ruby :: should be o: {html}"
        );
        assert!(
            html.contains("<span class=\"nf\">gems</span>"),
            "Ruby method gems should be nf: {html}"
        );
        assert!(
            html.contains("<span class=\"nf\">each</span>"),
            "Ruby method each should be nf: {html}"
        );
        assert!(
            html.contains("<span class=\"nf\">add_dependency</span>"),
            "Ruby method add_dependency should be nf: {html}"
        );
        assert!(
            html.contains("<span class=\"o\">|</span>"),
            "Ruby | should be o: {html}"
        );
        assert!(
            html.contains("<span class=\"n\">gem</span>"),
            "Ruby gem should be n (not nf): {html}"
        );
        assert!(
            html.contains("<span class=\"n\">version</span>"),
            "Ruby version should be n: {html}"
        );
        assert!(
            html.contains("<span class=\"si\">"),
            "Ruby should have si (string interpolation) spans: {html}"
        );
        assert!(
            html.contains("<span class=\"k\">end</span>"),
            "Ruby end should be k: {html}"
        );
    }

    #[test]
    fn test_ruby_non_ascii_string_290() {
        // Non-ASCII / Unicode content
        let code =
            "puts \"\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442} \u{043c}\u{0438}\u{0440}\"\n";
        let html = highlight_code("ruby", code).unwrap();
        assert!(
            html.contains("\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}"),
            "Ruby should preserve non-ASCII characters: {html}"
        );
        assert!(
            html.contains("<span class=\""),
            "Ruby non-ASCII code should produce spans: {html}"
        );
    }

    // ── Issue 290: JavaScript token mapping verification ──

    #[test]
    fn test_js_var_function_are_kd_290() {
        let html = highlight_code("js", "var x = function() {}\n").unwrap();
        assert!(
            html.contains("<span class=\"kd\">var</span>"),
            "JS var should be kd: {html}"
        );
        assert!(
            html.contains("<span class=\"kd\">function</span>"),
            "JS function should be kd: {html}"
        );
    }

    #[test]
    fn test_js_let_const_are_kd_290() {
        let html = highlight_code("js", "let y = 5;\nconst z = \"hello\";\n").unwrap();
        assert!(
            html.contains("<span class=\"kd\">let</span>"),
            "JS let should be kd: {html}"
        );
        assert!(
            html.contains("<span class=\"kd\">const</span>"),
            "JS const should be kd: {html}"
        );
    }

    // ── Issue 290: Python token mapping verification ──

    #[test]
    fn test_python_in_is_ow_290() {
        let html = highlight_code("python", "x in [1, 2]\n").unwrap();
        assert!(
            html.contains("<span class=\"ow\">in</span>"),
            "Python 'in' should be ow (operator.word): {html}"
        );
    }

    #[test]
    fn test_python_class_name_is_nc_290() {
        let html = highlight_code("python", "class MyClass:\n    pass\n").unwrap();
        assert!(
            html.contains("<span class=\"nc\">MyClass</span>"),
            "Python class name should be nc: {html}"
        );
    }

    // ── PHP token mapping tests (issue 293) ──

    #[test]
    fn test_php_variable_is_nv() {
        // PHP requires <?php prefix for syntect to recognize PHP code
        let html = highlight_code("php", "<?php\n$foo = new Bar;\n").unwrap();
        // Extract the second line (after <?php)
        let second_line = html.split('\n').nth(1).unwrap_or(&html);
        assert!(
            second_line.contains("<span class=\"nv\">$foo</span>"),
            "PHP variable $foo should be nv (Name.Variable): {html}"
        );
    }

    #[test]
    fn test_php_class_name_is_nc() {
        let html = highlight_code("php", "<?php\n$foo = new Bar;\n").unwrap();
        assert!(
            html.contains("<span class=\"nc\">Bar</span>"),
            "PHP class name Bar should be nc (Name.Class): {html}"
        );
    }

    #[test]
    fn test_php_unicode_variable_is_nv() {
        // Non-ASCII: PHP allows Unicode in variable names
        let html = highlight_code("php", "<?php\n$caf\u{00e9} = new B\u{00e4}r;\n").unwrap();
        assert!(
            html.contains("<span class=\"nv\">$caf\u{00e9}</span>"),
            "PHP Unicode variable should be nv: {html}"
        );
    }

    #[test]
    fn test_php_new_keyword_is_k() {
        let html = highlight_code("php", "<?php\n$foo = new Bar;\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">new</span>"),
            "PHP 'new' should be k (keyword): {html}"
        );
    }

    // ── Issue 300: JS/Java token class fixes ──

    #[test]
    fn test_js_new_keyword_is_k() {
        // Rouge classifies `new` as `k` (keyword), not `o` (operator)
        let html = highlight_code(
            "js",
            "var x = new Function(\"a\", \"b\", \"return a + b\");\n",
        )
        .unwrap();
        assert!(
            html.contains("<span class=\"k\">new</span>"),
            "JS 'new' should be k (keyword), not o (operator): {html}"
        );
    }

    #[test]
    fn test_js_class_name_after_new_is_nc() {
        // Rouge classifies class names after `new` as `nc` (name.class), not `nb` (name.builtin)
        let html = highlight_code(
            "js",
            "var x = new Function(\"a\", \"b\", \"return a + b\");\n",
        )
        .unwrap();
        assert!(
            html.contains("<span class=\"nc\">Function</span>"),
            "JS class name 'Function' after new should be nc (name.class), not nb: {html}"
        );
    }

    #[test]
    fn test_js_integer_literal_is_mi() {
        // Rouge classifies integer literals as `mi` (number.integer), not `m` (number)
        let html = highlight_code("js", "var x = 42;\n").unwrap();
        assert!(
            html.contains("<span class=\"mi\">42</span>"),
            "JS integer literal 42 should be mi (number.integer), not m: {html}"
        );
    }

    #[test]
    fn test_js_new_keyword_unicode_context() {
        // Non-ASCII: ensure new keyword handling works with Unicode nearby
        let html = highlight_code("js", "var caf\u{00e9} = new Date();\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">new</span>"),
            "JS 'new' should be k even with Unicode variable nearby: {html}"
        );
    }

    // ── Issue 310: Python rouge token mapping ──

    #[test]
    fn test_issue310_python_len_is_nb() {
        let html = highlight_code("python", "x = len(items)\n").unwrap();
        assert!(
            html.contains("<span class=\"nb\">len</span>"),
            "Python 'len' should be nb (name.builtin): {html}"
        );
    }

    #[test]
    fn test_issue310_python_range_is_nb() {
        let html = highlight_code("python", "x = range(10)\n").unwrap();
        assert!(
            html.contains("<span class=\"nb\">range</span>"),
            "Python 'range' should be nb (name.builtin): {html}"
        );
    }

    #[test]
    fn test_issue310_python_class_keyword_is_k() {
        let html = highlight_code("python", "class MyClass:\n    pass\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">class</span>"),
            "Python 'class' should be k (keyword): {html}"
        );
    }

    #[test]
    fn test_issue310_python_def_keyword_is_k() {
        let html = highlight_code("python", "def func():\n    pass\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">def</span>"),
            "Python 'def' should be k (keyword): {html}"
        );
    }

    #[test]
    fn test_issue310_python_unicode_string() {
        // Non-ASCII: ensure Python highlighting works with CJK content
        let html = highlight_code("python", "print(\"\u{4e16}\u{754c}\")\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">print</span>"),
            "Python 'print' should be k (keyword) even with CJK string: {html}"
        );
    }

    #[test]
    fn test_issue340_python_invalid_concat_line_recovers_return() {
        let code = "def process_data(df):\n\
    df = df.fillna()\n\
    df['column_name'] = df['another_column'] * 5\n\
    df = df.groupby('major_column').sum()\n\
    df = pandas.concat([df.iloc[0:100], df.iloc[200:300])\n\
    return df\n";
        let html = highlight_code("python", code).unwrap();
        assert!(
            html.contains("<span class=\"nb\">sum</span><span class=\"p\">()</span>"),
            "Python builtin-like sum() should match Rouge/Jekyll output: {html}"
        );
        let invalid_line_expected = concat!(
            "<span class=\"n\">df</span> <span class=\"o\">=</span> ",
            "<span class=\"n\">pandas</span><span class=\"p\">.</span>",
            "<span class=\"n\">concat</span><span class=\"p\">([</span>",
            "<span class=\"n\">df</span><span class=\"p\">.</span>",
            "<span class=\"n\">iloc</span><span class=\"p\">[</span>",
            "<span class=\"mi\">0</span><span class=\"p\">:</span>",
            "<span class=\"mi\">100</span><span class=\"p\">],</span> ",
            "<span class=\"n\">df</span><span class=\"p\">.</span>",
            "<span class=\"n\">iloc</span><span class=\"p\">[</span>",
            "<span class=\"mi\">200</span><span class=\"p\">:</span>",
            "<span class=\"mi\">300</span><span class=\"p\">])</span>"
        );
        assert!(
            html.contains(invalid_line_expected),
            "Python invalid concat line should keep the closing `])` punctuation span together.\nExpected to contain: {invalid_line_expected}\nActual: {html}"
        );
        let expected = "<span class=\"k\">return</span> <span class=\"n\">df</span>";
        assert!(
            html.contains(expected),
            "Python highlighting should recover on the line after an unterminated '['.\nExpected to contain: {expected}\nActual: {html}"
        );
    }

    #[test]
    fn test_issue340_python_invalid_filter_line_recovers_next_assignment() {
        let code = "p = numpy.percentile(df.groupby('user')['sales'].mean(), 0.95)\n\
x = df.groupby('user')['date'].min().max()\n\
df = df[(df['sales'] >= p) & (df['date'] > x]\n\
u = df['user'].unique()\n";
        let html = highlight_code("python", code).unwrap();
        assert!(
            html.contains("<span class=\"nb\">min</span><span class=\"p\">().</span><span class=\"nb\">max</span><span class=\"p\">()</span>"),
            "Python min()/max() should match Rouge/Jekyll output: {html}"
        );
        let expected = concat!(
            "<span class=\"n\">x</span><span class=\"p\">]</span>\n",
            "<span class=\"n\">u</span> <span class=\"o\">=</span> ",
            "<span class=\"n\">df</span><span class=\"p\">[</span>",
            "<span class=\"s\">'user'</span><span class=\"p\">].</span>",
            "<span class=\"n\">unique</span><span class=\"p\">()</span>"
        );
        assert!(
            html.contains(expected),
            "Python highlighting should recover on the line after an unterminated '['.\nExpected to contain: {expected}\nActual: {html}"
        );
    }

    #[test]
    fn test_issue340_bash_promptfoo_command_matches_rouge() {
        let html = highlight_code("bash", "$ promptfoo eval config.yaml\n").unwrap();
        let expected =
            "<span class=\"nv\">$ </span>promptfoo <span class=\"nb\">eval </span>config.yaml";
        assert!(
            html.contains(expected),
            "Bash prompt command should match Rouge/Jekyll output.\nExpected to contain: {expected}\nActual: {html}"
        );
    }

    // ── Issue 310: SQL rouge token mapping ──

    #[test]
    fn test_issue310_sql_select_from_where_are_k() {
        let html = highlight_code("sql", "SELECT name FROM users WHERE id = 1\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">SELECT</span>"),
            "SQL SELECT should be k (keyword): {html}"
        );
        assert!(
            html.contains("<span class=\"k\">FROM</span>"),
            "SQL FROM should be k (keyword): {html}"
        );
        assert!(
            html.contains("<span class=\"k\">WHERE</span>"),
            "SQL WHERE should be k (keyword): {html}"
        );
    }

    #[test]
    fn test_issue310_sql_join_is_k() {
        let html = highlight_code("sql", "SELECT a FROM t1 JOIN t2 ON t1.id = t2.id\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">JOIN</span>"),
            "SQL JOIN should be k (keyword): {html}"
        );
    }

    #[test]
    fn test_issue310_sql_group_by_order_by_are_k() {
        let html = highlight_code(
            "sql",
            "SELECT name FROM users GROUP BY name ORDER BY name\n",
        )
        .unwrap();
        assert!(
            html.contains("<span class=\"k\">GROUP</span>"),
            "SQL GROUP should be k (keyword): {html}"
        );
        assert!(
            html.contains("<span class=\"k\">ORDER</span>"),
            "SQL ORDER should be k (keyword): {html}"
        );
    }

    #[test]
    fn test_issue310_sql_unicode_string_literal() {
        // Non-ASCII: SQL with Unicode string literal
        let html = highlight_code("sql", "SELECT * FROM t WHERE name = 'caf\u{00e9}'\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">SELECT</span>"),
            "SQL SELECT should be k even with Unicode string: {html}"
        );
        assert!(
            html.contains("<span class=\"k\">WHERE</span>"),
            "SQL WHERE should be k even with Unicode string: {html}"
        );
    }

    // ── Issue 310: Java rouge token mapping ──

    #[test]
    fn test_issue310_java_new_keyword_is_k() {
        let html = highlight_code("java", "ArrayList list = new ArrayList();\n").unwrap();
        assert!(
            html.contains("<span class=\"k\">new</span>"),
            "Java 'new' should be k (keyword), not o (operator): {html}"
        );
    }

    #[test]
    fn test_issue310_java_class_name_after_new_is_nc() {
        let html = highlight_code("java", "ArrayList list = new ArrayList();\n").unwrap();
        // The ArrayList after new should be nc (name.class)
        // Check that there's at least one nc for ArrayList
        assert!(
            html.contains("<span class=\"nc\">ArrayList</span>"),
            "Java class name 'ArrayList' should be nc (name.class): {html}"
        );
    }

    #[test]
    fn test_issue310_java_integer_literal_is_mi() {
        let html = highlight_code("java", "int x = 42;\n").unwrap();
        assert!(
            html.contains("<span class=\"mi\">42</span>"),
            "Java integer literal 42 should be mi: {html}"
        );
    }

    #[test]
    fn test_issue310_java_public_class_keywords() {
        let html = highlight_code("java", "public class Main {\n}\n").unwrap();
        assert!(
            html.contains("<span class=\"k\"") || html.contains("<span class=\"kd\">public</span>"),
            "Java 'public' should be k or kd (keyword): {html}"
        );
        assert!(
            html.contains("<span class=\"nc\">Main</span>"),
            "Java class name 'Main' should be nc (name.class): {html}"
        );
    }

    #[test]
    fn test_issue310_java_unicode_string() {
        // Non-ASCII: Java with Unicode string
        let html = highlight_code("java", "String s = \"\u{00e9}t\u{00e9}\";\n").unwrap();
        // Just verify it doesn't crash and produces reasonable output
        assert!(
            html.contains("<span class="),
            "Java with Unicode string should produce highlighted output: {html}"
        );
    }

    // ── Issue 407: Bash env var assignment highlighting ──

    #[test]
    fn test_issue407_bash_env_var_assignment_nv_o() {
        // Simulate what syntect produces: bare VAR= followed by an s2 string span
        let input = "  -e POSTGRES_USER=<span class=\"s2\">\"root\"</span>";
        let result = postprocess_bash_env_var_assignments(input);
        assert!(
            result.contains("<span class=\"nv\">POSTGRES_USER</span><span class=\"o\">=</span><span class=\"s2\">\"root\"</span>"),
            "Bare UPPER_CASE_VAR= should be wrapped with nv and o spans.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue407_bash_env_var_with_e_flag() {
        // Full pattern: -e FLAG followed by VAR=
        let input = "  -e POSTGRES_USER=<span class=\"s2\">\"root\"</span> <span class=\"se\">\\\\</span>\n  -e POSTGRES_PASSWORD=<span class=\"s2\">\"root\"</span>";
        let result = postprocess_bash_env_var_assignments(input);
        assert!(
            result.contains("<span class=\"nv\">POSTGRES_USER</span><span class=\"o\">=</span>"),
            "First VAR= should be wrapped.\nActual: {result}"
        );
        assert!(
            result
                .contains("<span class=\"nv\">POSTGRES_PASSWORD</span><span class=\"o\">=</span>"),
            "Second VAR= should be wrapped.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue407_already_wrapped_not_doubled() {
        let input = "<span class=\"nv\">POSTGRES_USER</span><span class=\"o\">=</span><span class=\"s2\">\"root\"</span>";
        let result = postprocess_bash_env_var_assignments(input);
        assert_eq!(
            result, input,
            "Already-wrapped variables should not be double-wrapped."
        );
    }

    #[test]
    fn test_issue407_lowercase_not_matched() {
        let input = "  foo=bar";
        let result = postprocess_bash_env_var_assignments(input);
        assert_eq!(
            result, input,
            "Lowercase variable names should not be wrapped."
        );
    }

    #[test]
    fn test_issue407_unicode_value() {
        let input = "  -e MY_VAR=<span class=\"s2\">\"\u{00e9}t\u{00e9}\"</span>";
        let result = postprocess_bash_env_var_assignments(input);
        assert!(
            result.contains("<span class=\"nv\">MY_VAR</span><span class=\"o\">=</span>"),
            "Env var with Unicode value should be wrapped.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue407_bare_e_flag_wrapped() {
        // The -e flag should also be wrapped as nt when bare
        let input = "  -e POSTGRES_USER=<span class=\"s2\">\"root\"</span>";
        let result = postprocess_bash_env_var_assignments(input);
        assert!(
            result.contains("<span class=\"nt\">-e</span>"),
            "Bare -e flag should be wrapped as nt.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue407_var_inside_span_not_touched() {
        // YAML-style: the entire VAR=val is already inside a span
        let input = "<span class=\"s\">POSTGRES_USER=root</span>";
        let result = postprocess_bash_env_var_assignments(input);
        assert_eq!(
            result, input,
            "VAR= inside an existing span should not be touched."
        );
    }

    #[test]
    fn test_issue407_full_highlight_integration() {
        let code = "docker run -it \\\n  -e POSTGRES_USER=\"root\" \\\n  -e POSTGRES_PASSWORD=\"root\" \\\n  postgres:13\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nv\">POSTGRES_USER</span><span class=\"o\">=</span>"),
            "Full bash highlight should wrap POSTGRES_USER.\nActual: {html}"
        );
        assert!(
            html.contains("<span class=\"nv\">POSTGRES_PASSWORD</span><span class=\"o\">=</span>"),
            "Full bash highlight should wrap POSTGRES_PASSWORD.\nActual: {html}"
        );
    }

    // ── Issue 412: Bash flag name class + flag argument unwrapping ──

    #[test]
    fn test_issue412_n_class_flag_becomes_nt() {
        // <span class="n">--network</span> should become <span class="nt">--network</span>
        let input = r#"<span class="n">--network</span><span class="o">=</span><span class="s">pg-network</span>"#;
        let result = postprocess_bash_flag_argument_scope(input);
        assert!(
            result.contains(r#"<span class="nt">--network</span>"#),
            "Flag --network in class 'n' should be remapped to class 'nt'.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue412_unwrap_s_after_flag_equals() {
        // <span class="nt">--network</span><span class="o">=</span><span class="s">pg-network</span>
        // should unwrap <span class="s">pg-network</span> to bare pg-network
        let input = r#"<span class="nt">--network</span><span class="o">=</span><span class="s">pg-network</span> <span class="se">\\</span>"#;
        let result = postprocess_bash_flag_argument_scope(input);
        assert!(
            result.contains(r#"<span class="nt">--network</span><span class="o">=</span>pg-network <span class="se">\\</span>"#),
            "Value after --flag= should be bare text, not wrapped in <span class='s'>.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue412_combined_n_to_nt_and_unwrap() {
        // Full pattern from the issue
        let input = r#"<span class="n">--network</span><span class="o">=</span><span class="s">pg-network</span> <span class="se">\\</span>"#;
        let result = postprocess_bash_flag_argument_scope(input);
        assert_eq!(
            result,
            r#"<span class="nt">--network</span><span class="o">=</span>pg-network <span class="se">\\</span>"#,
            "Both remap n->nt and unwrap s should happen."
        );
    }

    #[test]
    fn test_issue412_no_unwrap_non_flag_n() {
        // <span class="n">somevar</span> should NOT be remapped (not a flag)
        let input = r#"<span class="n">somevar</span>"#;
        let result = postprocess_bash_flag_argument_scope(input);
        assert_eq!(result, input, "Non-flag 'n' spans should not be changed.");
    }

    #[test]
    fn test_issue412_no_unwrap_s_without_flag_context() {
        // <span class="s">pg-network</span> without a preceding flag= should not be unwrapped
        let input = r#"<span class="s">pg-network</span>"#;
        let result = postprocess_bash_flag_argument_scope(input);
        assert_eq!(
            result, input,
            "Standalone s spans not after a flag should not be unwrapped."
        );
    }

    #[test]
    fn test_issue412_unicode_flag_value() {
        let input = r#"<span class="nt">--name</span><span class="o">=</span><span class="s">données</span>"#;
        let result = postprocess_bash_flag_argument_scope(input);
        assert!(
            result.contains(r#"<span class="o">=</span>données"#),
            "Unicode value after flag= should be unwrapped.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue412_full_highlight_integration() {
        let code = "docker run -it --rm \\\n  -p 5432:5432 \\\n  --network=pg-network \\\n  --name pg-database \\\n  postgres:13\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains(r#"<span class="nt">--network</span><span class="o">=</span>pg-network"#),
            "Full bash highlight: --network should be nt and pg-network should be bare.\nActual: {html}"
        );
    }

    // === Issue 414: Bash n -> nv for uppercase variable names ===

    #[test]
    fn test_issue414_bash_n_to_nv_uppercase_var() {
        // In bash, <span class="n">DOCKER_IMAGE</span> should become <span class="nv">DOCKER_IMAGE</span>
        let code = "echo ${DOCKER_IMAGE}\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains(r#"<span class="nv">DOCKER_IMAGE</span>"#),
            "Uppercase var name should be nv, not n.\nActual: {html}"
        );
    }

    #[test]
    fn test_issue414_bash_n_to_nv_no_lowercase() {
        // Lowercase names should NOT be remapped
        let code = "echo ${filename}\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            !html.contains(r#"<span class="nv">filename</span>"#),
            "Lowercase var name should NOT be remapped to nv.\nActual: {html}"
        );
    }

    #[test]
    fn test_issue414_bash_n_to_nv_mixed_case_no_remap() {
        // Mixed case like DockerImage should NOT be remapped
        let input = r#"<span class="n">DockerImage</span>"#;
        let result = postprocess_bash_n_to_nv_uppercase(input);
        assert!(
            result.contains(r#"<span class="n">DockerImage</span>"#),
            "Mixed-case name should NOT be remapped.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue414_bash_n_to_nv_with_digits() {
        let input = r#"<span class="n">AWS_REGION_2</span>"#;
        let result = postprocess_bash_n_to_nv_uppercase(input);
        assert!(
            result.contains(r#"<span class="nv">AWS_REGION_2</span>"#),
            "Uppercase var with digits should be nv.\nActual: {result}"
        );
    }

    // === Issue 413: Bash ${VAR} substitution classes ===

    #[test]
    fn test_issue413_bash_var_substitution_braces() {
        // Jekyll: <span class="k">${</span><span class="nv">DOCKER_IMAGE</span><span class="k">}</span>
        // Rustkyll before fix: <span class="p">${</span><span class="n">DOCKER_IMAGE</span><span class="p">}</span>
        let code = "docker build -t ${DOCKER_IMAGE} .\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains(r#"<span class="k">${</span>"#),
            "${{ should have class k, not p.\nActual: {html}"
        );
        assert!(
            html.contains(r#"<span class="k">}</span>"#),
            "}} should have class k, not p.\nActual: {html}"
        );
    }

    #[test]
    fn test_issue413_bash_var_substitution_unit() {
        let input = r#"<span class="p">${</span><span class="nv">DOCKER_IMAGE</span><span class="p">}</span>"#;
        let result = postprocess_bash_var_substitution(input);
        assert_eq!(
            result,
            r#"<span class="k">${</span><span class="nv">DOCKER_IMAGE</span><span class="k">}</span>"#,
            "Should remap p to k for ${{}} braces.\nActual: {result}"
        );
    }

    // === Issue 415: Bash line continuation -> se ===

    #[test]
    fn test_issue415_bash_line_continuation_is_se() {
        // Jekyll: <span class="se">\</span>\n
        // Rustkyll before fix: <span class="p">\<newline></span>
        let code = "curl -XPOST http://example.com \\\n    -d 'data'\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains(r#"<span class="se">\</span>"#),
            "Line continuation (single \\) should be se.\nActual: {html}"
        );
    }

    #[test]
    fn test_issue415_bash_line_continuation_unit() {
        let input = "<span class=\"p\">\\\n</span>";
        let result = postprocess_bash_line_continuation_se(input);
        assert_eq!(
            result,
            "<span class=\"se\">\\</span>\n",
            "Should remap p to se for line continuation and move newline outside.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue416_bash_var_assignment_unwrap_s() {
        // DOCKER_IMAGE=serverless-ml should NOT have <span class="s"> around the value
        let code = "DOCKER_IMAGE=serverless-ml\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            !html.contains(r#"<span class="s">serverless-ml</span>"#),
            "Unquoted value after VAR= should not be wrapped in s span.\nActual: {html}"
        );
        assert!(
            html.contains(r#"<span class="o">=</span>serverless-ml"#),
            "Unquoted value after VAR= should be bare text.\nActual: {html}"
        );
    }

    #[test]
    fn test_issue416_bash_var_assignment_unwrap_unit() {
        // Unit test for the postprocessing function
        let input = r#"<span class="nv">DOCKER_IMAGE</span><span class="o">=</span><span class="s">serverless-ml</span>"#;
        let result = postprocess_bash_var_eq_unwrap_s(input);
        assert_eq!(
            result, r#"<span class="nv">DOCKER_IMAGE</span><span class="o">=</span>serverless-ml"#,
            "Should unwrap s span after = for single-word unquoted value.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue416_bash_var_assignment_no_unwrap_quoted() {
        // Quoted values should NOT be unwrapped
        let input =
            r#"<span class="nv">VAR</span><span class="o">=</span><span class="s">"hello"</span>"#;
        let result = postprocess_bash_var_eq_unwrap_s(input);
        assert_eq!(
            result, input,
            "Quoted value should not be unwrapped.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue416_bash_var_assignment_no_unwrap_s2() {
        // s2 class should NOT be unwrapped (only exact "s")
        let input =
            r#"<span class="nv">VAR</span><span class="o">=</span><span class="s2">value</span>"#;
        let result = postprocess_bash_var_eq_unwrap_s(input);
        assert_eq!(
            result, input,
            "s2 span should not be unwrapped.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue416_bash_var_assignment_no_unwrap_spaces() {
        // Values with spaces should NOT be unwrapped
        let input = r#"<span class="nv">VAR</span><span class="o">=</span><span class="s">hello world</span>"#;
        let result = postprocess_bash_var_eq_unwrap_s(input);
        assert_eq!(
            result, input,
            "Multi-word value should not be unwrapped.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue416_bash_var_assignment_no_unwrap_html_quote() {
        // Values starting with &quot; should NOT be unwrapped
        let input = r#"<span class="nv">VAR</span><span class="o">=</span><span class="s">&quot;val&quot;</span>"#;
        let result = postprocess_bash_var_eq_unwrap_s(input);
        assert_eq!(
            result, input,
            "HTML-quoted value should not be unwrapped.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue416_bash_var_assignment_multiple() {
        // Multiple VAR= on different lines
        let input = r#"<span class="nv">A</span><span class="o">=</span><span class="s">foo</span>
<span class="nv">B</span><span class="o">=</span><span class="s">bar</span>"#;
        let expected = r#"<span class="nv">A</span><span class="o">=</span>foo
<span class="nv">B</span><span class="o">=</span>bar"#;
        let result = postprocess_bash_var_eq_unwrap_s(input);
        assert_eq!(
            result, expected,
            "Should unwrap both s spans.\nActual: {result}"
        );
    }

    // ── Issue 417: Bash $ prompt with leading whitespace should get nv class ──

    #[test]
    fn test_issue417_bash_prompt_leading_space() {
        // " $ docker build" — leading space before $ prompt
        let code = " $ docker build -t myimage .\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nv\">$ </span>"),
            "Leading-space $ prompt should be wrapped as nv.\nActual: {html}"
        );
    }

    #[test]
    fn test_issue417_bash_prompt_no_leading_space() {
        // Already works: "$ docker run" at absolute line start
        let code = "$ docker run myimage\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains("<span class=\"nv\">$ </span>"),
            "Bare $ prompt should be wrapped as nv.\nActual: {html}"
        );
    }

    #[test]
    fn test_issue417_bash_prompt_not_var_substitution() {
        // $HOME should NOT get nv wrapping as prompt
        let code = "echo $HOME\n";
        let html = highlight_code("bash", code).unwrap();
        // $HOME is a variable, not a prompt — it should NOT start with nv $ prompt
        let nv_prompt_count = html.matches("<span class=\"nv\">$ </span>").count();
        assert_eq!(
            nv_prompt_count, 0,
            "$HOME should not be treated as a $ prompt.\nActual: {html}"
        );
    }

    #[test]
    fn test_issue417_bash_prompt_multiline() {
        let code = "$ sam build\n$ sam deploy\n";
        let html = highlight_code("bash", code).unwrap();
        let count = html.matches("<span class=\"nv\">$ </span>").count();
        assert_eq!(
            count, 2,
            "Both $ prompts should be wrapped.\nActual: {html}"
        );
    }

    #[test]
    fn test_issue417_bash_prompt_no_double_wrap() {
        // If already wrapped, don't double-wrap
        let input = "<span class=\"nv\">$ </span>docker run\n$ ls\n";
        let result = postprocess_bash_prompt_lines(input);
        let count = result.matches("<span class=\"nv\">$ </span>").count();
        assert_eq!(
            count, 2,
            "Should not double-wrap already-wrapped prompt.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue418_bash_angle_bracket_placeholder_unwrap() {
        // Syntect wraps < and > as operators in bash; Jekyll leaves them as plain &lt; &gt;
        let input = r#"docker build ./<span class="o">&lt;</span>path-to-Dockerfile<span class="o">&gt;</span>"#;
        let result = postprocess_bash_angle_bracket_placeholders(input);
        assert_eq!(
            result, r#"docker build ./&lt;path-to-Dockerfile&gt;"#,
            "Angle bracket placeholders should be unwrapped from operator spans"
        );
    }

    #[test]
    fn test_issue418_bash_angle_bracket_placeholder_stack_name() {
        let input = r#"aws cloudformation delete-stack <span class="nt">--stack-name</span> <span class="o">&lt;</span>stack-name<span class="o">&gt;</span>"#;
        let result = postprocess_bash_angle_bracket_placeholders(input);
        assert_eq!(
            result,
            r#"aws cloudformation delete-stack <span class="nt">--stack-name</span> &lt;stack-name&gt;"#,
            "Angle bracket placeholder after flag should be unwrapped"
        );
    }

    #[test]
    fn test_issue418_bash_angle_bracket_real_operators_preserved() {
        // Real operators like > for redirection should NOT be affected
        // because they don't match the placeholder pattern (word chars between < >)
        let input = r#"echo hello <span class="o">&gt;</span> file.txt"#;
        let result = postprocess_bash_angle_bracket_placeholders(input);
        assert_eq!(result, input, "Lone > redirect should not be affected");
    }

    #[test]
    fn test_issue417_bash_prompt_leading_spaces_unit() {
        let input = " $ docker build -t myimage\n";
        let result = postprocess_bash_prompt_lines(input);
        assert!(
            result.contains("<span class=\"nv\">$ </span>"),
            "Unit: leading-space $ should be wrapped.\nActual: {result}"
        );
        assert!(
            result.starts_with(" <span class=\"nv\">$ </span>"),
            "Unit: leading space should be preserved.\nActual: {result}"
        );
    }

    #[test]
    fn test_issue419_bash_json_braces_wrapped_as_operator() {
        // Bare { and } in bash JSON output should be wrapped with class="o"
        let input = r#"{<span class="s2">"statusCode"</span>: 200, <span class="s2">"body"</span>: <span class="s2">"{\"prediction\": \"1\"}"</span>}"#;
        let result = postprocess_bash_json_braces(input);
        assert_eq!(
            result,
            r#"<span class="o">{</span><span class="s2">"statusCode"</span>: 200, <span class="s2">"body"</span>: <span class="s2">"{\"prediction\": \"1\"}"</span><span class="o">}</span>"#,
            "Bare JSON braces should be wrapped as operator spans"
        );
    }

    #[test]
    fn test_issue419_bash_json_braces_simple() {
        // Simple JSON output: {"prediction": 1}
        let input = r#"{<span class="s2">"prediction"</span>: 1}"#;
        let result = postprocess_bash_json_braces(input);
        assert_eq!(
            result,
            r#"<span class="o">{</span><span class="s2">"prediction"</span>: 1<span class="o">}</span>"#,
            "Simple JSON braces should be wrapped as operator spans"
        );
    }

    #[test]
    fn test_issue419_bash_json_braces_no_double_wrap() {
        // Already wrapped braces should not be double-wrapped
        let input = r#"<span class="o">{</span>foo<span class="o">}</span>"#;
        let result = postprocess_bash_json_braces(input);
        assert_eq!(
            result, input,
            "Already wrapped braces should not be touched"
        );
    }

    #[test]
    fn test_issue419_bash_json_braces_skip_dollar_brace() {
        // ${VAR} pattern should not be affected ($ precedes the {)
        let input = r#"<span class="p">${</span><span class="n">DOCKER_IMAGE</span><span class="p">}</span>"#;
        let result = postprocess_bash_json_braces(input);
        assert_eq!(
            result, input,
            "Dollar-brace variables should not be affected"
        );
    }

    #[test]
    fn test_issue419_bash_json_braces_no_braces() {
        // No braces at all
        let input = r#"<span class="nv">$ </span>echo hello"#;
        let result = postprocess_bash_json_braces(input);
        assert_eq!(result, input, "Input without braces should be unchanged");
    }

    #[test]
    fn test_issue420_bash_json_string_escape_tokenization() {
        // Jekyll/Rouge splits \" escape sequences inside bash double-quoted strings
        // into separate <span class="se"> spans, with surrounding text in <span class="s2">.
        // Input bash code: {"statusCode": 200, "body": "{\"prediction\": \"1\"}"}
        let code = "{\"statusCode\": 200, \"body\": \"{\\\"prediction\\\": \\\"1\\\"}\"}\n";
        let html = highlight_code("bash", code).unwrap();

        // The inner string "{\"prediction\": \"1\"}" should be split into
        // alternating s2 and se spans, matching Jekyll/Rouge output:
        // <span class="s2">"{</span><span class="se">\"</span><span class="s2">prediction</span>
        // <span class="se">\"</span><span class="s2">: </span><span class="se">\"</span>
        // <span class="s2">1</span><span class="se">\"</span><span class="s2">}"</span>
        assert!(
            html.contains(r#"<span class="se">\"</span>"#),
            "escaped quotes in bash strings should be split into se spans: {html}"
        );
        assert!(
            html.contains(r#"<span class="s2">prediction</span>"#),
            "text between escapes should stay in s2 spans: {html}"
        );
        // The outer strings without escapes should remain unchanged
        assert!(
            html.contains(r#"<span class="s2">"statusCode"</span>"#),
            "strings without escapes should remain as s2: {html}"
        );
    }

    #[test]
    fn test_issue420_bash_string_no_escape_unchanged() {
        // Bash strings without escape sequences should not be modified
        let code = "echo \"hello world\"\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains(r#"<span class="s2">"hello world"</span>"#),
            "strings without escapes should be unchanged: {html}"
        );
        assert!(
            !html.contains(r#"<span class="se">"#),
            "no se spans should appear for non-escaped strings: {html}"
        );
    }

    #[test]
    fn test_issue420_bash_simple_json_output() {
        // Simple JSON without escapes: {"prediction": 1}
        let code = "{\"prediction\": 1}\n";
        let html = highlight_code("bash", code).unwrap();
        // No escape sequences, so no se spans
        assert!(
            !html.contains(r#"<span class="se">"#),
            "no se spans for simple JSON: {html}"
        );
        // Braces should be wrapped as operators (from issue 419)
        assert!(
            html.contains(r#"<span class="o">{</span>"#),
            "braces should be operator spans: {html}"
        );
    }

    #[test]
    fn test_issue421_bash_bracket_class_remap() {
        // `docker [run|exec] ${DOCKER_IMAGE}` — `[` should be class `o`, not `k`
        let code = "docker [run|exec] ${DOCKER_IMAGE}\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains(r#"<span class="o">[</span>"#),
            "open bracket should be class 'o' in bash: {html}"
        );
        // `]` should be bare text (not wrapped in a span), matching Jekyll/Rouge
        assert!(
            !html.contains(r#"<span class="k">]</span>"#),
            "close bracket should not be class 'k' in bash: {html}"
        );
    }

    #[test]
    fn test_issue421_bash_bracket_preserves_variable_expansion() {
        // `${DOCKER_IMAGE}` — `${` and `}` must stay class `k`
        let code = "echo ${DOCKER_IMAGE}\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            html.contains(r#"<span class="k">${</span>"#),
            "variable expansion open should remain class 'k': {html}"
        );
        assert!(
            html.contains(r#"<span class="k">}</span>"#),
            "variable expansion close should remain class 'k': {html}"
        );
    }

    #[test]
    fn test_issue421_bash_pipe_class_remap() {
        // `aws ecr get-login-password | \` — `|` should be bare text, not class `ow`
        let code = "aws ecr get-login-password | \\\n";
        let html = highlight_code("bash", code).unwrap();
        assert!(
            !html.contains(r#"<span class="ow">|</span>"#),
            "pipe should not be class 'ow' in bash: {html}"
        );
    }
}
