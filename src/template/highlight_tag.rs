//! Custom `{% highlight lang %}...{% endhighlight %}` block tag.
//!
//! Uses syntect-based syntax highlighting via `crate::syntax::highlight_code()`
//! to produce Rouge-compatible `<span>` markup, wrapped in
//! `<figure class="highlight"><pre><code class="language-{lang}" data-lang="{lang}">...</code></pre></figure>`.
//!
//! When the language is unknown or plaintext, falls back to HTML-escaped plain
//! text inside the same wrapper structure. Accepts and ignores the optional
//! `linenos` parameter.

use std::io::Write;

use liquid_core::error::ResultLiquidReplaceExt;
use liquid_core::parser::TryMatchToken;
use liquid_core::{
    BlockReflection, Language, ParseBlock, Renderable, Runtime, TagBlock, TagTokenIter,
};

/// The `{% highlight %}` block tag parser/reflection.
#[derive(Copy, Clone, Debug, Default)]
pub struct HighlightBlock;

impl BlockReflection for HighlightBlock {
    fn start_tag(&self) -> &str {
        "highlight"
    }

    fn end_tag(&self) -> &str {
        "endhighlight"
    }

    fn description(&self) -> &str {
        "Wrap code in <pre><code> with a language class"
    }
}

impl ParseBlock for HighlightBlock {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        mut _block: TagBlock<'_, '_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        // Parse the language identifier (required).
        let lang_token = arguments.expect_next("language identifier expected")?;
        let lang = match lang_token.expect_identifier() {
            TryMatchToken::Matches(id) => id.to_owned(),
            TryMatchToken::Fails(token) => token.as_str().to_owned(),
        };

        // Check for optional `linenos` and `hl_lines` parameters.
        // The token stream from Liquid gives us individual tokens, so we
        // accumulate them to detect `hl_lines="1 3 5"` patterns.
        let mut linenos = false;
        let mut hl_lines: Vec<usize> = Vec::new();
        // State machine: after seeing "hl_lines", expect "=", then the quoted value.
        let mut hl_state = 0u8; // 0=idle, 1=saw hl_lines, 2=saw =
        while let Ok(next) = arguments.expect_next("") {
            let s = next.as_str();
            match hl_state {
                1 if s == "=" => {
                    hl_state = 2;
                }
                2 => {
                    let val = s.trim_matches('"');
                    hl_lines = parse_hl_lines(val);
                    hl_state = 0;
                }
                _ => {
                    hl_state = 0;
                    if s == "linenos" {
                        linenos = true;
                    } else if s == "hl_lines" {
                        hl_state = 1;
                    }
                }
            }
        }

        arguments.expect_nothing()?;

        // Capture the raw block body using `escape_liquid` so that Liquid
        // tags/expressions inside the highlight block are NOT parsed.
        // Use allow_nesting=true so that nested {% highlight %}...{% endhighlight %}
        // pairs (e.g. inside {% raw %} blocks showing example code) are properly
        // handled instead of closing on the inner {% endhighlight %}.
        let body = _block.escape_liquid(true)?.to_owned();

        Ok(Box::new(Highlight {
            lang,
            body,
            linenos,
            hl_lines,
        }))
    }

    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

#[derive(Debug)]
struct Highlight {
    lang: String,
    body: String,
    linenos: bool,
    hl_lines: Vec<usize>,
}

/// Parse a space-separated list of 1-based line numbers from an `hl_lines` value.
fn parse_hl_lines(value: &str) -> Vec<usize> {
    value
        .split_whitespace()
        .filter_map(|s| s.parse::<usize>().ok())
        .collect()
}

/// Wrap specified lines (1-based) in `<span class="hll">...</span>`.
/// Each wrapped line includes its trailing newline inside the span, matching
/// Jekyll/Rouge behavior.
pub fn wrap_hl_lines(content: &str, hl_lines: &[usize]) -> String {
    if hl_lines.is_empty() {
        return content.to_string();
    }
    let mut result = String::with_capacity(content.len() + hl_lines.len() * 30);
    // Split preserving trailing content: we split on '\n' and rejoin manually.
    // Jekyll wraps each highlighted line including the trailing newline.
    let lines: Vec<&str> = content.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1; // 1-based
        let is_last = i == lines.len() - 1;
        if hl_lines.contains(&line_num) {
            result.push_str("<span class=\"hll\">");
            result.push_str(line);
            if !is_last {
                result.push('\n');
            }
            result.push_str("</span>");
        } else {
            result.push_str(line);
            if !is_last {
                result.push('\n');
            }
        }
    }
    result
}

