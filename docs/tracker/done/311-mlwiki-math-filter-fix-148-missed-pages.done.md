# Issue 311: Fix mlwiki math bug DOM filter -- catching 28 of 176 expected pages

## Problem

Issue 309 added three math bug filters to `dom_compare.py` to detect cases
where Jekyll's kramdown incorrectly processes content inside `$...$` math
delimiters (pipe triggering tables, underscore triggering emphasis, `\\`
converting to `<br>`). The spec predicted these filters would catch 176 pages,
bringing mlwiki from ~331/644 (51%) to ~507/644 (79%).

**Actual result:** Only 28 pages are filtered. 148 pages that should be
caught are being missed. mlwiki currently sits at 359/644 (56%).

### Why the filters miss 148 pages

The filters work on individual `DiffResult` objects, checking `diff_type`,
`expected`, and `actual` fields. But the DOM comparator produces cascading
diffs when structural mismatches occur. For example, when Jekyll creates a
`<table>` from a `$x | y$` expression:

1. The first diff might be `tag_name_differs` with expected `td` and actual
   `p` -- no math context visible in either field
2. Subsequent diffs cascade as element alignment shifts: `missing_element`
   for `<tr>`, `<td>`, etc.
3. The `text_differs` diff that has actual math content may appear several
   diffs later, disconnected from the structural diffs

The filter checks each diff independently and cannot see that a cluster of
diffs originates from a single math-pipe-table bug.

### Specific filter gaps

**Pipe-in-math table filter (`is_acceptable_jekyll_math_pipe_table_diff`):**
- Line 243: `tag_name_differs` where expected is a table tag returns True
  unconditionally -- this is overly aggressive for non-math table diffs but
  may still miss cases where the expected tag is NOT a table tag (e.g.,
  `expected='p', actual='td'` when alignment shifts)
- The `_text_contains_math_with_pipe()` helper requires `$...|...$` in the
  `actual` field, but many cascading diffs have tag names or text fragments
  in `actual`, not the full math expression
- Pages where the pipe-in-math causes a `<table>` that shifts ALL subsequent
  elements produce dozens of `tag_name_differs` diffs where neither expected
  nor actual is a table tag (e.g., `expected='h2', actual='p'`)

**Emphasis-in-math filter (`is_acceptable_jekyll_math_emphasis_diff`):**
- The `missing_element` check for `<em>` at line 275 returns True for ALL
  missing `<em>` elements, not just those in math context. This may
  incorrectly filter real emphasis diffs. But the more common miss is that
  the cascading text_differs and tag_name_differs diffs around the `<em>`
  are not caught.

**Br-in-math filter (`is_acceptable_jekyll_math_br_diff`):**
- The `missing_element` check for `<br>` at line 322 returns True for ALL
  missing `<br>` elements -- but issue 308 (book comment rendering) has
  real missing `<br>` diffs that should NOT be filtered.
- The `text_differs` check requires `\\\\` in `actual`, but in many diffs
  the actual text is the rustkyll output which already processed the `\\`
  and it may not contain literal double-backslash.

### Proposed fix approach

Instead of filtering individual diffs, implement **page-level context
analysis**:

1. Before filtering individual diffs, examine the full set of diffs for a
   page and the actual HTML content of both files
2. For pipe-in-math: if Jekyll HTML contains `<table>` inside a `<li>` or
   `<p>` context AND rustkyll HTML contains `$...|...$` text in the same
   parent element, mark ALL diffs descended from that parent as acceptable
3. For emphasis-in-math: if Jekyll HTML contains `<em>` adjacent to `$` in
   text AND rustkyll HTML has continuous math text, mark related diffs
4. For br-in-math: if Jekyll HTML has `<br>` inside text containing LaTeX
   commands AND rustkyll has `\\` in the same position, mark related diffs
5. After marking math-bug diffs, any remaining diffs on the page are real

This requires the filter to have access to the source HTML trees (or at
least the rendered HTML files), not just the diff results.

## Scope

### In scope

1. **Refactor math bug filters to use page-level context** -- pass the
   Jekyll and rustkyll HTML trees (or file paths) to the filter function
   so it can inspect the actual content around diff locations

2. **Implement parent-element-based matching** -- when a diff occurs in a
   region where Jekyll has a `<table>` from math pipes, mark the entire
   cluster of diffs in that region as acceptable

3. **Add test cases for the missed patterns** -- extract real examples from
   the 148 missed pages and add them as unit tests

4. **Ensure non-math table/emphasis/br diffs are NOT filtered** -- add
   negative test cases for real diffs that should remain

### Out of scope

- Changing rustkyll's rendering (this is a comparison tool fix only)
- Fixing other mlwiki diff categories (rouge tokens, ellipsis, etc.)
- Changes to any Rust code

## Dependencies

- Issue 309 (math bug filter implementation) -- IN PROGRESS. This issue
  supersedes and fixes the filters added by 309.

## Key Files to Modify

- `scripts/dom_compare.py` -- refactor `is_acceptable_jekyll_math_bug_diff()`
  to accept page context; modify `filter_acceptable_diffs()` to pass context;
  add `_analyze_page_math_bug_context()` helper
- `scripts/test_dom_compare.py` -- add tests for missed filter patterns

## Acceptance Criteria

