# Issue 538: Highlight tag does not support line highlighting (hl_lines / hll class)

## Problem

Jekyll's `{% highlight %}` tag supports the `hl_lines` option to highlight specific
lines with `<span class="hll">` wrappers. Rustkyll does not implement this feature,
so highlighted lines are rendered without the `hll` wrapper.

### Example

Source:
```liquid
{% highlight html hl_lines="1 4" %}
<html>
  <head>
    <meta charset="utf-8" />
    <title>Hello World</title>
  </head>
</html>
{% endhighlight %}
```

Jekyll (correct -- lines 1 and 4 wrapped in `<span class="hll">`):
```html
<figure class="highlight"><pre><code class="language-html" data-lang="html"><span class="hll"><span class="nt">&lt;html&gt;</span>
</span>  <span class="nt">&lt;head&gt;</span>
    <span class="nt">&lt;meta</span> <span class="na">charset=</span><span class="s">"utf-8"</span> <span class="nt">/&gt;</span>
<span class="hll">    <span class="nt">&lt;title&gt;</span>Hello World<span class="nt">&lt;/title&gt;</span>
</span>  <span class="nt">&lt;/head&gt;</span>
```

Rustkyll (wrong -- no `hll` wrappers, different structure):
```html
<figure class="highlight"><pre><code class="language-html" data-lang="html">
<span class="nt">&lt;html&gt;</span>
  <span class="nt">&lt;head&gt;</span>
   <span class="nt">&lt;meta</span> ...
```

### Affected page

`codeblocks-ahoy.html` in minima (2 highlight blocks with hl_lines)

## Root Cause

The `{% highlight %}` tag parser does not extract or use the `hl_lines` parameter.
The line highlighting wrapping step is missing from the syntax highlighting pipeline.

## Dependencies

None.

## Scope

- Parse `hl_lines` parameter from `{% highlight %}` tag
- After syntax highlighting, wrap specified lines in `<span class="hll">...</span>`
- Line numbers are 1-based

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes
- [ ] DTC DOM baseline: 790/790 (must not regress)
- [ ] `{% highlight html hl_lines="1 4" %}` wraps lines 1 and 4 with `<span class="hll">`
- [ ] Lines NOT in hl_lines are NOT wrapped
- [ ] Works with `linenos` option combined with `hl_lines`
- [ ] At least 3 new unit tests

## Test Scenarios

### Unit: hl_lines parsing
- `{% highlight ruby hl_lines="1 3" %}` -> parsed as lines [1, 3]
- `{% highlight ruby %}` (no hl_lines) -> no line highlighting
- `{% highlight ruby hl_lines="1" %}` -> single line highlighted

### Unit: line wrapping
- 3-line code block with hl_lines="2" -> only line 2 wrapped in `<span class="hll">`

### Integration: minima build
- Build minima, verify `codeblocks-ahoy.html` has `<span class="hll">` wrappers

## Baselines

- DTC: 790/790
- Minima codeblocks-ahoy.html: this fix should eliminate ~20 diffs (hll-related)
