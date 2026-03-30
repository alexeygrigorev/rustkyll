# Issue 247: Mediumish page layouts are not applied

## Problem

The Mediumish demo pages that declare `layout: default`, `layout: page`, `layout: post`, or `layout: categories` are rendering as raw Liquid template code or empty pages under rustkyll instead of being wrapped in the theme layouts.

Current state: 4/24 pages match, 106 total DOM differences. The 20 pages with differences all show the same pattern -- raw Liquid code (e.g., `{% if page.url == "/" %}`) in the output instead of rendered HTML wrapped in the `default.html` layout.

This affects:
- `index.html` -- renders raw Liquid instead of homepage with featured posts
- `about.html` -- raw content without `page` -> `default` layout chain
- `categories.html` -- empty (0 bytes)
- All post pages (`customer-service/`, `education/`, `red-riding/`, etc.) -- raw markdown/Liquid without `post` -> `default` layout
- Pagination pages (`page2/`, `page3/`) -- not wrapped in default layout
- `tags.html` -- missing head/body elements
- `404.html` -- missing head/body elements

## Root Cause

The layout chain is not being applied to pages from the Mediumish site. The pages have front matter specifying layouts (`layout: page`, `layout: post`, etc.), and the layout files exist in `websites/mediumish/_layouts/`, but rustkyll is not resolving and applying them. The pages from `_pages/` directory (configured via `include: ["_pages"]`) have their content rendered but layouts are skipped entirely.

Key observations:
1. Pages are being generated (correct file paths and counts match Jekyll), so the `include: ["_pages"]` config is working.
2. The template engine is NOT being invoked on the output -- raw `{% if %}`, `{% for %}`, and `{{ }}` tags appear in the HTML.
3. Nested layout chains (`page` -> `default`, `post` -> `default`) are not being resolved.

## Scope

Fix layout resolution and template rendering for the Mediumish theme so that all 24 pages have their layout chains applied and Liquid templates rendered.

## Dependencies

- Issue #239 (must be `.in-progress.md` or `.done.md`)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests for this issue
- [ ] Building the Mediumish site with rustkyll applies the correct layout chain for all pages (e.g., `page` -> `default`, `post` -> `default`, `categories` -> `default`)
- [ ] No raw Liquid code (`{% ... %}`, `{{ ... }}`) appears in any generated HTML file
- [ ] Every generated HTML page contains `<!DOCTYPE html>`, `<head>`, and `<body>` elements from the `default.html` layout
- [ ] DOM comparison improves from 4/24 matched to at least 20/24 matched (remaining differences may be due to Liquid filter issues tracked in issue #248)
- [ ] Total DOM diff count drops from 106 to 30 or fewer
- [ ] DTC DOM match count stays at 788/790 or better (no regressions)
- [ ] No site-specific hardcoding -- the fix must be generic Jekyll layout resolution behavior

## Test Scenarios

### Unit: Layout chain resolution
- Given a page with `layout: page` and a `page` layout with `layout: default`, verify the full chain is resolved in order: content -> page -> default
- Given a page with `layout: post` and a `post` layout with `layout: default`, verify the layout chain is applied
- Given a page with no layout, verify it renders without layout wrapping

### Unit: Template rendering in layout context
- Verify that Liquid tags in layouts (`{% if %}`, `{% for %}`, `{{ site.name }}`) are evaluated during rendering
- Verify that `{{ content }}` in a layout is replaced with the rendered child content
- Verify that `{% include %}` tags within layouts resolve correctly (e.g., `{% include featuredbox.html %}`, `{% include disqus.html %}`)

### Integration: Mediumish site build
- Build the Mediumish site and verify `index.html` contains the `<!DOCTYPE html>` doctype from `default.html`
- Build the site and verify `about.html` has rendered content wrapped in the page layout and default layout
- Build the site and verify a post page (e.g., `customer-service/index.html`) has the post layout with author box and default layout wrapper
- Verify that `categories.html` is non-empty and contains rendered category links
- Verify pagination pages (`page2/index.html`, `page3/index.html`) contain rendered post listings

### Integration: Output verification
- Run DOM comparison and verify at least 20/24 pages match
- Verify total diff count is 30 or fewer
- Run DTC DOM comparison and verify no regression from 788/790

## Output Verification

After building the site, inspect the following pages manually:
- `index.html` must contain `<nav class="navbar">` from the default layout
- `about.html` must contain `<div class="section-title">` from the page layout
- Any post page must contain `<div class="article-post">` from the post layout
- All pages must have `<footer class="footer">` from the default layout
