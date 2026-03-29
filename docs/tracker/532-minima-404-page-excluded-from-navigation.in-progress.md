# Issue 532: 404.html page incorrectly appears in site navigation

## Problem

Rustkyll includes `404.html` in the site header navigation menu. Jekyll's minima theme
(v3) excludes pages whose `title` is not set from the navigation, and `404.html` has
no `title` front matter key.

### Example

Jekyll (correct -- only "About" in nav):
```html
<div class="nav-items">
  <a class="nav-item" href="/about/">About</a>
</div>
```

Rustkyll (wrong -- 404 appears):
```html
<div class="nav-items">
  <a class="nav-item" href="/about/">About</a>
  <a class="nav-item" href="/404.html">404</a>
</div>
```

### Affected pages

All 9 minima pages (the navigation is rendered in the site header on every page).

## Root Cause

The minima theme's `_includes/header.html` iterates over `site.pages` and filters
by `page.title`. Pages without a title are excluded. Rustkyll's handling of this
iteration may not correctly filter out pages with nil/absent titles, or it may be
auto-generating a title from the filename.

Investigate:
1. Does rustkyll auto-assign `title: "404"` to 404.html? (Jekyll does not)
2. Does the Liquid `{% if my_page.title %}` correctly evaluate to false for nil titles?
3. Does `site.pages` include 404.html in both Jekyll and rustkyll?

## Dependencies

None.

## Scope

- Fix page title handling so pages without explicit `title:` in front matter have nil title
- OR fix the `site.pages` filtering to match Jekyll's behavior
- Ensure pages with explicit titles still appear in navigation

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes
- [ ] DTC DOM baseline: 790/790 (must not regress)
- [ ] Minima: 404.html does NOT appear in the navigation on any page
- [ ] Minima: "About" page still appears in navigation
- [ ] Pages with explicit `title:` front matter still included in navigation
- [ ] At least 2 new unit tests

## Test Scenarios

### Unit: page title nil handling
- Page with no `title:` in front matter -> `page.title` is nil in Liquid context
- Page with `title: "About"` -> `page.title` is "About"
- `{% if page.title %}` evaluates to false for page without title

### Integration: minima build
- Build minima, verify header nav contains only "About" link
- Verify 404.html is NOT linked in the header nav

## Baselines

- DTC: 790/790
- Minima: 0/9 (this fix should eliminate 1 diff per page = 9 diffs)
