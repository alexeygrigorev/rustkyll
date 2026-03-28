# Issue 444: Liquid template rendering gaps (includes, filters, dates)

## Problem

Some sites show raw Liquid syntax in output instead of rendered content:
- `{{ page.date | date: ... }}` appearing as literal text
- `{% include %}` blocks not resolving
- Future-dated posts not filtered correctly

## Affected Sites

- hydeout (0/13) — raw Liquid in category pages
- jekyll-vitepress-theme (0/17) — unresolved includes

## Scope

Investigate which Liquid features are not rendering and fix.
