# Issue 246: TeXt `exclude` handling emits `test/` pages

## Status: ALREADY RESOLVED (invalid premise)

Investigation shows that Jekyll ALSO generates the `test/` pages for the TeXt theme. The `_config.yml` has `/test` in its exclude list, but Jekyll still produces `test/404.html`, `test/about.html`, `test/archive.html`, and `test/index.html` (mostly as empty files). rustkyll matches this behavior exactly.

## Verification

- DOM comparison of text-theme: 11/11 pages match (0 differences).
- Both Jekyll and rustkyll generate the same set of test/ pages.
- The file counts and content match between Jekyll and rustkyll outputs.

## Dependencies

- Issue #238

