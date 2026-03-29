# Issue 512: government-github -- `page.id` for posts should use permalink-based path

## Problem

For posts with custom permalinks, `page.id` in rustkyll uses the date-based path (`/YYYY/MM/DD/slug`) instead of matching Jekyll's behavior. Jekyll computes `page.id` as the URL without trailing slash for posts, which respects the configured permalink pattern.

This breaks 10 story redirect pages on the government-github site. The `case_study_redirect` layout uses:
```liquid
{% capture url %}...{{ page.id | replace:"/stories/", "" }}.md{% endcapture %}
```

With the site's permalink config `permalink: "/stories/:title"`:
- Jekyll: `page.id` = `/stories/canadian-web-experience-toolkit` -> after replace = `canadian-web-experience-toolkit`
- Rustkyll: `page.id` = `/2013/10/14/canadian-web-experience-toolkit` -> after replace = `/2013/10/14/canadian-web-experience-toolkit` (unchanged, wrong)

## Affected Pages (10 pages, 4 diffs each = 40 total)

All 10 `stories/*.html` pages:
- stories/canadian-web-experience-toolkit.html
- stories/design-a-street-with-streetmix.html
- stories/forking-your-city.html
- stories/gds-source-control.html
- stories/modern-approach-to-open-data.html
- stories/opening-up-informatics-for-cancer-research.html
- stories/philadelphia-gets-going-and-gets-open.html
- stories/project-open-data.html
- stories/public-private-collaborations.html
- stories/xas-software.html

Each page has 4 diffs: the link href, meta refresh content, anchor href, and script location all contain the wrong path.

## Root Cause

In `src/collection.rs` around line 931, the `id` for posts is computed as:
```rust
format!("/{}/{}/{}/{}", parts[0], parts[1], parts[2], raw_slug)
```
This always uses the date-based format regardless of the site's permalink configuration. Jekyll instead computes `Document#id` based on the URL (the permalink-resolved path).

## Solution

Change the `id` computation for collection items (at least posts) to use the URL-based path (with trailing slash/extension stripped) instead of always using the date-based format. Specifically, for posts, `page.id` should be the URL with any trailing `/` or `.html` removed, matching Jekyll's `Document#url.chomp('/')` behavior.

IMPORTANT: This must be regression-tested against other sites that use `page.id` (e.g., Type theme issue #357, beautiful-jekyll issue #455). The date-based format may be correct for the default permalink (`/:categories/:year/:month/:day/:title`), so the fix should respect the configured permalink pattern.

## Acceptance Criteria

- [ ] For posts with custom permalink `/stories/:title`, `page.id` = `/stories/<slug>`
- [ ] For posts with default permalink, `page.id` = `/<YYYY>/<MM>/<DD>/<slug>` (unchanged)
- [ ] All 10 `stories/*.html` redirect pages produce correct URLs
- [ ] The `replace:"/stories/", ""` filter correctly extracts just the slug
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM baseline must not drop below 790/790
- [ ] government-github DOM score improves by +10 pages (stories)

## Test Scenarios

### Unit: page.id computation
- Post with permalink `/stories/:title` and slug `my-post` -> `page.id` = `/stories/my-post`
- Post with default permalink and date 2024-01-15 and slug `my-post` -> `page.id` = `/2024/01/15/my-post`
- Post with permalink `/:year/:month/:title` -> `page.id` = `/2024/01/my-post`
- Non-post collection item -> `page.id` = `/<collection>/<slug>` (unchanged)

### Integration: government-github redirect pages
- Build government-github site, verify stories/canadian-web-experience-toolkit.html contains `case-studies/canadian-web-experience-toolkit.md` (not date path)
- Build government-github site, verify all 10 story pages have matching redirect URLs
- Verify no regression on sites using `page.id` conditionals (Type theme, beautiful-jekyll)

## Dependencies

- Related to issue #357 (Type theme page.id) -- must not regress that fix
- Related to issue #455 (beautiful-jekyll page.id truthy) -- must not regress
