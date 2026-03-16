# Issue 121: Fix sidebar navigation sort order on large-docs-site

## Priority

HIGH — 9.62% pixel diff on large-docs-site homepage due to sidebar links in wrong order.

## Problem

The sidebar navigation renders links in a different sort order than Jekyll, producing a visually different layout. This is a significant visual difference on documentation sites.

## Goal

Sidebar navigation links must appear in the same order as Jekyll. The sort order likely depends on how pages are iterated in the template (alphabetical by filename, by weight/order front matter value, etc.).

## Acceptance criteria

- large-docs-site homepage achieves 0% pixel diff
- Sidebar links appear in same order as Jekyll
- Fix is generic (works for any site with sidebar navigation)
- No regressions

## Log

### [SWE] 2026-03-16

- **Root cause**: `load_pages_recursive` in `src/collection.rs` sorted entries within each directory (depth-first), producing pages grouped by directory (all api-reference, then all configuration, etc.). Jekyll sorts `site.pages` by `(basename, full_url)` -- i.e., by filename first, then by full path for tie-breaking. This interleaves pages from different directories by filename.
- **Fix**: Added a sort step in `load_pages()` after recursive loading, sorting pages by `(basename_of_url, full_url)` to match Jekyll's order.
- **Verification**: Built both Jekyll and rustkyll for large-docs-site. The index.html nav section (800 page links) is now byte-identical between the two builds.
- **Test added**: `test_pages_sorted_by_basename_then_url` in `src/collection.rs` -- creates pages in two subdirectories and verifies they are returned in (basename, full_url) order.
- **Also fixed**: Pre-existing compilation errors in `src/main.rs`, `tests/integration_build.rs`, `tests/integration_performance.rs` caused by a prior change of `DataTree` from `HashMap` to `BTreeMap` in `data.rs`.
- **Build**: 1445 tests pass, 0 fail, clippy clean, fmt clean
- **Files modified**: `src/collection.rs`, `src/main.rs`, `tests/integration_build.rs`, `tests/integration_performance.rs`
