# Issue 309: mlwiki DOM comparison -- filter Jekyll kramdown math bugs as acceptable diffs

## Problem

mlwiki.org matches 331/644 (51%). Of the 313 diff pages, 176 are caused by
three related Jekyll kramdown bugs where inline math content (`$...$`) is
incorrectly processed by kramdown's block/span parsers. Rustkyll correctly
preserves math content via `protect_math_content()`. These diffs represent
Jekyll bugs, not rustkyll bugs.

### Bug 1: Pipe-in-math triggers table parsing -- 131 pages

Source: `$x \ | | \ y$`
Jekyll (buggy): `<table><tbody><tr><td>x \</td><td> </td><td>\ y</td></tr></tbody></table>`
Rustkyll (correct): `$x \ | | \ y$` (inline math text)

Jekyll's kramdown block parser sees `|` on a line and creates a table, even
inside `$...$` math delimiters. Rustkyll protects math content before
markdown processing.

### Bug 2: Underscore-in-math triggers emphasis -- 23 pages

Source: `$\underbrace{[x P y]}_\text{(1)}$`
Jekyll (buggy): `$\underbrace{[x P y]}<em>\text{(1)}</em>$` (emphasis from `}_`)
Rustkyll (correct): `$\underbrace{[x P y]}_\text{(1)}$` (preserved math text)

Jekyll's kramdown span parser treats `_` after `}` as an emphasis marker,
even inside math delimiters.

### Bug 3: Double-backslash in math converts to `<br>` -- 22 pages

Source: `$\begin{bmatrix} 1 & 2 \\ 3 & 4 \end{bmatrix}$`
Jekyll (buggy): `$\begin{bmatrix} 1 & 2` + `<br />` + `3 & 4 \end{bmatrix}$`
Rustkyll (correct): `$\begin{bmatrix} 1 & 2 \\ 3 & 4 \end{bmatrix}$` (preserved)

Jekyll's kramdown converts `\\` to `<br />` even inside math environments.
MathJax needs `\\` for matrix row breaks.

### Why filtering is correct

1. All three bugs are documented Jekyll kramdown issues with math content.
2. Rustkyll's behavior is what users expect -- math content should be passed
   through to MathJax/KaTeX without kramdown interference.
3. Replicating these bugs would break math rendering for end users.
4. This follows the same pattern as issue 307 (filtering DTC build timestamps).

### Scale

176 of 313 diff pages (56%). Filtering these as acceptable would bring
mlwiki from 331/644 (51%) to approximately 507/644 (79%).

## Scope

### In scope

