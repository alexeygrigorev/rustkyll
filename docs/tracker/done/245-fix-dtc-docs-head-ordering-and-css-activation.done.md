# Issue 245: Fix DTC/docs DOM match blockers (absolute_url doubling, SEO title stripping)

## Problem

Issue 233 fixed the core just-the-docs theme support. The original issue 245 description hypothesized head element ordering and CSS activation issues, but PM grooming comparison of a fresh `bundle exec jekyll build` against rustkyll output reveals those are already fixed. The actual blockers preventing DOM matches are different.

**Current state**: 0/57 DOM matches. Fresh Jekyll build compared against rustkyll reveals:

### Root Cause 1: `absolute_url` filter doubles already-absolute URLs (CRITICAL - blocks ALL 57 pages)

The `absolute_url` Liquid filter prepends `site.url + site.baseurl` unconditionally. When the input is already an absolute URL (starts with `http://` or `https://`), this produces doubled URLs like `https://datatalks.club/https://datatalks.club/`.

In Jekyll, `absolute_url` checks if the input already starts with a protocol scheme and returns it unchanged if so. Rustkyll does not perform this check.

This affects the external nav link in the just-the-docs `site_nav.html` include:
```liquid
<a href="{{ node.url | absolute_url }}" ...>
```
Where `node.url` is `https://datatalks.club/` from `_config.yml:nav_external_links`. Every page includes the nav, so all 57 pages have this wrong `href`.

Fixing this alone would give 33/57 DOM matches (pages where the URL doubling is the only difference).

### Root Cause 2: SEO tag title includes `nav_order` number prefix (blocks 9 ML Zoomcamp pages)

The ML Zoomcamp subpages have front matter like `title: "1. Your First Actions"`. Jekyll's `jekyll-seo-tag` produces `<title>Your First Actions | DataTalks.Club Documentation</title>` (stripping the leading number prefix), while rustkyll produces `<title>1. Your First Actions | DataTalks.Club Documentation</title>`.

Investigation shows this is a just-the-docs theme behavior: the theme sets a `seo_title` or modifies the page title before the SEO tag processes it, stripping leading `N. ` patterns from titles that have `nav_order` set. This affects title, og:title, twitter:title, and JSON-LD headline on 9 pages.

Fixing both root causes would give 42/57 DOM matches (74%).

### Not in scope (separate follow-up issues)

The remaining 15 pages have other differences:
- 8 parent pages missing auto-generated child page listings (`<hr>`, `<h2>`, `<ul>` children) -- this is a just-the-docs `has_children` feature
- 3 pages with kramdown IAL `{: .class }` not parsed (rendered as literal text)
- 1 page with JSON-LD `Q&A` vs `Q&amp;A` entity encoding
- 1 page with missing `language-plaintext` CSS class on code blocks
- 1 page with markdown emphasis/italic parsing difference
- 1 page with text quoting difference

## Descoped from

Issue 233 acceptance criteria:
- "head elements appear in the same order as fresh Jekyll output" -- VERIFIED ALREADY FIXED
- "At least 40/57 pages achieve DOM match (>70%)" -- this issue targets this

## Acceptance Criteria

- [ ] `absolute_url` filter returns the input unchanged when it already starts with `http://` or `https://` (matching Jekyll behavior)
- [ ] The external nav link in DTC/docs output produces `href="https://datatalks.club/"` (not doubled)
- [ ] SEO tag strips leading `N. ` number prefix from titles when the page has `nav_order` front matter set (matching Jekyll/just-the-docs behavior)
- [ ] `<title>` for `courses/ml-zoomcamp/getting-started/index.html` is `Your First Actions | DataTalks.Club Documentation` (not `1. Your First Actions | ...`)
- [ ] JSON-LD key ordering in SEO tag output matches Jekyll's alphabetical ordering: `@context`, `@type`, then remaining keys sorted alphabetically
- [ ] DOM comparison: at least 40/57 pages achieve DOM match when comparing against `bundle exec jekyll build` output
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing and new tests
- [ ] `cargo clippy -- -D warnings` passes (ignoring vendor warnings)

## Dependencies

- Issue 233 (done)

## Test Scenarios

### TDD approach: write each test FIRST, verify it FAILS, implement the fix, verify it PASSES.

### Unit: `absolute_url` filter with already-absolute URLs

1. Write a test that applies the `absolute_url` filter to `https://example.com/path` with `site.url` set to `https://mysite.com`. Verify the output is `https://example.com/path` (unchanged). Run test, verify it FAILS (currently produces `https://mysite.com/https://example.com/path`). Implement the protocol-scheme check in `absolute_url`. Run test, verify it PASSES.

2. Write a test that applies `absolute_url` to `http://other.com/` (http, not https). Verify it returns `http://other.com/` unchanged. Run test, verify it FAILS. Verify it PASSES after the same fix.

3. Write a test that applies `absolute_url` to `/relative/path`. Verify it still prepends `site.url` to produce `https://mysite.com/relative/path`. This is a regression test -- should PASS both before and after the fix.

### Unit: SEO tag title stripping of nav_order number prefix

4. Write a test that renders a page with front matter `title: "3. My Page Title"` and `nav_order: 2`, with `{% seo %}` tag. Verify the `<title>` output contains `My Page Title` without the `3. ` prefix. Run test, verify it FAILS (currently includes `3. `). Implement the title stripping. Run test, verify it PASSES.

5. Write a test that renders a page with front matter `title: "Regular Title"` (no leading number) and `nav_order: 1`, with `{% seo %}` tag. Verify the `<title>` contains `Regular Title` unchanged. This should PASS both before and after (no regression).

