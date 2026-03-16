# Issue 112: Fix collection sort stability for tie-breaking

Affects podcast.html (0.05% pixel diff). Two episodes with identical season=3, episode=4 appear in different order. Need consistent tie-breaking (e.g. by filename) when sort keys are equal.

## Acceptance criteria
- Same-value sort keys produce same order as Jekyll
- podcast.html achieves 0% pixel diff

## Log

### [SWE] 2026-03-16
- Root cause: The Liquid `sort` filter (from liquid crate) sorts by the requested property but has no tiebreaker for equal values. When two podcast episodes share season=3 episode=4, the output order depends on input order, which may differ from Jekyll's.
- Fix: Created a custom `Sort` filter (`src/template/filters/sort.rs`) that overrides the liquid crate's built-in sort. When sorting by a property, items with equal sort key values are tie-broken by `slug`, then by `path`. This matches Jekyll's behavior where equal items preserve the underlying filename/path order.
- Also added slug-based tiebreaking to all other sort sites in the codebase:
  - `src/generator.rs`: `build_related_posts` (was missing tiebreaker for descending date sort)
  - `src/pagination.rs`: pagination post sort (was missing tiebreaker)
  - `src/feed.rs`: feed post sort (was missing tiebreaker)
- Tests added: 8 unit tests for the custom Sort filter covering basic sorting, tiebreaking by slug, tiebreaking by path, scalar sorting, empty arrays, nil values, non-array input, and integer-as-string sorting.
- Build: 1386 tests pass, 0 fail, clippy clean, fmt clean
- Files created: src/template/filters/sort.rs
- Files modified: src/template/filters/mod.rs, src/template/engine.rs, src/generator.rs, src/pagination.rs, src/feed.rs, src/kramdown.rs (pre-existing clippy fix)
