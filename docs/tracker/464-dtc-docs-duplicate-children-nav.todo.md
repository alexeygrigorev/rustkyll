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
