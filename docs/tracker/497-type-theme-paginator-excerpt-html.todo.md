# Issue 497: type-theme paginator excerpt renders raw markdown

## Problem

type-theme index.html shows post excerpts as raw markdown text instead
of rendered HTML. Jekyll wraps excerpts in `<p>` tags.

9 diffs on index.html — all from missing `<p>` wrapping around excerpts.

## Root Cause

The paginator excerpt is likely passed as raw markdown instead of
rendered HTML. Check how `post.excerpt` is generated in the paginator.

## Affected Sites

- type-theme (1 page, index.html)

## Baseline

DTC 790/790. type-theme 5/8. Must not regress.
