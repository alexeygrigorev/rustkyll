# Issue 171: Fix layout not applied when Liquid conditionals in layout preamble

## Problem

Many sites have layouts that begin with Liquid logic (assign, if/elsif/else) before the `<!doctype>` or `<html>` tag. When rustkyll fails to evaluate these Liquid blocks, the layout is silently not applied, and pages are rendered as bare HTML fragments (just the markdown-to-HTML content, no `<head>`, `<body>`, or layout wrapper).

This is the single largest source of DOM diffs across the benchmark: **over 4,400 files** across multiple sites render without layouts.

## Root cause

The `default.html` layout in muan-blog starts with:
```liquid
{% if page.path contains "zh-TW" %}
  {% assign lang = "zh-TW" %}
{% elsif page.path contains "de-DE" %}
  {% assign lang = "de-DE" %}
{% else %}
  {% assign lang = "en-US" %}
{% endif %}
<!doctype html>
<html lang="{{ lang }}">
```

When the Liquid `if/elsif/else` or `assign` evaluation fails or is skipped, the entire layout is not applied. The rustkyll output is just the bare content (e.g., `<p>...text...</p>`) without any wrapping HTML structure.

## Affected sites (from DOM analysis)

| Site | Files affected | Pattern |
|------|---------------|---------|
| muan-blog | ~2187/2218 (99%) | Layout starts with `{% if page.path contains %}` |
| opensource-guide | ~336/388 (87%) | i18n pages, layout with conditional logic |
| just-the-docs | 47/47 (100%) | just-the-docs theme layout |
| DataTalksClub/docs | 57/57 (100%) | just-the-docs theme layout |
| documentation-theme-jekyll | ~97/98 (99%) | Complex data-driven layout |
| alexeygrigorev/snippets | ~17/25 (68%) | Layout with conditionals |
| government-github | ~19/21 (90%) | Layout with conditionals |

## Acceptance criteria

- [ ] Layouts that start with Liquid logic (assign, if/elsif/else, for) before HTML doctype are correctly processed
- [ ] muan-blog pages render with full `<html>`, `<head>`, `<body>` wrapping (spot-check 5 pages)
- [ ] opensource-guide i18n pages (ar/, zh-TW/, etc.) render with layout applied
- [ ] just-the-docs pages render with layout applied
- [ ] Existing tests continue to pass

## Dependencies

None -- this is a core Liquid/layout rendering bug.
