# Issue 391: Add GFM paragraph interruption mode to kramdown parser

## Problem

The kramdown parser does not support GFM-style paragraph interruption by list
markers. In GFM mode, `1.` can interrupt a paragraph. This is needed before
the markdownify pipeline can switch to the kramdown parser.

## Scope

1. Add a `gfm_paragraph_interruption` option to the kramdown parser
2. When enabled, list markers (1., -, *) can interrupt paragraphs
3. This matches `kramdown-parser-gfm` behavior used by DTC's Jekyll

## Dependencies

- Prerequisite for #390 (kramdown parser in markdownify)
