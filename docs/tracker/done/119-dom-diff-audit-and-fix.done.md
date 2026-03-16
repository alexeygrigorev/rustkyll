# Issue 119: DOM diff audit — categorize and fix ALL structural differences

## Priority

HIGH — 785/787 DTC files have DOM differences. We need to systematically eliminate them.

## Problem

The DOM comparison tool (scripts/dom_compare.py) shows 8000+ differences across 787 DTC HTML files. Only 2 files have zero DOM diffs. These differences are NOT minor — they represent structural mismatches between rustkyll and Jekyll output.

## Goal

1. Run dom_compare.py on all 787 DTC files
2. Capture the FULL output to a report file
3. Categorize every difference type
4. Create a separate tracked issue for each category
5. Fix them systematically

## Deliverables

### 1. Full DOM diff report

Run: `python scripts/dom_compare.py --jekyll-dir /path/to/jekyll/_site --rustkyll-dir /path/to/rustkyll/_site --output docs/comparison/dom-diff-full-report.txt`

This file should contain every single difference found. No sampling, no summarizing.

### 2. Categorized difference summary

Create `docs/comparison/dom-diff-categories.md` with a table:

| Category | Count | Example | Issue # | Status |
|----------|-------|---------|---------|--------|
| Entity encoding (&amp; vs &) | N | `<a href="?a=1&b=2">` vs `<a href="?a=1&amp;b=2">` | #102 | partial |
| Missing SEO meta tags | N | `<meta property="og:title">` | #NNN | todo |
| Extra <p> wrapping | N | `<li><p>text</p></li>` vs `<li>text</li>` | #92 | done |
| ... | | | | |

Each row = one type of difference, with count, example, and linked issue.

### 3. One issue per category

For each NEW category found, create a .todo.md issue in docs/tracker/.

## Approach

1. Build DTC site with both Jekyll and rustkyll
2. Run dom_compare.py with full output
3. Parse the output to extract difference types
4. Group by category (entity encoding, missing elements, attribute diffs, text diffs, etc.)
5. Count occurrences per category
6. Create the summary table
7. Create issues for unfixed categories

## Fix process for each category

For each difference category:
1. Write a failing test that demonstrates the difference (test must FAIL before fix)
2. Implement the fix
3. Test goes GREEN
4. Re-run DOM comparison to verify the category count decreased
5. Document the fix

This ensures every fix is test-driven and verified.

## Acceptance criteria

- Full DOM diff report saved (every difference, every file)
- Categorized summary with counts and examples
- One tracked issue per category
- Categories linked to existing issues where already fixed
- No categories left without a tracked issue
- Each fix has a test that fails before and passes after
- Results reproducible (scripts/commands documented)

## Log

### [SWE] 2026-03-16

- Built both Jekyll and rustkyll sites (787 HTML files each)
- Created `scripts/dom_compare_full.py` -- variant of dom_compare.py that shows ALL diffs (no 10-per-file limit)
- Ran full DOM comparison: **323 files matched, 464 files with differences, 4472 total differences**
  - (Down from the original 8000+ estimate -- previous issues fixed many diffs)
- Created analysis scripts to categorize diffs:
  - `scripts/categorize_diffs.py` -- raw diff type categorization
  - `scripts/analyze_jsonld_full.py` -- deep JSON-LD subcategorization
  - `scripts/analyze_attr_diffs.py` -- attribute diff analysis
  - `scripts/analyze_text_diffs.py` -- text content diff analysis
  - `scripts/analyze_cascade_text.py` -- structural vs text diff correlation

**Deliverables completed:**
1. Full report: `docs/comparison/dom-diff-full-report.txt` (4942 lines, every diff, every file)
2. Category summary: `docs/comparison/dom-diff-categories.md` (22 categories with counts, examples, issue links)
3. New issues created: #137 through #148 (12 new issues)

**Category summary:**
- 22 categories identified
- 4 linked to existing issues (#102, #109, #113, #124)
- 12 new issues created (#137-#148)
- 2 categories are not bugs (stale content build, cascade effects from structural diffs)

**New issues created:**
- #137: JSON-LD trailing newline in description (~211 files)
- #138: JSON-LD podcast date format (~386 date fields)
- #139: Extra JSON-LD script tags (~199 instances)
- #140: Book listing end-date off-by-one (78 entries)
- #141: Heading ID generation (19 instances)
- #142: JSON-LD keyword types and null dates (7 instances)
- #143: URL percent-encoding for spaces (4 instances)
- #144: Accordion script placement (9 instances)
- #145: Extra class on inline code (9 instances)
- #146: Ordered list start attribute (33 instances)
- #147: Extra target='_blank' (3 instances)
- #148: Misc markdown rendering edge cases (~50 instances)

**Estimated fix impact:** Top 5 categories cover ~81% of all diffs (3624 of 4472)

**Files created/modified:**
- docs/comparison/dom-diff-full-report.txt (new)
- docs/comparison/dom-diff-full-report-categories.json (new)
- docs/comparison/dom-diff-categories.md (new)
- scripts/dom_compare_full.py (new)
- scripts/categorize_diffs.py (new)
- scripts/analyze_jsonld_full.py (new)
- scripts/analyze_jsonld_raw.py (new)
- scripts/analyze_jsonld_detailed.py (new)
- scripts/analyze_attr_diffs.py (new)
- scripts/analyze_text_diffs.py (new)
- scripts/analyze_cascade_text.py (new)
- scripts/sample_jsonld_diff.py (new)
- scripts/analyze_heading_ids.py (new)
- scripts/analyze_string_diffs.py (new)
- docs/tracker/137-fix-jsonld-trailing-newline.todo.md (new)
- docs/tracker/138-fix-jsonld-podcast-date-format.todo.md (new)
- docs/tracker/139-fix-extra-jsonld-script-tags.todo.md (new)
- docs/tracker/140-fix-book-listing-end-date-off-by-one.todo.md (new)
- docs/tracker/141-fix-heading-id-generation.todo.md (new)
- docs/tracker/142-fix-jsonld-keyword-types-and-null-dates.todo.md (new)
- docs/tracker/143-fix-url-percent-encoding.todo.md (new)
- docs/tracker/144-fix-accordion-script-placement.todo.md (new)
- docs/tracker/145-fix-extra-class-on-inline-code.todo.md (new)
- docs/tracker/146-fix-ordered-list-start-attribute.todo.md (new)
- docs/tracker/147-fix-extra-target-blank.todo.md (new)
- docs/tracker/148-fix-misc-markdown-rendering-edge-cases.todo.md (new)
