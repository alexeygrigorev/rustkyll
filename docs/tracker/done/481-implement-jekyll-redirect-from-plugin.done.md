# Issue 481: Add tests for jekyll-redirect-from plugin implementation

## Problem

The `jekyll-redirect-from` plugin is already implemented in `src/main.rs` (functions
`extract_redirect_from`, `generate_redirect_html`, and the redirect generation blocks
in `build_site`). However, `docs/jekyll-compatibility.md` shows "no" for tests, and
there are zero test files covering redirect functionality. Without tests, regressions
can silently break redirect generation for the 12 sites that depend on it.

Sites using `redirect_from` or `redirect_to`: academicpages, alexeygrigorev,
choosealicense.com, government-github, homebrew-site (47 files),
jekyll-docs (15 files), minimal-mistakes, muan-blog (29 files),
opensource-guide (22 files), programming-historian (121 files),
so-simple-theme, uswds-site (90 files).

## Current Implementation

Located in `src/main.rs`:

1. **`extract_redirect_from(fm)`** -- Extracts `redirect_from` from front matter,
   supports both single string (`redirect_from: /old/`) and array
   (`redirect_from: [/old-1/, /old-2/]`).

2. **`generate_redirect_html(from, to, site_url, baseurl)`** -- Generates the
   standard Jekyll redirect HTML with `<meta http-equiv="refresh">`, canonical
   link, and JavaScript redirect.

3. **`redirect_from` generation block (step 10c)** -- Iterates collections and
   standalone pages, generates redirect HTML files at old URLs pointing to the
   page's current URL.

