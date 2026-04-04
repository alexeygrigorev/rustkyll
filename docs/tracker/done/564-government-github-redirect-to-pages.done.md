# Issue 564: government-github redirect_to pages not generating redirect HTML

## Problem

government-github is at 18/21 pages (86%) with 3 pages having diffs. Two of the three diffing pages (`index.html` and `accessibility/index.html`) use `redirect_to` in their front matter but rustkyll renders the full layout instead of generating a redirect page.

### A. redirect_to pages render full layout instead of redirect (2 pages, 16 diffs)

**index.html**: Front matter has `redirect_to: https://github.com/solutions/industry/government`. Jekyll's `jekyll-redirect-from` plugin generates a minimal redirect page:
```html
<html lang="en-US">
  <title>Redirecting...</title>
  <link rel="canonical" href="https://github.com/solutions/industry/government">
  <script>location="https://github.com/solutions/industry/government"</script>
  <meta http-equiv="refresh" content="0; url=https://github.com/solutions/industry/government">
  <meta name="robots" content="noindex">
  <h1>Redirecting...</h1>
  <a href="https://github.com/solutions/industry/government">Click here if you are not redirected.</a>
</html>
```

Rustkyll renders the full page layout with all the site content, ignoring the `redirect_to` directive.

**accessibility/index.html**: Same issue -- has `redirect_to: https://accessibility.github.com/conformance`.

### B. community/index.html data drift (1 page, 3164 diffs)

The community page diffs are almost entirely caused by GitHub avatar URL format changes between when the Jekyll cache was built and now (`avatars2.githubusercontent.com` vs `avatars.githubusercontent.com`, `v=3` vs `v=4`). The page structure, attribute ordering, and indentation also have minor whitespace differences. **This is data drift, not a rustkyll bug.** The structural output is functionally equivalent.

## Affected Site

- government-github: 18/21 (86%)
- Fixing A would bring it to 20/21 (95%) -- the community page's 3164 diffs are data drift
- If community data drift is excluded, effective match would be 20/21

## Root Cause

Rustkyll does not implement the `redirect_to` front matter directive (from the `jekyll-redirect-from` plugin). When a page has `redirect_to: <url>`, it should generate a minimal redirect HTML page instead of rendering the normal layout.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests
- [ ] Pages with `redirect_to` front matter generate redirect HTML (not full layout)
- [ ] Redirect HTML contains: meta refresh, canonical link, JavaScript redirect, noindex meta, fallback link
- [ ] government-github `index.html` generates redirect to `https://github.com/solutions/industry/government`
- [ ] government-github `accessibility/index.html` generates redirect to `https://accessibility.github.com/conformance`
- [ ] DTC DOM match count must not drop below 790/790

## Test Scenarios

### Unit: redirect_to page generation
- Page with `redirect_to: https://example.com` generates redirect HTML
- Redirect HTML contains `<meta http-equiv="refresh" content="0; url=https://example.com">`
- Redirect HTML contains `<script>location="https://example.com"</script>`
- Redirect HTML contains `<link rel="canonical" href="https://example.com">`
- Redirect HTML contains `<meta name="robots" content="noindex">`
- Page with `redirect_to` does NOT render the normal layout

### Unit: redirect_to coexists with layout
- Page with both `layout: home` and `redirect_to: url` should generate redirect, ignoring layout

### Integration: government-github build
- Build government-github and verify index.html is a redirect page
- Build government-github and verify accessibility/index.html is a redirect page
- Run DOM comparison to verify 20/21 match (community page excluded as data drift)

## Dependencies

None

## DTC DOM Baseline

790/790 (100%)

## Log

### [PM] 2026-04-02 10:00
- Groomed from batch site analysis
- government-github at 18/21 (86%)
- 2 pages need redirect_to support, 1 page has data drift (not a bug)
- redirect_to is from jekyll-redirect-from plugin, commonly used

### [SWE] 2026-04-02

**Root cause:** The redirect_to handler (lines 1020-1026 of src/main.rs) skipped generating redirect HTML whenever the page had ANY layout that existed in the layout engine. For government-github, `index.html` has `layout: home` and `accessibility.md` has `layout: support-page` -- both exist, so the redirect_to was skipped and the full layout was rendered instead. Jekyll's `jekyll-redirect-from` plugin always generates redirect HTML regardless of layout.

**Fix 1: redirect_to with non-redirect layout generates redirect HTML**
- Wrote test: test_integration_redirect_to_with_non_redirect_layout (src/main.rs)
- Ran test: FAILS -- got `<html><body><h1>Home Layout</h1><p>Full page content...</p></body></html>`, expected meta refresh redirect HTML
- Implemented fix in src/main.rs: changed layout check from `layout_engine.has_layout(layout_name)` to `layout_name == "redirect" && layout_engine.has_layout(layout_name)` for both standalone pages and collections
- Ran test: PASSES

**Fix 2: redirect_to with unicode target URL**
- Wrote test: test_integration_redirect_to_with_unicode_target (src/main.rs)
- Ran test: PASSES (unicode target URLs already worked, this adds coverage)

**Summary:**
- Files modified: src/main.rs (2 lines changed in redirect_to handler, 2 test functions added)
- Tests added: 2 (test_integration_redirect_to_with_non_redirect_layout, test_integration_redirect_to_with_unicode_target)
- All existing redirect_to tests still pass (5 total)
- Full test suite: all pass, 0 failures
- Clippy clean, fmt clean
- DTC DOM: 790/790 with 0 total diffs (baseline maintained)
- DTC build time: 0.639s (under 1.0s threshold)
- government-github DOM: 20/21 (up from 18/21), remaining 1 is community page data drift (3164 diffs from avatar URL format changes)

### [PM] 2026-04-02 12:30
- Reviewed diff: 1 file relevant (src/main.rs, 2 lines changed in redirect handler + 2 tests added); collection.rs and generator.rs changes belong to other issues, excluded from commit
- Output verification:
  - government-github index.html: correct redirect HTML with meta refresh, canonical link, JS redirect, noindex, fallback link to https://github.com/solutions/industry/government
  - government-github accessibility/index.html: correct redirect HTML to https://accessibility.github.com/conformance
  - government-github DOM: 20/21 (up from 18/21), remaining 1 page is community data drift (3164 diffs)
  - muan-blog: verified pages use `layout: redirect` with custom redirect.html template -- fix correctly preserves custom redirect layout behavior (2204/2219, no regression)
  - DTC DOM: 790/790 (baseline maintained)
- Results verified: real DOM comparison data present for DTC, government-github, and muan-blog
- Acceptance criteria: all met
  - [x] cargo build compiles
  - [x] cargo test passes (all tests pass)
  - [x] Pages with redirect_to generate redirect HTML
  - [x] Redirect HTML contains all required elements (meta refresh, canonical, JS redirect, noindex, fallback)
  - [x] government-github index.html generates correct redirect
  - [x] government-github accessibility/index.html generates correct redirect
  - [x] DTC DOM 790/790 maintained
- Follow-up issues: none needed
- VERDICT: ACCEPT
