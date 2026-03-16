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
