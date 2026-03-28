# Issue 464: DTC docs — duplicate children navigation (Table of contents)

## Problem

9 DTC docs pages output "Table of contents" twice — once formatted,
once minified. Jekyll outputs it once. All 9 pages have
`has_children: true` without `has_toc: false`.

Pages with `has_toc: false` match correctly (no ToC at all).

## Diffs

Each page has 3 extra elements: `<hr>`, `<h2>`, `<ul>` from the
duplicate ToC section. Total: 9 pages × ~6 diffs = ~56 diffs.

## Root Cause

The `_layouts/default.html` includes `children_nav.html` when
`page.has_toc != false`. The children nav is being injected twice
in the rendering pipeline — likely from both the Liquid include
and a separate `inject_children_nav` function in generator.rs.

## Scope

Find and fix the duplicate injection. Either:
1. Remove the generator.rs injection (let the template handle it)
2. Or guard against double injection

## Baseline

DTC 790/790. DTC docs 48/57. Target: 57/57.

## Log

### [SWE] 2026-03-28

- Root cause confirmed: `_layouts/default.html` line 32 includes `children_nav.html` via Liquid,
  AND `inject_children_nav()` in generator.rs also injects the same HTML before `</main>`.
  Both fire for pages with `has_children: true` and `has_toc != false`.
- TDD: wrote `test_inject_children_nav_skipped_when_already_present` -- FAILS with 2 occurrences
- Fix: added early-return guard at top of `inject_children_nav()` that checks if
  `<h2 class="text-delta">Table of contents</h2>` is already in the HTML
- Test now PASSES (1 occurrence only)
- All 7 inject_children_nav tests pass, all 3419 tests pass, 0 failures
- Clippy clean, fmt clean
- DOM results: DTC 790/790 (no regression), DTC docs 57/57 (improved from 48/57)
- Files modified: src/generator.rs
