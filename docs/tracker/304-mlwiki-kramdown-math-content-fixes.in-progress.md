# Issue 304: mlwiki kramdown math content fixes -- brace escaping, pipe/table, underscore emphasis

## Problem

mlwiki.org matches 311/644 (48%). Of the 333 diff pages, 175+ are caused by
the kramdown parser mishandling content inside `$...$` and `$$...$$` math
delimiters. Three distinct bugs:

### Bug A: Backslash-escaped braces not unescaped (90 pages)

Source markdown contains `\{` and `\}` inside math (e.g., `$A = \{x, y\}$`).
Jekyll's kramdown unescapes these to `{` and `}` in the HTML output. Rustkyll
keeps them as literal `\{` and `\}`.

Example:
- Source: `$C_1 = \{ \text{all itemsets of len 1} \}$`
- Jekyll output: `$C_1 = { \text{all itemsets of len 1} }$`
- Rustkyll output: `$C_1 = \{ \text{all itemsets of len 1} \}$`

Root cause: The kramdown span parser's backslash escape handling
(`is_escapable_char` in `span_parser.rs`) recognizes `{` and `}` as
escapable characters, but either (a) the escape processing is not being
applied inside math content, or (b) the math content protection is
preventing the escape from being processed.

The fix must ensure `\{` becomes `{` and `\}` becomes `}` in the final
HTML output when these occur inside math delimiters.

### Bug B: Pipe character triggering table parsing inside math (67 pages)

When `|` appears inside `$...$` math (e.g., `$x | y$` or `$F = {x | x > 0}$`),
the kramdown block parser treats it as a table separator row, producing a
`<table>` element instead of inline math text.

Example:
- Source: `$x \ | | \ y$`
- Jekyll output: inline text `$x \ | | \ y$`
- Rustkyll output: `<table>` element with rows

Root cause: The kramdown block parser's table detection sees `|` on a line
and starts table parsing before the span parser can identify it as math
content. The fix requires the block parser to recognize math delimiters and
skip table detection for content inside `$...$`.

Key files: `src/kramdown_parser/parser.rs` (block-level table detection),
`src/kramdown_parser/span_parser.rs` (math content identification)

### Bug C: Underscore after `}` in math triggering emphasis (17 pages)

In expressions like `$\underbrace{[x P y]}_\text{(1)}$`, the `}_` sequence
causes the span parser to interpret `_` as an emphasis marker, splitting
the math expression and wrapping part of it in `<em>`.

Example:
- Source: `$\underbrace{[x P y]}_\text{(1)}$`
- Jekyll output: single text node with complete math expression
- Rustkyll output: text node + `<em>` element (emphasis triggered)

Root cause: The span parser's emphasis detection does not recognize that
`_` inside `$...$` math should not trigger emphasis. The fix must suppress
emphasis parsing inside math delimiters.

## Scope

All three bugs (A, B, C) are in scope. They share a common theme: the
kramdown parser must treat content inside `$...$` and `$$...$$` as opaque,
applying only backslash escape unescaping and no other processing (no table
detection, no emphasis parsing, no IAL processing).

### Out of scope

- Rouge syntax highlighting token class diffs (30 pages) -- tracked by issue 293
- Missing `<br>` from `\\` in math matrices (19 pages) -- separate issue
- Missing `&nbsp;` (15 pages) -- separate issue
- Hash `#` escaping in math (1 page) -- minor, can be included if trivial
- Ellipsis conversion (9 pages) -- should already be fixed by issue 302 changes;
  if not, include in this issue

## Dependencies

- Issue 302 (mlwiki ellipsis/braces for pulldown-cmark path) -- DONE (code committed
  to frontmatter.rs, but this issue targets the kramdown parser path)
- No other blockers

## Key Files to Modify

- `src/kramdown_parser/span_parser.rs` -- backslash escape processing inside math,
  emphasis suppression inside math
- `src/kramdown_parser/parser.rs` -- block-level table detection must skip math content
- `src/kramdown.rs` -- integration tests, possibly preprocessing of math content
- `src/kramdown_parser/html.rs` -- HTML rendering of math content with unescaped braces

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests below
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] Bug A: `\{` and `\}` inside `$...$` math are unescaped to `{` and `}`
      in HTML output, matching Jekyll behavior
- [ ] Bug A: `\{` and `\}` outside math continue to be unescaped normally
      (existing behavior preserved)
- [ ] Bug B: `|` inside `$...$` math does NOT trigger table parsing -- the
      content stays as inline text
- [ ] Bug B: `|` outside math continues to trigger table parsing when appropriate
- [ ] Bug C: `_` inside `$...$` math does NOT trigger emphasis parsing
- [ ] Bug C: `_` outside math continues to trigger emphasis normally
- [ ] mlwiki.org DOM match improves to 420+/644 (from current 311/644)
- [ ] No regressions on DTC (must remain 662+/790)
- [ ] No regressions on muan-blog (must remain 2174+/2218)
- [ ] No regressions on kramdown conformance (must remain 656+/658)
- [ ] No regressions on any of the 13+ sites currently at 100%
- [ ] Tests include non-ASCII/Unicode content (e.g., math with Greek letters)

