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
