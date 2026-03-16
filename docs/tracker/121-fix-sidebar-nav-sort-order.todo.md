# Issue 121: Fix sidebar navigation sort order on large-docs-site

## Priority

HIGH — 9.62% pixel diff on large-docs-site homepage due to sidebar links in wrong order.

## Problem

The sidebar navigation renders links in a different sort order than Jekyll, producing a visually different layout. This is a significant visual difference on documentation sites.

## Goal

Sidebar navigation links must appear in the same order as Jekyll. The sort order likely depends on how pages are iterated in the template (alphabetical by filename, by weight/order front matter value, etc.).

## Acceptance criteria

- large-docs-site homepage achieves 0% pixel diff
- Sidebar links appear in same order as Jekyll
- Fix is generic (works for any site with sidebar navigation)
- No regressions
