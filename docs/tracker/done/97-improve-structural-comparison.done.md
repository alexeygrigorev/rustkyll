# Issue 97: Improve structural comparison to catch all HTML differences

## Problem

The structural comparison script (`scripts/compare-output.sh`) only extracts headings, links, and images for comparison using grep. It misses:
- `<p>&nbsp;</p>` spacer elements
- CSS classes on elements
- Inline styles
- Data attributes
- Form elements
- Script/noscript blocks
- Arbitrary HTML elements that aren't headings/links/images

Additionally, it only samples up to 50 files, so differences in the remaining files go undetected.

This means the "structural match" claim is incomplete -- many HTML differences go undetected.

## Goal

The structural comparison should detect ALL meaningful HTML differences, not just headings and links. Two HTML files are structurally equivalent when their normalized DOM trees are identical (same elements, attributes, text -- ignoring attribute ordering and insignificant whitespace).

## Approach

Replace the grep-based `extract_structural_elements()` function in `scripts/compare-output.sh` with a proper DOM tree comparison using a Python script:

1. Create `scripts/dom_compare.py` -- a Python script that:
   - Accepts two directory paths (Jekyll output and rustkyll output)
   - Finds all common `.html` files between the two directories
   - Parses each pair into DOM trees using BeautifulSoup (with `html.parser` or `lxml`)
   - Normalizes each tree: sort attributes alphabetically, collapse whitespace between tags, strip trailing whitespace from text nodes
   - Compares the full normalized DOM trees recursively
   - Reports every difference with element path context (e.g., `html > body > div.container > p[3]`)
   - Outputs per-file status (MATCH or DIFF with details)
   - Outputs summary (total files, matches, diffs, total difference count)
   - Exits nonzero when any file has DOM differences
2. Update `scripts/compare-output.sh` to call `scripts/dom_compare.py` instead of using `extract_structural_elements()` for the structural comparison phase
3. The shell script retains its role for: argument parsing, building sites, file tree comparison, and output validation. Only the structural element comparison is replaced.

## What "same DOM tree" means

