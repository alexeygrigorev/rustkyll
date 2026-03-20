# Issue 276: DTC LaTeX math block rendering

## Problem

On 2 DTC blog posts, `$$...$$` block math delimiters are wrapped in `<p>` tags instead of rendered as bare text nodes. This causes 198 diffs on `ner-reformers.html` and 47 diffs on `regularization-in-regression.html` (245 total), almost entirely cascading from the `<p>` wrapping shifting all subsequent sibling indices.

### Root Cause

Jekyll/kramdown converts `$$...$$` display math to `\[...\]` and emits it as a bare text node (not wrapped in any HTML element):

```
\[Attention(Q,K,V) = softmax(\frac{QK^T}{\sqrt{d_k}})V\]
```

Rustkyll does NOT enable pulldown-cmark's `ENABLE_MATH` option, so `$$...$$` is treated as regular paragraph text and wrapped in `<p>` tags:

```
<p>$$ Attention(Q,K,V) = softmax(\frac{QK^T}{\sqrt{d_k}})V $$</p>
```

pulldown-cmark 0.13 does support `Options::ENABLE_MATH`, which emits `Event::DisplayMath` and `Event::InlineMath` events instead of treating `$`-delimited content as regular text. However, the default HTML renderer wraps these in `<span class="math math-display">` / `<span class="math math-inline">`, which is also wrong for matching Jekyll output.

### Existing Code Context

- `protect_math_content()` in `src/frontmatter.rs` (line 636) already saves and restores math content to protect backslash sequences inside math from pulldown-cmark's escape processing (issue 227). It replaces the content between `$$...$$` delimiters with placeholders, then `restore_math_content()` puts it back after HTML generation.
- The problem is that even with content protection, pulldown-cmark still wraps the `$$PLACEHOLDER$$` in `<p>` tags during its paragraph parsing phase. The protection only helps with backslash escaping, not with the block-level wrapping.

### What Jekyll/kramdown Does

kramdown's default `math_engine` is `mathjax`. When it encounters `$$...$$` on its own line (display math), it:
1. Strips the `$$` delimiters
2. Emits the formula as `\[...\]` (LaTeX display math notation)
3. Outputs it as a bare text node at block level (no `<p>` wrapper)

For inline `$...$`, kramdown emits `\(...\)` as an inline text node within the paragraph.

The DTC site includes MathJax via `_includes/mathjax.html`, which configures `tex2jax` to recognize `$...$` for inline math. But the actual HTML emitted by kramdown uses `\[...\]` and `\(...\)`.

### Affected Source Files

- `datatalksclub.github.io/_posts/2020-12-17-ner-reformers.md` -- 1 display math block
- `datatalksclub.github.io/_posts/2022-09-22-regularization-in-regression.md` -- 4 display math blocks

### Expected vs Actual Output

**ner-reformers.html:**

Jekyll (expected):
```
<h3 id="attention-scaled-dot-product">Attention: Scaled Dot-Product</h3>

\[Attention(Q,K,V) = softmax(\frac{QK^T}{\sqrt{d_k}})V\]

<p>The inputs are:</p>
```

Rustkyll (actual):
```
<h3 id="attention-scaled-dot-product">Attention: Scaled Dot-Product</h3>

<p>$$ Attention(Q,K,V) = softmax(\frac{QK^T}{\sqrt{d_k}})V $$</p>

<p>The inputs are:</p>
```

**regularization-in-regression.html:**

Jekyll (expected):
```
\[g(X) \approx y\]
```

Rustkyll (actual):
```
<p>$$g(X) \approx y$$</p>
```

## Proposed Approach

There are two viable approaches. Either can satisfy the acceptance criteria.

### Option A: Post-process the HTML output

After `restore_math_content()` runs, scan the HTML for `<p>$$...$$</p>` patterns (display math blocks that are the sole content of a paragraph) and replace them with `\[...\]` bare text nodes. This is simpler and lower-risk since it does not change the pulldown-cmark options or event processing pipeline.

### Option B: Enable ENABLE_MATH and handle events

Enable `Options::ENABLE_MATH` in the pulldown-cmark parser options. Then, in the event processing loop, intercept `Event::DisplayMath` and `Event::InlineMath` events and emit them as raw HTML: `\[content\]` for display math and `\(content\)` for inline math. This would also allow removing the `protect_math_content` / `restore_math_content` workaround (since pulldown-cmark would handle `$` delimiters natively), but carries more risk of regressions on other sites.

