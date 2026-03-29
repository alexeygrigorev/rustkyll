# Issue 507: Generate al-folio year/tag/category archive pages (jekyll-archives)

## Problem

al-folio uses `jekyll-archives` (specifically `jekyll-archives-v2`) to generate year, tag, and category archive pages. rustkyll does not generate these, resulting in 46+ pages missing from the output:

- **Year archives** (6 pages): `blog/2015/index.html`, `blog/2020/index.html`, `blog/2021/index.html`, `blog/2022/index.html`, `blog/2023/index.html`, `blog/2024/index.html`, `blog/2025/index.html`
- **Tag archives** (17 pages): `blog/tag/audios/`, `blog/tag/bib/`, `blog/tag/blockquotes/`, `blog/tag/charts/`, `blog/tag/citation/`, `blog/tag/code/`, `blog/tag/comments/`, `blog/tag/diagrams/`, `blog/tag/distill/`, `blog/tag/formatting/`, `blog/tag/google/`, `blog/tag/images/`, `blog/tag/jupyter/`, `blog/tag/links/`, `blog/tag/maps/`, `blog/tag/math/`, `blog/tag/medium/`, `blog/tag/sidebar/`, `blog/tag/tables/`, `blog/tag/toc/`, `blog/tag/videos/`
- **Category archives** (3 pages): `blog/category/external-posts/`, `blog/category/external-services/`, `blog/category/sample-posts/`
- **Books archives**: `books/2024/index.html`, `books/category/classics/`, etc. (8 pages)

## Relationship to Issue 480

Issue #480 already tracks the generic `jekyll-archives` plugin implementation. This issue is specifically about verifying that the implementation works for al-folio's configuration, which includes:

- **Multi-collection archives**: al-folio configures archives for both `posts` and `books` collections
- **Custom permalink patterns**: `year: "/blog/:year/"`, tags/categories: `"/blog/:type/:name/"`
- **`jekyll-archives-v2`**: al-folio uses the v2 variant; verify compatibility

If issue #480 is not yet done, this issue depends on it. If #480 is done, this issue verifies al-folio-specific behavior.

## Baseline

- al-folio files: 60/108 (rustkyll generates 60, Jekyll generates 108)
- DTC DOM baseline: 790/790

## Acceptance Criteria

- [ ] Year archive pages are generated for al-folio blog posts (e.g., `blog/2015/index.html`).
- [ ] Tag archive pages are generated (e.g., `blog/tag/code/index.html`).
- [ ] Category archive pages are generated (e.g., `blog/category/sample-posts/index.html`).
- [ ] Books collection archives are generated (e.g., `books/category/classics/index.html`).
- [ ] The al-folio file coverage increases from 60/108 to at least 90/108.
- [ ] DTC DOM match count does not drop below 790/790.

## Test Scenarios

### Integration: archive page generation
- Build al-folio and verify year archive pages exist and contain links to posts from that year.
- Verify tag archive pages list posts with the corresponding tag.
- Verify category archive pages list posts with the corresponding category.
- Verify books collection archives are generated separately from blog archives.

## Dependencies

- Issue #480 (jekyll-archives plugin implementation)
- Issue #235 (al-folio site is set up)
- Issue #505 (layout support needed for archives to render correctly)
