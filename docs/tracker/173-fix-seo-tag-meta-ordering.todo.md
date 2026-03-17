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
