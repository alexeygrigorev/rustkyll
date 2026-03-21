# Issue 275: Kramdown inline emphasis parsing -- adjacent bold, emphasis+link interaction

## Problem

The kramdown span parser incorrectly resolves emphasis markers when:

1. **Adjacent bold markers** separated by non-emphasis text:
   `**text1**" or "**text2**` produces `<strong>text1<strong>" or "</strong>text2</strong>`
   instead of `<strong>text1</strong>" or "<strong>text2</strong>`.

2. **Bold followed by italic+link in parenthetical context**:
   `**Apache Airflow** locally (see [*What means...*](url))` misorders the
   `<strong>` and `<a>` elements.

3. **Italic emphasis wrapping links**:
   `*[Link Text](url), text* ... *[Link Text](url), text*` produces wrong
   element nesting where `<em>` and `<a>` boundaries don't match Jekyll.

## Affected Pages

### DTC body content diffs directly caused by emphasis parsing:

1. **blog/data-engineers-arent-plumbers.html** (7 diffs)
   - Pattern: `"**What is a data engineer?**" or "**The difference...**"`
   - The closing `**` of the first bold and opening `**` of the second are
     being mismatched, producing `<strong>...<strong>" or "</strong>...</strong>`
   - Source line 20

2. **blog/how-to-setup-lightweight-local-version-for-airflow.html** (453 diffs)
   - Pattern: `**Apache Airflow** locally (see [*text*](url){:target="_blank"}) with **Docker** and **Docker Compose**`
   - The `<strong>` and `<a>` elements are being swapped/misordered, causing
     a cascading diff through the entire page
   - Source line 22

3. **blog/interview-with-valerii-chetvertakov.html** (8 emphasis-related diffs)
   - Pattern: `*[EV Connect, Inc.](url){:target="_blank"}, text* ... *[Schneider Electric](url){:target="_blank"}, text*`
   - Italic emphasis wrapping links with IALs produces wrong nesting
   - Source line 80

### Total diff reduction: ~468 diffs across 3 pages

The airflow page alone accounts for 453 diffs, making this a high-impact fix
despite affecting few pages.

## Root Cause Analysis

The kramdown span parser in `src/kramdown_parser/span_parser.rs` resolves
emphasis markers (`*`, `**`, `***`) by scanning for matching open/close pairs.
The issue is in how it determines which `**` is a closer vs opener when
multiple bold spans appear in the same paragraph separated by non-emphasis
content.

Specifically, the pattern `**A**B**C**` should produce
`<strong>A</strong>B<strong>C</strong>` (two separate bold spans), but the
parser is treating the second `**` as an opener inside the first span,
producing `<strong>A<strong>B</strong>C</strong>` (nested bold).

For the emphasis+link interaction, the issue is likely in how IAL
`{:target="_blank"}` interacts with emphasis boundary detection -- the `"`
characters inside the IAL may be confusing the emphasis resolver.

## Key Files to Modify

- `src/kramdown_parser/span_parser.rs` -- emphasis marker resolution logic
  (the core fix)
- `src/kramdown.rs` -- tests for the new behavior

## Dependencies

- None. The kramdown span parser is independent of other in-progress work.
- Issue 296 (DTC remaining diffs, in-progress) explicitly defers these pages
  to this issue.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] Input `"**What is a data engineer?**" or "**The difference**"` produces
      `"<strong>What is a data engineer?</strong>" or "<strong>The difference</strong>"`
      (two separate `<strong>` elements, no nesting)
- [ ] Input `**Apache Airflow** locally (see [*text*](url))` produces
      `<strong>Apache Airflow</strong> locally (see <a href="url"><em>text</em></a>)`
      with correct element ordering
- [ ] Input `*[Link](url), text*` produces `<em><a href="url">Link</a>, text</em>`
      (italic wrapping link correctly)
- [ ] DTC DOM comparison: `data-engineers-arent-plumbers.html` emphasis diffs
      resolved (body content matches Jekyll)
- [ ] DTC DOM comparison: `how-to-setup-lightweight-local-version-for-airflow.html`
      diff count reduced significantly (from 453 toward 0)
- [ ] DTC DOM comparison: `interview-with-valerii-chetvertakov.html` emphasis
      diffs resolved
- [ ] No regressions on any of the 13+ sites currently at 100%
- [ ] No regressions on DTC, muan-blog, choosealicense, or lanyon match counts
- [ ] Tests include non-ASCII/Unicode content (curly quotes, em-dashes)