- [ ] Running `dom_compare.py` on mlwiki produces 500+/644 matched files
      (up from current 359)
- [ ] At least 150 pages are filtered as acceptable math bug diffs (up from
      current 28)
- [ ] The filter correctly identifies pipe-in-math table diffs on pages where
      the DOM cascade produces 10+ diffs from a single math expression
- [ ] The filter correctly identifies emphasis-in-math diffs where `<em>`
      appears adjacent to math delimiters
- [ ] The filter correctly identifies br-in-math diffs where `<br>` appears
      in LaTeX command context
- [ ] Real table diffs (where both sides have tables with different content)
      are NOT filtered
- [ ] Real emphasis diffs (emphasis outside math context) are NOT filtered
- [ ] Real `<br>` diffs (line breaks outside math/LaTeX context, such as
      book comment `newline_to_br` diffs from issue 308) are NOT filtered
- [ ] No change in match counts for DTC (681/790), muan-blog (2174/2218),
      choosealicense, or any other site
- [ ] `python3 -m pytest scripts/test_dom_compare.py` passes with new tests
- [ ] Tests include non-ASCII/Unicode content (Greek letters, CJK in math)

## Test Scenarios

### Unit: Pipe-in-math cascade detection

- Jekyll page with `<table><tbody><tr><td>x \</td><td>...</td></tr></tbody>
  </table>` inside a `<li>`, rustkyll page with `<li>$x \ | | \ y$</li>` --
  ALL cascading diffs (tag_name_differs for shifted elements, missing_element
  for table sub-elements) should be FILTERED
- Jekyll page with a real data table (`| Name | Value |`) -- diffs should
  NOT be filtered even if there is also math content on the page
- Page with BOTH a math-pipe table AND a real diff elsewhere -- math-pipe
  diffs FILTERED, real diff REMAINS

### Unit: Emphasis-in-math cascade detection

- Jekyll page with `$\underbrace{x}<em>\text{label}</em>$`, rustkyll with
  `$\underbrace{x}_\text{label}$` -- the `<em>` diff plus the surrounding
  text_differs and missing_text diffs should all be FILTERED
- Real `<em>` diff outside math on the same page -- should NOT be filtered
- Page with Unicode math content: `$\alpha_{\beta}$` with emphasis bug --
  FILTERED

### Unit: Br-in-math cascade detection

- Jekyll page with `<br>` inside bmatrix content, rustkyll with `\\`
  preserved -- FILTERED
- Jekyll page with `<br>` from `newline_to_br` in a book comment (no math
  context) -- NOT filtered
- Page with both math `<br>` diffs and real `<br>` diffs -- only math ones
  FILTERED

### Integration: mlwiki full comparison

- Run `python3 scripts/dom_compare.py` on mlwiki
- Verify matched count is 500+/644
- Verify acceptable diffs filtered count includes 150+ math bug diffs
- Spot-check pages:
  - `index.php/Alpha_Algorithm.html` -- matched (pipe-in-math, multi-cascade)
  - `index.php/Arrow's_Impossibility_Theorem.html` -- matched (emphasis)
  - `index.php/Basis_(Linear_Algebra).html` -- matched (br-in-math)
  - A page with real rouge class diffs -- still shows as DIFF

### Regression

- Run dom_compare.py on DTC -- verify 681/790 unchanged
- Run dom_compare.py on muan-blog -- verify 2174/2218 unchanged
- `python3 -m pytest scripts/test_dom_compare.py` -- all tests pass

## Output Verification

```bash
python3 scripts/dom_compare.py \
  --jekyll-dir websites/alexeygrigorev/mlwiki.org/_site_jekyll_cached \
  --rustkyll-dir websites/alexeygrigorev/mlwiki.org/_site_rustkyll \
  --output /tmp/mlwiki_after_311.txt

# Summary line must show >= 500 files matched
# Acceptable diffs filtered count should be >= 150

# Spot-checks
grep "Alpha_Algorithm" /tmp/mlwiki_after_311.txt
# Should NOT appear as DIFF

# Regression
python3 scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_rustkyll
# Must still show 681+ matched
```

## Log

### [SWE] 2026-03-23
- Wrote 11 page-level math filter tests in TestPageLevelMathBugFilter class
- Ran tests: FAIL as expected (TypeError: filter_acceptable_diffs() got unexpected keyword argument)
- Implemented page-level math bug filter:
  - Added `_is_latex_math_content()` to distinguish real LaTeX from dollar amounts in prose
  - Added `_page_has_math_with_pipe/underscore/double_backslash()` with strict LaTeX content checks
  - Added `_filter_page_level_math_diffs()` for page-level cascade filtering
  - Modified `filter_acceptable_diffs()` to accept optional rustkyll_html/jekyll_html params
  - Added `compare_html_files_with_context()` to return raw HTML alongside diffs
  - Modified `compare_directories()` to pass HTML context to filter
  - Tightened per-diff filters: missing_element for `<br>`, `<em>`, table tags now require math context in actual field
- Ran tests: 116 passed, 0 failed
- mlwiki: 552/644 matched (up from 359 baseline, target was 500+)
- DTC: 740/790 matched (baseline 742; -2 is correct -- book comment `<br>` diffs properly not filtered)
- Files modified: scripts/dom_compare.py, scripts/test_dom_compare.py
