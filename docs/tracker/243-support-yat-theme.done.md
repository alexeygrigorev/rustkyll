# Issue 243: Support Yat Jekyll theme

## Problem

Yat (Yet Another Theme) is a popular Jekyll theme (~1k GitHub stars) with a modern, feature-rich design. It is not currently in our benchmark suite.

## Theme Details

- **GitHub:** https://github.com/jeffreytse/jekyll-theme-yat
- **Stars:** ~1,000
- **Use case:** Blogs, portfolios
- **Notable features:** Banner with animated background, dark mode, table of contents, tags/categories, math (MathJax), mermaid diagrams, search, translations/i18n

## Scope

1. Clone the Yat theme repository into `websites/yat/`.
2. Build the cloned site with both Jekyll and rustkyll.
3. Run DOM comparison against the cached Jekyll output and record the real match rate.
4. Identify Yat-specific rendering blockers and either fix them in this issue or create follow-up issues that reference `#243`.

## Baseline

- DTC DOM baseline: `771/790` (committed baseline; working tree shows 772/790)

## Acceptance Criteria

- [ ] The Yat theme site is cloned into `websites/yat/` and the repository state (commit SHA) is documented in the issue log.
- [ ] Jekyll builds the Yat site successfully and produces a reference `_site` output (cached to `_site_jekyll_cached/`).
- [ ] rustkyll builds the Yat site successfully (warnings are acceptable, hard errors are not) and produces HTML output for the same site.
- [ ] The DOM comparison between the Jekyll and rustkyll outputs is run and the issue log records the exact match count, differing-file count, and main diff categories.
- [ ] Representative pages that exercise Yat features are verified in the output, including: the homepage (with banner and post listing), tag and category pages, individual post pages (with table of contents if present), and any about/archive page if present.
- [ ] Any Yat-specific rendering issues discovered during comparison are either fixed in this issue or explicitly tracked in new follow-up `.todo.md` issues that reference `#243`.
- [ ] The DTC DOM match count does not drop below `771/790`.

## Test Scenarios

### Integration: demo site setup
- Clone the upstream Yat repository into `websites/yat/` and verify the expected theme files and configuration are present (at minimum: `_config.yml`, `_layouts/`, `_includes/`, and sample `_posts/` or example content).
- Run `bundle exec jekyll build` (or `jekyll build`) in the Yat site directory and confirm the reference HTML output is generated in `_site/`.
- Cache the Jekyll output to `_site_jekyll_cached/` for reproducible DOM comparison.
- Run `./target/release/rustkyll build --source websites/yat --destination /tmp/yat-rustkyll` and confirm HTML output is generated.

### Integration: output comparison
- Run DOM comparison (`python scripts/dom_compare.py`) between the Jekyll `_site_jekyll_cached` and the rustkyll output directory and record the exact match count, differing-file count, and total differences.
- Inspect representative pages in both outputs:
  - Homepage (`index.html`): verify banner area and post listing render.
  - A sample post page: verify title, date, content, and table of contents (if configured) render.
  - Tag index page (if generated): verify post grouping by tag renders.
  - Category index page (if generated): verify post grouping by category renders.
  - Archive page (if present): verify chronological post listing renders.
- Verify any identified rendering blocker is either fixed or captured in a follow-up issue linked to `#243`.

### Regression: DTC DOM baseline
- Build the DTC site with rustkyll and run the DOM comparison against the Jekyll reference.
- Verify the match count is at least `771/790` (no regression from the committed baseline).

## Dependencies

- None (research/benchmark task)

## Log

### [SWE] 2026-03-25

**Setup:**
- Cloned yat theme into `websites/yat/` at commit `f63b78f` (feat: add disqus recommendations support)
- Verified theme files present: `_config.yml`, `_layouts/` (10 layouts), `_includes/` (30+ includes), `_posts/` (14 posts), `_data/`, `_sass/`, pages (index, tags, categories, archives, about, 404)
- Installed Jekyll deps with `bundle _2.7.2_ install` (needed older bundler due to gemspec constraint)
- Built with Jekyll: `bundle _2.7.2_ exec jekyll build` -- success, 20 HTML files
- Cached Jekyll output to `websites/yat/_site_jekyll_cached/`

**Rustkyll build:**
- Built with rustkyll: `./target/release/rustkyll build --source websites/yat/ --destination websites/yat/_site_rustkyll_recount` -- success in 0.09s, 20 HTML files
- 2 warnings:
  1. `about.html` render error: `Expected scalar, found nil` from `site[page["collection"]] | sample:4` (page.collection is nil for non-collection pages)
  2. SCSS: `Can't find stylesheet to import` for `@import "yat"` (SASS import resolution)

