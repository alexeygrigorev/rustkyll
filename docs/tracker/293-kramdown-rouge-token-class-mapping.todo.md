# Issue 293: Rouge-compatible syntax highlighting token class mapping

## Problem

Syntect's scope-to-CSS-class mapping differs from Rouge for certain languages:

- PHP: variables map to `n` (identifier) instead of `nv` (name.variable)
- PHP: class names map to `nb` (name.builtin) instead of `nc` (name.class)
- Ruby: double-quoted strings use `dl`+`s2`+`dl` delimiter splitting instead of single `s2`
- rouge_multiple test needs `custom-class` wrapper div from formatter options

Descoped from issue 291 (kramdown remaining ignored tests).

## What's needed

- Update scope_map in `src/syntax.rs` to map PHP variable scopes to `nv`
- Map PHP class name scopes to `nc`
- Consider merging adjacent `dl`+`s2`+`dl` spans for Ruby string tokens
- Support `formatter: RougeHTMLFormatters` option for custom wrapper div

## Key files

- `src/syntax.rs` (scope-to-CSS-class mapping)
- `src/kramdown_parser/html.rs` (formatter wrapper div)

## Tests

The conformance tests `block/06_codeblock/rouge/simple` and `block/06_codeblock/rouge/multiple` cover these features.