/// HTML-escape a string: replace `&`, `<`, `>`, and `"`.
fn html_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            _ => result.push(c),
        }
    }
    result
}

impl Renderable for Highlight {
    fn render_to(&self, writer: &mut dyn Write, _runtime: &dyn Runtime) -> liquid_core::Result<()> {
        let escaped_lang = html_escape(&self.lang);

        write!(
            writer,
            "<figure class=\"highlight\"><pre><code class=\"language-{}\" data-lang=\"{}\">",
            escaped_lang, escaped_lang
        )
        .replace("Failed to render")?;

        // Get the highlighted or escaped content, then apply hl_lines wrapping.
        let raw_content =
            if let Some(highlighted) = crate::syntax::highlight_code(&self.lang, &self.body) {
                highlighted
            } else {
                html_escape(&self.body)
            };
        let content = wrap_hl_lines(&raw_content, &self.hl_lines);

        if self.linenos {
            // Count lines in the original body, stripping leading/trailing
            // whitespace that escape_liquid may capture around the code block.
            let trimmed_body = self.body.trim();
            let line_count = trimmed_body.lines().count().max(1);
            let mut line_numbers = String::new();
            for i in 1..=line_count {
                line_numbers.push_str(&i.to_string());
                line_numbers.push('\n');
            }

            write!(
                writer,
                "<table class=\"rouge-table\"><tbody><tr>\
                 <td class=\"gutter gl\"><pre class=\"lineno\">{}</pre></td>\
                 <td class=\"code\"><pre>{}</pre></td>\
                 </tr></tbody></table>",
                line_numbers, content
            )
            .replace("Failed to render")?;
        } else {
            write!(writer, "{}", content).replace("Failed to render")?;
        }

        write!(writer, "</code></pre></figure>").replace("Failed to render")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::engine::TemplateEngine;
    use liquid::Object;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    fn render(template: &str) -> String {
        let eng = engine();
        let ctx = Object::new();
        eng.parse_and_render(template, &ctx).unwrap()
    }

    // --- Syntax-highlighted output tests ---

    #[test]
    fn test_highlight_python_has_spans() {
        let output = render("{% highlight python %}print(\"hello\"){% endhighlight %}");
        // Must contain Rouge-compatible span markup from syntect
        assert!(
            output.contains("<span class=\""),
            "Expected syntax-highlighted spans, got: {output}"
        );
    }

    #[test]
    fn test_highlight_python_figure_wrapper() {
        let output = render("{% highlight python %}print(\"hello\"){% endhighlight %}");
        assert!(
            output.starts_with("<figure class=\"highlight\"><pre><code class=\"language-python\" data-lang=\"python\">"),
            "Expected <figure class=\"highlight\"> wrapper, got: {output}"
        );
        assert!(
            output.ends_with("</code></pre></figure>"),
            "Expected </code></pre></figure> ending, got: {output}"
        );
    }

    #[test]
    fn test_highlight_js_has_spans() {
        let output = render("{% highlight js %}var x = 1;{% endhighlight %}");
        assert!(
            output.contains("<span class=\""),
            "Expected syntax-highlighted spans for JS, got: {output}"
        );
        assert!(
            output.contains("data-lang=\"js\""),
            "Expected data-lang attribute, got: {output}"
        );
    }

    #[test]
    fn test_highlight_scss_figure_wrapper() {
        let output = render("{% highlight scss %}.class { color: red; }{% endhighlight %}");
        assert!(
            output.starts_with("<figure class=\"highlight\"><pre><code class=\"language-scss\" data-lang=\"scss\">"),
            "Expected figure wrapper for scss, got: {output}"
        );
        assert!(
            output.ends_with("</code></pre></figure>"),
            "Expected closing wrapper, got: {output}"
        );
    }

    #[test]
    fn test_highlight_multiline_has_spans() {
        let output = render("{% highlight python %}x = 1\ny = 2\nprint(x + y){% endhighlight %}");
        // Multiple lines should all get highlighted
        assert!(
            output.contains("<span class=\""),
            "Expected spans in multiline output, got: {output}"
        );
        assert!(
            output.contains("data-lang=\"python\""),
            "Expected data-lang attribute, got: {output}"
        );
    }

    // --- Fallback for unknown language ---

    #[test]
    fn test_highlight_unknown_lang_fallback() {
        let output = render("{% highlight unknownlang123 %}some code{% endhighlight %}");
        assert!(
            output.starts_with("<figure class=\"highlight\"><pre><code class=\"language-unknownlang123\" data-lang=\"unknownlang123\">"),
            "Expected figure wrapper for unknown lang, got: {output}"
        );
        assert!(
            output.contains("some code"),
            "Expected plain text content, got: {output}"
        );
        assert!(
            !output.contains("<span class=\""),
            "Expected no span tags for unknown lang, got: {output}"
        );
        assert!(
            output.ends_with("</code></pre></figure>"),
            "Expected closing wrapper, got: {output}"
        );
    }

    #[test]
    fn test_highlight_plaintext_fallback() {
        let output = render("{% highlight plaintext %}some code{% endhighlight %}");
        assert!(
            !output.contains("<span class=\""),
            "Expected no span tags for plaintext, got: {output}"
        );
        assert!(
            output.starts_with("<figure class=\"highlight\"><pre><code class=\"language-plaintext\" data-lang=\"plaintext\">"),
            "Expected figure wrapper for plaintext, got: {output}"
        );
    }

    // --- Edge cases ---

    #[test]
    fn test_highlight_empty_content() {
        let output = render("{% highlight python %}{% endhighlight %}");
        assert_eq!(
            output,
            "<figure class=\"highlight\"><pre><code class=\"language-python\" data-lang=\"python\"></code></pre></figure>"
        );
    }

    #[test]
    fn test_highlight_html_special_chars_fallback() {
        // Unknown language: manual HTML escaping must happen
        let output = render(
            "{% highlight unknownlang123 %}<div class=\"test\">&amp;</div>{% endhighlight %}",
        );
        assert!(
            output.contains("&lt;div"),
            "Expected HTML-escaped content in fallback, got: {output}"
        );
        assert!(
            output.contains("&amp;amp;"),
            "Expected double-escaped ampersand in fallback, got: {output}"
        );
    }

    #[test]
    fn test_highlight_html_special_chars_highlighted() {
        // Known language (html): syntect handles escaping
        let output =
            render("{% highlight html %}<div class=\"test\">&amp;</div>{% endhighlight %}");
        assert!(
            output.starts_with("<figure class=\"highlight\"><pre><code class=\"language-html\" data-lang=\"html\">"),
            "Expected figure wrapper, got: {output}"
        );
        // syntect should handle the escaping internally
        assert!(
            output.contains("<span class=\""),
            "Expected spans for html highlighting, got: {output}"
        );
    }

    #[test]
    fn test_highlight_linenos_table_structure() {
        let output =
            render("{% highlight ruby linenos %}puts \"hi\"\nputs \"bye\"{% endhighlight %}");
        // linenos should produce a <table class="rouge-table"> inside the <code>
        assert!(
            output.contains("<table class=\"rouge-table\">"),
            "Expected rouge-table for linenos, got: {output}"
        );
        assert!(
            output.contains("<td class=\"gutter gl\">"),
            "Expected gutter column for linenos, got: {output}"
        );
        assert!(
            output.contains("<pre class=\"lineno\">"),
            "Expected lineno pre element, got: {output}"
        );
        assert!(
            output.contains("<td class=\"code\">"),
            "Expected code column for linenos, got: {output}"
        );
        // Line numbers should be present
        assert!(
            output.contains("1\n2\n"),
            "Expected line numbers 1 and 2, got: {output}"
        );
        // Should still have the figure wrapper
        assert!(
            output.starts_with("<figure class=\"highlight\"><pre><code class=\"language-ruby\" data-lang=\"ruby\">"),
            "Expected figure wrapper with ruby, got: {output}"
        );
    }

    #[test]
    fn test_highlight_linenos_single_line() {
        let output = render("{% highlight python linenos %}x = 1{% endhighlight %}");
        assert!(
            output.contains("<table class=\"rouge-table\">"),
            "Expected rouge-table for single-line linenos, got: {output}"
        );
        assert!(
            output.contains("1\n</pre>"),
            "Expected single line number, got: {output}"
        );
    }

    #[test]
    fn test_highlight_unicode_content() {
        let output = render("{% highlight python %}x = \"cafe\\u0301\"{% endhighlight %}");
        assert!(
            output.starts_with("<figure class=\"highlight\"><pre><code class=\"language-python\" data-lang=\"python\">"),
            "Expected figure wrapper, got: {output}"
        );
        assert!(
            output.ends_with("</code></pre></figure>"),
            "Expected closing wrapper, got: {output}"
        );
    }

    #[test]
    fn test_highlight_data_lang_attribute() {
        let output = render("{% highlight javascript %}var x;{% endhighlight %}");
        assert!(
            output.contains("data-lang=\"javascript\""),
            "Expected data-lang=\"javascript\", got: {output}"
        );
        assert!(
            output.contains("class=\"language-javascript\""),
            "Expected class=\"language-javascript\", got: {output}"
        );
    }

    // --- hl_lines tests ---

    #[test]
    fn test_highlight_hl_lines_wraps_specified_lines() {
        // hl_lines="2" should wrap only line 2 in <span class="hll">
        let output = render(
            "{% highlight plaintext hl_lines=\"2\" %}line one\nline two\nline three{% endhighlight %}",
        );
        assert!(
            !output.contains("<span class=\"hll\">line one"),
            "Line 1 should NOT be wrapped, got: {output}"
        );
        assert!(
            output.contains("<span class=\"hll\">line two\n</span>"),
            "Line 2 should be wrapped in hll span, got: {output}"
        );
        assert!(
            !output.contains("<span class=\"hll\">line three"),
            "Line 3 should NOT be wrapped, got: {output}"
        );
    }

    #[test]
    fn test_highlight_hl_lines_multiple_lines() {
        let output =
            render("{% highlight plaintext hl_lines=\"1 3\" %}aaa\nbbb\nccc{% endhighlight %}");
        assert!(
            output.contains("<span class=\"hll\">aaa\n</span>"),
            "Line 1 should be wrapped, got: {output}"
        );
        assert!(
            !output.contains("<span class=\"hll\">bbb"),
            "Line 2 should NOT be wrapped, got: {output}"
        );
        assert!(
            output.contains("<span class=\"hll\">ccc\n</span>")
                || output.contains("<span class=\"hll\">ccc</span>"),
            "Line 3 should be wrapped, got: {output}"
        );
    }

    #[test]
    fn test_highlight_hl_lines_single_line() {
        let output = render("{% highlight plaintext hl_lines=\"1\" %}only line{% endhighlight %}");
        assert!(
            output.contains("<span class=\"hll\">only line\n</span>")
                || output.contains("<span class=\"hll\">only line</span>"),
            "Single line should be wrapped, got: {output}"
        );
    }

    #[test]
    fn test_highlight_no_hl_lines_no_hll_spans() {
        let output = render("{% highlight plaintext %}line one\nline two{% endhighlight %}");
        assert!(
            !output.contains("class=\"hll\""),
            "Without hl_lines, no hll spans should appear, got: {output}"
        );
    }

    #[test]
    fn test_highlight_hl_lines_with_syntax_highlighting() {
        // hl_lines with a real language -- the hll span wraps the entire line including inner spans
        let output = render("{% highlight ruby hl_lines=\"1\" %}puts \"hello\"{% endhighlight %}");
        assert!(
            output.contains("<span class=\"hll\">"),
            "hl_lines should produce hll span even with syntax highlighting, got: {output}"
        );
    }

    #[test]
    fn test_highlight_hl_lines_with_linenos() {
        let output = render(
            "{% highlight plaintext linenos hl_lines=\"1\" %}line one\nline two{% endhighlight %}",
        );
        assert!(
            output.contains("<span class=\"hll\">"),
            "hl_lines should work with linenos, got: {output}"
        );
        assert!(
            output.contains("<table class=\"rouge-table\">"),
            "linenos table should still be present, got: {output}"
        );
    }

    // --- Registration tests ---

    #[test]
    fn test_highlight_registered_in_engine_new() {
        let eng = TemplateEngine::new().unwrap();
        let result = eng.parse("{% highlight js %}code{% endhighlight %}");
        assert!(
            result.is_ok(),
            "highlight tag should be registered in new()"
        );
    }

    #[test]
    fn test_highlight_registered_in_engine_with_includes_map() {
        let includes = std::collections::HashMap::new();
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let result = eng.parse("{% highlight js %}code{% endhighlight %}");
        assert!(
            result.is_ok(),
            "highlight tag should be registered in with_includes_map()"
        );
    }
}
