# Issue 170: Update all documentation — limitations, features, benchmarks

## Problem

Documentation is stale after 170+ issues. Many docs reference old numbers, outdated limitations, or features that have been implemented.

## Goal

Update all documentation to reflect current state. Remove bold formatting. Remove docs that are no longer needed. Ensure everything is committed first so we can roll back.

## Pages to update

1. **README.md** — quickstart, benchmark table, feature list, known limitations, tested sites
2. **docs/jekyll-compatibility.md** — feature matrix (many features changed from no/partial to yes)
3. **docs/benchmark/results.md** — stale benchmark numbers
4. **docs/comparison/pixel-perfect-results.md** — stale pixel diff numbers
5. **docs/comparison/dom-diff-categories.md** — stale diff counts
6. **docs/release.md** — any outdated process info
7. **docs/PROCESS.md** — any outdated process info
8. **docs/cross-platform-testing.md** — ensure current

## Rules

- Remove bold/italic formatting (no ** or *)
- Remove documentation files that are no longer needed
- Commit current state FIRST before any changes (so we can roll back)
- Update all numbers to reflect current state (19x speed, 593/787 DOM, 21/22 pixel-perfect, etc.)
- Update known limitations list (many former limitations are now fixed)

## Acceptance criteria

- All docs reflect current state
- No bold/italic formatting
- Stale docs removed
- Feature compatibility matrix accurate
- Benchmark numbers current
- Known limitations list accurate

## Log

### [SWE] 2026-03-17

Files modified:
- README.md -- updated benchmark table (19x, 1.0s vs 19.1s), known limitations (removed SCSS/pagination/plugin gaps, added real remaining issues), added --no-browser flag, updated quickstart text
- docs/jekyll-compatibility.md -- SCSS: no->yes, highlight tag: partial->yes, serve command flags updated, added progress bar and auto-open browser features, summary counts updated (128/6/27)
- docs/benchmark/results.md -- updated DTC numbers to 19x/1.0s/593 DOM matches, removed stale regression section references, removed all bold/italic
- docs/comparison/pixel-perfect-results.md -- updated test count to 1572, removed bold/italic
- docs/comparison/dom-diff-categories.md -- updated summary to 593/787, removed bold/italic
- docs/comparison/structural-results.md -- updated DTC to 787/787 file match, 593/787 DOM match, removed stale missing files list
- docs/plan.md -- updated vision (generic Jekyll replacement, not DTC-only), removed stale non-goals
- docs/PROCESS.md -- removed bold/italic formatting
- docs/cross-platform-testing.md -- removed bold/italic formatting

Files removed (stale, superseded by benchmark/results.md):
- docs/complex-site-results.md
- docs/cross-site-results.md
- docs/comparison/visual-results.md

Files unchanged (already current):
- docs/release.md
- docs/cross-platform-testing.md (content was current)

Bold/italic removed from all non-tracker docs (13 files updated).

Tests: 1572 passed, 0 failed, 43 ignored
Clippy: clean
Format: clean
