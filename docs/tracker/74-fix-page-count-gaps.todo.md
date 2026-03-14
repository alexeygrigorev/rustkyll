# Issue 74: Fix page count gaps on benchmark sites

## Problem

Several benchmark sites show massive page count discrepancies between rustkyll and Jekyll. rustkyll renders a fraction of the pages, making speed comparisons meaningless:

- large-blog-3000: rustkyll 1 page vs Jekyll 3001 (synthetic, we control this)
- large-docs-site: rustkyll 1 page vs Jekyll 801 (synthetic, we control this)
- documentation-theme-jekyll: rustkyll 8 pages vs Jekyll 100
- homebrew-site: rustkyll 53 pages vs Jekyll 134
- muan-blog: rustkyll FAIL vs Jekyll 2218

A "10x speedup" is meaningless if rustkyll only renders 1 page while Jekyll renders 3001.

## Goal

For every benchmark site where both tools succeed, rustkyll must render the same number of pages as Jekyll (within 5%). Sites where rustkyll renders <50% of Jekyll's pages must be investigated and fixed.

## Priority: real sites first

Real sites are more important — they represent actual user workloads. Synthetic sites are useful for stress testing but fixing real site compatibility matters more.

1. muan-blog: rustkyll FAIL vs Jekyll 2218 pages (large real blog)
2. documentation-theme-jekyll: 8 vs 100 pages (real documentation theme)
3. homebrew-site: 53 vs 134 pages (real community site)

## Lower priority: synthetic sites

4. large-blog-3000: 1 vs 3001 pages (synthetic)
5. large-docs-site: 1 vs 801 pages (synthetic)

## Approach

1. For each site, investigate why pages are missing (likely: posts not being discovered, collection pages not generated, pagination not supported)
2. Fix the root cause
3. Re-run benchmark and verify page counts match

## Dependencies

None

## Acceptance criteria

- muan-blog: rustkyll builds successfully and renders close to 2218 pages
- documentation-theme-jekyll: rustkyll renders close to 100 pages
- homebrew-site: rustkyll renders close to 134 pages
- large-blog-3000: rustkyll renders 3001 pages (same as Jekyll)
- large-docs-site: rustkyll renders 801 pages (same as Jekyll)
- Benchmark results updated with correct page counts
- Speed comparisons only shown for sites with matching page counts
