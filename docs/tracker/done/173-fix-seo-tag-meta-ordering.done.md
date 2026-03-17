# Issue 173: Fix SEO tag meta tag ordering and content

## Problem

All 9 GitHub Pages theme sites (architect, cayman, dinky, hacker, leap-day, merlot, midnight, slate, time-machine) plus aihero, mlbookcamp-page, and opensource-guide produce meta tags in a different order than Jekyll, with some content values swapped between tags.

The pattern is consistent across all theme sites: 14 attribute_differs, 2 text_differs, 2 missing_attribute, 2 extra_attribute per page.

## Root cause

The `{% seo %}` tag implementation (issue #38) generates meta tags in a different order than the jekyll-seo-tag gem. Specifically:

1. Title separator: rustkyll uses `--` (en dash), Jekyll uses `|` (pipe)
2. `<meta name="generator">` tag: Jekyll outputs `Jekyll vX.Y.Z`, rustkyll outputs the page description instead
3. Meta tag ordering: `og:locale`, `og:description`, `og:site_name`, `og:type` are in different positions
4. Some meta tags are missing (`name="description"` vs `property="og:description"` mixup)

Example from architect-theme:
- Jekyll: `title: 'Architect theme | Architect is a theme...'`
- Rustkyll: `title: 'Architect theme'` (missing site description after separator)

## Affected sites

| Site | Files | Diffs per page |
|------|-------|---------------|
| architect-theme | 2 | ~18 each |
| cayman-theme | 2 | ~18 each |
| dinky-theme | 2 | ~18 each |
| hacker-theme | 2 | ~18 each |
| leap-day-theme | 2 | ~18 each |
| merlot-theme | 2 | ~18 each |
| midnight-theme | 2 | ~18 each |
| slate-theme | 2 | ~18 each |
| time-machine-theme | 2 | ~18 each |
| alexeygrigorev/aihero | 2 | 5 each |
| opensource-guide (index pages) | ~52 | ~30 each |

## Acceptance criteria

- [ ] Title tag uses `|` separator (matching `jekyll-seo-tag` default), not `--`
- [ ] `<meta name="generator" content="Jekyll vX.Y.Z">` is emitted (or omitted consistently)
- [ ] Meta tag order matches jekyll-seo-tag output: title, description, og:title, og:locale, og:description, og:site_name, og:type
- [ ] architect-theme index.html has zero DOM diffs in `<head>` meta tags (spot-check)
- [ ] Existing tests continue to pass

## Dependencies

Depends on issue #38 (jekyll-seo-tag) which is already done.

## Log

### [SWE] 2026-03-17
- Analyzed jekyll-seo-tag gem v2.8.0 template.html and drop.rb to understand exact output format
- Built architect-theme with Jekyll to verify expected output
- Root causes identified and fixed in src/template/seo_tag.rs:
  1. Title separator: changed from en-dash to `|` (TITLE_SEPARATOR constant)
  2. Title logic: when page_title == site_title, now appends site_tagline_or_description (matching Jekyll)
  3. Added `<meta name="generator" content="Jekyll v4.4.1" />` tag
  4. og:title now uses page_title only (not combined title with separator)
  5. Reordered all meta tags to match Jekyll template: title, generator, og:title, author, og:locale, description+og:description, canonical+og:url, og:site_name, og:image, og:type, twitter:card, twitter:image, twitter:title, twitter:site, JSON-LD
  6. Added both `name="description"` and `og:description` together (were in wrong positions)
  7. Added `twitter:title` and `twitter:image` tags (were missing)
  8. Added `<!-- Begin/End Jekyll SEO tag -->` comment markers
  9. Extracted `absolute_image_url()` helper to reduce duplication
- Tests: 9 new tests added (45 total SEO tag tests, up from 36)
  - test_title_page_equals_site_title_with_description
  - test_title_page_equals_site_title_no_description
  - test_og_title_uses_page_title_only
  - test_og_title_falls_back_to_site_title
  - test_meta_tag_order_matches_jekyll
  - test_generator_meta_tag
  - test_twitter_title_present
  - test_twitter_image_present_with_image
  - test_begin_end_comments
  - test_description_and_og_description_emitted_together
  - test_architect_theme_like_output
- Updated existing tests to use `|` separator instead of `&ndash;`
- Removed test_title_page_contains_site_title (replaced with more accurate tests)
- Build: 1612 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: src/template/seo_tag.rs, docs/tracker/173-fix-seo-tag-meta-ordering.in-progress.md
