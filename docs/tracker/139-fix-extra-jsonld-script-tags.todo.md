# Issue 139: Fix extra JSON-LD script tags in books and other pages

## Problem

Rustkyll emits extra `<script type="application/ld+json">` tags that Jekyll does not produce. This affects ~100 files, primarily book detail pages. Jekyll either does not emit JSON-LD for these pages, or emits it inline (without `type="application/ld+json"`).

Also, some pages have FAQ JSON-LD scripts appearing at different DOM positions (inside content div vs in head/body).

Discovered in issue #119 DOM diff audit.

## Example

Jekyll book page: no JSON-LD script tag
Rustkyll book page: `<script type="application/ld+json">{ "@context": "https://schema.org", ... }</script>`

## Acceptance criteria

- Book pages only emit JSON-LD if Jekyll also emits it
- No extra `<script>` elements in rustkyll output compared to Jekyll
- No regressions
