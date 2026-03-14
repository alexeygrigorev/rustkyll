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

For every benchmark site, rustkyll must render the exact same number of pages as Jekyll. Not "close to", not "within 5%" — the exact same number. If there's a difference, it's a bug.

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

- muan-blog: rustkyll builds successfully and renders exactly 2218 pages (same as Jekyll)
- documentation-theme-jekyll: rustkyll renders exactly 100 pages (same as Jekyll)
- homebrew-site: rustkyll renders exactly 134 pages (same as Jekyll)
- large-blog-3000: rustkyll renders exactly 3001 pages (same as Jekyll)
- large-docs-site: rustkyll renders exactly 801 pages (same as Jekyll)
- Page counts match Jekyll exactly for every site — any difference is a bug to fix
- Benchmark results updated with correct page counts
- Speed comparisons only shown for sites with matching page counts
