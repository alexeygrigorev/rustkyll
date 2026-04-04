# Issue 559: type-theme search.html strip_html produces empty content

## Problem

type-theme is at 7/8 pages (88%) with 1 page having 1 diff. The `search.html` page generates a JavaScript `window.store` object containing post content for client-side search. For the "Markdown and HTML" post (`2014-11-28-markdown-and-html.md`), the `content` field is empty `""` in rustkyll but contains the full stripped text in Jekyll.

The post content goes through `post.content | strip_html` in the search template. In rustkyll, this produces an empty string, suggesting the rendered HTML is structured in a way that `strip_html` eliminates everything.

Additionally, there's a minor whitespace difference in the first post's content: Jekyll has `"  Suspendisse"` (with leading spaces) while rustkyll has `"Suspendisse"` (no leading spaces).

## Root Cause

The `strip_html` filter is removing all content from the markdown-and-html post. The post contains tables, code blocks, and inline HTML which may render differently, causing `strip_html` to produce an empty result. Investigate whether the rendered HTML for this post contains all content, and whether `strip_html` handles it correctly.

## Affected Site

- type-theme: `search.html` (1 diff)
- Fixing this would bring type-theme to 8/8 (100%)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests
- [ ] type-theme DOM comparison shows 8/8 matched (100%)
- [ ] The `window.store` content field for "Markdown and HTML" post contains the full stripped text
- [ ] DTC DOM match count must not drop below 790/790

## Test Scenarios

### Unit: strip_html with complex HTML content
- Test `strip_html` on content containing tables, code blocks, and inline HTML
- Test `strip_html` on content that mixes markdown-rendered HTML with raw HTML elements
- Verify the result is non-empty and contains expected text fragments

### Integration: type-theme build
- Build type-theme and verify search.html contains non-empty content for all posts
- Run DOM comparison to verify 8/8 match

## Dependencies

None

## DTC DOM Baseline

790/790 (100%)

## Log

### [PM] 2026-04-02 10:00
- Groomed from batch site analysis
- type-theme currently at 7/8 (88%), single diff in search.html
- Root cause: strip_html producing empty content for markdown-and-html post

### [SWE] 2026-04-02

**Root Cause Analysis:**
- Posts with non-highlight Liquid tags (e.g., `{{ site.baseurl }}`) have `html_content = ""` in collection loading (deferred to page generation with full site context)
- `build_site_context` builds `site.posts` using `html_content` via `collection_item_to_liquid_slim`
- Since `html_content` is empty, `post.content` in templates is empty string
- `strip_html` on empty string produces empty string
- The `CachedSiteContext` is built ONCE before post rendering and never updated

**Fix 1: Fallback markdown rendering for empty html_content**
- Wrote test: `test_slim_content_fallback_when_html_content_empty_due_to_liquid` (src/generator.rs)
- Ran test: FAILS -- "Content should not be empty for posts with Liquid tags, got empty string"
- Wrote test: `test_slim_content_fallback_unicode_post_with_liquid` (src/generator.rs)
- Ran test: FAILS -- "Unicode content should not be empty"
- Implemented fix in src/generator.rs: `collection_item_to_liquid_slim` now falls back to rendering raw markdown (with highlight pre-processing) when `html_content` is empty for markdown source files
- Made `pre_render_highlight_blocks` public in src/collection.rs
- Ran tests: PASSES (both)

**Summary:**
- Files modified: src/generator.rs, src/collection.rs
- Tests added: 2 unit tests for empty html_content fallback (with and without Unicode)
- Full test suite: all pass
- Clippy: clean (0 warnings)
- Fmt: clean
- type-theme DOM: 8/8 (100%) -- was 7/8
- DTC DOM: 790/790 (100%) with 0 total diffs -- no regression
- DTC build time: 0.81s (under 1.0s threshold)

### [PM] 2026-04-02 16:45
- Reviewed diff: 2 files changed (src/collection.rs, src/generator.rs), 115 insertions, 2 deletions
- Code review: Clean approach -- when html_content is empty (due to deferred Liquid processing) but raw markdown exists, falls back to rendering markdown via pre_render_highlight_blocks + markdown_to_html. Uses Cow to avoid unnecessary allocation on the normal path. Only triggers for .md/.markdown files.
- Output verification: Built type-theme, DOM comparison shows 8/8 (100%), up from 7/8. Built DTC, DOM comparison shows 790/790 (100%), no regression.
- Tests: 2 meaningful unit tests -- one for complex content with tables/code/inline HTML/Liquid tags, one for Unicode content. Both verify non-empty output and specific text fragments.
- Acceptance criteria: all met
  - [x] cargo build compiles without errors
  - [x] cargo test passes with all existing tests
  - [x] type-theme DOM comparison shows 8/8 matched (100%)
  - [x] window.store content field for "Markdown and HTML" post contains full stripped text
  - [x] DTC DOM match count at 790/790 (no regression)
- Follow-up issues: none needed
- VERDICT: ACCEPT
