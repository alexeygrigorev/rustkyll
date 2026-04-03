# Issue 498: Fix type-theme remaining 3 differing pages

## Problem

type-theme had 3 pages with DOM differences (5/8 match):

1. **tags.html (1 diff)**: Extra `<h2>` element due to `site.tags.size` returning 3 instead of 2 (counting the internal `__key_order` metadata key).

2. **index.html (9 diffs)**: Post excerpts rendered as raw markdown instead of HTML. The paginator `post.excerpt` was using the raw markdown excerpt instead of the pre-rendered `excerpt_html`.

3. **markdown-and-html.html (4 diffs)**: `class='nx'` expected but got `class='nb'` for `console` and `log` tokens in JavaScript code highlighting. Syntect scopes `console` as `support.type.object.console.js` and `log` as `support.function.console.js`, both mapping to `nb` (builtin). Rouge classifies them as `nx` (Name.Other).

## Fixes

1. **tags.html**: Modified `.size()` on `LenientValue` and `LenientObject` in `src/template/engine.rs` to exclude the `__key_order` metadata key from the count.

2. **index.html**: Modified `collection_item_to_liquid_full()` in `src/pagination.rs` to use `excerpt_html` (pre-rendered HTML) instead of `excerpt` (raw markdown).

3. **markdown-and-html.html**: Added JS-specific scope overrides in `src/syntax.rs` for `source.js support.type` -> `nx` and `source.js support.function.console` -> `nx`.

## Baseline

- DTC: 790/790 (no regression)
- type-theme: 8/8 (improved from 5/8)

## Log

### [SWE] 2026-03-29
- TDD: tags.html fix
  - Wrote test `test_size_excludes_key_order_metadata` -- FAILS (returns "3", expected "2")
  - Fixed `LenientValue::size()` and `LenientObject::size()` to subtract 1 when `__key_order` present
  - Test PASSES
- TDD: index.html excerpt fix
  - Wrote test `test_paginator_post_excerpt_is_html` -- FAILS (gets raw markdown)
  - Fixed `collection_item_to_liquid_full()` to use `excerpt_html` over raw `excerpt`
  - Test PASSES
- TDD: markdown-and-html.html syntax fix
  - Debugged syntect scopes: `console` = `support.type.object.console.js`, `log` = `support.function.console.js`
  - Wrote test `test_js_console_log_is_nx`
  - Added scope overrides for JS support.type and support.function.console -> nx
  - Test PASSES
- Build: all tests pass (3118 lib + integration), clippy clean, fmt clean
- DOM: DTC 790/790, type-theme 8/8
- Files modified: src/template/engine.rs, src/pagination.rs, src/syntax.rs
