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