Two HTML files have the same DOM tree when:
- Same elements in same order at same nesting depth
- Same attributes on each element (values match, order doesn't matter)
- Same text content in each text node (whitespace-normalized)
- Same number of child elements per parent
- `<p>&nbsp;</p>` in one but not the other = DIFFERENT
- `class="foo"` in one but not the other = DIFFERENT
- `target="_blank"` in one but not the other = DIFFERENT

What to ignore:
- Attribute ordering (`class="a" id="b"` == `id="b" class="a"`)
- Whitespace between tags (`<div> <span>` == `<div><span>`)
- Trailing whitespace in text nodes

## Dependencies

- None. This is a tooling improvement to the comparison scripts, independent of rustkyll engine features.

## Acceptance Criteria

### DOM comparison script (`scripts/dom_compare.py`)

- [ ] `scripts/dom_compare.py` exists and is executable
- [ ] Accepts `--jekyll-dir <path> --rustkyll-dir <path>` arguments
- [ ] Accepts an optional `--output <path>` argument to write a detailed JSON or text report
- [ ] Parses HTML files into DOM trees using BeautifulSoup (or lxml)
- [ ] Normalizes DOM trees before comparison:
  - [ ] Sorts attributes alphabetically on every element
  - [ ] Collapses whitespace-only text nodes between tags (removes them)
  - [ ] Strips leading/trailing whitespace from text nodes
  - [ ] Normalizes `&nbsp;` consistently (does NOT strip it -- `&nbsp;` is meaningful content)
- [ ] Compares ALL common HTML files between the two directories (no sampling limit)
- [ ] For each file with differences, reports:
  - [ ] The file path (relative to the output directory)
  - [ ] Each difference with element path context (e.g., `html > body > div > p[2]: text differs`)
  - [ ] The type of difference (missing element, extra element, attribute difference, text difference)
  - [ ] The expected value (Jekyll) and actual value (rustkyll) for each difference
- [ ] Prints a summary line: `Summary: X files matched, Y files with differences, Z total differences`
- [ ] Exits 0 when all files match, exits 1 when any file has differences

### Detection capabilities

- [ ] Detects `<p>&nbsp;</p>` in one file but not the other
- [ ] Detects CSS class differences on ANY element (not just headings)
- [ ] Detects missing or extra elements at any nesting depth
- [ ] Detects attribute differences: `target`, `rel`, `class`, `id`, `style`, `data-*`, `aria-*`, etc.
- [ ] Detects text content differences within elements
- [ ] Detects element tag name differences (e.g., `<div>` vs `<span>` at the same position)
- [ ] Correctly ignores attribute ordering
- [ ] Correctly ignores insignificant whitespace between tags

### Integration with compare-output.sh

- [ ] `scripts/compare-output.sh` calls `scripts/dom_compare.py` for structural comparison instead of the grep-based `extract_structural_elements()` function
- [ ] The `extract_structural_elements()` function is removed from `compare-output.sh`
- [ ] The shell script still handles: argument parsing, site building, file tree comparison, output validation, and summary
- [ ] The shell script passes through the exit code from `dom_compare.py` for the structural comparison verdict
- [ ] The existing `--site`, `--jekyll-dir`, `--rustkyll-dir`, `--validate-only`, `--threshold`, and `--min-files` flags continue to work

### Performance

- [ ] Comparing 787+ HTML files completes in under 60 seconds
- [ ] No excessive memory usage (should handle large HTML files without loading entire site into memory at once)

## Test Scenarios

### Unit: DOM normalization (`scripts/dom_compare.py` internal tests)

- Parse `<div class="a" id="b">` and `<div id="b" class="a">`, verify they normalize to the same result
- Parse `<div>  <span>text</span>  </div>` and `<div><span>text</span></div>`, verify they normalize to the same result
- Parse `<p>&nbsp;</p>` vs `<p></p>`, verify they are detected as DIFFERENT (nbsp is meaningful)
- Parse `<p>hello  world</p>` vs `<p>hello world</p>`, verify trailing/leading whitespace is stripped but internal spacing differences within text nodes are detected or normalized consistently
- Parse `<a href="/" target="_blank">` vs `<a href="/">`, verify the missing `target` attribute is detected

### Unit: DOM comparison

- Compare two identical simple HTML documents, verify 0 differences reported
- Compare two documents where one has an extra `<p>` element, verify it reports a missing/extra element difference
- Compare two documents where a `<div>` has `class="foo"` in one but not the other, verify class difference detected
- Compare two documents with different text in a deeply nested element, verify the difference is reported with the full element path
- Compare two documents where element order differs (e.g., `<p>` then `<ul>` vs `<ul>` then `<p>`), verify structural difference detected

### Integration: Full directory comparison

- Create two temp directories with 3 HTML files each (2 matching, 1 different), run `dom_compare.py`, verify exit code 1, verify summary shows "2 files matched, 1 files with differences"
- Create two temp directories with identical HTML files, run `dom_compare.py`, verify exit code 0
- Create two temp directories where one has an extra file not in the other, verify the script only compares common files and reports the count of common files compared

### Integration: compare-output.sh

- Verify `scripts/compare-output.sh --jekyll-dir <dir> --rustkyll-dir <dir>` invokes the DOM comparison and reports results
- Verify `--validate-only` mode still works (does not require DOM comparison since there is no Jekyll output to compare against)

### Regression: Known difference detection

- Create an HTML file pair where the only difference is `<p>&nbsp;</p>` (spacer element present in one, absent in the other) -- verify detected
- Create an HTML file pair where the only difference is `<script>` content -- verify detected
- Create an HTML file pair where the only difference is a `data-*` attribute -- verify detected

## Log

### [SWE] 2026-03-15

- Created `scripts/dom_compare.py` -- full DOM tree comparison using BeautifulSoup
  - Parses HTML with `html.parser`, normalizes trees (sort attrs, collapse whitespace, strip text, preserve nbsp)
  - Recursive tree comparison detecting: missing/extra elements, attribute diffs, text diffs, tag name diffs
  - CLI: `--jekyll-dir`, `--rustkyll-dir`, `--output` arguments
  - Progress reporting to stderr, summary to stdout
  - Exits 0 on all match, 1 on differences, 2 on invalid input
- Created `scripts/test_dom_compare.py` -- 30 unit/integration/CLI tests
  - Tests: normalization (attr order, whitespace, nbsp preservation), comparison (identical, extra/missing elements, class diffs, nested text, element order, tag names), directory comparison (matching/differing/only-common), CLI (exit codes, output file), regression (nbsp spacer, script content, data attributes)
- Updated `scripts/compare-output.sh`:
  - Removed `extract_structural_elements()` function
  - Replaced grep-based structural comparison with call to `dom_compare.py`
  - Updated summary to use DOM comparison result
  - All existing flags (--site, --jekyll-dir, --rustkyll-dir, --validate-only, --threshold, --min-files) still work
- All 30 Python tests pass
- Performance: 787 DTC HTML files compared in 21 seconds (well under 60s requirement)
- DTC site results: 250 files matched, 537 files with differences, 8423 total differences
- Files created: `scripts/dom_compare.py`, `scripts/test_dom_compare.py`
- Files modified: `scripts/compare-output.sh`