## Test Scenarios

### Unit: Backslash-escaped braces in math (Bug A)

- Parse `$A = \{x, y\}$` through kramdown, verify output contains
  `$A = {x, y}$` (braces unescaped)
- Parse `$$F = \{x \mid x > 0\}$$` through kramdown, verify braces unescaped
- Parse `\{escaped\}` outside math, verify it renders as `{escaped}` (existing behavior)
- Parse `$\alpha \in \{1, 2, 3\}$` (Unicode + braces), verify braces unescaped
- Parse `{: .class}` IAL outside math, verify it still works (not affected by fix)

### Unit: Pipe in math not triggering tables (Bug B)

- Parse `$x | y$` through kramdown, verify output is text (no `<table>`)
- Parse `- $F = {x | x > 0}$` (pipe inside math inside list item), verify
  no table is generated
- Parse `$x \ | | \ y$` through kramdown, verify output is inline text
- Parse a line with `|` outside math like `| A | B |\n|---|---|\n| 1 | 2 |`,
  verify it still produces a `<table>` (regression check)

### Unit: Underscore in math not triggering emphasis (Bug C)

- Parse `$\underbrace{x}_\text{label}$` through kramdown, verify output is
  a single text node (no `<em>` element)
- Parse `$a_{ij}$` through kramdown, verify no emphasis (subscript notation)
- Parse `_italic_ and $a_b$` (emphasis outside math + subscript inside),
  verify `<em>italic</em>` is produced but `$a_b$` is untouched
- Parse `$\hat{\beta}_0$` with Unicode, verify no emphasis

### Integration: mlwiki.org page rendering

- Build mlwiki.org with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify match count is >= 420/644
- Spot-check `index.php/Apriori.html`: `$C_1 = { \text{all itemsets...} }$`
  (braces not escaped)
- Spot-check `index.php/Alpha_Algorithm.html`: no `<table>` from pipe in math
- Spot-check `index.php/Arrow's_Impossibility_Theorem.html`:
  `$A = {x, y, z}$` (braces unescaped) and no `<em>` from `}_\text`

### Regression: Other sites

- Run `cargo test` full suite
- Run DOM comparison on DTC, muan-blog to verify no regression
- Verify all 13+ sites at 100% remain at 100%

## Output Verification

```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/alexeygrigorev/mlwiki.org/ \
  --destination /tmp/mlwiki_test

python3 scripts/dom_compare.py \
  --jekyll-dir websites/alexeygrigorev/mlwiki.org/_site_jekyll_cached \
  --rustkyll-dir /tmp/mlwiki_test
```

Spot-checks:
- `grep '\\\\{' /tmp/mlwiki_test/index.php/Apriori.html` -- must NOT show `\{` in math context
- `grep '<table>' /tmp/mlwiki_test/index.php/Alpha_Algorithm.html` -- must NOT show tables from math pipes
- Summary line must show >= 420 files matched (up from 311)

## Log

### [SWE] 2026-03-21

TDD Cycle:

1. Wrote 16 tests covering all 3 bugs (A: brace escaping in math, B: pipe/table in math, C: underscore/emphasis in math) plus regression tests for normal tables and emphasis outside math. Includes non-ASCII/Unicode tests with Greek letters.

2. Ran tests: 4 FAILED as expected
   - `test_math_brace_escape_display` FAILED -- `$$F = \{x \mid x > 0\}$$` output `\{` not unescaped in display math
   - `test_math_pipe_no_table` FAILED -- `$x | y$` produced `<table>` instead of inline text
   - `test_math_pipe_in_list` FAILED -- `- $F = {x | x > 0}$` produced `<table>`
   - `test_math_pipe_multiple` FAILED -- `$x \ | | \ y$` produced `<table>`
   - Bug C tests (underscore/emphasis) all PASSED already -- backslash escape handler consumes `\` before `_` in patterns like `\underbrace`

3. Implemented fixes:
   - **Bug A**: Added `unescape_kramdown_in_math()` in `span_parser.rs` that converts `\{` to `{` and `\}` to `}` in math content. Applied in both inline math (span_parser.rs) and display math block (html.rs). Only unescapes braces, not `\\` (which is LaTeX line break).
   - **Bug B**: Added `skip_inline_math_in_line()` helper in `parser.rs` that skips over `$...$` and `$$...$$` content. Applied to `is_table_line()` and `has_unescaped_pipe_ignoring_backticks()` so pipes inside math delimiters don't trigger table parsing.

4. Ran tests: ALL 16 new tests PASS, all 2492 existing tests PASS (0 failed), clippy clean, fmt clean.

Files modified:
- `src/kramdown_parser/span_parser.rs` -- added `unescape_kramdown_in_math()` (public), applied to inline math rendering
- `src/kramdown_parser/html.rs` -- applied `unescape_kramdown_in_math()` to math block rendering
- `src/kramdown_parser/parser.rs` -- added `skip_inline_math_in_line()`, updated `is_table_line()` and `has_unescaped_pipe_ignoring_backticks()` to skip math content
- `src/kramdown_parser/tests.rs` -- added 16 new tests for issue 304
