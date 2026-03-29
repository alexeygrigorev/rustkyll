# Issue 528: jekyll-docs news/releases and jekyllconf pages not rendering layout

## Problem

Three pages produce completely broken HTML -- they output raw Liquid instead of
rendered content, suggesting the layout is not being applied at all:

1. **news/releases/index.html** (3 diffs): The output starts with raw Liquid
   `{% for post in site.categories.release -%}` instead of a proper HTML document
   with `<head>` and `<body>`. The layout is completely missing.

2. **jekyllconf/index.html** (9 diffs): The first child is `<p>` instead of
   `<head>`, and `<h2>` instead of `<body>`. The page renders as plain HTML
   fragments without the site layout wrapper.

### Specific diffs

news/releases:
```
(root): expected_element_got_text - expected: '<head>', actual: '{% for post in site.categories.release -%} ...'
head: missing_element
body: missing_element
```

jekyllconf:
```
child[1]: tag_name_differs - expected: 'head', actual: 'p'
child[2]: tag_name_differs - expected: 'body', actual: 'h2'
```

## Root Cause

These pages likely use a layout that rustkyll cannot find or fails to apply:
- The page may reference a layout that does not exist in the theme
- The page may use front matter defaults for its layout assignment
- The page may be a collection page where layout resolution differs

## Scope

Investigate why these 3 pages are not getting their layout applied and fix.
This is likely a layout resolution or front matter defaults issue.

## Dependencies

- Issue 500 (jekyll-docs feed/meta/SEO) may be related

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] news/releases/index.html renders with full layout (`<head>`, `<body>`)
- [ ] jekyllconf/index.html renders with full layout (`<head>`, `<body>`)
- [ ] DTC DOM match count must not drop below 790/790
- [ ] These pages produce valid HTML documents

## Test Scenarios

### Unit: Layout resolution for special pages

- Page with layout in front matter defaults for a specific path gets correct layout
- Page in a non-standard directory (jekyllconf/) gets correct layout

### Integration: jekyll-docs site

- Build jekyll-docs, verify news/releases/index.html starts with `<!DOCTYPE html>`
- Build jekyll-docs, verify jekyllconf/index.html starts with `<!DOCTYPE html>`
- Run DOM comparison, verify improvement
