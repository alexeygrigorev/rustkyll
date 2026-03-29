# Issue 448: muan-blog border-box post structural diffs (34 diffs)

## Problem

`posts/border-box-in-github.html` has 34 DOM diffs. These stem from two root causes that cascade into many positional mismatches in the DOM comparison.

## Root Cause Analysis

### Root Cause A: `<br>` between blockquotes absorbed into preceding blockquote

The source markdown contains:

```markdown
> Wouldn't it require we rewrite every element with a width/border/padding?
<br>

> I'm pretty sure the internet would break in half if we added that rule in today.
```

**Jekyll** renders the `<br>` as a standalone element between the two blockquotes:
```html
<blockquote>
<p>Wouldn't it require we rewrite every element with a width/border/padding?</p>
</blockquote>
<br>
<blockquote>
<p>I'm pretty sure the internet would break in half...</p>
</blockquote>
```

**Rustkyll** absorbs the `<br>` into the first blockquote's `<p>` and adds a second spurious `<br>`:
```html
<blockquote>
<p>Wouldn't it require we rewrite every element with a width/border/padding?<br>
<br></p>
</blockquote>
<blockquote>
<p>I'm pretty sure the internet would break in half...</p>
</blockquote>
```

This shifts child element indices for everything after child[5] in the article, causing a cascade of ~30 DOM diffs (text_differs, tag_name_differs, expected_text_got_element, missing_text, etc.) as the comparator walks misaligned siblings.

### Root Cause B: `{% highlight %}` with `linenos` produces flat markup instead of table

The source uses `{% highlight erb linenos %}`. Jekyll renders this with a `<table class="rouge-table">` containing gutter and code columns. Rustkyll renders a flat `<pre><code>` without line number gutter and with different syntax token class names (e.g., `cm` vs `c`, `si` vs `cp`, `nb` vs `n`, `no` vs `ss`).

This is a known issue (highlight linenos table structure) and contributes additional diffs. However, most of the 34 diffs are from Root Cause A's cascade.

## Scope

Fix Root Cause A: the `<br>` tag appearing between blockquotes in markdown must be rendered as a standalone element between the blockquotes, not absorbed into the preceding blockquote.

Root Cause B (highlight linenos table structure and token class differences) is out of scope for this issue -- it should be tracked separately if not already covered by an existing issue.

## Dependencies

None.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] When markdown contains `> quote\n<br>\n\n> quote2`, the `<br>` renders as a standalone element between the two blockquotes, matching Jekyll behavior
- [ ] The rustkyll output for `posts/border-box-in-github.html` in muan-blog matches the Jekyll output for the blockquote section (the `<br>` is between the two `<blockquote>` elements, not inside the first one)
- [ ] No extra `<br>` elements appear inside blockquote paragraphs that are not in the source
- [ ] DTC DOM baseline: must not drop below 790/790
- [ ] muan-blog DOM baseline: must not drop below 2197/2218 (should improve by fixing this page)
- [ ] `cargo test` passes with all existing tests plus new tests

## Test Scenarios

### Unit: `<br>` between blockquotes

- Parse markdown with `> quote\n<br>\n\n> quote2`, verify the rendered HTML has `<br>` as a standalone element between two `<blockquote>` elements
- Parse markdown with `> quote\n<br>\n<br>\n\n> quote2` (multiple `<br>` tags), verify all render as standalone elements between blockquotes
- Parse markdown with `> quote\n\n> quote2` (no `<br>`), verify normal blockquote rendering is not affected

### Integration: muan-blog border-box page

- Build muan-blog site, compare `posts/border-box-in-github.html` blockquote section against Jekyll cached output
- Verify the DOM diff count for this page decreases (the cascade of ~30 diffs from the blockquote shift should be eliminated)

### Regression: existing sites

- Run DTC DOM comparison, verify 790/790
- Run muan-blog DOM comparison, verify no drop below 2197/2218

## Baseline

- DTC: 790/790
- muan-blog: 2197/2218

## Reference Files

- Source: `websites/muan-blog/_posts/2015-05-14-border-box-in-github.md`
- Jekyll output: `websites/muan-blog/_site_jekyll_cached/posts/border-box-in-github.html`
- Rustkyll output: `websites/muan-blog/_site/posts/border-box-in-github.html`
- DOM details: `docs/comparison/dom-details/muan-blog.txt`

## Log

### [SWE] 2026-03-29
- Wrote 3 tests first (TDD): test_issue448_br_between_blockquotes_standalone, test_issue448_multiple_br_between_blockquotes, test_issue448_no_br_blockquotes_unchanged
- Ran tests: 2 FAIL as expected (br inside blockquote, not between), 1 PASS (no-br case unaffected)
- Root cause: pulldown-cmark absorbs `<br>` on a line after `>` blockquote text into the blockquote's `<p>`, also generating a spurious hard-break `<br />`. Jekyll/kramdown treats the `<br>` line (without `>` prefix) as outside the blockquote.
- Implemented fix: added `extract_br_between_blockquotes()` postprocessing step in kramdown.rs. Detects trailing `<br />` tags at end of blockquote `<p>` when followed by another `<blockquote>`, extracts them as a single standalone `<br />` between the blockquotes.
- Also added helper `try_extract_trailing_br()` struct `ExtractedBr`.
- Ran tests: all 3 PASS
- Full test suite: 3074 passed, 0 failed (lib + integration)
- Clippy: clean (fixed strip_suffix lint)
- fmt: clean
- DOM baselines verified:
  - DTC: 790/790 (unchanged)
  - DTC docs: 57/57 (unchanged)
  - muan-blog: 2197/2218 (unchanged -- border-box page blockquote structure now matches Jekyll, but remaining diffs on that page are from Root Cause B: highlight linenos table structure, which is out of scope)
- Files modified: src/kramdown.rs
