# Issue 367: DTC URL asterisk rendering in markdown

## Parent

Follow-up from #363 (RC-F).

## Problem

O'Reilly URLs containing `*` characters (e.g., `_gl=1*95hemv*_ga*MTA2...`) are being parsed as `<em>` emphasis markers instead of literal characters within the URL text.

## Affected Pages

- `books/20221121-reliable-machine-learning.html` (partial of 15 diffs)

## Acceptance Criteria

- [ ] Asterisks inside URLs are not parsed as emphasis markers
- [ ] URL text with `*` characters renders as literal text matching Jekyll/kramdown behavior
- [ ] DTC DOM match count does not regress
- [ ] No site-specific hardcoding

## Priority

LOW