1. **Add Jekyll math bug detection to `dom_compare.py`** -- three filters:

   a. **Pipe-in-math table filter**: When Jekyll has `<table>` inside a
      `<li>` or `<p>` and rustkyll has text containing `$...$` with `|`,
      filter the table-related diffs as acceptable.

   b. **Emphasis-in-math filter**: When Jekyll has `<em>` elements adjacent
      to or inside math content (`$` or `\` markers), and rustkyll has no
      corresponding `<em>` (the math text is preserved), filter the
      emphasis-related diffs as acceptable.

   c. **Br-in-math filter**: When Jekyll has `<br>` elements inside text
      that contains math delimiters or LaTeX commands (`\begin`, `\end`,
      `bmatrix`, `cfrac`, etc.), and rustkyll has the same content without
      `<br>`, filter the br-related diffs as acceptable.

2. **Count filtered diffs in the "acceptable diffs filtered out" total.**

3. **Per-page filtering**: A page with both math-bug diffs and real diffs
   should filter only the math-bug diffs. If ALL diffs on a page are
   math-bug diffs, the page counts as matched.

### Out of scope

- Changing rustkyll's rendering to match Jekyll's buggy behavior
- Fixing other mlwiki diff categories (rouge tokens, ellipsis, markdown
  parsing differences)
- Changes to any Rust code -- this is a Python-only DOM comparison tool fix

## Dependencies

- Issue 307 (DTC dom_compare fixes) -- DONE. This issue builds on that pattern.

## Key Files to Modify

- `scripts/dom_compare.py` -- add `is_acceptable_jekyll_math_bug_diff()`
  function (or separate functions for each bug type) and integrate into the
  filtering pipeline after `compare_trees()` returns diffs
- `scripts/test_dom_compare.py` -- add tests for each filter

## Acceptance Criteria

- [ ] Running `dom_compare.py` on mlwiki produces 500+/644 matched files
      (up from 331)
- [ ] Pipe-in-math table diffs are filtered as acceptable (131 pages)
- [ ] Emphasis-in-math diffs are filtered as acceptable (23 pages)
- [ ] Br-in-math diffs are filtered as acceptable (22 pages)
- [ ] Filtered diffs are counted in the "acceptable diffs filtered out" total
- [ ] Pages with REAL table diffs (where both sides have tables but differ)
      are NOT filtered
- [ ] Pages with REAL emphasis diffs (emphasis outside math context) are NOT
      filtered
- [ ] Pages with REAL `<br>` diffs (line breaks outside math context) are NOT
      filtered
- [ ] No change in match counts for DTC (681/790), muan-blog (2174/2218),
      choosealicense (17/72), or any other site
- [ ] `python3 -m pytest scripts/test_dom_compare.py` passes
- [ ] Tests include non-ASCII/Unicode content (Greek letters in math)

## Test Scenarios

### Unit: Pipe-in-math table filter

- Jekyll `<li><table><tbody><tr><td>x \</td><td> </td><td>\ y</td></tr>
  </tbody></table></li>` vs rustkyll `<li>$x \ | | \ y$</li>` -- FILTERED
- Both sides have `<table>` with real content (names, values) -- NOT filtered
- Jekyll has table from `|` but no math context (`$`) in rustkyll text --
  NOT filtered
- Unicode math: `$\alpha \ | | \ \beta$` vs table -- FILTERED
- Multiple math-pipe tables on same page -- all FILTERED

### Unit: Emphasis-in-math filter

- Jekyll `$\underbrace{x}<em>\text{label}</em>$` vs rustkyll
  `$\underbrace{x}_\text{label}$` -- FILTERED
- Real emphasis difference outside math (`<em>word</em>` vs plain text) --
  NOT filtered
- Mixed page: emphasis-in-math + real emphasis diff -- only math one FILTERED

### Unit: Br-in-math filter

- Jekyll text with `<br>` inside bmatrix content vs rustkyll text with `\\`
  preserved -- FILTERED
- Jekyll `<br>` after `<p>` text (normal line break) -- NOT filtered
- Page with `<br>` inside cfrac/matrix LaTeX command context -- FILTERED
- Unicode text with `<br>`: `$\alpha \\ \beta$` -- FILTERED

### Integration: mlwiki full comparison

- Run `python3 scripts/dom_compare.py` on mlwiki
- Verify matched count is 500+/644 (up from 331)
- Spot-check pages:
  - `index.php/Alpha_Algorithm.html` -- matched (pipe-in-math filtered)
  - `index.php/Arrow's_Impossibility_Theorem.html` -- matched (emphasis filtered)
  - `index.php/Basis_(Linear_Algebra).html` -- matched (br-in-math filtered)
  - `index.php/ANTLR4_Maven.html` -- still diff (rouge class diffs, not math)

### Regression

- Run dom_compare.py on DTC -- verify 681/790 unchanged
- Run dom_compare.py on muan-blog -- verify 2174/2218 unchanged
- Run dom_compare.py on choosealicense -- verify 17/72 unchanged
- Run `python3 -m pytest scripts/test_dom_compare.py` -- all tests pass

## Output Verification

```bash
python3 scripts/dom_compare.py \
  --jekyll-dir websites/alexeygrigorev/mlwiki.org/_site_jekyll_cached \
  --rustkyll-dir websites/alexeygrigorev/mlwiki.org/_site_rustkyll \
  --output /tmp/mlwiki_after_309.txt

# Summary line must show >= 500 files matched (up from 331)
# Acceptable diffs filtered count should increase by ~176

# Spot-checks
grep "Alpha_Algorithm" /tmp/mlwiki_after_309.txt
# Should NOT appear as DIFF (pipe-in-math filtered)

grep "Arrow's_Impossibility" /tmp/mlwiki_after_309.txt
# Should NOT appear as DIFF (emphasis-in-math filtered)

grep "Basis_(Linear_Algebra)" /tmp/mlwiki_after_309.txt
# Should NOT appear as DIFF (br-in-math filtered)

# Regression check
python3 scripts/dom_compare.py \
  --jekyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_jekyll_cached \
  --rustkyll-dir websites/DataTalksClub/datatalksclub.github.io/_site_rustkyll
# Must still show 681+ matched
```
