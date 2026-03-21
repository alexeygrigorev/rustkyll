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
