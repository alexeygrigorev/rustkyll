# Issue 172: Fix related posts ordering to match Jekyll

## Problem

On large-blog-3000, every single one of 3,001 files differs because the "related posts" links point to different posts than Jekyll produces. Jekyll's `site.related_posts` returns the 10 most recent posts (sorted by date descending), but rustkyll produces a different ordering.

This causes 119,860 total DOM diffs (the largest diff count of any site) -- all from `text_differs` and `attribute_differs` in the related post links.

## Root cause

The related posts list in rustkyll is either:
1. Not sorted by date descending, or
2. Using a different tiebreaking mechanism than Jekyll when dates are equal (many synthetic posts share the same date)

Jekyll sorts related posts by date descending, then by slug/path alphabetically for posts with the same date.

## Affected sites

| Site | Files affected | Diffs |
|------|---------------|-------|
| large-blog-3000 | 3001/3001 | 119,860 |
| mojombo-blog | 1/17 | ~3 (one page has wrong related post date/href) |

## Acceptance criteria

- [ ] `site.related_posts` returns posts sorted by date descending, with stable tiebreaking matching Jekyll (alphabetical by slug/path for same-date posts)
- [ ] large-blog-3000 related post links match Jekyll output (spot-check 5 pages)
- [ ] The current post is excluded from its own related posts list
- [ ] Existing tests continue to pass

## Dependencies

Depends on issue #42 (site.related_posts) which is already done.

## Log

### [SWE] 2026-03-17

**Root cause analysis:**
1. `build_related_posts` used ascending slug tiebreaking (`a.slug.cmp(&b.slug)`) for same-date posts. Jekyll uses descending slug ordering (matching `site.posts` reverse chronological order).
2. `build_categories_and_tags` iterated posts in load order (date ascending) and pushed into per-category/per-tag vecs. Jekyll exposes `site.categories[cat]` and `site.tags[tag]` in reverse chronological order (newest first). This was the main cause of the 3001-file diff on large-blog-3000 (which uses `site.categories[cat]` in post templates, not `site.related_posts` directly).

**Fixes applied:**
- `build_related_posts`: Changed tiebreaking from `a.slug.cmp(&b.slug)` (ascending) to `b.slug.cmp(&a.slug)` (descending) to match Jekyll's ordering
- `build_categories_and_tags`: Added `.reverse()` to each category and tag post array so posts appear newest-first, matching Jekyll

**Per-post exclusion (AC #3):** The current architecture uses a shared `CachedSiteContext` across all page renders, making per-post `site.related_posts` exclusion non-trivial without significant engine changes. The large-blog-3000 template does NOT use `site.related_posts` (it uses `site.categories[cat]` and `site.tags[tag]` with explicit `if post.url != page.url` guards). The DTC site's related-posts.html include also self-excludes. This criterion may need a follow-up issue if a test site actually depends on it.

**Tests added:** 3 new unit tests
- `test_related_posts_tiebreaking_same_date_by_slug_descending`
- `test_categories_posts_sorted_reverse_chronological`
- `test_tags_posts_sorted_reverse_chronological`

**Build:** All tests pass (20 suites, 0 failures), clippy clean, fmt clean
**Files modified:** `src/generator.rs`
