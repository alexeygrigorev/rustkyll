# Issue 63: RSS/Atom feed and sitemap validation

## Problem

Issues #49 required RSS/Atom feed validation and sitemap comparison but these were never tested.

## Goal

Validate that rustkyll produces correct RSS/Atom feeds and sitemaps that match Jekyll's output.

## Sites to validate

- DataTalksClub/datatalksclub.github.io
- kids-horror-stories-ru

## Acceptance criteria

- RSS/Atom feed files are valid XML (parse without errors)
- Feed contains the expected number of entries (within 5% of Jekyll's feed)
- Feed entries have correct titles, links, dates, and content snippets
- Sitemap is valid XML
- Sitemap lists the same URLs as Jekyll's sitemap (within 5% tolerance)
- No broken URLs in sitemap (all listed URLs correspond to actual generated HTML files)
- Results documented

## Dependencies

None
