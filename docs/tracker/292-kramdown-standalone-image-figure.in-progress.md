# Issue 292: Kramdown standalone image to figure conversion

## Problem

The `{:standalone}` IAL attribute on images should convert a paragraph containing a single image into a `<figure>` element with `<figcaption>`. This is a kramdown-specific feature not yet implemented.

Descoped from issue 291 (kramdown remaining ignored tests).

## What's needed

- When an image has the `standalone` attribute in its inline IAL, and it's the only content in a paragraph, convert the paragraph to `<figure>`
- Block-level IAL should apply to the `<figure>`, image-level IAL to the `<img>`
- Add `<figcaption>` with the image alt text

## Key files

- `src/kramdown_parser/html.rs` (paragraph rendering)
- `src/kramdown_parser/span_parser.rs` (image parsing, inline IAL)

## Test

The conformance test `block/03_paragraph/standalone_image` covers this feature.

## Log

### [SWE] 2026-03-21
- Enabled conformance test `kramdown_block_03_paragraph_standalone_image` (was deferred/ignored)
- Ran test: FAILS as expected -- paragraph renders `<p><img ...>{:...}</p>` instead of `<figure>`
- Implemented `try_parse_standalone_image()` in `src/kramdown_parser/span_parser.rs` to detect standalone images
- Implemented `convert_standalone_image_figure()` in `src/kramdown_parser/html.rs` to render `<figure>` with proper attribute routing
- Attribute routing: block IAL -> figure, inline IAL -> img when block IAL exists; id/class -> figure, rest -> img when no block IAL
- Ran conformance test: PASSES
- Added 3 unit tests: Unicode alt text, Unicode with block IAL, non-standalone stays paragraph
- All tests pass: 2442 lib tests, 0 failures
- Clippy clean, fmt clean
- Files modified: `src/kramdown_parser/html.rs`, `src/kramdown_parser/span_parser.rs`, `src/kramdown_parser/tests.rs`
