# Issue 257: Implement {% highlight %} Liquid tag with syntax highlighting

## Problem

The `{% highlight lang %}...{% endhighlight %}` block tag exists in `src/template/highlight_tag.rs` but only renders plain `<pre><code class="language-X">` with manual HTML escaping. It does NOT use the syntect-based syntax highlighting pipeline in `src/syntax.rs`. This means `{% highlight %}` blocks produce unstyled code, while markdown fenced code blocks (processed through kramdown) get full Rouge-compatible syntax highlighting spans.

Jekyll's `{% highlight %}` tag produces server-side syntax-highlighted output with colored spans, wrapped in `<figure class="highlight">`. The chirpy theme relies on this structure.

## Scope

Wire the existing `highlight_tag.rs` renderer through `syntax::highlight_code()` so it produces the same Rouge-compatible `<span>` markup that markdown code blocks already get. Match Jekyll's output structure.

This is a small, focused change: the parsing is already correct, only the `Renderable::render_to` implementation needs updating.

## Current behavior

```
{% highlight python %}print("hello"){% endhighlight %}
```

Produces:
```html
<pre><code class="language-python">print(&quot;hello&quot;)</code></pre>
```

## Expected behavior

```
{% highlight python %}print("hello"){% endhighlight %}
```

Should produce (when syntect can highlight the language):
```html
<figure class="highlight"><pre><code class="language-python" data-lang="python"><span class="k">print</span><span class="p">(</span><span class="s2">"hello"</span><span class="p">)</span>
</code></pre></figure>
```

When syntect does NOT recognize the language, fall back to HTML-escaped plain code:
```html
<figure class="highlight"><pre><code class="language-unknownlang" data-lang="unknownlang">escaped content here
</code></pre></figure>
```

## Dependencies

None. The `highlight_tag.rs` parser and `syntax::highlight_code()` both exist and work.

## Acceptance Criteria

- [ ] `{% highlight lang %}code{% endhighlight %}` renders with Rouge-compatible `<span>` markup from `syntax::highlight_code()` when the language is supported
- [ ] Output is wrapped in `<figure class="highlight"><pre><code class="language-X" data-lang="X">...</code></pre></figure>` matching Jekyll's structure
- [ ] When syntax highlighting is unavailable (unknown language, plaintext), falls back to HTML-escaped plain text inside the same `<figure>` wrapper
- [ ] The `linenos` parameter continues to be accepted and ignored (no regression)
- [ ] Empty highlight blocks render correctly (empty `<code>` inside the wrapper)
- [ ] Multiline code content is highlighted correctly
- [ ] Content containing HTML special characters (`<`, `>`, `&`, `"`) is handled correctly (syntect handles escaping for highlighted code; manual escaping for fallback)
- [ ] Non-ASCII/Unicode content in highlight blocks renders correctly
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` reports no changes needed
- [ ] `cargo test` passes with all existing tests updated and new tests added
- [ ] No regressions in existing highlight tag tests or other template tests

## Test Scenarios

### Unit: Highlight tag rendering with syntect

- Render `{% highlight python %}print("hello"){% endhighlight %}` and verify output contains `<span class="k">print</span>` (or appropriate Rouge class)
- Render `{% highlight python %}print("hello"){% endhighlight %}` and verify output is wrapped in `<figure class="highlight"><pre><code class="language-python" data-lang="python">...</code></pre></figure>`
- Render a multiline Python snippet and verify spans appear on each line
- Render `{% highlight js %}var x = 1;{% endhighlight %}` and verify JavaScript-appropriate spans

### Unit: Fallback for unknown languages

- Render `{% highlight unknownlang123 %}some code{% endhighlight %}` and verify output is HTML-escaped plain text inside the `<figure>` wrapper
- Render `{% highlight plaintext %}some code{% endhighlight %}` and verify no `<span>` tags in output

### Unit: Edge cases

- Empty content: `{% highlight python %}{% endhighlight %}` produces valid wrapper with empty code element
- HTML special characters in code: verify `<`, `>`, `&`, `"` are properly escaped in fallback path
- `linenos` parameter: `{% highlight ruby linenos %}puts "hi"{% endhighlight %}` still works (linenos ignored)
- Non-ASCII content: `{% highlight python %}x = "cafe"{% endhighlight %}` with Unicode characters renders correctly

### Unit: Wrapper structure

- Verify the `data-lang` attribute matches the language identifier
- Verify `class="language-X"` on the `<code>` element matches the language identifier
- Verify the `<figure class="highlight">` wrapper is present

## Implementation Notes

- The `Highlight` struct in `highlight_tag.rs` already has `lang` and `body` fields
- In `render_to`, call `crate::syntax::highlight_code(&self.lang, &self.body)`
- If it returns `Some(highlighted_html)`, write the highlighted content inside the wrapper
- If it returns `None`, HTML-escape the body manually (as currently done) and write inside the wrapper
- Remove the local `html_escape` function if it becomes unused, or keep it for the fallback path
- Update existing tests to expect the new wrapper structure and (where applicable) span-based output

## Log

### [SWE] 2026-03-20
- Wrote 15 new tests replacing 10 old tests in src/template/highlight_tag.rs (TDD: tests first)
  - Tests for figure wrapper, data-lang attribute, syntax-highlighted spans (python, js, scss, html)
  - Tests for fallback (unknown lang, plaintext) with HTML escaping
  - Tests for edge cases: empty content, linenos param, unicode, HTML special chars
  - Registration tests preserved
- Ran tests: all 12 new tests FAIL as expected (old impl has no figure wrapper, no syntect)
- Implemented fix in render_to: call crate::syntax::highlight_code(), wrap in figure/pre/code with data-lang
- Ran tests: all 15 pass
- Full test suite: 1927 lib + all integration tests pass, 0 failures
- Clippy: pre-existing error in liquid-core dep + map_or warning (not from our change); no new warnings
- cargo fmt --check: clean
- Files modified: src/template/highlight_tag.rs
