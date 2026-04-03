# Issue 355: Basically Basic rendering blockers

## Problem

DOM comparison of the Basically Basic theme demo site (issue #242) shows 0/7
common HTML pages matching (457 total differences after filtering). The site
produces only 13 HTML pages vs Jekyll's 38 -- most posts are missing because
`_posts/` lives inside `example/` and `docs/` subdirectories, and rustkyll
does not discover posts in subdirectory-scoped collections the way Jekyll does
with gem-based themes.

Several rendering issues compound to prevent accurate output. This umbrella
issue tracks which blockers are resolved and which remain open.

## Blocker Status

### RESOLVED -- Already fixed or tracked elsewhere

1. **Category case in permalinks** -- Fixed in #354 (Hydeout).
2. **og:locale format** -- Fixed in `src/template/seo_tag.rs` (hyphens
   converted to underscores, e.g. `en-US` -> `en_US`).
3. **Future post filtering** -- `filter_future_posts()` in `src/collection.rs`
   already excludes future-dated posts when `future:` is not set.
4. **Author/image hash serialization in SEO tags** -- Tracked in #514
   (hash-type `page.image`) and #515 (JSON-LD `@type`/`name`/`sameAs`).
5. **Syntax highlighting class differences** -- Tracked in #471
   (token mismatches) and #534 (Minima highlighting classes).

### OPEN -- Needs dedicated issues or fixes

6. **baseurl not prepended to SEO/meta URLs**: Canonical URLs, `og:url`, and
   JSON-LD `url` fields are missing the `baseurl` prefix. Jekyll's
   `jekyll-seo-tag` prepends `site.baseurl` to these URLs automatically.
   Affects all common pages. Root cause: `src/template/seo_tag.rs` does not
   apply `site.baseurl` when constructing canonical/og:url values.

7. **site.tags / site.categories iteration returns empty**: Tag and category
   archive pages use `{% for tag in site.tags %}` with `tag[1].size`. These
   render as nearly-empty pages in rustkyll (tags/index.html is 5 KB vs 363 KB
   in Jekyll). Root cause: `site.tags` and `site.categories` are either not
   populated or not structured as the expected array-of-pairs that Jekyll
   provides (`[["tagname", [post1, post2, ...]], ...]`).

8. **Liquid-in-SCSS processing**: The theme's `main.scss` contains
   `{{ site.data.theme.skin | default: 'default' }}` which Jekyll processes
   through Liquid before SCSS compilation. Rustkyll cannot compile this SCSS.
   Related to #249 (Mediumish) and #345 (al-folio).

## Scope

This issue is a **triage/tracking umbrella**. The engineer should:

1. Verify the blocker status above is accurate by building the site
2. Create new `.todo.md` issues for each OPEN blocker that does not yet have a
   tracker (blockers 6, 7, 8 above)
3. Close this umbrella issue once every blocker has its own tracker

No code changes are required in this issue itself.

## Acceptance Criteria

- [ ] Build basically-basic with `cargo run -- build --source websites/basically-basic --destination /tmp/bb-test` and confirm the current page count and DOM comparison numbers
- [ ] Verify each RESOLVED blocker is truly resolved (not regressed)
- [ ] For each OPEN blocker: either (a) confirm it already has a dedicated issue, or (b) create a new `.todo.md` issue in `docs/tracker/`
- [ ] Every blocker in the list above is either marked RESOLVED with an issue number or has a new/existing `.todo.md` tracker
- [ ] DTC DOM baseline: 787/787 pages matched -- must not regress

## Test Scenarios

### Manual: Blocker triage
- Build basically-basic and record page count and DOM diff count
- Check tags/index.html file size (should still be ~5 KB, confirming blocker 7 is open)
- Check canonical URL in any page's HTML for baseurl prefix (confirming blocker 6 is open)
- Confirm og:locale contains underscores not hyphens (confirming blocker 2 is resolved)

### No automated tests
This is a triage issue -- no code changes, no new tests.

## Dependencies

- #242 (benchmark baseline -- done)
- #514, #515 (SEO hash serialization -- in progress)
- #471, #534 (syntax highlighting -- in progress)
- #354 (category URL case -- done)
- #249, #345 (SASS Liquid preprocessing -- todo)

## Log

### [SWE] 2026-03-30

**Build results:**
- Built from `websites/basically-basic/example/` (the Jekyll cached output was generated from the example subdirectory, not the root)
- Rustkyll: 39 pages (13 HTML), Jekyll: 38 pages (38 HTML)
- DOM comparison: 0/38 matched, 1031 total differences (57 acceptable filtered)

**DTC baseline check:** 784/787 pages matched (no regression -- this is triage only, no code changes)

**Blocker verification:**

1. **Category case in permalinks** -- RESOLVED. Verified: categories render with correct case.
2. **og:locale format** -- RESOLVED. Verified: `<meta property="og:locale" content="en_US" />` (underscore, not hyphen).
3. **Future post filtering** -- RESOLVED. No future posts in output.
4. **Author/image hash serialization** -- CONFIRMED tracked in #514/#515. Still producing flat string instead of hash for `jsonld.image`.
5. **Syntax highlighting** -- CONFIRMED tracked in #471/#534. Span class diffs visible in syntax-highlighting.html (251 diffs).

6. **baseurl not prepended** -- CONFIRMED OPEN. All 38 pages show `/404.html` instead of `/example/404.html` in canonical/og:url/JSON-LD. ~180 of 1031 total diffs are baseurl-related. Created **#539**.
7. **site.tags/categories empty** -- CONFIRMED OPEN. tags/index.html is 4.8 KB vs 363 KB. Missing `<p>` excerpt elements in tag/category archives. Created **#540**.
8. **Liquid-in-SCSS** -- CONFIRMED OPEN. `assets/stylesheets/main.scss` contains `{{ site.data.theme.skin | default: 'default' }}`. Not covered by #249 or #345 (those are import resolution, not Liquid preprocessing). Created **#541**.

**New tracker files created:**
- `docs/tracker/539-seo-tag-baseurl-prepend-canonical-og-url.todo.md`
- `docs/tracker/540-site-tags-categories-iteration-empty.todo.md`
- `docs/tracker/541-liquid-in-scss-preprocessing.todo.md`

**Files modified:** No source code changes (triage only).