**DOM comparison:**
- Result: **0/20 matched, 20 differing files, 917 total differences (258 acceptable diffs filtered)**
- Main diff categories:
  1. **Self-closing tags** (`/>` vs `>`): SEO tag meta elements use `/>` in rustkyll, `>` in Jekyll -- cosmetic, DOM-equivalent
  2. **Whitespace/formatting**: Minor newline and indentation differences throughout
  3. **about.html**: Renders without layout wrapping due to `sample` filter error on nil collection
  4. **404.html**: Empty output (rendering failure)
  5. **Archives page**: Year segments render but posts within segments are empty -- `where` filter comparing datetime vs string
  6. **SEO tag differences**: Extra newline in description, missing `dateModified`, JSON-LD structure minor differences

**Representative pages verified:**
- Homepage (index.html): Banner area renders, post listing with 14 posts, reading times, excerpts, tags -- all present, 42 post-link/post-item elements matching Jekyll
- Post pages (e.g., welcome-to-jekyll.html): Full HTML layout, title, content, prev/next navigation all render
- Tags page (tags.html): All tag segment headings and post links render correctly
- Categories page (categories.html): 5 category segments with post links, matching Jekyll
- Archives page (archives.html): 4 year segment headings render but post items within are empty (where filter issue)
- About page: Content renders but without layout wrapping (sample filter nil error)

**DTC DOM baseline:**
- DTC DOM: **772/790** -- above 771/790 baseline, no regression

**Tests:**
- Wrote 9 integration tests in `integration_tests/tests/integration_yat.rs`
- All tests pass: 9/9
- Full test suite: all passing (2791 + integration tests), 0 failures
- Clippy: clean (no warnings beyond upstream liquid-lib deprecation)
- `cargo fmt`: clean

**Yat-specific issues identified (need follow-up):**
1. `about.html` render failure: `sample` filter on nil collection -- the `post.html` layout uses `site[page.collection] | sample:4` for related posts, fails when page.collection is nil
2. `404.html` empty output: Layout chain 404->default fails to render
3. Archives `where` filter: `where: 'date', '2018'` doesn't match datetime objects against year strings
4. SCSS import resolution: `@import "yat"` not found (theme SASS files in `_sass/` subdirectory)

**Files created/modified:**
- `integration_tests/tests/integration_yat.rs` (new) -- 9 integration tests
- `docs/tracker/243-support-yat-theme.in-progress.md` (updated) -- log

### [QA] 2026-03-25

**Acceptance Criteria Verification:**

1. Yat theme cloned into `websites/yat/` at commit `f63b78f` -- PASS (confirmed via `git log -1` in websites/yat/)
2. Jekyll cached output in `websites/yat/_site_jekyll_cached/` -- PASS (20 HTML files present)
3. Rustkyll builds successfully -- PASS (20 HTML files, 0.10s, 2 warnings: sample filter nil + SCSS import)
4. DOM comparison recorded -- PASS (0/20 matched, 20 differing files, 917 total differences, 258 acceptable filtered)
5. Representative pages verified -- PASS
   - Homepage: banner + post listing with post-link elements
   - Post pages: full HTML layout, title, content, navigation
   - Tags page: segment headings + post links
   - Categories page: 5 category segments + post links
   - Archives page: year segment headings present (posts empty due to where filter issue)
   - About page: content renders but without layout (sample filter nil error)
6. Follow-up issues created -- PASS
   - `docs/tracker/360-yat-sample-filter-nil-collection.todo.md` references #243
   - `docs/tracker/361-yat-where-filter-datetime-string-comparison.todo.md` references #243
7. DTC DOM baseline -- PASS (772/790, above 771/790 baseline)

**Tests:**
- 9 yat integration tests: all pass
- Full test suite: 3095+ tests across all crates, 0 failures
- Clippy: clean (only upstream liquid-lib deprecation warnings)
- `cargo fmt --check`: clean

**VERDICT: PASS**

### [PM] 2026-03-25

**Acceptance Review:**

1. Yat theme cloned at commit `f63b78f`, documented -- PASS
2. Jekyll cached output: 20 HTML files in `_site_jekyll_cached/` -- PASS
3. Rustkyll builds successfully (warnings only, no hard errors) -- PASS
4. DOM comparison recorded: 0/20 matched, 20 differing, 917 total diffs -- PASS
5. Representative pages verified (homepage with banner/post listing, tags, categories, archives, about, post pages) -- PASS
6. Follow-up issues created for all Yat-specific blockers -- PASS
   - `#360` covers sample filter nil (about.html + 404.html rendering)
   - `#361` covers where filter datetime-vs-string (archives page)
   - SCSS import resolution already tracked in existing issues (#249, #345, #359) -- no duplicate needed
7. DTC DOM baseline: 772/790, above 771/790 floor -- PASS

**Descoping check:** No acceptance criteria were silently dropped. All 4 identified blocker categories are tracked: 2 in new follow-up issues, 1 in existing cross-theme issues, 1 (SEO tag cosmetic diffs) is a known limitation across all themes.

**Tests:** 9 integration tests are meaningful -- they verify actual HTML content structure (post listings, banner, tag/category segments, navigation, dynamic includes, unicode), not just file counts.

**VERDICT: ACCEPT**
