# Issue 276: DTC LaTeX math block rendering

## Problem

On 2 DTC blog posts, `$$...$$` display math delimiters are wrapped in `<p>` tags instead of being converted to `\[...\]` bare text nodes. This causes cascading DOM shifts (198 diffs on ner-reformers.html, 47 on regularization-in-regression.html = 245 total diffs).

## Root Cause Analysis

**Jekyll behavior:** Jekyll's default markdown processor is kramdown, which recognizes `$$...$$` as display math blocks and converts them to `\[...\]` bare text nodes (no `<p>` wrapper) for MathJax to pick up client-side.

**Rustkyll behavior (current bug):** The main rendering pipeline uses `pulldown_cmark` (via `markdown_to_html()` in `src/frontmatter.rs`), which has no knowledge of `$$...$$` math syntax. It treats `$$...$$` on its own line as a regular paragraph, wrapping it in `<p>$$...$$</p>`.

The rustkyll codebase already has two relevant pieces of code:

1. **`src/kramdown_parser/` module** -- A full kramdown parser that correctly parses `$$...$$` as `MathBlock` elements (see `try_parse_math_block()` in `parser.rs` line 4074) and renders them as `\[...\]` via `convert_math_block()` in `html.rs` line 1807. However, this parser is NOT used in the main rendering pipeline -- it is only used internally within its own test suite.

2. **`convert_math_delimiters()` in `src/kramdown.rs` line 69** -- A post-processing function that converts `<p>$$...$$</p>` to `\[...\]` bare text nodes. This function is implemented and tested but deliberately commented out of the pipeline (line 266: `// let html = convert_math_delimiters(&html);`) with the comment "Enable only when site config sets math_engine."

**Specific evidence** (ner-reformers.html):
- Jekyll cached: `\[Attention(Q,K,V) = softmax(\frac{QK^T}{\sqrt{d_k}})V\]` (bare text node)
- Rustkyll: `<p>$$ Attention(Q,K,V) = softmax(\frac{QK^T}{\sqrt{d_k}})V $$</p>` (wrapped in `<p>`, `$$` not converted)

**Specific evidence** (regularization-in-regression.html):
- Jekyll cached: `\[g(X) \approx y\]`, `\[Xw = y\]`, `\[w = (X^T X)^{-1}  X^T y\]`, `\[w = (X^T X + \alpha I)^{-1}  X^T y\]`
- Rustkyll: `<p>$$g(X) \approx y$$</p>`, `<p>$$Xw = y$$</p>`, etc.

## Key Files

- `src/kramdown.rs`: `convert_math_delimiters()` (line 69) -- the fix function, already implemented but not wired in
- `src/kramdown.rs`: `convert_display_math_blocks()` (line 101) -- display math `<p>$$...$$</p>` to `\[...\]`
- `src/kramdown.rs`: `convert_inline_math()` (line 149) -- inline `$...$` to `\(...\)`
- `src/kramdown.rs`: line 266 -- commented-out pipeline integration point
- `src/frontmatter.rs`: `markdown_to_html()` -- main markdown rendering function using pulldown_cmark
- `src/main.rs`: line 463 -- `is_kramdown` flag already computed from site config
- Affected source files:
  - `datatalksclub.github.io/_posts/2020-12-17-ner-reformers.md` (1 display math block)
  - `datatalksclub.github.io/_posts/2022-09-22-regularization-in-regression.md` (4 display math blocks)

## Fix Strategy

Enable `convert_math_delimiters()` in the markdown postprocessing pipeline when the site uses kramdown (which is the default and the case for DTC). The `is_kramdown` flag is already computed in `src/main.rs` line 463.

The simplest approach is to uncomment the `convert_math_delimiters()` call in `src/kramdown.rs` and gate it on whether kramdown is the markdown processor. The function already handles:
- Display math: `<p>$$...$$</p>` becomes `\[...\]` (bare text node, no wrapper)
- Inline math: `$...$` becomes `\(...\)`
- Skips `$` inside `<code>` and `<pre>` elements
- Skips lone `$` signs (e.g., "$100")
- Multi-line display math blocks

Alternatively, the kramdown parser module could be wired into the main pipeline for kramdown-mode sites, but that is a larger change tracked elsewhere.

