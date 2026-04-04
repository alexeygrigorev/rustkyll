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
