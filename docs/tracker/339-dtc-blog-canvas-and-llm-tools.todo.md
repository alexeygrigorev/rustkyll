# Issue 339: DTC blog canvas data attributes and LLM tools page (1 page)

## Problem

`blog/how-do-professionals-use-llm-tools-and-frameworks.html` (9 diffs)

The page uses `<canvas>` elements with custom `data-*` attributes (`data-type="bar"`, `data-orientation="horizontal"`, `data-title="..."`) that are stripped by rustkyll. Jekyll preserves them.

Also has `<figcaption>` ordering differences relative to `<canvas>` elements.

## Root cause

HTML sanitization or attribute handling strips `data-*` attributes from `<canvas>` and possibly other elements. Jekyll's kramdown passes all attributes through.
