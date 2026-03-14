# Issue 50: Fix DataTalksClub/docs site build

## Problem

DataTalksClub/docs (57 pages) builds with Jekyll in 1.8s but fails with rustkyll.

## Goal

Investigate the build failure, fix the missing feature or compatibility gap, and get this site building with rustkyll.

## Approach

1. Run `cargo run --release -- build --source websites/DataTalksClub/docs` and capture the error
2. Identify the root cause (missing filter, tag, plugin, template error, etc.)
3. Implement the fix
4. Verify the site builds and produces correct output

## Dependencies

None

## Output quality verification

After fixing the build, structurally compare rustkyll output against Jekyll output:

1. Same HTML files generated (file tree diff)
2. For each HTML file, compare structural elements: title, headings (h1-h6), links, images
3. No missing pages, no empty pages, no raw Liquid tags in output
4. RSS/Atom feeds and sitemap (if any) must match

### Visual comparison with Playwright

Sites MUST be served over HTTP so CSS, images, fonts, and JS all load. Serve Jekyll _site/ and rustkyll _site/ on separate ports (e.g. python -m http.server). Use Playwright to screenshot key pages from both fully-rendered sites and compare. Verify no 404s in browser console. Flag any visual differences beyond a minor threshold.

## Acceptance criteria

- DataTalksClub/docs builds successfully with rustkyll
- Output page count matches Jekyll's (close to 57 pages)
- Structural comparison against Jekyll output passes (see above)
- Playwright visual screenshot comparison passes
- No regressions on currently-passing sites
- All existing tests still pass
