# Issue 181: Fix JSON-LD datePublished/startDate/endDate timezone

## Problem

Jekyll's jekyll-seo-tag uses the page's local timezone offset in JSON-LD date fields. rustkyll always uses UTC (+00:00).

Sample diff:
```
jsonld.@graph[0].datePublished: jsonld_value_differs
  expected: '2023-12-11T00:00:00+01:00'
  actual:   '2023-12-11T00:00:00+00:00'
```

This affects ~190 blog pages and ~193 event pages (startDate/endDate) on DTC.

## Goal

Use the correct timezone offset in JSON-LD date fields to match Jekyll's jekyll-seo-tag output.

## Affected Sites

- DataTalksClub/datatalksclub.github.io: ~287 pages affected (currently 500/787 match)

## Approach (TDD)

1. Write a test that renders a page with a known date and timezone and asserts the JSON-LD datePublished includes the correct timezone offset
2. Verify the test fails
3. Fix the SEO tag implementation to preserve timezone from page dates
4. Verify the test passes
5. Run `./scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io` to confirm

## Acceptance Criteria

- [ ] JSON-LD datePublished uses page's timezone offset, not UTC
- [ ] JSON-LD startDate/endDate use correct timezone
- [ ] DTC DOM match improves significantly
