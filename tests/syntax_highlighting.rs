//! Tests for syntax highlighting Rouge/Pygments compatibility (issue 197).
//!
//! These tests verify that rustkyll's syntax highlighter produces the same
//! CSS class names as Rouge (used by Jekyll) for various programming languages.

use rustkyll::syntax::highlight_code;

// ── JavaScript ──

#[test]
fn test_js_var_is_kd() {
    // Rouge classifies `var` as `kd` (keyword.declaration) in JavaScript.
    // Unicode test: string with non-ASCII chars.
    let code = "var greeting = \"Bonjour le m\u{00F4}nde\";\n";
    let html = highlight_code("javascript", code).unwrap();
    assert!(
        html.contains("<span class=\"kd\">var</span>"),
        "JS 'var' should be classified as 'kd' (keyword.declaration). Got: {}",
        html
    );
}

#[test]
fn test_js_function_is_kd() {
    // Rouge classifies `function` as `kd` in JavaScript.
    let code = "var fun = function lang(l) {\n  return true;\n}\n";
    let html = highlight_code("javascript", code).unwrap();
    assert!(
        html.contains("<span class=\"kd\">function</span>"),
        "JS 'function' should be classified as 'kd' (keyword.declaration). Got: {}",
        html
    );
}

#[test]
fn test_js_identifiers_are_nx() {
    // Rouge classifies plain identifiers as `nx` (name.other) in JavaScript.
    let code =
        "var fun = function lang(l) {\n  dateformat.i18n = require('./lang/' + l)\n  return true;\n}\n";
    let html = highlight_code("javascript", code).unwrap();
    // Check that identifiers like fun, dateformat, i18n get nx
    assert!(
        html.contains("class=\"nx\""),
        "JS identifiers should use 'nx' (name.other) class. Got: {}",
        html
    );
}

#[test]
fn test_js_theme_code_exact() {
    // The exact code from Jekyll theme sites. Rouge expected output includes kd, nx classes.
    // Unicode: non-ASCII comment character.
    let code = "// Javascript code with syntax highlighting. T\u{00E9}st\nvar fun = function lang(l) {\n  dateformat.i18n = require('./lang/' + l)\n  return true;\n}\n";
    let html = highlight_code("javascript", code).unwrap();
    assert!(
        html.contains("<span class=\"kd\">var</span>"),
        "JS 'var' should be 'kd'. Got: {}",
        html
    );
    assert!(
        html.contains("<span class=\"kd\">function</span>"),
        "JS 'function' should be 'kd'. Got: {}",
        html
    );
    // fun should be nx (function name in JS context)
    assert!(
        html.contains("<span class=\"nx\">fun</span>"),
        "JS identifier 'fun' should be 'nx'. Got: {}",
        html
    );
    // i18n should be wrapped in nx span
    assert!(
        html.contains("<span class=\"nx\">i18n</span>"),
        "JS property 'i18n' should be 'nx'. Got: {}",
        html
    );
    // require should be wrapped in nx span
    assert!(
        html.contains("<span class=\"nx\">require</span>"),
        "JS function call 'require' should be 'nx'. Got: {}",
        html
    );
}

#[test]
fn test_js_equals_is_o() {
    // Rouge classifies `=` as `o` (operator) in JavaScript.
    let code = "var x = 1;\n";
    let html = highlight_code("javascript", code).unwrap();
    assert!(
        html.contains("<span class=\"o\">=</span>"),
        "JS '=' should be classified as 'o' (operator). Got: {}",
        html
    );
}

// ── Ruby ──

#[test]
fn test_ruby_do_is_k() {
    // Ruby theme site code. Unicode non-ASCII in comment.
    let code = "# Ruby code. T\u{00E9}st\nGitHubPages::Dependencies.gems.each do |gem, version|\n  s.add_dependency(gem, \"= #{version}\")\nend\n";
    let html = highlight_code("ruby", code).unwrap();
    assert!(
        html.contains("<span class=\"k\">do</span>"),
        "Ruby 'do' should be 'k'. Got: {}",
        html
    );
}

// ── Bash ──

#[test]
fn test_bash_docker_flags() {
    // Unicode container name
    let code = "docker run --rm --name postgr\u{00E9}sql\n";
    let html = highlight_code("bash", code).unwrap();
    assert!(
        html.contains("docker") && html.contains("--rm"),
        "Bash should contain docker and --rm. Got: {}",
        html
    );
}

// ── XML ──

#[test]
fn test_xml_closing_tag_single_nt_span() {
    // Unicode tag content
    let code = "<action>ex\u{00E9}cute</action>\n<plugin>my-plugin</plugin>\n";
    let html = highlight_code("xml", code).unwrap();
    assert!(
        html.contains("<span class=\"nt\">&lt;/action&gt;</span>"),
        "XML </action> should be single 'nt' span. Got: {}",
        html
    );
    assert!(
        html.contains("<span class=\"nt\">&lt;/plugin&gt;</span>"),
        "XML </plugin> should be single 'nt' span. Got: {}",
        html
    );
}

// ── SQL ──

#[test]
fn test_sql_select_from_where_are_k() {
    // Unicode identifiers
    let code = "SELECT n\u{00E4}me FROM us\u{00E9}rs WHERE id = 1;\n";
    let html = highlight_code("sql", code).unwrap();
    assert!(
        html.contains("<span class=\"k\">SELECT</span>"),
        "SQL 'SELECT' should be 'k'. Got: {}",
        html
    );
    assert!(
        html.contains("<span class=\"k\">FROM</span>"),
        "SQL 'FROM' should be 'k'. Got: {}",
        html
    );
    assert!(
        html.contains("<span class=\"k\">WHERE</span>"),
        "SQL 'WHERE' should be 'k'. Got: {}",
        html
    );
}
