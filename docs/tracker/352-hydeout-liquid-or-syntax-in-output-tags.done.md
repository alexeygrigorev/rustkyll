# Issue 352: Hydeout Liquid `or` syntax in output tags

## Status

**Already fixed.** The `{{ a or b }}` Liquid output syntax was resolved in issue #441. This issue now serves as verification that the fix works correctly for Hydeout and that the DOM baseline has improved.

## Original Problem

Rustkyll's Liquid parser failed on `{{ page.guid or page.id }}` found in Hydeout's `_includes/disqus.html`. This caused all 24 Hydeout posts to render without proper layout (falling back to content-only rendering).

## Current State (post-fix)

After #441:
- All 24 Hydeout posts now render with full layout (head, body, sidebar, navigation)
- No more parse errors on `{{ page.guid or page.id }}`
- The `find` filter issue (#353) is also resolved
- Hydeout DOM score remains 0/13, but the remaining differences are caused by entirely different issues (tracked in #354):
  - Category URL casing (lowercase vs preserved case)
  - Category nav link sort order
  - Pagination path (`/page2/` vs `/blog/page2/`)
  - Future-dated post inclusion
  - Syntax highlighting span class differences
  - Markdown rendering differences (footnotes, noscript/script tags)

Related to issue #241 (Hydeout theme support). Category/pagination/future-post issues tracked in #354.

## Scope

Verify the #441 fix resolves this issue completely. No new code changes expected -- this is a verification-only issue.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] Building the Hydeout site (`websites/hydeout/`) succeeds with no `or`-related parse errors or warnings
- [ ] All 24 Hydeout posts render with full layout (HTML contains `<head>`, `<body>`, sidebar navigation)
- [ ] The `disqus.html` include does not cause any template parse failures
- [ ] Hydeout DOM baseline is recorded (currently 0/13 due to unrelated issues in #354)
- [ ] DTC DOM match count does not drop below current baseline
- [ ] `cargo test` passes with no regressions

## Test Scenarios

### Unit: or syntax in output tags
- Verify that `{{ page.guid or page.id }}` inside a false conditional branch does not cause parse errors
- Verify that `{{ a or b }}` syntax is handled gracefully (either parsed or treated as raw text)

### Integration: Hydeout full site build
- Build the Hydeout site with rustkyll and verify all 24 posts produce HTML files with full layout markup
- Verify the about page, category pages, and index page all render with complete HTML structure
- Verify no template parse warnings mention `or` syntax

### Regression: DTC DOM baseline
- Build the DTC site and verify DOM match count has not regressed

## Dependencies

- Issue #441 (already done -- implements the `or` syntax fix)
