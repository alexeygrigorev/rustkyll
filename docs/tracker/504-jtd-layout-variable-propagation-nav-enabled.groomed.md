# Issue 504: Layout front matter variables not propagating through layout chain

## Problem

When a page uses a layout that itself extends another layout (layout chaining), variables set in the intermediate layout's front matter are not accessible via `layout.variable_name` in the parent layout's Liquid templates.

In just-the-docs, the `minimal` layout sets `nav_enabled: false` in its front matter and extends `default`. The `default` layout checks `layout.nav_enabled` to decide whether to render the sidebar. Because `layout.nav_enabled` is not propagated, the sidebar renders on pages that should not have it.

### Example

`_layouts/minimal.html`:
```yaml
---
layout: default
nav_enabled: false
---
{{ content }}
```

`_layouts/default.html`:
```liquid
{% if page.nav_enabled == true %}
  {% include components/sidebar.html %}
{% elsif layout.nav_enabled == true and page.nav_enabled == nil %}
  {% include components/sidebar.html %}
{% elsif site.nav_enabled != false and layout.nav_enabled == nil and page.nav_enabled == nil %}
  {% include components/sidebar.html %}
{% endif %}
```

**Jekyll**: minimal layout pages have no sidebar (correct -- `layout.nav_enabled` is `false`, so the third branch's `layout.nav_enabled == nil` check fails)

**Rustkyll**: minimal layout pages have sidebar rendered (broken -- `layout.nav_enabled` is nil/missing, so the third branch fires and the sidebar appears)

### Affected Pages (3 pages)

- docs/layout/minimal/minimal/index.html (5 diffs)
- docs/layout/minimal/minimal-child/index.html (5 diffs)
- docs/minimal-test/index.html (5 diffs)

## Root Cause

When a page's layout (e.g., `minimal`) is processed, and that layout extends another layout (e.g., `default`), the inner layout's front matter variables should be accessible as `layout.*` in the outer layout. Rustkyll is not setting up this `layout` variable correctly during layout chain rendering.

## Dependencies

None (but related to #450 which also involves just-the-docs layout resolution).

## Baseline

- just-the-docs: 1/47 (or higher if #501/#502/#503 are fixed first)
- DTC: 790/790 (must not regress)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `layout.nav_enabled` is accessible and equals `false` when the page uses a layout that sets `nav_enabled: false`
- [ ] Minimal layout pages in just-the-docs do NOT render the sidebar `<header>`
- [ ] Normal pages still render the sidebar correctly
- [ ] DTC DOM baseline remains at 790/790

## Test Scenarios

### Unit: Layout variable propagation
- Page with layout A, layout A extends layout B, layout A has `foo: bar` -- verify `layout.foo` is `"bar"` in layout B
- Page with layout A, layout A has `nav_enabled: false` and extends default -- verify `layout.nav_enabled == false`
- Page with direct layout (no chaining) -- verify `layout.*` still works

### Integration: just-the-docs minimal pages
- Build just-the-docs, check docs/minimal-test/index.html does NOT contain `<header class="side-bar">`
- Verify docs/layout/minimal/minimal/index.html has no sidebar
- Verify normal pages still have sidebar
