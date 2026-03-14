# Issue 62: Playwright visual screenshot comparison

## Problem

Issues #49 and #57 required Playwright visual comparison of rustkyll vs Jekyll output but this was descoped. We need to verify that the rendered pages look the same in a real browser.

## Goal

Build a Playwright test suite that:
1. Serves Jekyll _site/ on one port, rustkyll _site/ on another (over HTTP so CSS, images, fonts, JS all load)
2. Visits key pages on both servers
3. Takes full-page screenshots
4. Compares screenshots with a pixel diff threshold
5. Verifies no 404 errors in browser console (all assets loading)

## Sites to compare

- DataTalksClub/datatalksclub.github.io
- alexeygrigorev/kids-horror-stories-ru

## Pages to screenshot (at minimum)

For DataTalksClub:
- Homepage (index.html)
- A blog post (pick one from _posts/)
- A collection page (e.g., a books/ page, a podcast/ page, or a people/ page)
- An archive/listing page (e.g., books.html, events.html, or articles.html)
- One other distinct page (e.g., courses.html or a conference page)

For kids-horror-stories-ru:
- Homepage (index.html)
- At least 2 other pages that the site generates

Total: at least 5 pages per site, at least 8 distinct page screenshots overall.

## Architecture

The test suite lives in a new directory (e.g., `tests/playwright/` or `playwright/`) at the project root. It must:

1. Have its own `package.json` with Playwright as a dependency.
2. Have a `playwright.config.ts` (or `.js`) that does NOT hardcode a webServer command -- the servers are started externally (see below).
3. Include a helper script (e.g., `scripts/visual-compare.sh`) that:
   a. Builds both Jekyll and rustkyll output for a given site (or accepts pre-built directories).
   b. Starts two HTTP servers (e.g., `python3 -m http.server` or Node `http-server`) -- one serving Jekyll output on port A, one serving rustkyll output on port B.
   c. Runs the Playwright tests, passing the two base URLs as environment variables.
   d. Stops the servers when done (trap cleanup).
   e. Reports results: pass/fail plus saved screenshots and diff images.

## Dependencies

- Issue #61 (structural comparison) is NOT a hard dependency -- this issue is independent.
- Node.js and Playwright must be installed (npx playwright install).
- Python 3 (for http.server) or a Node HTTP server package.
- Both Jekyll and rustkyll must be able to build the test sites. Jekyll output can be pre-built.

## Acceptance Criteria

- [ ] A Playwright test project exists under the repo (e.g., `tests/playwright/` or `playwright/`) with its own `package.json` and `playwright.config`.
- [ ] `npm install` (or equivalent) in that directory installs Playwright and any required dependencies.
- [ ] `npx playwright install` installs the required browser(s).
- [ ] A runner script exists (e.g., `scripts/visual-compare.sh`) that orchestrates the full flow: build sites, start HTTP servers, run Playwright, stop servers.
- [ ] Both sites (Jekyll output and rustkyll output) are served over HTTP on different ports -- NOT read as raw file:// URLs. CSS, images, fonts, and JS must all load via HTTP.
- [ ] The Playwright tests capture console errors and network failures. Any 404 errors on assets (CSS, JS, images, fonts) cause the test for that page to fail.
- [ ] At least 5 distinct pages are screenshotted per site (DataTalksClub), and at least 3 for kids-horror-stories-ru -- totaling at least 8 page comparisons.
- [ ] Each page comparison produces: a Jekyll screenshot, a rustkyll screenshot, and a pixel diff image.
- [ ] A configurable pixel diff threshold is defined (default: less than 5% pixel difference). Tests fail if the threshold is exceeded.
- [ ] All screenshots and diff images are saved to an output directory (e.g., `tests/playwright/screenshots/`) for manual review.
- [ ] The runner script exits nonzero if any page fails the visual diff threshold or has 404 asset errors.
- [ ] The runner script exits zero and prints a summary when all pages pass.
- [ ] The test suite works when run with `--site DataTalksClub/datatalksclub.github.io` and `--site alexeygrigorev/kids-horror-stories-ru` (or equivalent parameterization).
- [ ] A brief usage section is included in the test directory's README or as comments in the runner script, explaining how to run the comparison.

## Test Scenarios

### Setup: Playwright project structure
- `package.json` exists and lists `@playwright/test` as a dependency.
- `playwright.config` configures at least one browser project (e.g., Chromium desktop).
- Running `npm install && npx playwright install` succeeds without errors.

### HTTP serving: sites served correctly
- Start an HTTP server on a port serving Jekyll _site/. Curl the homepage -- verify it returns 200 and contains `<html`.
- Start an HTTP server on another port serving rustkyll _site/. Curl the homepage -- verify it returns 200 and contains `<html`.
- Verify CSS files are accessible (curl a known CSS path, check 200 status).
- After tests complete, verify both server processes are killed (no orphan processes).

### Console error capture: detect 404s
- For each page visited, collect browser console errors and failed network requests.
- If a page has a broken image, broken CSS link, or broken JS reference, the test for that page must fail with a message indicating which resource returned 404.

### Screenshot capture: correct pages captured
- For DataTalksClub: screenshots exist for at least homepage, one blog post, one collection item page, one listing page, and one other page.
- For kids-horror-stories-ru: screenshots exist for at least homepage and 2 other pages.
- Each screenshot is a non-empty PNG file (> 1 KB).
- Screenshots are full-page (not just viewport -- they capture the full scrollable content).

### Pixel diff comparison: threshold enforced
- Given two identical pages, the pixel diff should be 0% (sanity check with same source).
- Given two pages with known differences, the diff percentage should be nonzero.
- If the diff exceeds the configured threshold (default 5%), the test fails.
- Diff images are saved showing highlighted pixel differences.

### End-to-end: runner script
- Run `scripts/visual-compare.sh --site DataTalksClub/datatalksclub.github.io` -- it builds, serves, screenshots, compares, and reports.
- Run `scripts/visual-compare.sh --site alexeygrigorev/kids-horror-stories-ru` -- same flow.
- If all pages pass the threshold, the script exits 0 and prints a pass summary.
- If any page fails, the script exits nonzero and prints which pages failed and their diff percentages.

### Cleanup
- After the script runs (pass or fail), no orphan HTTP server processes remain.
- The screenshot output directory contains all expected files.

## Notes

- The Playwright config from `websites/DataTalksClub/docs/playwright.config.js` is for the DTC docs site specifically and should NOT be reused directly. This issue requires a new, project-level Playwright setup.
- Use `pixelmatch` or Playwright's built-in `toHaveScreenshot` with `maxDiffPixelRatio` for pixel comparison. If using `toHaveScreenshot`, the golden screenshots come from the Jekyll side and the test checks the rustkyll side against them.
- Viewport should be fixed (e.g., 1280x720 for desktop) to ensure reproducible screenshots.
- Consider also adding a mobile viewport (e.g., 375x667) as a stretch goal, but desktop-only is acceptable for this issue.
