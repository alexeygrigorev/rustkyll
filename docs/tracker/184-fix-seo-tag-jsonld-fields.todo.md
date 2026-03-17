# Issue 184: Fix jekyll-seo-tag JSON-LD field accuracy

## Problem

rustkyll's SEO tag JSON-LD output differs from Jekyll's jekyll-seo-tag in several fields:
- `@type` is `WebPage` when Jekyll uses `WebSite` (or vice versa)
- Missing `url` field in JSON-LD
- `name` field differences (page title vs site title)

Sample diff (theme sites):
```
jsonld.@type: jsonld_value_differs - expected: 'WebSite', actual: 'WebPage'
jsonld.url: jsonld_missing_field - expected: '"/"', actual: '(none)'
jsonld.name: jsonld_extra_field - expected: '(none)', actual: '"Architect theme"'
```

## Goal

Match jekyll-seo-tag's JSON-LD output field-for-field.

## Affected Sites

- All 9 theme sites (architect, cayman, dinky, hacker, midnight, merlot, slate, time-machine, leap-day)
- DTC (some pages)

## Approach (TDD)

1. Write tests for @type selection logic, url field inclusion, name field logic
2. Verify tests fail
3. Fix SEO tag implementation
4. Verify tests pass
5. Recount theme sites

## Acceptance Criteria

- [ ] `@type` matches jekyll-seo-tag logic (WebSite for homepage, WebPage for other pages, or as jekyll-seo-tag decides)
- [ ] `url` field included when jekyll-seo-tag includes it
- [ ] `name` field matches jekyll-seo-tag behavior
- [ ] Theme sites show improvement in DOM match
