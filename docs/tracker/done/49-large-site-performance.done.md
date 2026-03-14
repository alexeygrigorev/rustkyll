# Issue 49: Fix large-site performance (CRITICAL)

## Priority

CRITICAL -- this is the project's reason for existence. If rustkyll cannot render the DTC site at least 10x faster than Jekyll, there is no point in this project.

## Problem

Benchmark results show rustkyll has a catastrophic performance regression on large sites:

- kids-horror-stories-ru (1345 pages): 72s vs Jekyll 3.8s (20x SLOWER)
- DataTalksClub/datatalksclub.github.io (787 pages): TIMEOUT at 300s+ vs Jekyll 19.4s

Small sites (under 10 pages) are 5x-164x faster, so the issue is O(n^2) or worse scaling in template rendering, not startup overhead.

## Goal

The DTC main site must build successfully and be at least 10x faster than Jekyll (under 2 seconds, vs Jekyll's 19.4s). kids-horror-stories-ru should also be faster than Jekyll.

## Root cause analysis

The primary suspect is `LenientValue::from_value()` in `src/template/engine.rs`. This function recursively deep-clones every `Value` in the site context (including `children`, `array_children`, and `positional_children` maps) for every single page render. On a site with N pages, the `site.posts` array alone means each render creates N deep copies of all N post objects -- O(n^2) total allocation.

### Likely causes to investigate (ordered by expected impact)

1. **LenientValue deep-cloning per render:** The `site` Object is converted to a tree of `LenientValue` wrappers for every page. Each conversion recursively walks and clones the entire site context. With 787 pages in `site.posts`, each render clones all 787 post objects, yielding ~620K object clones total.
2. **Template re-parsing:** Check whether templates and includes are being parsed from source on every render rather than cached as compiled ASTs.
3. **site.posts / site.pages being deep-cloned:** The `build_site_context()` function in `generator.rs` may be rebuilding the entire site Object per page instead of sharing it.
4. **Quadratic loops in collection iteration or variable resolution.**
5. **Include file re-reading:** Include files read from disk on every use instead of cached.

### Approaches

#### Option A: Optimize the current liquid crate usage
- Build the `LenientValue` tree ONCE for the shared site context, then share it (via `Arc` or by reference) across all page renders.
- Cache parsed template ASTs (includes and layouts).
- Avoid rebuilding the site Object per page -- only inject page-specific variables (`page.*`, `content`).
- Profile with `cargo flamegraph` or `perf` to confirm the bottleneck before and after.

#### Option B: Write a custom template renderer
Replace the liquid crate entirely with a custom Liquid-compatible renderer optimized for our use case. Benefits:
- Full control over template AST caching, lazy evaluation, and memory layout
- Can avoid deep-cloning site context by using references/Rc/Arc
- Can compile templates to a more efficient IR

#### Recommended approach
Start with profiling (Option A) to identify the exact bottleneck. If the bottleneck is in the liquid crate's core (value conversion, template parsing per-render), go with Option B. Option A is likely sufficient given the architectural analysis above.

## Dependencies

None -- this issue can be picked up immediately.

## Acceptance Criteria

### Performance targets (MUST pass)

- [ ] DTC site (`websites/DataTalksClub/datatalksclub.github.io`) builds successfully with no errors and no timeout
- [ ] DTC site builds in under 2 seconds wall-clock time (10x faster than Jekyll's 19.4s), measured as median of 3 runs via `scripts/benchmark.sh --site DataTalksClub/datatalksclub.github.io`
- [ ] kids-horror-stories-ru (`websites/alexeygrigorev/kids-horror-stories-ru`) builds in under 3.8 seconds (faster than Jekyll), measured as median of 3 runs via `scripts/benchmark.sh --site alexeygrigorev/kids-horror-stories-ru`
- [ ] All other previously-passing sites still build successfully (no regressions); verify with `scripts/benchmark.sh`

### Correctness targets (MUST pass)

- [ ] `./scripts/cargo-safe test` passes -- all existing unit and integration tests pass with no failures
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes with no warnings
- [ ] DTC site output: same number of HTML files as Jekyll output (within 5% tolerance)
- [ ] DTC site output: for at least 10 sample pages (homepage, 3 posts, 2 collection pages, 2 archive/tag pages, RSS feed, sitemap), the structural HTML content matches Jekyll output -- same `<title>`, same `<h1>`-`<h6>` headings, same `<a href>` links, same `<img src>` images
- [ ] DTC site output: no raw Liquid tags (e.g., `{{`, `{%`) appear in any generated HTML file
- [ ] DTC site output: no empty HTML files (every generated `.html` file has >100 bytes of content)
- [ ] kids-horror-stories-ru output: same number of HTML files as Jekyll output (within 5% tolerance)
- [ ] RSS/Atom feed files are valid XML and contain the expected number of entries
- [ ] Sitemap lists the same URLs as Jekyll's sitemap (within 5% tolerance)

### Structural comparison (MUST pass)

- [ ] A comparison script or test exists that:
  - Builds the site with both Jekyll and rustkyll
  - Compares the file tree (list of generated files) between both outputs
  - For each HTML file, extracts structural elements (title, h1-h6, links, images) and diffs them
  - Reports any structural differences
  - Exits with nonzero status if differences exceed thresholds
- [ ] The structural comparison passes for DTC site
- [ ] The structural comparison passes for kids-horror-stories-ru

### Visual comparison with Playwright (MUST pass)

- [ ] Both Jekyll and rustkyll outputs are served over HTTP (e.g., `python -m http.server` on two ports) so CSS, JS, images, and fonts load correctly
- [ ] Playwright visits at least 5 key pages on both servers: homepage, a blog post, a collection page (e.g., events or books), an archive/listing page, and one other page
- [ ] Full-page screenshots are taken of each page from both servers
- [ ] Pixel-by-pixel or perceptual diff comparison is performed with a defined threshold (e.g., <5% pixel difference)
- [ ] No pages exceed the visual diff threshold
- [ ] No 404 errors in the browser console for either server (all assets load)
- [ ] Visual comparison passes for both DTC site and kids-horror-stories-ru

### Code quality

- [ ] The optimization is documented with comments explaining what was changed and why
- [ ] No `unwrap()` calls in library code (only in tests and main)
- [ ] Profiling results (before/after) are captured in the PR description or a doc file

## Test Scenarios

### Profiling (pre-work, not automated)
- Run `cargo flamegraph` or `perf record` on the DTC site build to identify the actual hotspot
- Document the top 3 hotspots and their percentage of total CPU time
- After optimization, re-profile and document the improvement

### Unit: LenientValue caching (if Option A)
- Create a `LenientValue` from a large Object (1000+ keys), measure that `from_value` is called once and the result can be reused across multiple renders
- Verify that page-specific variables (`page.title`, `page.url`, `content`) are correctly injected without re-wrapping the entire site context

### Unit: Template caching
- Parse a template string, verify the parsed AST is reused when rendering with different contexts
- Parse an include file, verify it is not re-read from disk on second use

### Integration: Large site build time
- Build kids-horror-stories-ru (1345 pages) and assert wall-clock time < 3.8s (mark as `#[ignore]` so it does not run in normal test suite)
- Build DTC site (787 pages) and assert wall-clock time < 2s (mark as `#[ignore]`)

### Integration: Output correctness after optimization
- Build kids-horror-stories-ru, count generated HTML files, assert count matches expected (1345 +/- 5%)
- Build DTC site, count generated HTML files, assert count matches expected (784 +/- 5%)
- Build DTC site, read the homepage HTML, assert it contains expected `<title>` and at least one `<a>` link
- Build DTC site, verify no generated HTML file contains raw `{{` or `{%` Liquid tags

### Integration: Structural comparison
- Run the structural comparison script on DTC site output vs Jekyll output
- Run the structural comparison script on kids-horror-stories-ru output vs Jekyll output
- Assert both pass with zero structural differences above threshold

### Integration: Visual comparison
- Serve both outputs over HTTP
- Run Playwright screenshot comparison on at least 5 pages
- Assert all pages pass the pixel diff threshold

### Regression: Small sites unaffected
- Build each of the 5 small sites that currently pass (opensource-guide, government-github, wtf-html-css, academicpages, hyde)
- Assert each still builds successfully with correct page counts
- Assert build times do not regress (should remain under 1 second each)

## Notes

- The `#[ignore]` attribute is required for tests that build large sites, per project convention -- the default test suite must stay fast.
- The benchmark script at `scripts/benchmark.sh` already exists and can be used for timing measurements.
- The engineer should profile BEFORE coding any optimization. Do not guess at the bottleneck.
- If Option B (custom renderer) is chosen, it must still pass all existing template-related tests.
