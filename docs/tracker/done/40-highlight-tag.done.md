# Issue 40: Support `{% highlight %}` Tag for Syntax Highlighting

## Problem

Complex site testing (Issue 35) revealed that many Jekyll sites use the `{% highlight lang %}...{% endhighlight %}` tag for syntax-highlighted code blocks. This tag is not recognized by rustkyll's Liquid engine, causing a parse error that prevents the entire page from rendering.

## Affected Sites

- Hyde (poole/hyde) -- `{% highlight js %}`
- So Simple Theme -- `{% highlight scss %}`
- Any Jekyll site using `{% highlight %}` instead of fenced code blocks

## Requirements

- Implement the `{% highlight lang %}...{% endhighlight %}` block tag as a custom Liquid block tag
- Wrap the content in `<pre><code class="language-{lang}">...</code></pre>`, matching the structure that CSS syntax highlighting libraries (e.g., Prism, highlight.js) expect
- HTML-escape the content inside the code block (the content is raw code, not HTML)
- Support the optional `linenos` parameter (Jekyll uses this to add line numbers). For this issue, `linenos` can be accepted and ignored -- actual line number rendering is out of scope
- Register the tag in `TemplateEngine::builder()` in `src/template/engine.rs` so it is available in all parser configurations
- The tag should not perform actual syntax highlighting (no colored spans) -- just produce the correct HTML structure. Actual highlighting is typically done client-side with JS libraries or could be added later with a Rust crate

## Approach

Create a new file `src/template/highlight_tag.rs` following the same pattern as `include_tag.rs` and `seo_tag.rs`:

1. Define a `HighlightTag` struct implementing `liquid_core::ParseBlock` (not `ParseTag`, since this is a block tag with `{% endhighlight %}`)
2. Parse the language identifier from the tag arguments
3. Optionally accept and ignore `linenos`
4. At render time, HTML-escape the block body and wrap it in `<pre><code class="language-{lang}">...escaped content...</code></pre>`
5. Register in `TemplateEngine::builder()` alongside existing tags

## Dependencies

- None

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `{% highlight js %}console.log("hello");{% endhighlight %}` renders to `<pre><code class="language-js">console.log(&quot;hello&quot;);</code></pre>` (or equivalent HTML-escaped output)
- [ ] `{% highlight python %}print("hello"){% endhighlight %}` works with any language identifier
- [ ] `{% highlight ruby linenos %}code{% endhighlight %}` parses without error (linenos accepted but may be ignored)
- [ ] Content inside the highlight block is HTML-escaped (e.g., `<div>` becomes `&lt;div&gt;`)
- [ ] The tag is registered in all parser builder paths (base builder, with_includes, rebuild_parser)
- [ ] Pages using `{% highlight %}` in test sites (Hyde, So Simple) no longer cause parse errors
- [ ] `cargo test` passes with all new tests

## Test Scenarios

### Unit: Basic highlight rendering

- Render `{% highlight js %}var x = 1;{% endhighlight %}` -- verify output is `<pre><code class="language-js">var x = 1;</code></pre>`
- Render `{% highlight python %}print("hello"){% endhighlight %}` -- verify language class is `language-python`
- Render `{% highlight scss %}.class { color: red; }{% endhighlight %}` -- verify output wraps correctly

### Unit: HTML escaping

- Render `{% highlight html %}<div class="test">&amp;</div>{% endhighlight %}` -- verify `<` becomes `&lt;`, `>` becomes `&gt;`, `"` becomes `&quot;` or is preserved inside the code block, `&` becomes `&amp;`

### Unit: Linenos parameter

- Render `{% highlight ruby linenos %}puts "hi"{% endhighlight %}` -- verify it parses without error and produces valid output (line numbers not required in output)

### Unit: Multiline content

- Render a highlight block with multiple lines of code -- verify all lines are preserved in the output with line breaks intact

### Unit: Empty content

- Render `{% highlight js %}{% endhighlight %}` -- verify it produces `<pre><code class="language-js"></code></pre>` without error

### Integration: Template engine registration

- Verify that a `TemplateEngine::new()` (without includes) can parse a template containing `{% highlight %}` without error
- Verify that a `TemplateEngine::with_includes_map()` can parse a template containing `{% highlight %}` without error
