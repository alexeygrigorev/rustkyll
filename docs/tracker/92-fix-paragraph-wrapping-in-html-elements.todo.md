# Issue 92: Fix unwanted paragraph wrapping inside HTML elements

## Priority

HIGH — causes visible structural differences on the DTC homepage and other pages.

## Problem

rustkyll wraps content inside HTML elements (like `<li>`) in `<p>` tags when it shouldn't. Jekyll/kramdown preserves the inline flow.

### Jekyll output (correct):
```html
<li class="podcast">
  <a href="..." target="_blank">Event Title</a>
  on 16 Mar 2026
  by
  <a href="/people/name.html">Name</a>
</li>
```

### rustkyll output (wrong):
```html
<li class="podcast">
<p><a href="..." target="_blank">Event Title</a>
on 16 Mar 2026
by</p>
<p><a href="/people/name.html">Name</a></p>
</li>
```

The extra `<p>` tags change spacing, layout, and break CSS styling that targets `li > a` (direct child).

## Root cause

When Liquid templates produce HTML inside list items (e.g. an include inside a `{% for %}` loop), the HTML gets passed through markdown rendering which wraps inline content in `<p>` tags. kramdown recognizes that content inside HTML block elements should not be paragraph-wrapped; pulldown-cmark does not.

## Goal

Content inside HTML block elements (`<li>`, `<div>`, `<td>`, etc.) must not be wrapped in `<p>` tags when the content is already HTML. The output must match Jekyll's structure exactly.

## Approach

This likely needs a post-processing step in the kramdown module (src/kramdown.rs) or a change to how markdown rendering interacts with Liquid-generated HTML:
1. Detect when content inside block elements is already HTML
2. Strip unwanted `<p>` wrapping inside those elements
3. Or prevent markdown from processing content that's already HTML

## Dependencies

None

## Acceptance criteria

- `<li>` elements in the DTC homepage events list have no `<p>` wrapper
- Same fix applies to `<div>`, `<td>`, and other block elements
- Jekyll and rustkyll produce structurally identical HTML for the events list
- No regressions on actual markdown content that should have `<p>` tags
- All existing tests still pass
- Playwright visual comparison shows improvement on homepage
