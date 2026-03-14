# Issue 85: Fenced code block kramdown wrapping (no language tag)

## Priority

LOW -- affects visual fidelity for fenced code blocks without a language tag, but these are less common than inline code and language-tagged blocks.

## Problem

Descoped from issue #84 (kramdown compatibility). Jekyll/kramdown wraps fenced code blocks that have no language tag in a specific structure:

```html
<div class="language-plaintext highlighter-rouge">
  <div class="highlight">
    <pre class="highlight">
      <code>plain code here</code>
    </pre>
  </div>
</div>
```

Rustkyll currently outputs:

```html
<pre><code>plain code here
</code></pre>
```

This difference can cause visual discrepancies on sites that style `.highlighter-rouge` or `.highlight` classes.

## What must be fixed

When pulldown-cmark produces `<pre><code>...</code></pre>` (fenced code block with no language), the kramdown post-processor should wrap it in the kramdown-style `<div>` structure with the appropriate classes.

This must NOT affect:
- Fenced code blocks WITH a language tag (these already have `class="language-xxx"`)
- Inline `<code>` elements (these already get `language-plaintext highlighter-rouge` from issue #84)

## Dependencies

- Issue 84 (kramdown compatibility) -- done

## Acceptance Criteria

- [ ] Fenced code blocks without a language tag are wrapped in `<div class="language-plaintext highlighter-rouge"><div class="highlight"><pre class="highlight"><code>...</code></pre></div></div>`
- [ ] Fenced code blocks WITH a language tag are NOT affected
- [ ] Inline `<code>` elements are NOT affected (they already have correct classes from issue #84)
- [ ] All existing tests still pass
- [ ] New unit tests cover the wrapping behavior
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes

## Test Scenarios

### Unit: Fenced code without language

- Parse a fenced code block (triple backticks, no language) through `markdown_to_html`, verify output contains the full kramdown wrapper structure
- Parse a fenced code block WITH a language tag, verify it is NOT wrapped in the extra `<div>` structure
- Parse inline backtick code, verify it is NOT wrapped (only gets the class attribute as before)
- Parse a document with both fenced-no-language and fenced-with-language blocks, verify only the no-language block is wrapped

## Reference

Descoped from issue #84 (AC3 bullet 3). See `src/kramdown.rs` for the existing post-processing infrastructure.
