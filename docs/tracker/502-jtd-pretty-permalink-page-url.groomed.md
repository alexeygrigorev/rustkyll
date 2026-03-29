# Issue 502: Fix page.url for `permalink: pretty` in Liquid templates

## Problem

When a site has `permalink: pretty` in `_config.yml`, Jekyll generates URLs without `.html` extensions (e.g., `/docs/configuration/` instead of `/docs/configuration.html`). Rustkyll generates the correct directory structure (files at `docs/configuration/index.html`) but the `page.url` Liquid variable still returns `.html`-suffixed URLs.

This causes internal links in templates that use `{{ page.url }}` or iterate over `site.pages` to produce wrong href values.

### Example

**Jekyll** (correct): `href='/docs/configuration/'`
**Rustkyll** (broken): `href='/docs/configuration.html'`

### Affected Pages (about 10 pages in just-the-docs)

Pages that contain internal links using `page.url` or `site.pages[].url`:
- docs/navigation/auxiliary/index.html -- link to `/docs/configuration/#aux-links`
- docs/navigation/main/index.html -- links to order, exclude, levels pages
- docs/navigation/main/external/index.html -- link to `/docs/configuration/`
- docs/ui-components/callouts/index.html -- link to `/docs/configuration/#callouts`
- index.html -- link to `/CHANGELOG/`
- Multiple other pages with cross-references

## Root Cause

The `page.url` property is not being adjusted when `permalink: pretty` is configured. The URL generation produces `/path/to/page.html` instead of `/path/to/page/`.

Note: This overlaps with issue #347 (Jasper2 pretty permalink) which describes the same underlying bug for a different site. This issue is generic and should fix it for all sites with `permalink: pretty`.

## Dependencies

None (but should check if #347 is still open -- they share the same root cause).

## Baseline

- just-the-docs: 1/47 (or higher if #501 is fixed first)
- DTC: 790/790 (must not regress)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] When `permalink: pretty`, `page.url` returns `/path/to/page/` not `/path/to/page.html`
- [ ] Internal links in just-the-docs templates use extensionless URLs
- [ ] DTC DOM baseline remains at 790/790

## Test Scenarios

### Unit: URL generation with pretty permalinks
- Site with `permalink: pretty`, page at `docs/config.md` -- verify `page.url` is `/docs/config/`
- Site with `permalink: pretty`, page at `index.md` -- verify `page.url` is `/`
- Site with default permalink -- verify `page.url` still has `.html`

### Integration: just-the-docs links
- Build just-the-docs, check that `href='/docs/configuration/'` (not `.html`) in navigation
- Verify anchor links like `/docs/configuration/#aux-links` also use pretty format
