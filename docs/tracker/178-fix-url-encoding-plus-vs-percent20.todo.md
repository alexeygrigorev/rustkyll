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
