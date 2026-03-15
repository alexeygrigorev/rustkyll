# Issue 102: Fix HTML entity encoding preservation (D17)

Descoped from issue #90. Bare `&` in raw HTML blocks passes through pulldown-cmark without being re-encoded to `&amp;`. Jekyll/kramdown re-encodes entities.

## Acceptance criteria
- Bare `&` in HTML output is encoded as `&amp;` where Jekyll does so
- No double-encoding (`&amp;amp;`)
- No regressions on correctly encoded content
