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

        // Check for optional `linenos` parameter.
        let mut linenos = false;
        while let Ok(next) = arguments.expect_next("") {
            if next.as_str() == "linenos" {
                linenos = true;
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

        // Get the highlighted (or escaped) content
        let content =
            if let Some(highlighted) = crate::syntax::highlight_code(&self.lang, &self.body) {
                highlighted
            } else {
                html_escape(&self.body)
            };

        if self.linenos {
            // Count lines in the body (trim leading/trailing newlines from block capture)
            let trimmed_body = self.body.trim_matches('\n');
            let line_count = trimmed_body.lines().count();
            // Build line numbers: "1\n2\n3\n..."
            let mut lineno_str = String::new();
            for n in 1..=line_count {
                lineno_str.push_str(&n.to_string());
                lineno_str.push('\n');
            }

            write!(
                writer,
                "<table class=\"rouge-table\"><tbody><tr>\
                 <td class=\"gutter gl\"><pre class=\"lineno\">{}</pre></td>\
                 <td class=\"code\"><pre>{}\n</pre></td>\
                 </tr></tbody></table>",
                lineno_str, content
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
        let output = render("{% highlight ruby linenos %}puts \"hi\"{% endhighlight %}");
        assert!(
            output.starts_with("<figure class=\"highlight\"><pre><code class=\"language-ruby\" data-lang=\"ruby\">"),
            "Expected figure wrapper with ruby, got: {output}"
        );
        assert!(
            output.contains("<table class=\"rouge-table\">"),
            "Expected rouge-table when linenos used, got: {output}"
        );
        assert!(
            output.contains("<td class=\"gutter gl\">"),
            "Expected gutter td when linenos used, got: {output}"
        );
        assert!(
            output.contains("<pre class=\"lineno\">"),
            "Expected lineno pre when linenos used, got: {output}"
        );
        assert!(
            output.contains("<td class=\"code\">"),
            "Expected code td when linenos used, got: {output}"
        );
        assert!(
            output.ends_with("</code></pre></figure>"),
            "Expected closing wrapper, got: {output}"
        );
    }

    #[test]
    fn test_highlight_linenos_line_numbers() {
        let output =
            render("{% highlight javascript linenos %}var x = 1;\nvar y = 2;{% endhighlight %}");
        assert!(
            output.contains("<pre class=\"lineno\">1\n2\n</pre>"),
            "Expected line numbers 1 and 2, got: {output}"
        );
    }

    #[test]
    fn test_highlight_without_linenos_no_table() {
        let output = render("{% highlight javascript %}var x = 1;{% endhighlight %}");
        assert!(
            !output.contains("<table"),
            "Should NOT have table without linenos, got: {output}"
        );
        assert!(
            !output.contains("rouge-table"),
            "Should NOT have rouge-table without linenos, got: {output}"
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
