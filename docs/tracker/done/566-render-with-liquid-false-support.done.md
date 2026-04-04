# Issue 548: Support `render_with_liquid: false` front matter flag

## Problem

Jekyll supports a `render_with_liquid: false` front matter flag that tells the engine to skip Liquid template processing for a page's content. The content is passed through to the layout as-is (still goes through Markdown rendering and layout wrapping, but no Liquid tag/filter evaluation).

Rustkyll does not recognize this flag. When a page has `render_with_liquid: false`, rustkyll produces an empty output (just a newline character), because the Liquid engine likely encounters `{{ }}` or `{% %}` patterns in the raw content and fails or produces empty output.

## Impact

Chirpy's `posts/write-a-new-post/index.html` is completely empty (1 byte) instead of the expected ~80KB page. This post contains extensive Liquid syntax examples that are meant to be displayed as code, not executed.

This is a standard Jekyll feature documented at https://jekyllrb.com/docs/liquid/tags/ and used by any site that documents Liquid syntax.

## Root Cause

The rendering pipeline always passes page content through the Liquid engine. There is no check for the `render_with_liquid` front matter variable.

## Scope

- Check for `render_with_liquid: false` in front matter before Liquid processing
- When false, skip Liquid template rendering of the page content (but still apply Markdown conversion, layout wrapping, and other processing)
- This does NOT affect includes or layouts -- only the page's own content is skipped

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new ones
- [ ] A page with `render_with_liquid: false` and Liquid syntax in content renders the Liquid syntax as literal text
- [ ] The page still gets its layout applied (the content is inserted into the layout via `{{ content }}`)
- [ ] The page's Markdown is still converted to HTML
- [ ] Chirpy `posts/write-a-new-post/index.html` generates a non-empty page (~80KB)
- [ ] DTC DOM match count must not drop below 790/790

## Test Scenarios

### Unit: render_with_liquid flag parsing
- Parse front matter with `render_with_liquid: false`, verify flag is recognized
- Parse front matter with `render_with_liquid: true` (or absent), verify Liquid processing happens normally

### Unit: Content rendering with flag
- Page with `render_with_liquid: false` containing `{{ site.title }}`: verify output contains literal `{{ site.title }}` text
- Page with `render_with_liquid: false` containing `{% if %}...{% endif %}`: verify output contains literal tag text
- Page with the flag still gets Markdown-to-HTML conversion (headings, code blocks, etc.)

### Integration: Chirpy write-a-new-post
- Build chirpy site, verify `posts/write-a-new-post/index.html` is non-empty
- Verify the page contains expected content (e.g., "Writing a New Post" heading)
- Verify the page has `<head>` and `<body>` elements (layout was applied)

## Dependencies

None.

## DTC Baseline

790/790 matched (must not regress)

## Log

### [SWE] 2026-04-02

**Fix 1: Add `should_skip_liquid()` helper and skip Liquid rendering when `render_with_liquid: false`**

- Wrote test_render_with_liquid_false_skips_liquid_processing (src/template/layout.rs)
- Ran test: FAILS -- ParseError("Unclosed block. {% endif %} tag expected.") because Liquid tries to parse content with `{% if %}` as real tags
- Wrote test_render_with_liquid_false_non_markdown_page (src/template/layout.rs)
- Ran test: FAILS -- got empty string where `{{ site.title }}` was, Liquid consumed it
- Wrote test_render_with_liquid_false_unicode_content (src/template/layout.rs)
- Ran test: FAILS -- Liquid tag disappeared from Unicode content
- Wrote test_render_with_liquid_true_processes_liquid_normally (src/template/layout.rs)
- Ran test: PASSES (baseline, confirms normal Liquid processing works)
- Implemented fix: added `should_skip_liquid()` helper function and updated all 8 render functions in layout.rs to check `render_with_liquid: false` before Liquid processing
- Ran all 4 tests: PASSES

**Summary:**
- Files modified: src/template/layout.rs
- Tests added: 4 (render_with_liquid_false_skips_liquid_processing, render_with_liquid_true_processes_liquid_normally, render_with_liquid_false_non_markdown_page, render_with_liquid_false_unicode_content)
- Build results: 3870 lib tests pass, 0 fail, clippy clean, fmt clean
- DTC DOM: 790/790 matched, 0 total diffs (no regression)
- DTC build time: 0.916s (under 1.0s threshold)

### [PM] 2026-04-02 14:30
- Reviewed diff: 1 file changed (src/template/layout.rs), 263 insertions, 52 deletions
- Output verification: Built DTC site, confirmed 790/790 DOM match (no regression)
- Code review: should_skip_liquid() helper added, all 8 render functions guarded consistently
- Tests: 4 new tests covering markdown+flag, HTML+flag, unicode+flag, and baseline (no flag)
- All acceptance criteria met
- VERDICT: ACCEPT
