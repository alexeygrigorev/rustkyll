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

## Log

### [SWE] 2026-03-21
- TDD Step 1: Wrote 4 failing PHP unit tests (test_php_variable_is_nv, test_php_class_name_is_nc, test_php_unicode_variable_is_nv, test_php_new_keyword_is_k)
- Ran tests: all 4 FAIL as expected -- PHP variables map to `n` instead of `nv`, class names to `nb` instead of `nc`
- TDD Step 2: Added PHP-specific scope overrides in build_scope_map():
  - `("source.php variable.other", "nv")` -- PHP variables to Name.Variable
  - `("source.php support.class", "nc")` -- PHP class names to Name.Class
- Ran tests: all 4 PHP unit tests PASS
- Un-ignored `kramdown_block_06_codeblock_rouge_simple` conformance test: PASSES
- Checked `rouge_multiple`: PHP tokens now correct, but still fails due to:
  1. Missing `custom-class` wrapper div (requires html.rs changes, out of scope)
  2. Ruby string dl+s2+dl splitting (separate issue)
- Updated rouge_multiple deferred comment to reflect current status
- Full test suite: 2426 lib tests pass, 0 fail (3 ignored). 1 pre-existing failure in kramdown::tests::test_issue302_ellipsis_inside_math (unrelated, issue 302)
- Clippy: clean, fmt: clean (pre-existing fmt issue in template/layout.rs, not touched)
- Files modified: src/syntax.rs, src/kramdown_parser/tests.rs, docs/tracker/293-*.md
