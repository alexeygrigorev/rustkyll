# Issue 440: SEO/OG meta tag handling for theme sites

## Problem

Several sites have SEO/OG meta tag ordering and content issues in `<head>`.
The `jekyll-seo-tag` plugin generates meta tags in a specific order that
rustkyll doesn't match for complex theme configurations.

## Affected Sites

- aihero (0/2, 178 diffs) — OG tag ordering
- so-simple-theme (0/11, 624 diffs) — author data corruption in meta tags
- basically-basic (0/7, 399 diffs) — title mismatch, OG ordering

## Root Cause

`src/template/seo_tag.rs` doesn't handle complex `site.author` objects
(maps with name/email/twitter/links) correctly. Data is serialized as
`__key_order...` strings instead of properly rendered.

## Scope

Fix SEO tag rendering for complex author/site config objects.