## Test Scenarios

### Unit: Adjacent bold markers

- Parse `"**text1**" or "**text2**"` through kramdown, verify output contains
  two separate `<strong>` elements with no nesting
- Parse `**A** and **B** and **C**` through kramdown, verify three separate
  `<strong>` elements
- Parse `**bold** *italic* **bold**` through kramdown, verify correct
  alternating `<strong>` and `<em>` elements
- Parse `"**What is a data engineer?**" or "**The difference between data engineer and data scientist**" we get a cliche answer: *Data engineers are like plumbers.*`
  through kramdown, verify output matches:
  `"<strong>What is a data engineer?</strong>" or "<strong>The difference between data engineer and data scientist</strong>" we get a cliche answer: <em>Data engineers are like plumbers.</em>`

### Unit: Emphasis + link interaction

- Parse `**Apache Airflow** locally (see [*What means "to run one software locally"*](https://example.com){:target="_blank"}) with **Docker** and **Docker Compose**`
  through kramdown, verify:
  - `<strong>Apache Airflow</strong>` comes first
  - `<a href="..." target="_blank"><em>What means...</em></a>` follows
  - `<strong>Docker</strong>` and `<strong>Docker Compose</strong>` are
    separate bold elements
- Parse `*[Link Text](url){:target="_blank"}, additional text*` through
  kramdown, verify `<em>` wraps the `<a>` and trailing text

### Unit: Unicode emphasis content

- Parse `**"Ziel"** und **"Ergebnis"**` (German quotes) through kramdown,
  verify two separate `<strong>` elements
- Parse `**"emphasis"** or **"emphasis"**` with curly quotes through kramdown,
  verify correct parsing

### Integration: DTC page rendering

- Build DTC site with rustkyll
- Compare `blog/data-engineers-arent-plumbers.html` against Jekyll cached:
  verify the paragraph about "What is a data engineer?" matches exactly
- Compare `blog/how-to-setup-lightweight-local-version-for-airflow.html`:
  verify diff count drops from 453 to near 0
- Compare `blog/interview-with-valerii-chetvertakov.html`: verify emphasis
  diffs resolved

### Regression: Existing emphasis tests

- All existing kramdown conformance tests continue to pass
- All existing DTC, muan-blog, and other site comparisons maintain or improve
  their match counts

## Output Verification

Build and inspect:
```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/DataTalksClub/datatalksclub.github.io/ \
  --destination /tmp/dtc_test
```

Check specific pages:
```bash
# data-engineers-arent-plumbers: look for correct <strong> nesting
grep 'What is a data engineer' /tmp/dtc_test/blog/data-engineers-arent-plumbers.html
# Should show: <strong>What is a data engineer?</strong>" or "<strong>The difference

# airflow: look for correct strong/a ordering
grep 'Apache Airflow' /tmp/dtc_test/blog/how-to-setup-lightweight-local-version-for-airflow.html
# Should show: <strong>Apache Airflow</strong> locally (see <a ...

python3 scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir /tmp/dtc_test
```

## Log

### [SWE] 2026-03-21
- TDD cycle:
  - Wrote 9 tests first: adjacent bold markers (5 tests), bold+italic+link IAL interaction (2 tests), italic wrapping links with IAL (1 test), bold/italic alternating (1 test)
  - Ran tests: 7 passed, 2 FAILED as expected (test_bold_then_italic_link_interaction, test_italic_wrapping_link_with_ial)
  - Root cause: IAL `{:target="_blank"}` after inline links `[text](url){:attrs}` was not being parsed inside emphasis spans. The `parse_spans_until_emphasis_close` function had no IAL handling after links. The main `parse_spans` function also lacked it.
  - Implemented fix: Added IAL parsing after links in both `parse_spans` (line ~1286) and `parse_spans_until_emphasis_close` (line ~2870), plus a new `apply_ial_to_a_tag` helper function
  - Ran tests: all 9 new tests PASS, all existing emphasis/link tests PASS
- Adjacent bold markers (pattern 1) already worked correctly -- no code change needed
- Build: 2451 tests pass (2442 baseline + 9 new), 0 fail, clippy clean, fmt clean
- Files modified: src/kramdown_parser/span_parser.rs, src/kramdown_parser/tests.rs
