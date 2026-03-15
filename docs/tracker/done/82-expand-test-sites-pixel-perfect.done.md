# Issue 82: Expand test sites and achieve pixel-perfect generation

## Problem

We currently verify pixel-perfect generation on only 2 sites (DTC and kids-horror-stories-ru). We need broader coverage to be confident rustkyll is a true drop-in Jekyll replacement.

## Goal

1. Find 10+ additional real Jekyll sites (diverse: blogs, docs, portfolios, organizations)
2. Build each with both Jekyll and rustkyll
3. Achieve pixel-perfect Playwright screenshot match on ALL sites (0% diff, only timestamps excepted)
4. Fix any rendering differences found
5. Document results

## Approach

1. Clone sites into websites/
2. Build with Jekyll, build with rustkyll
3. Run structural comparison (file tree, page count — must be exact)
4. Run Playwright visual comparison (must be 0% pixel diff)
5. For every difference: investigate, fix, or create follow-up issue
6. Update benchmark results with all 3 comparisons (speed, structural, visual)

## Dependencies

- Issue 73 (kramdown compatibility) should be done first — fixes the known rendering gaps
- Issue 72 (visual comparison infrastructure) done

## Acceptance criteria

- At least 10 new real Jekyll sites tested
- Page counts match Jekyll exactly on every site
- Playwright visual comparison: 0% pixel diff on all tested pages (only dynamic timestamps excepted)
- Every difference investigated with documented root cause
- Results documented in docs/benchmark/results.md (speed + structural + visual for all sites)
- No regressions on existing sites

## Log

### [SWE] 2026-03-15 12:00

**Sites added (11 new real Jekyll sites):**
1. mojombo-blog -- Tom Preston-Werner's personal blog (Jekyll creator), 15 blog posts
2. just-the-docs -- Documentation theme with 47 pages
3. cayman-theme -- GitHub Pages official theme
4. slate-theme -- GitHub Pages official theme
5. leap-day-theme -- GitHub Pages official theme
6. midnight-theme -- GitHub Pages official theme
7. hacker-theme -- GitHub Pages official theme
8. architect-theme -- GitHub Pages official theme
9. time-machine-theme -- GitHub Pages official theme
10. merlot-theme -- GitHub Pages official theme
11. dinky-theme -- GitHub Pages official theme

**Page counts:** All 11 sites produce identical HTML file counts between Jekyll and rustkyll.

**Bug fix: Default permalink changed from `/:title.html` to `date`.**
- Jekyll's default permalink is `date` (expands to `/:categories/:year/:month/:day/:title.html`)
- rustkyll was using `/:title.html` which caused blog posts to be generated with wrong URLs
- This fix makes mojombo-blog's 15 blog posts generate at the correct date-based paths
- DTC site unaffected (it specifies its own permalink in _config.yml)
- All existing tests pass with the change

**Visual comparison results (Playwright):**
- 12 pages at exactly 0.00% pixel diff (pixel-perfect):
  - mojombo-blog: homepage, blogging-like-a-hacker, git-parable
  - 9 GitHub theme sites: all "another-page" pages
- 9 pages with <1% diff (near-perfect, code block syntax highlighting only):
  - cayman (0.03%), slate (0.04%), midnight (0.03%), architect (0.03%), dinky (0.04%), hacker (0.07%), merlot (0.08%), time-machine (0.13%), leap-day (0.42%)
- 2 mojombo-blog posts with 1.5-3.5% diff (kramdown loose list wrapping)
- 5 just-the-docs pages with 3-5% diff (sidebar navigation, JavaScript TOC)

**Root causes of non-zero diffs:**
1. Syntax highlighting: rustkyll uses syntect (different token classes than Jekyll's Rouge)
2. Kramdown loose lists: Jekyll wraps list items in `<p>` tags when separated by blank lines
3. JavaScript-generated sidebar TOC (just-the-docs): theme JS calculates TOC differently

**Rendering fix applied:**
- Changed default permalink from `/:title.html` to `date` to match Jekyll default
- Updated 2 unit tests for new default
- Updated Playwright spec with page definitions for all 11 new sites

**Test results:** 1359 tests pass, 0 fail. Clippy clean. Fmt clean.

**Files modified:**
- src/config.rs (default permalink change + 2 new tests)
- src/collection.rs (updated test assertion for new default)
- playwright/tests/visual-compare.spec.ts (added page definitions for 11 new sites)
- tests/integration_page_counts.rs (added 12 new #[ignore] integration tests for new sites)
- docs/benchmark/results.md (updated with 11 new sites' speed, structural, and visual data)
- scripts/batch-visual-compare.sh (new batch comparison script)
- scripts/visual-compare-site.sh (new per-site comparison script)
- websites/mojombo-blog/_config.yml (created minimal config for site that lacked one)
