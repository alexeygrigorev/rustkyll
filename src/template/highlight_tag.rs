//! Custom `{% highlight lang %}...{% endhighlight %}` block tag.
//!
//! Wraps the block content in `<pre><code class="language-{lang}">...</code></pre>`,
//! HTML-escaping the body. Accepts and ignores the optional `linenos` parameter.
//!
//! This does not perform actual syntax highlighting (no colored spans); it
//! produces the HTML structure expected by client-side libraries such as
//! Prism or highlight.js.

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

        // Accept and ignore optional `linenos` parameter.
        while let Ok(next) = arguments.expect_next("") {
            // Consume any remaining tokens (e.g., "linenos") silently.
            let _ = next;
        }

        arguments.expect_nothing()?;

        // Capture the raw block body using `escape_liquid` so that Liquid
        // tags/expressions inside the highlight block are NOT parsed.
        let body = _block.escape_liquid(false)?.to_owned();

        Ok(Box::new(Highlight { lang, body }))
    }

    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

#[derive(Debug)]
struct Highlight {
    lang: String,
    body: String,
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
        write!(
            writer,
            "<pre><code class=\"language-{}\">",
            html_escape(&self.lang)
        )
        .replace("Failed to render")?;

        write!(writer, "{}", html_escape(&self.body)).replace("Failed to render")?;

        write!(writer, "</code></pre>").replace("Failed to render")?;

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

    #[test]
    fn test_highlight_basic_js() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng
            .parse_and_render("{% highlight js %}var x = 1;{% endhighlight %}", &ctx)
            .unwrap();
        assert_eq!(
            output,
            "<pre><code class=\"language-js\">var x = 1;</code></pre>"
        );
    }

    #[test]
    fn test_highlight_python() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng
            .parse_and_render(
                "{% highlight python %}print(\"hello\"){% endhighlight %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(
            output,
            "<pre><code class=\"language-python\">print(&quot;hello&quot;)</code></pre>"
        );
    }

    #[test]
    fn test_highlight_scss() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng
            .parse_and_render(
                "{% highlight scss %}.class { color: red; }{% endhighlight %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(
            output,
            "<pre><code class=\"language-scss\">.class { color: red; }</code></pre>"
        );
    }

    #[test]
    fn test_highlight_html_escaping() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng
            .parse_and_render(
                "{% highlight html %}<div class=\"test\">&amp;</div>{% endhighlight %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(
            output,
            "<pre><code class=\"language-html\">&lt;div class=&quot;test&quot;&gt;&amp;amp;&lt;/div&gt;</code></pre>"
        );
    }

    #[test]
    fn test_highlight_linenos_ignored() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng
            .parse_and_render(
                "{% highlight ruby linenos %}puts \"hi\"{% endhighlight %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(
            output,
            "<pre><code class=\"language-ruby\">puts &quot;hi&quot;</code></pre>"
        );
    }

    #[test]
    fn test_highlight_multiline() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng
            .parse_and_render(
                "{% highlight js %}line1\nline2\nline3{% endhighlight %}",
                &ctx,
            )
            .unwrap();
        assert_eq!(
            output,
            "<pre><code class=\"language-js\">line1\nline2\nline3</code></pre>"
        );
    }

    #[test]
    fn test_highlight_empty_content() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng
            .parse_and_render("{% highlight js %}{% endhighlight %}", &ctx)
            .unwrap();
        assert_eq!(output, "<pre><code class=\"language-js\"></code></pre>");
    }

    #[test]
    fn test_highlight_registered_in_engine_new() {
        // Verify that TemplateEngine::new() (without includes) can parse highlight tags.
        let eng = TemplateEngine::new().unwrap();
        let result = eng.parse("{% highlight js %}code{% endhighlight %}");
        assert!(
            result.is_ok(),
            "highlight tag should be registered in new()"
        );
    }

    #[test]
    fn test_highlight_registered_in_engine_with_includes_map() {
        // Verify that TemplateEngine::with_includes_map() can parse highlight tags.
        let includes = std::collections::HashMap::new();
        let eng = TemplateEngine::with_includes_map(&includes).unwrap();
        let result = eng.parse("{% highlight js %}code{% endhighlight %}");
        assert!(
            result.is_ok(),
            "highlight tag should be registered in with_includes_map()"
        );
    }
}
