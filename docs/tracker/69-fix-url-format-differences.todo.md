# Issue 69: Fix URL format differences between rustkyll and Jekyll

## Problem

Structural comparison (issue #61) found URL format differences between rustkyll and Jekyll output. For example `/articles` vs `/articles.html` in canonical URLs. These are NOT minor — wrong canonical URLs break SEO, wrong link formats break navigation.

## Goal

rustkyll must produce the exact same URLs as Jekyll for all pages. No format differences (trailing slash, .html extension, etc.) should exist.

## Approach

1. Run the structural comparison and collect all URL differences
2. Identify the root cause — likely in permalink generation, url_to_output_path, or canonical URL construction
3. Fix the URL generation to match Jekyll exactly
4. Re-run structural comparison and verify 0 URL format differences

## Sites to verify

- DataTalksClub/datatalksclub.github.io
- kids-horror-stories-ru

## Dependencies

- Issue 61 (structural comparison) done

## Acceptance criteria

- Structural comparison shows 0 URL format differences for both sites
- Canonical URLs in `<link rel="canonical">` match Jekyll exactly
- `<a href>` links in navigation match Jekyll exactly
- Permalink generation matches Jekyll's behavior for all permalink styles (date, pretty, none, custom)
- Sitemap URLs match Jekyll's sitemap URLs exactly
- All existing tests still pass
