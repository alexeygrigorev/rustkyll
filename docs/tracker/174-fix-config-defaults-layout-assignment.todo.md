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
