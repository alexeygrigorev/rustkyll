# Issue 545: made-mistakes _pages discovery and v2 pagination verification

## Problem

The made-mistakes-jekyll site has pages in `src/_pages/` (e.g., `articles.md`, `notes.md`, `mastering-paper.md`) that use jekyll-paginate-v2 per-page pagination with category filtering. These pages are not being discovered/loaded by rustkyll, so v2 pagination cannot generate paginated output for them.

This was identified during acceptance review of issue #482 (jekyll-paginate-v2). The v2 pagination code is correct (verified on al-folio), but made-mistakes pages are not loaded at all.

## Acceptance Criteria

- [ ] `articles/index.html` is generated with posts filtered to category "articles"
- [ ] `notes/index.html` is generated with posts filtered to category "notes"
- [ ] `mastering-paper/index.html` is generated with posts filtered to category "mastering-paper"
- [ ] If articles has >15 posts, `articles/page/2/index.html` exists
- [ ] DTC DOM match count does not regress

## Dependencies

- Issue #482 (jekyll-paginate-v2 -- DONE)

## Origin

Descoped from issue #482 acceptance criteria 9 and 10.
