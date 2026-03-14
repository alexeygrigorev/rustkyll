# Issue 70: Fix missing pages in DTC site build

## Problem

Structural comparison shows 5 pages that Jekyll generates but rustkyll does not, and 2 pages that rustkyll generates but Jekyll does not:

### Missing from rustkyll (Jekyll has them):
- people/aashishnair.html
- podcast/production-ml-search-vector-search-embeddings-hybrid-search.html
- slack/guidelines.html
- tools/modelstore.html
- tools/obsei.html

### Extra in rustkyll (Jekyll doesn't have them):
- Unknown — need to identify which 2 files

## Goal

rustkyll must produce the exact same set of HTML files as Jekyll for the DTC site. No missing pages, no extra pages.

## Approach

1. Investigate each missing page — why does Jekyll generate it but rustkyll doesn't?
2. Investigate each extra page — why does rustkyll generate it but Jekyll doesn't?
3. Fix the root causes
4. Re-run structural comparison and verify 0 missing/extra files

## Dependencies

- Issue 61 (structural comparison) done

## Acceptance criteria

- DTC site produces the exact same HTML files as Jekyll (0 missing, 0 extra)
- All 5 currently missing pages are now generated
- The 2 extra pages are either removed or explained (if Jekyll intentionally skips them)
- kids-horror-stories-ru still has 0 missing files
- All existing tests still pass
