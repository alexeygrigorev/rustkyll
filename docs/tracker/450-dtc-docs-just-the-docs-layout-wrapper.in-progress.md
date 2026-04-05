# Issue 450: DTC docs — just-the-docs theme layout wrapper missing

## Problem

DTC docs (48/57, 84%) has 9 pages missing the `<head>/<body>` HTML
wrapper. The just-the-docs theme layout is not being applied, so
pages output raw content starting with `<h1>` instead of `<html><head>`.

All 9 diffs are the same pattern: `child[1]: tag_name_differs - expected 'head', actual 'h1'`.
The 40+ "extra element" diffs are cascading from the missing wrapper.

## Root Cause

The just-the-docs gem theme's default layout is not being resolved
or applied to these 9 pages during rendering.

## Scope

1. Investigate why the layout wrapper is missing for these pages
2. Check how remote/gem theme layouts are resolved
3. Fix layout application for just-the-docs theme
4. All 9 pages should get proper HTML document structure

## Baseline

DTC 790/790. DTC docs 48/57, 56 total diffs.
Target: 57/57 (100%).

## Log

### [SWE] 2026-03-28

Root cause: The Liquid expression `site[page.collection]` where `page.collection` is nil
caused a "Expected scalar, found nil" error in vendor/liquid-core. This crashed the layout
rendering for 9 pages with `has_children: true` (they use the just-the-docs children_nav.html
include which contains this expression). These pages fell through to fallback rendering
(raw HTML without layout wrapper).

Two fixes applied:

1. vendor/liquid-core/src/runtime/expression.rs: When a variable index evaluates to nil
   (e.g., `site[page.collection]` where `page.collection` is nil), return nil instead of
   erroring. This matches Jekyll/Ruby Liquid behavior.

2. src/generator.rs: Added guard in `inject_children_nav` to skip injection when the layout
   template already rendered the children nav (detected by `class="text-delta"`). Previously,
   the fallback prevented the layout from running, so inject_children_nav was the only source.
   Now that layouts render correctly, both paths produced duplicate navigation.

Tests added:
- test_nil_bracket_index_returns_nil: verifies `site[page.collection]` with nil collection
  returns nil and the default filter kicks in
- test_nil_bracket_index_in_where_exp_chain: verifies the full just-the-docs expression chain
  (site[page.collection] | default: site.html_pages | where_exp | group_by) works

TDD cycle:
- Wrote tests first, verified they FAIL with "Expected scalar, found nil"
- Applied fix, verified tests PASS

Results:
- DTC main: 790/790 (unchanged)
- DTC docs: 38/57 (improved from 31/57 effective baseline with dirty tree)
  - Note: issue 449 kramdown changes in dirty tree regressed DTC docs from 48 to 31;
    this issue's fix improves it to 38 (the original 9 head/h1 diffs are resolved)
- All tests pass: 3053+ unit tests, 0 failures
- Clippy clean, fmt clean

Files modified:
- vendor/liquid-core/src/runtime/expression.rs (nil bracket index returns nil)
- src/generator.rs (skip duplicate children nav injection)
- src/template/engine.rs (2 new tests)