6. Write a test that renders a page with front matter `title: "5. Numbered Title"` but WITHOUT `nav_order` set. Verify the `<title>` contains `5. Numbered Title` (number NOT stripped because nav_order is absent). This is a regression guard.

### Unit: JSON-LD key ordering

7. Write a test that renders a page with SEO tag and verifies the JSON-LD keys appear in alphabetical order after `@context` and `@type`. Run test, verify it FAILS (keys currently in insertion order). Implement sorted key output. Run test, verify it PASSES.

### Integration: DTC/docs full site DOM match

8. Write an `#[ignore]` integration test that builds DTC/docs with rustkyll, then checks that the external nav link `href` attribute in `index.html` equals `https://datatalks.club/` (not doubled). Run test, verify it FAILS. Verify it PASSES after `absolute_url` fix.

9. Write an `#[ignore]` integration test that builds DTC/docs, extracts `<title>` from `courses/ml-zoomcamp/getting-started/index.html`, and verifies it equals `Your First Actions | DataTalks.Club Documentation`. Run test, verify it FAILS. Verify it PASSES after SEO title fix.

### Output verification

10. After all fixes, build DTC/docs with `./target/release/rustkyll build --source websites/DataTalksClub/docs --destination /tmp/dtc-docs-test` and run `python3 scripts/dom_compare.py --jekyll-dir /tmp/dtc-docs-jekyll-fresh --rustkyll-dir /tmp/dtc-docs-test`. Verify at least 40/57 DOM matches.

## Log

### [PM] 2026-03-19: Grooming

- Read original issue 245 (descoped from issue 233)
- Built DTC/docs with fresh Jekyll (`bundle exec jekyll build`) at `/tmp/dtc-docs-jekyll-fresh/`
- Built DTC/docs with rustkyll at `/tmp/dtc-docs-pm/`
- Compared head element ordering element-by-element: ALREADY MATCHING (both have: meta, meta, link, link, style, script, script, meta, link, title, meta, ...)
- Compared CSS activation selectors: ALREADY CORRECT (`:nth-child` selectors present, not fallback)
- Missing meta element from original report: NOT A REAL ISSUE (fresh Jekyll has same elements)
- Ran `dom_compare.py`: 0/57 matches, 148 total differences
- Identified actual root causes:
  - Root cause 1: `absolute_url` filter doubles already-absolute URLs (affects all 57 pages as external nav link)
  - Root cause 2: SEO title includes `nav_order` number prefix (9 ML Zoomcamp pages have `"N. Title"` instead of `"Title"`)
  - Root cause 3: JSON-LD key ordering differs (minor, but affects comparison)
- 33 pages have ONLY the URL doubling bug; fixing it alone gives 33/57 matches
- Fixing both URL doubling + title prefix gives 42/57 matches (74%)
- Descoped 15 pages with other differences to new issue 246
- Created `docs/tracker/246-fix-dtc-docs-remaining-dom-diffs.todo.md` for: child page listings (8 pages), kramdown IAL (3 pages), entity encoding (1 page), language-plaintext class (1 page), emphasis parsing (1 page), text quoting (1 page)
- Renamed `.todo.md` to `.groomed.md`

### [SWE] 2026-03-19: Implementation (TDD)

**Fix 1: `absolute_url` filter skips already-absolute URLs**
- Wrote tests `test_absolute_url_skips_https_input` and `test_absolute_url_skips_http_input` (src/template/filters/absolute_url.rs)
- Ran tests: FAIL as expected -- got `https://mysite.com/https://example.com/path`, expected `https://example.com/path`
- Wrote regression test `test_absolute_url_still_prepends_for_relative_paths`: PASSES before fix
- Implemented fix: early return in `AbsoluteUrlFilter::evaluate()` when input starts with `http://` or `https://`
- Ran tests: PASS -- all 3 tests pass

**Fix 2: SEO title strips leading `N. ` prefix when `nav_order` is set**
- Wrote test `test_seo_title_strips_number_prefix_with_nav_order` (src/template/seo_tag.rs)
- Ran test: FAIL as expected -- title shows `3. My Page Title` instead of `My Page Title`
- Wrote regression tests: `test_seo_title_no_strip_without_nav_order` (PASSES), `test_seo_title_no_strip_without_number_prefix` (PASSES)
- Wrote test `test_seo_og_title_strips_number_prefix_with_nav_order`: FAILS as expected
- Implemented fix: added `strip_nav_order_prefix()` function, called when `page.nav_order` is present in context
- Ran tests: PASS -- all 4 tests pass

**Fix 3: JSON-LD key ordering (alphabetical after @context, @type)**
- Wrote test `test_jsonld_keys_alphabetical_order` (src/template/seo_tag.rs)
- Ran test: FAIL as expected -- `url` comes before `datePublished`
- Implemented fix: sort `jsonld_fields[2..]` before joining
- Updated two pre-existing tests (`test_jsonld_homepage_name_position`, `test_jsonld_article_date_published_position`) to match new alphabetical ordering
- Ran tests: PASS -- all 27 JSON-LD tests pass

**Results:**
- All new tests (8 tests): PASS
- Full lib test suite: 1808 passed, 5 failed (all 5 pre-existing kramdown failures from uncommitted work on other issues)
- Clippy (lib): clean (only vendor warnings from liquid-core)
- Format: clean

**Files modified:**
- `src/template/filters/absolute_url.rs` -- early return for already-absolute URLs + 3 new tests
- `src/template/seo_tag.rs` -- `strip_nav_order_prefix()` function, nav_order check in render_to, JSON-LD key sorting, 5 new tests + 2 updated tests
