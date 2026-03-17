# Issue 178: Fix URL encoding: `+` vs `%20` for spaces in query parameters

## Problem

On alexeygrigorev/mlbookcamp-page, Twitter share URLs use `+` to encode spaces (Jekyll/Ruby behavior) while rustkyll uses `%20`. Both are valid URL encodings, but they cause DOM diffs.

Example:
- Jekyll: `text=Creating an AWS Account+by+%40Al_Grigor+https%3A%2F%2F...`
- Rustkyll: `text=Creating an AWS Account%20by%20%40Al_Grigor%20https%3A%2F%2F...`

## Affected sites

| Site | Files affected | Diffs |
|------|---------------|-------|
| alexeygrigorev/mlbookcamp-page | 12/15 | 12 (one per page) |

## Acceptance criteria

- [ ] The `url_encode` or `cgi_escape` Liquid filter uses `+` for spaces in query string context (matching Ruby's CGI.escape behavior)
- [ ] OR the DOM comparison tool treats `+` and `%20` as equivalent in URL attributes (acceptable diff)
- [ ] mlbookcamp-page Twitter share URLs match Jekyll output
- [ ] Existing tests continue to pass

## Notes

This may be better handled as an acceptable diff filter in dom_compare.py rather than changing rustkyll behavior, since both encodings are semantically valid. The impact is small (12 pages, 1 diff each).

## Dependencies

None.

## Log

### [SWE] 2026-03-17
- Root cause: The stdlib `url_encode` filter (from `liquid-lib`) uses `%20` for spaces (Shopify Liquid behavior), but Jekyll uses Ruby's `CGI.escape` which encodes spaces as `+`. Also, Jekyll's `cgi_escape` filter was not implemented at all (falling through to passthrough).
- Implemented two new filters:
  - `cgi_escape` -- encodes spaces as `+`, matching Ruby's `CGI.escape` behavior
  - `url_encode` -- overrides the stdlib version to also use `+` for spaces (Jekyll behavior)
- Both filters share the same `cgi_escape_string()` encoding function
- Registered both filters in the template engine builder (after `with_stdlib()` so `url_encode` overrides the default)
- Tests added: 9 unit tests for cgi_escape, 5 unit tests for url_encode, 4 integration tests in engine.rs
- Build: 1414 tests pass (1404 lib + others), 2 pre-existing failures (issue 177 debug tests), clippy clean, fmt clean
- Files created: `src/template/filters/cgi_escape.rs`, `src/template/filters/url_encode.rs`
- Files modified: `src/template/filters/mod.rs`, `src/template/engine.rs`
