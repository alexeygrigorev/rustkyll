# Issue 523: jekyll-docs render_mapping filter leaking into code block output

## Problem

On 11 jekyll-docs pages, code blocks that contain Liquid template examples show
`| render_mapping` appended to Liquid variable output tags. For example,
`{{ page.title }}` renders as `{{ page.title | render_mapping }}` in the HTML
output inside `<pre><code>` blocks.

This happens because our Liquid engine processes Liquid syntax inside code blocks
that Jekyll would treat as raw/literal content. Jekyll's kramdown integration
does NOT process Liquid inside fenced code blocks or `{% highlight %}` blocks.

### Affected pages (11)

- docs/liquid/index.html (1 diff - `{{ variable | render_mapping }}`)
- docs/datafiles/index.html
- docs/layouts/index.html
- docs/step-by-step/04-layouts/index.html
- docs/step-by-step/05-includes/index.html
- docs/step-by-step/06-data-files/index.html
- docs/step-by-step/07-assets/index.html
- docs/upgrading/3-to-4/index.html
- docs/continuous-integration/github-actions/index.html
- tutorials/csv-to-table/index.html
- tutorials/navigation/index.html

### Example

Source markdown (inside {% raw %} block or fenced code):
```
{{ page.title }}
```

Expected output in HTML code block:
```
{{ page.title }}
```

Actual output:
```
{{ page.title | render_mapping }}
```

## Root Cause

The `render_mapping` filter is being applied to Liquid variable tags that appear
inside code examples. These should be rendered as literal text, not processed
through the Liquid engine. This suggests that either:

1. `{% raw %}` blocks are not fully preventing Liquid processing, or
2. Fenced code blocks are being Liquid-processed when they should be literal, or
3. The render_mapping filter is being inserted at a stage after raw/code detection

## Scope

Investigate and fix why Liquid variable tags inside code blocks/raw blocks are
being processed with the render_mapping filter. The fix should ensure:

- `{% raw %}...{% endraw %}` blocks produce truly literal output
- Fenced code blocks (triple-backtick) are not Liquid-processed
- `{% highlight %}` blocks are not Liquid-processed
- Normal Liquid outside code blocks continues to work

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] Liquid tags inside `{% raw %}` blocks render as literal text (no render_mapping)
- [ ] Liquid tags inside fenced code blocks render as literal text
- [ ] Liquid tags inside `{% highlight %}` blocks render as literal text
- [ ] Normal Liquid rendering outside code blocks is unaffected
- [ ] DTC DOM match count must not drop below 790/790
- [ ] jekyll-docs liquid/index.html `{{ variable }}` renders correctly (not `{{ variable | render_mapping }}`)
- [ ] jekyll-docs tutorials/navigation page code examples show literal Liquid syntax

## Test Scenarios

### Unit: Raw block literal output

- `{% raw %}{{ page.title }}{% endraw %}` -> `{{ page.title }}` (no render_mapping)
- `{% raw %}{{ site.data.nav | where: ... }}{% endraw %}` -> literal text

### Unit: Fenced code block literal output

- Triple-backtick code block with `{{ page.title }}` -> literal Liquid syntax
- Indented code block with `{{ page.title }}` -> literal Liquid syntax

### Unit: Highlight block literal output

- `{% highlight html %}{{ page.title }}{% endhighlight %}` -> literal in `<pre><code>`

### Integration: jekyll-docs site

- Build jekyll-docs, verify docs/liquid/index.html shows `{{ variable }}` not `{{ variable | render_mapping }}`
- Build jekyll-docs, verify tutorials/navigation code examples are literal
- Run DOM comparison, verify improvement and no regression
