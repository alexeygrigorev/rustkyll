# Issue 368: DTC definition list rendering (dl/dt/dd elements)

## Parent

Follow-up from #363 (RC-G).

## Problem

Jekyll/kramdown renders certain text patterns as `<dl>` (definition lists) inside `<ol><li>`. Rustkyll does not support `<dl>` rendering. Also, `mailto:` link with pipe character has encoding difference (`|` vs `%7C`).

## Affected Pages

- `books/20210405-the-practitioners-guide-to-graph-data.html` (6 diffs)

## Acceptance Criteria

- [ ] Definition list patterns render as `<dl>`/`<dt>`/`<dd>` elements matching Jekyll/kramdown behavior
- [ ] `mailto:` links with pipe characters use correct encoding
- [ ] DTC DOM match count does not regress
- [ ] No site-specific hardcoding

## Priority

LOW
