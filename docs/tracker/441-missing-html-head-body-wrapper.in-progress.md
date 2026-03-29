# Issue 441: Missing HTML head/body wrapper for theme sites

## Problem

Several sites output raw content without `<head>`/`<body>` wrapper.
Jekyll applies layouts that add the HTML document structure, but
rustkyll is not resolving or applying these layouts.

## Affected Sites

- text-theme (0/6, 25 diffs) — 404, archive, index missing wrapper
- hydeout (0/13, 207 diffs) — most pages missing wrapper
- minimal-mistakes (0/1, 2 diffs) — missing wrapper, only 1 file generated

## Root Cause

Theme layouts (from gems or _layouts/) are not being applied. May be
a remote theme resolution issue or layout chain resolution failure.

## Scope

Investigate why layouts are not applied for these themes and fix.
