# Issue 174: Fix _config.yml defaults layout assignment for non-post pages

## Problem

Pages that rely on `_config.yml` `defaults:` to set their layout (without explicit `layout:` in front matter) are rendered with the wrong layout or no layout wrapper.

On large-docs-site, 800 out of 801 pages are affected: Jekyll wraps content in `<article class="doc">` via the `doc` layout (assigned by `defaults:` scope matching path `docs`), but rustkyll renders content directly into `<main>` without the `<article>` wrapper.

## Root cause

The large-docs-site `_config.yml` has:
```yaml
defaults:
  - scope:
      path: "docs"
    values:
      layout: "doc"
  - scope:
      path: ""
    values:
      layout: "default"
```

Pages under `docs/` have no `layout:` in their front matter. Jekyll correctly assigns `layout: doc` from the defaults. Rustkyll appears to either:
1. Not apply the most-specific matching default (path "docs" should override path "")
2. Apply the default layout but not chain it correctly (doc layout has `layout: default` which should nest)

## Affected sites

| Site | Files affected | Pattern |
|------|---------------|---------|
| large-docs-site | 800/801 | Missing `<article class="doc">` wrapper |
| so-simple-theme | 11/11 | Missing layout (55 files only in Jekyll, likely similar cause) |

## Acceptance criteria

- [ ] Pages under `docs/` in large-docs-site get `layout: doc` from defaults
- [ ] The `doc` layout chains to `default` layout (nested layouts work)
- [ ] Content is wrapped in `<article class="doc">` as Jekyll produces
- [ ] Spot-check: `large-docs-site/docs/api-reference/page-1.html` matches Jekyll DOM structure
- [ ] Existing tests continue to pass

## Dependencies

Depends on issue #28 (front-matter-defaults) which is already done. This is a regression or incomplete implementation.

## Log

### [SWE] 2026-03-17

- **Root cause identified**: Both `defaults_for()` and `defaults_for_page()` in `src/config.rs` applied defaults in declaration order (later entries override earlier ones). Jekyll instead applies defaults by specificity -- more specific scopes (longer path prefix, having a type) override less specific ones regardless of declaration order. In the large-docs-site config, `path: "docs"` (specific) was declared before `path: ""` (catch-all), so the catch-all was winning and assigning `layout: "default"` instead of `layout: "doc"`.
- **Fix**: Added a `scope_specificity()` function that scores scopes by path length + type presence. Both `defaults_for()` and `defaults_for_page()` now collect matching defaults, sort by (specificity, declaration_order) ascending, then apply in that order so most-specific values win.
- **Tests added**: 7 new unit tests covering:
  - Specific path wins over empty path (both declaration orders)
  - Root page correctly gets less-specific default
  - Type-scoped default wins over path-only
  - Collection items also respect specificity
  - Three-level path specificity
  - Non-overlapping values merge across specificity levels
- **Build**: 1368 tests pass, 0 fail, clippy clean, fmt clean
- **Files modified**: `src/config.rs`
