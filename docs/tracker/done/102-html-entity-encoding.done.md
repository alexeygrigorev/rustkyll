# Issue 102: Fix HTML entity encoding preservation (D17)

Descoped from issue #90. Bare `&` in raw HTML blocks passes through pulldown-cmark without being re-encoded to `&amp;`. Jekyll/kramdown re-encodes entities.

## Acceptance criteria
- Bare `&` in HTML output is encoded as `&amp;` where Jekyll does so
- No double-encoding (`&amp;amp;`)
- No regressions on correctly encoded content

## Log

### [SWE] 2026-03-15
- Implemented `encode_bare_ampersands` function in `src/kramdown.rs`
- The function scans HTML output for `&` characters and checks if they begin a valid entity reference (`&name;`, `&#digits;`, `&#xhex;`). Bare `&` is encoded to `&amp;`.
- Added to `kramdown::postprocess` pipeline (runs early, after `strip_paragraphs_in_html_blocks` but before heading IDs and other transforms)
- Helper functions: `is_valid_entity_start`, `find_entity_end`
- Tests added: 16 unit tests covering bare `&` in text, no double-encoding of `&amp;`/`&lt;`/`&gt;`, numeric/hex entity preservation, URL attributes, UTF-8 content, edge cases (empty string, trailing `&`, `&foo` without semicolon), and integration via `markdown_to_html`
- Build: 1343 tests pass, 0 fail, 31 ignored. Clippy clean, fmt clean.
- Files modified: `src/kramdown.rs`