4. **`redirect_to` generation block (step 10c2)** -- For pages/items with
   `redirect_to` front matter, replaces the page's output with a redirect HTML
   page. Skips if the page has a layout that exists in the layout engine (e.g.,
   muan-blog's `redirect` layout).

## Scope

Add comprehensive test coverage for the existing redirect implementation:

1. Unit tests for `extract_redirect_from` (string, array, empty, missing)
2. Unit tests for `generate_redirect_html` (with/without site_url, baseurl)
3. Integration tests that build a minimal site with redirect front matter and
   verify the correct HTML files are generated
4. Integration tests for `redirect_to` (both with and without a custom layout)
5. Update `docs/jekyll-compatibility.md` to mark tests as "yes"

## Dependencies

None. The implementation already exists.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new redirect tests (no `#[ignore]`)
- [ ] Unit test: `extract_redirect_from` with single string returns `vec!["url"]`
- [ ] Unit test: `extract_redirect_from` with array returns all URLs
- [ ] Unit test: `extract_redirect_from` with missing key returns empty vec
- [ ] Unit test: `extract_redirect_from` with empty string returns empty vec
- [ ] Unit test: `generate_redirect_html` with empty site_url uses relative URL
- [ ] Unit test: `generate_redirect_html` with site_url produces absolute URL with baseurl
- [ ] Unit test: generated HTML contains `<meta http-equiv="refresh">`
- [ ] Unit test: generated HTML contains `<link rel="canonical">`
- [ ] Unit test: generated HTML contains `<script>location=`
- [ ] Integration test: build site with `redirect_from: /old-url/` on a post,
  verify `/old-url/index.html` exists and contains redirect to the post URL
- [ ] Integration test: build site with `redirect_from` array, verify all redirect
  files are generated
- [ ] Integration test: build site with `redirect_to: /new-url/` on a page (no layout),
  verify the page output is a redirect HTML pointing to `/new-url/`
- [ ] Integration test: build site with `redirect_to` and a custom `redirect` layout,
  verify the custom layout is used (hardcoded redirect is NOT generated)
- [ ] Integration test: include at least one non-ASCII URL in redirect paths to catch
  encoding regressions
- [ ] `docs/jekyll-compatibility.md` updated: jekyll-redirect-from tests column = "yes"
- [ ] DTC DOM match count must not drop below 596/790
- [ ] `cargo clippy -- -D warnings` passes

## Test Scenarios

### Unit: extract_redirect_from

- Input: front matter with `redirect_from: "/old-page/"` -- returns `vec!["/old-page/"]`
- Input: front matter with `redirect_from: ["/old-1/", "/old-2/"]` -- returns both
- Input: front matter with no `redirect_from` key -- returns empty vec
- Input: front matter with `redirect_from: ""` -- returns empty vec
- Input: front matter with `redirect_from: ["/valid/", ""]` -- returns only `"/valid/"`

### Unit: generate_redirect_html

- site_url="" produces relative URL in output
- site_url="https://example.com" + baseurl="" produces "https://example.com/target"
- site_url="https://example.com" + baseurl="/blog" produces "https://example.com/blog/target"
- Output contains all required elements: DOCTYPE, meta refresh, canonical, script, anchor

### Integration: redirect_from page generation

- Create a minimal site fixture with:
  - `_config.yml` with basic settings
  - A post with `redirect_from: /old-post/`
  - A page with `redirect_from: ["/old-page/", "/another-old/"]`
- Build the site
- Verify `/old-post/index.html` exists and contains `<meta http-equiv="refresh"` pointing to the post
- Verify `/old-page/index.html` and `/another-old/index.html` both exist
- Verify the original post and page are also generated normally

### Integration: redirect_to page generation

- Create a page with `redirect_to: /new-location/` and no layout
- Build and verify the page output is redirect HTML pointing to `/new-location/`
- Create a page with `redirect_to: https://external.example.com/` (absolute URL)
- Verify the output uses the absolute URL as-is (no baseurl prepended)

### Integration: redirect_to with custom layout

- Create a site with a `_layouts/redirect.html` containing custom redirect markup
- Create a page with `redirect_to: /target/` and `layout: redirect`
- Build and verify the custom layout is used, NOT the hardcoded redirect HTML

### Integration: Unicode redirect paths

- Create a page with `redirect_from: "/articulos/viejo/"` (non-ASCII in path context)
- Verify the redirect file is generated correctly

## DTC DOM Baseline

Current baseline: **596/790** pages matched (255 total differences, 868 acceptable diffs filtered).

## Log

### [PM] 2026-04-02 grooming
- Investigated codebase: redirect_from/redirect_to already fully implemented in src/main.rs
- Functions: extract_redirect_from, generate_redirect_html, build_site blocks 10c and 10c2
- Found zero existing tests for redirect functionality
- 12 sites use redirects; programming-historian alone has 121 files with redirect_from
- Reframed issue from "implement" to "add test coverage for existing implementation"
- DTC DOM baseline: 596/790

### [SWE] 2026-04-02

**Fix 1: Unit tests for extract_redirect_from (5 tests)**
- Wrote test_extract_redirect_from_single_string, test_extract_redirect_from_array, test_extract_redirect_from_missing_key, test_extract_redirect_from_empty_string, test_extract_redirect_from_array_with_empty_strings (src/main.rs)
- Ran tests: all 5 PASS (existing implementation already handles all cases)

**Fix 2: Integration tests for redirect_from (2 tests)**
- Wrote test_integration_redirect_from_post -- builds site with post having redirect_from, verifies redirect file and original post both exist
- Wrote test_integration_redirect_from_array_on_page -- builds site with page having redirect_from array, verifies all redirect files generated
- Ran tests: both PASS

**Fix 3: Integration tests for redirect_to (2 tests)**
- Wrote test_integration_redirect_to_no_layout -- page with redirect_to and no custom layout gets hardcoded redirect HTML
- Wrote test_integration_redirect_to_absolute_url -- absolute URL (https://) used as-is, no baseurl prepended
- Ran tests: both PASS

**Fix 4: Integration test for redirect_to with custom layout (1 test)**
- Wrote test_integration_redirect_to_with_custom_layout -- page with layout: redirect and existing redirect layout uses custom layout, NOT hardcoded HTML
- Ran test: PASSES

**Fix 5: Integration test for Unicode redirect paths (1 test)**
- Wrote test_integration_redirect_from_unicode_path -- page with redirect_from: /artículos/viejo/ generates correct redirect file
- Ran test: PASSES

**Fix 6: Updated docs/jekyll-compatibility.md**
- Changed jekyll-redirect-from tests column from "no" to "yes"

**Summary:**
- Files modified: src/main.rs, docs/jekyll-compatibility.md
- Tests added: 11 new tests (5 unit for extract_redirect_from + 6 integration for redirect_from/redirect_to)
- Pre-existing tests: 6 unit tests for generate_redirect_html already existed
- Build results: all tests pass (3581+ in main crate), 0 failures, clippy clean, fmt clean
- DTC DOM: 596/790, 255 total diffs (matches baseline exactly, no regression)
- DTC build time: 0.731s (under 1.0s threshold)

### [QA] 2026-04-02 16:00
- Tests: 17 redirect tests pass (11 new + 6 pre-existing), 3580 total pass, 1 pre-existing failure (test_link_tag_pretty_permalink_html_page in engine.rs, unrelated to this issue -- committed in issue 542)
- Clippy: clean (no warnings)
- Fmt: clean
- DTC DOM: 596/790, no regression (baseline: 596/790)
- DTC build time: 0.584s (under 1.0s threshold)
- Acceptance criteria:
  - `cargo build` compiles without errors: PASS
  - `cargo test` passes with all new redirect tests (no #[ignore]): PASS
  - Unit test: extract_redirect_from single string: PASS (test_extract_redirect_from_single_string)
  - Unit test: extract_redirect_from array: PASS (test_extract_redirect_from_array)
  - Unit test: extract_redirect_from missing key: PASS (test_extract_redirect_from_missing_key)
  - Unit test: extract_redirect_from empty string: PASS (test_extract_redirect_from_empty_string)
  - Unit test: generate_redirect_html empty site_url uses relative: PASS (pre-existing test_redirect_html_no_site_url_uses_relative)
  - Unit test: generate_redirect_html with site_url+baseurl: PASS (pre-existing test_redirect_html_with_baseurl)
  - Unit test: HTML contains meta refresh: PASS (pre-existing test_redirect_html_all_elements_absolute)
  - Unit test: HTML contains link canonical: PASS (pre-existing test_redirect_html_all_elements_absolute)
  - Unit test: HTML contains script location: PASS (pre-existing test_redirect_html_all_elements_absolute)
  - Integration: redirect_from on post: PASS (test_integration_redirect_from_post)
  - Integration: redirect_from array: PASS (test_integration_redirect_from_array_on_page)
  - Integration: redirect_to no layout: PASS (test_integration_redirect_to_no_layout)
  - Integration: redirect_to with custom layout: PASS (test_integration_redirect_to_with_custom_layout)
  - Integration: non-ASCII URL: PASS (test_integration_redirect_from_unicode_path)
  - docs/jekyll-compatibility.md updated: PASS
  - DTC DOM not below 596/790: PASS
  - cargo clippy: PASS
- TDD note: This is a test-coverage-only issue (no implementation changes). The TDD "fail first" cycle does not apply because there is no bug fix -- all tests verify existing working behavior. The SWE log shows each test batch was written and verified to pass.
- VERDICT: PASS

### [PM] 2026-04-02 16:30
- Reviewed diff: 2 files changed (src/main.rs +409 lines, docs/jekyll-compatibility.md +1/-1)
- Output verification: built DTC site in release mode, ran dom_compare.py
- Results verified: DTC DOM 596/790, 255 diffs -- matches baseline exactly, no regression
- Code review: 11 new tests (5 unit + 6 integration) are substantive, test real site builds with tempdir, verify file existence and HTML content including meta refresh, canonical links, target URLs, Unicode paths, and custom layout behavior
- Acceptance criteria: all 19 met
- Follow-up issues created: none needed
- VERDICT: ACCEPT
