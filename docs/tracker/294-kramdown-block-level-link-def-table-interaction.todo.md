# Issue 294: Block-level link definition and table parsing interaction

## Problem

In kramdown Ruby, link definitions are processed during block parsing, preserving block context. In rustkyll, link definitions are extracted in a pre-pass (`extract_definitions`), which removes them from the text before block parsing. This causes a difference when a link definition is immediately followed by a pipe-delimited line:

```
[5]: test
|no|table|here|
```

In kramdown Ruby: `[5]: test` is consumed as a link definition during block parsing. The remaining `|no|table|here|` inherits the paragraph context and becomes a paragraph.

In rustkyll: `[5]: test` is removed during pre-pass. `|no|table|here|` becomes a standalone block after a blank line and is parsed as a single-row table.

Descoped from issue 291 (kramdown remaining ignored tests).

## What's needed

Either:
1. Move link definition extraction from pre-pass into block parser (architectural change)
2. Modify pre-pass to preserve block context when link defs are removed adjacent to content

## Key files

- `src/kramdown_parser/span_parser.rs` (extract_definitions pre-pass)
- `src/kramdown_parser/parser.rs` (block parser)

## Test

The conformance test `block/14_table/errors` covers this interaction.
