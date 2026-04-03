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
- Root cause: `Expression::evaluate()` in `vendor/liquid-core/src/runtime/expression.rs` errors when a variable index (e.g. `site[page.collection]`) evaluates to nil. Jekyll/Ruby Liquid returns nil in this case.
- Wrote test `evaluate_nil_bracket_index_returns_nil` in expression.rs
- Implemented fix: in `Expression::evaluate()`, when `Variable::evaluate()` or `runtime.get()` errors, fall back to `try_evaluate()` which returns None gracefully, then map None to Nil
- All tests pass (3000+ passed, 0 failed), clippy clean, fmt clean
- DOM baselines verified:
  - DTC: 790/790 (baseline 790/790) -- PASS
  - DTC docs: 48/57 (baseline 48/57) -- PASS (no improvement yet, fix prevents errors but doesn't resolve layout application)
  - muan-blog: 2195/2218 (baseline 2195/2218) -- PASS
  - large-docs-site: 801/801 (baseline 801/801) -- PASS
- Files modified: `vendor/liquid-core/src/runtime/expression.rs`
- Note: This fix is a prerequisite -- it prevents crashes when nil is used as a bracket index. The 9 missing layout pages still need additional work (layout resolution for just-the-docs theme) which is the remaining scope of this issue.
