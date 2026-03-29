# Issue 516: where filter boolean-vs-string comparison breaks pin-based post sorting

## Problem

Chirpy's home layout uses `site.posts | where: 'pin', 'true'` to filter pinned posts.
In the frontmatter, `pin: true` is a YAML boolean (not a string). Jekyll's `where`
filter coerces the comparison value and matches boolean `true` against string `'true'`.
Rustkyll's `where` filter does a strict type comparison, so `true` (bool) != `'true'`
(string), and the filter returns no results (or wrong results).

This causes the chirpy homepage to show posts in the wrong order -- pinned posts are
not separated from normal posts, leading to 11 cascading diffs on the index page.

### Example

```yaml
# frontmatter
pin: true
```

```liquid
{% assign pinned = site.posts | where: 'pin', 'true' %}
```

**Jekyll:** Returns posts where pin is true (boolean or string)
**Rustkyll:** Returns empty array (boolean true != string 'true')

### Affected Pages

- chirpy: `index.html` (11 diffs: wrong post order, wrong titles, dates, hrefs)

## Root Cause

The `where` filter in rustkyll does strict equality comparison. Jekyll's `where` filter
performs type coercion: it converts both sides to strings before comparing, so
`true.to_s == "true"` matches.

## Fix

In the `where` filter implementation, when comparing values:
1. If one side is a boolean and the other is a string, convert the boolean to string
   before comparing (i.e., `true` matches `"true"`, `false` matches `"false"`)
2. This should also handle the reverse case: `where: 'field', true` matching string
   `"true"` values

This is a generic fix that benefits any site using boolean frontmatter with string
filter arguments (common pattern).

## Dependencies

None.

## Baseline

- DTC: 790/790 (must not regress)
- Chirpy: 12/17 (should not regress; index page should improve)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `where: 'pin', 'true'` matches items where pin is boolean `true`
- [ ] `where: 'pin', true` matches items where pin is string `"true"`
- [ ] `where: 'hidden', 'false'` matches items where hidden is boolean `false`
- [ ] Existing where filter behavior for string-to-string and number comparisons unchanged
- [ ] DTC DOM baseline remains at 790/790
- [ ] Chirpy DOM match count does not drop below 12/17

## Test Scenarios

### Unit: boolean-string coercion in where filter
- Array of items with `pin: true` (bool), filter `where: 'pin', 'true'` -- verify match
- Array of items with `pin: "true"` (string), filter `where: 'pin', true` -- verify match
- Array of items with `hidden: false` (bool), filter `where: 'hidden', 'false'` -- verify match
- Array of items with `pin: true` (bool), filter `where: 'pin', 'false'` -- verify no match
- Existing string comparison: `where: 'category', 'blog'` -- verify still works

### Integration: chirpy homepage
- Build chirpy, verify pinned posts appear first on index.html
- Verify post order matches Jekyll output (Customize the Favicon before Getting Started)
