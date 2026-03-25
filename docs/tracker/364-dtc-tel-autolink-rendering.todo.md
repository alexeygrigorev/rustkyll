# Issue 364: DTC tel: autolink rendering

## Parent

Follow-up from #363 (RC-C).

## Problem

Text like `<tel:100-1000|100-1000>` is being parsed as an autolink producing an `<a>` element. Jekyll/kramdown does not autolink `tel:` URIs -- it renders the pipe as a literal character and keeps the text inline with `<br>`.

## Affected Pages

- `books/20211004-transfer-learning-in-action.html` (5 diffs)

## Acceptance Criteria

- [ ] `tel:` URIs are not converted to `<a>` autolinks in markdownify output
- [ ] Pipe character inside angle-bracket `tel:` expression renders as literal text
- [ ] DTC DOM match count does not regress
- [ ] No site-specific hardcoding

## Priority

LOW