**Critical: only convert display math, not inline math.** The existing `convert_math_delimiters()` converts both display (`$$...$$`) and inline (`$...$`) math. However, the DTC Jekyll cached output shows that inline `$...$` is preserved as-is (e.g., `<li>$\alpha$ is a (typically small) factor.</li>`). Both Jekyll cached and rustkyll currently produce identical inline math output. Only display math `$$...$$` differs.

The fix should call `convert_display_math_blocks()` directly (not the full `convert_math_delimiters()`) to avoid regressing inline math. This converts `<p>$$...$$</p>` to `\[...\]` bare text nodes while leaving `$...$` untouched.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with no regressions
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean
- [ ] Display math `<p>$$...$$</p>` is converted to `\[...\]` bare text nodes (no `<p>` wrapper) for kramdown-mode sites
- [ ] Inline `$...$` in paragraph text is NOT converted (preserved as-is, matching Jekyll/DTC cached output)
- [ ] ner-reformers.html: the formula `$$ Attention(Q,K,V) = softmax(\frac{QK^T}{\sqrt{d_k}})V $$` renders as `\[Attention(Q,K,V) = softmax(\frac{QK^T}{\sqrt{d_k}})V\]`
- [ ] regularization-in-regression.html: all 4 display math blocks render as `\[...\]` bare text nodes
- [ ] `$$...$$` inside `<pre>` or `<code>` blocks is NOT converted
- [ ] No regressions on other DTC pages or theme sites

## Output Verification

- [ ] Build the DTC site and verify `_site_rustkyll/blog/ner-reformers.html` contains `\[Attention(Q,K,V)` instead of `<p>$$`
- [ ] Build the DTC site and verify `_site_rustkyll/blog/regularization-in-regression.html` contains `\[g(X) \approx y\]` instead of `<p>$$g(X)`
- [ ] Verify inline math like `$\alpha$` in regularization-in-regression.html is preserved as `$\alpha$` (NOT converted to `\(\alpha\)`)

## Test Scenarios

### Unit: display math block conversion

- `<p>$$x + y$$</p>` becomes `\[x + y\]` (single-line, basic)
- `<p>$$Attention(Q,K,V) = softmax(\frac{QK^T}{\sqrt{d_k}})V$$</p>` becomes `\[...\]` (real formula with backslashes)
- `<p>$$\nalpha + beta\n$$</p>` becomes `\[...\]` (multi-line display math)
- `<p>$$\alpha \approx y$$</p>` becomes `\[\alpha \approx y\]` (non-ASCII/LaTeX)
- `<pre><code>$$x$$</code></pre>` is NOT converted (code block)
- `<p>It costs $100</p>` is NOT converted (lone dollar sign)

### Unit: inline math NOT converted

- `<p>where $\alpha$ is a factor</p>` stays as-is (inline `$...$` not touched)
- `<li>$X^T X$ is a Gram matrix</li>` stays as-is

### Integration: full pipeline

- Run `markdown_to_html()` on DTC ner-reformers.md content and verify display math renders as `\[...\]`
- Run `markdown_to_html()` on DTC regularization-in-regression.md content and verify all 4 display math blocks render as `\[...\]` while inline math stays as `$...$`

## Dependencies

- None -- this is independent of kramdown parser integration

## Log

### [SWE] 2026-03-21
- Wrote 7 failing tests in src/kramdown.rs: test_issue276_postprocess_converts_display_math, test_issue276_postprocess_converts_real_formula, test_issue276_postprocess_converts_multiline_display_math, test_issue276_postprocess_preserves_inline_math, test_issue276_postprocess_preserves_code_block_dollars, test_issue276_postprocess_preserves_lone_dollar, test_issue276_postprocess_unicode_latex
- Ran tests: 4 FAIL as expected (display math not converted), 3 PASS (preservation tests)
- Wired convert_display_math_blocks() into postprocess() pipeline at line 265 (replacing commented-out convert_math_delimiters call)
- Removed #[allow(dead_code)] from convert_display_math_blocks since it is now used
- Ran tests: all 7 new tests PASS
- Full test suite: 2625+ tests pass, 0 failures
- Clippy: clean (no warnings from rustkyll crate)
- fmt: kramdown.rs clean (generator.rs has pre-existing fmt diff from other work)
- Files modified: src/kramdown.rs, docs/tracker/276-dtc-latex-math-block-rendering.in-progress.md
