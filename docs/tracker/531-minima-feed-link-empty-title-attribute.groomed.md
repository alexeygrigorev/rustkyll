# Issue 531: Feed link tag emits empty title="" attribute when site.title is absent

## Problem

When `site.title` is not configured, rustkyll emits the Atom feed link tag with an
empty `title=""` attribute. Jekyll omits the `title` attribute entirely when it has
no value.

### Example

Jekyll (correct):
```html
<link type="application/atom+xml" rel="alternate" href="/feed.xml" />
```

Rustkyll (wrong):
```html
<link type="application/atom+xml" rel="alternate" href="/feed.xml" title="" />
```

### Affected pages

All 9 minima pages (every page includes the feed link in `<head>`).

## Root Cause

The feed link rendering code in the layout/head template emits `title="{{ site.title }}"`.
When `site.title` is nil/empty, this produces `title=""` instead of omitting the attribute.

## Dependencies

None.

## Scope

- Fix the feed link tag to omit the `title` attribute when the title is empty/nil
- This is in the minima theme's `head.html` include or the generic feed link rendering

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes
- [ ] DTC DOM baseline: 790/790 (must not regress)
- [ ] Minima: feed link tag has no `title` attribute when `site.title` is absent
- [ ] Sites with `site.title` set: feed link tag still includes `title="Site Title"`
- [ ] At least 2 new unit tests

## Test Scenarios

### Unit: feed link rendering
- Site with no title -> feed link has no title attribute
- Site with title "My Blog" -> feed link has `title="My Blog"`

### Integration: minima build
- Build minima, verify all pages have `<link ... href="/feed.xml" />` without `title=""`

## Baselines

- DTC: 790/790
- Minima: 0/9 (this fix should eliminate 1 diff per page = 9 diffs)
