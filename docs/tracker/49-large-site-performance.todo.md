# Issue 49: Fix large-site performance (CRITICAL)

## Priority

CRITICAL -- this is the project's reason for existence. If rustkyll cannot render the DTC site at least 10x faster than Jekyll, there is no point in this project.

## Problem

Benchmark results show rustkyll has a catastrophic performance regression on large sites:

- kids-horror-stories-ru (1345 pages): 72s vs Jekyll 3.8s (20x SLOWER)
- DataTalksClub/datatalksclub.github.io (787 pages): TIMEOUT at 300s vs Jekyll 19.4s

Small sites (under 10 pages) are 5x-164x faster, so the issue is O(n^2) or worse scaling in template rendering, not startup overhead.

## Goal

The DTC main site must build successfully and be at least 10x faster than Jekyll (under 2 seconds, vs Jekyll's 19.4s). kids-horror-stories-ru should also be faster than Jekyll.

## Likely causes to investigate

1. Template rendering: re-parsing templates per page instead of caching parsed ASTs
2. site.posts / site.pages / site.related_posts being deep-cloned per render
3. LenientValue/LenientObject creation overhead (deep conversion of large site context)
4. Quadratic loops in collection iteration or variable resolution
5. Unnecessary work in include processing (re-reading files, re-parsing)

## Approaches

### Option A: Optimize the current liquid crate usage
Profile and fix the specific bottlenecks listed above. May hit limits of the liquid crate's architecture.

### Option B: Write our own template renderer
Replace the liquid crate entirely with a custom Liquid-compatible renderer optimized for our use case. Benefits:
- Full control over template AST caching, lazy evaluation, and memory layout
- Can avoid deep-cloning site context by using references/Rc/Arc
- Can compile templates to a more efficient IR
- No fighting upstream crate limitations
- Can optimize specifically for Jekyll's subset of Liquid

This is more work upfront but may be the only way to hit the 10x target on large sites. The liquid crate may have fundamental architectural issues (e.g., requiring owned values everywhere) that make optimization impossible.

### Recommended approach
Start with profiling (Option A) to identify the exact bottleneck. If the bottleneck is in the liquid crate's core (value conversion, template parsing per-render), go with Option B.

## Dependencies

None

## Output quality verification (MANDATORY)

Speed without correctness is worthless. After optimization, structurally compare rustkyll output against Jekyll output for every large site:

1. Same number of HTML files generated (page count must match Jekyll within 5%)
2. For a sample of pages, compare structural HTML: same headings, same links, same content blocks
3. Navigation, pagination, and collection pages must all be present
4. RSS/Atom feeds must be valid and contain the same entries
5. Sitemap must list the same URLs
6. No missing pages, no empty pages, no broken template rendering (e.g. raw Liquid tags in output)

Build a comparison script or test that:
- Builds the site with both Jekyll and rustkyll
- Compares file trees (list of generated files)
- For each HTML file, extracts structural elements (title, h1-h6, links, images) and diffs them
- Reports any structural differences

### Visual comparison with Playwright

After structural comparison passes, do a visual screenshot comparison. The sites MUST be served over HTTP so that CSS, images, fonts, and JS all load -- comparing raw HTML files misses styling and layout issues entirely.

1. Serve the Jekyll-built _site/ on one port (e.g. python -m http.server), rustkyll-built _site/ on another
2. Verify no 404 errors in the browser console (all assets loading)
3. Use Playwright to visit a sample of key pages on both (homepage, a post, a collection page, an archive page, etc.)
4. Take full-page screenshots of each page from both servers
5. Compare screenshots pixel-by-pixel (or with a perceptual diff threshold to tolerate minor whitespace differences)
6. Flag any pages where the visual diff exceeds the threshold

This catches rendering differences that structural comparison misses: CSS class mismatches, missing stylesheets, broken image paths, wrong element ordering, missing partials.

Both structural and visual comparison must pass for DTC site and kids-horror-stories-ru before the issue can be accepted.

## Acceptance criteria

- DTC site builds successfully (no timeout, no errors)
- DTC site builds in under 2 seconds (10x faster than Jekyll's 19.4s)
- kids-horror-stories-ru builds faster than Jekyll (under 3.8s)
- Structural comparison against Jekyll output passes for both sites (see above)
- Playwright visual screenshot comparison passes for both sites
- All existing tests still pass
- No correctness regressions (output should match pre-optimization output)
- Benchmark script confirms the speedup numbers