**Note:** If using Option B, be careful that enabling `ENABLE_MATH` does not break inline `$` usage on sites that do not use math (where `$` is used as a literal dollar sign). The `protect_math_content` function would likely still be needed as a fallback. Evaluate regressions carefully.

## Dependencies

- None. This is independent of other open issues.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (all existing tests continue to pass)
- [ ] Display math `$$...$$` on its own paragraph line renders as `\[...\]` bare text node (no `<p>` wrapper), matching Jekyll/kramdown output
- [ ] Inline math `$...$` within a paragraph renders as `\(...\)`, matching Jekyll/kramdown output
- [ ] The ner-reformers.html DOM diff count drops from 198 to near 0 (for math-related diffs)
- [ ] The regularization-in-regression.html DOM diff count drops from 47 to near 0 (for math-related diffs)
- [ ] Backslash sequences inside math (e.g., `\frac`, `\sqrt`, `\approx`) are preserved correctly (not escaped or modified)
- [ ] Non-math uses of `$` (literal dollar signs in text) are not affected on sites that do not use math
- [ ] No regressions on other DTC pages or other test sites (mlwiki, etc.)

## Test Scenarios

### Unit: Display math block conversion
- Parse markdown with `$$...$$` on its own line, verify HTML output contains `\[...\]` as a bare text node (not wrapped in `<p>`)
- Parse markdown with `$$formula$$` (no spaces), verify same `\[formula\]` output
- Parse markdown with multi-content between `$$` delimiters containing backslashes (`\frac`, `\sqrt`), verify backslashes preserved in output

### Unit: Inline math conversion
- Parse markdown with `$x$` inside a paragraph, verify HTML output contains `\(x\)` within the `<p>` tag
- Parse markdown with `$X^T X$` containing special chars, verify content preserved

### Unit: Edge cases
- Parse markdown with `$` used as a literal dollar sign (e.g., "$100"), verify it is NOT converted to math notation
- Parse markdown with `$$` that is not a math block (e.g., inside a code block), verify it is left alone
- Parse markdown containing both display math and inline math in the same document
- Parse markdown with non-ASCII/Unicode content inside math delimiters (e.g., `$$\alpha \approx y$$`), verify correct rendering

### Integration: DTC site output verification
- Build the DTC site and verify `blog/ner-reformers.html` contains `\[Attention(Q,K,V) = softmax(\frac{QK^T}{\sqrt{d_k}})V\]` as a bare text node
- Build the DTC site and verify `blog/regularization-in-regression.html` contains `\[g(X)` (with non-breaking spaces preserved) as bare text nodes for all 4 math blocks
- Run DOM comparison on both affected pages and verify diff counts drop significantly

## Output Verification

After implementation, the engineer must:
1. Build the DTC site with `./scripts/cargo-safe build` (or equivalent)
2. Inspect `_site/blog/ner-reformers.html` and confirm the math block is rendered as `\[...\]` without `<p>` wrapping
3. Inspect `_site/blog/regularization-in-regression.html` and confirm all 4 math blocks are rendered as `\[...\]` without `<p>` wrapping
4. Run the DOM comparison tool and report the new diff counts for these two pages

## Log

### [SWE] 2026-03-20
- Wrote 11 tests in src/kramdown.rs covering: display math (single-line, no-spaces, with-spaces, multiline, with-backslashes), inline math (basic, special chars), edge cases (dollar sign not converted, code block unchanged, both display+inline, unicode)
- Ran tests: 9 FAILED as expected (2 negative tests passed correctly)
- Implemented `convert_math_delimiters()`, `convert_display_math_blocks()`, and `convert_inline_math()` in src/kramdown.rs
- Added `convert_math_delimiters` call to `postprocess()` pipeline after paragraph wrapping
- First run: 10/11 passed, multiline display math failed (line-by-line approach didn't handle `<p>$$\n...\n$$</p>`)
- Fixed: split into two-pass approach -- first pass handles display math blocks (including multiline), second pass converts inline math
- Ran tests: 11/11 PASS
- Full test suite: 2342 passed, 0 failed
- Clippy: clean (only pre-existing vendor warnings)
- Formatting: clean after `cargo fmt`
- Files modified: src/kramdown.rs
