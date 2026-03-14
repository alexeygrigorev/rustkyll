# Issue 72: Investigate and fix visual differences found by Playwright

## Problem

The Playwright visual comparison (issue #62) was run as a self-comparison (rustkyll vs rustkyll) which always produces 0% diff. The real comparison (rustkyll vs Jekyll) has not been done yet. When it is, any pixel differences must be investigated and fixed — not tolerated with a threshold.

## Goal

1. Run Playwright visual comparison of rustkyll output vs Jekyll output for DTC site and kids-horror-stories-ru
2. The pages should be pixel-perfect matches. If they are not, investigate every difference and fix the root cause
3. A 5% threshold is acceptable only for known, documented exceptions (e.g. timestamp differences)

## Approach

1. Build both sites with Jekyll and rustkyll
2. Serve both over HTTP
3. Run `scripts/visual-compare.sh` with threshold 0 (or near 0)
4. For every page that fails:
   - Inspect the diff image
   - Identify the root cause (missing CSS class, wrong content, missing sidebar, wrong URL, etc.)
   - Fix the root cause or create a follow-up issue if the fix is large
5. Re-run until all pages pass

## Sites to compare

- DataTalksClub/datatalksclub.github.io
- kids-horror-stories-ru

## Dependencies

- Issue 62 (Playwright infrastructure) done
- Issue 69 (URL format differences) should ideally be fixed first
- Issue 70 (missing pages) should ideally be fixed first
- Issue 71 (sidebar/related content) should ideally be fixed first

## Acceptance criteria

- Playwright comparison run against Jekyll output (not self-comparison)
- All compared pages have <1% pixel difference
- Every difference >0% is investigated with documented root cause
- Diff images saved and reviewed
- No 404 errors in browser console for rustkyll server that don't also appear on Jekyll server
- Results documented in docs/comparison/visual-results.md with:
  - Per-page pixel diff percentage
  - Screenshot file paths (jekyll, rustkyll, diff)
  - Root cause analysis for every page with >0% diff
  - Summary: total pages compared, pages passing, pages failing
  - Date of comparison run
