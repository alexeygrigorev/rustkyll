# Issue 239: Support Mediumish Jekyll theme

## Problem

Mediumish is a popular Jekyll theme that provides a Medium.com-like blogging experience. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/wowthemesnet/mediumish-theme-jekyll
- **Stars:** ~2,000
- **Use case:** Blogs, content publishing
- **Notable features:** Medium-like design, featured posts, author boxes, lazy-loaded images, related posts, categories, SEO-oriented layouts, newsletter integration

## Scope

1. Clone the Mediumish demo site into `websites/mediumish/`.
2. Build the cloned site with both Jekyll and rustkyll.
3. Run DOM comparison against the Jekyll reference output and record the actual match rate.
4. Identify Mediumish-specific rendering blockers and either fix them or create follow-up issues that reference this issue.

## Baseline

- DTC DOM baseline: `316/790`

## Acceptance Criteria

- [ ] The Mediumish demo site is cloned into `websites/mediumish/` and the repository state is documented in the issue log.
- [ ] Jekyll builds the demo site successfully and produces a reference `_site` output.
- [ ] rustkyll builds the demo site successfully without errors and produces HTML output for the same site.
- [ ] The DOM comparison between the Jekyll and rustkyll outputs is run and the issue log records the exact match count, differing-file count, and main diff categories.
- [ ] Representative pages that exercise Mediumish features are verified in the output, including the homepage, featured posts, individual posts, author boxes, categories, related posts, lazy-loaded images, and newsletter content if present in the demo.
- [ ] Any Mediumish-specific rendering issues discovered during comparison are either fixed in this issue or explicitly tracked in follow-up issues that reference `#239`.
- [ ] The DTC DOM match count does not drop below `316/790`.

## Test Scenarios

### Integration: demo site setup
- Clone the upstream Mediumish demo site into `websites/mediumish/` and verify the expected theme files and configuration are present.
- Run `jekyll build` in the demo site and confirm the reference HTML output is generated.
- Run `rustkyll build --source websites/mediumish --destination /tmp/mediumish-rustkyll` and confirm the same page set is generated.

### Integration: output comparison
- Run DOM comparison against the Jekyll reference output and record the exact match count, differing-file count, and notable diff categories.
- Inspect representative pages for featured posts, author box rendering, category pages, related-posts sections, image loading markup, and newsletter-related content if present in the demo.
- Verify any identified rendering blocker is either fixed or captured in a follow-up issue linked to `#239`.

## Dependencies

- None (research/benchmark task)

## Log

### [PM] 2026-03-24
- Groomed the issue into a benchmark-oriented spec for the Mediumish demo site.
- Recorded DTC baseline: `316/790`.
- Added explicit output verification requirements, representative feature coverage, and traceable follow-up handling for any blockers.
