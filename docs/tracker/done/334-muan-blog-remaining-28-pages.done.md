# Issue 334: muan-blog -- remaining 28/2218 pages with diffs

## Problem

After issue 330 fixed 18 pages (2172 -> 2190/2218), 28 pages still have diffs. These are from categories NOT addressed in issue 330 (Categories A/B were inapplicable; Categories C/D/E/F were fixed where applicable).

Detailed DOM comparison analysis reveals these root-cause categories:

### Category 1: Underscore-in-text treated as emphasis (7 pages, easy)

The kaomoji `¯\_(ツ)_/¯` and `*Controller` / `*other*` inside backtick code spans are being parsed as emphasis (`<em>`) instead of literal text.

Affected pages:
- `notes/2020-09-10-ww.html` (3 diffs) -- `¯\_(ツ)_/¯` rendered with `<em>` instead of literal underscores
- `notes/2023-08-24-uu.html` (5 diffs) -- same kaomoji issue, also affects meta description
- `notes/2025-02-27-cc.html` (3 diffs) -- same kaomoji issue
- `pages/hacking-with-swift/index.html` (partial, 2 of 26 diffs) -- `*Controller` in code span becomes `<em>`
- `posts/emoji-code.html` (2 diffs) -- `*other*` in code span becomes `<em>`
- `photos.html` (partial, some diffs) -- `...` vs `…` smart punctuation, plus hardbreak rendering

### Category 2: Permalink generation -- date prefix in post URLs (6 pages, medium)

Posts link to each other via `[text](/posts/title)` but rustkyll generates permalinks as `/posts/YYYY-MM-DD-title` instead of `/posts/title`. The `_config.yml` has `permalink: /posts/:title` which should strip the date prefix.

Affected pages:
- `posts/reparations.html` (1 of 2 diffs) -- href `/posts/2020-06-06-thoughts-on-reparations` should be `/posts/thoughts-on-reparations`
- `posts/thoughts-on-reparations.html` (3 of 5 diffs) -- href `/posts/2020-06-06-reparations` should be `/posts/reparations` (3 links)
- `pages/goodies.html` (1 diff) -- href `/posts/2024-11-02-javascript` should be `/posts/javascript`

Note: The `reparations.html` and `thoughts-on-reparations.html` pages cross-reference each other, so fixing the permalink will fix both directions. This may also affect the link in `leaving-github.html`.

### Category 3: Timezone handling -- UTC vs Asia/Taipei (4 pages, medium)

The `_config.yml` specifies `timezone: Asia/Taipei` (+08:00) but rustkyll renders some dates in UTC. This affects both `<time datetime="...">` attributes and displayed date text.

Affected pages:
- `posts/scribble.html` (2 diffs) -- `2013-05-06T04:38:50+08:00` rendered as `2013-05-05T20:38:50+08:00`
- `posts/scribble-the-jekyll-theme.html` (2 diffs) -- same timestamp issue
- `posts/noise.html` (1 of 5 diffs) -- `2013-05-21T11:02:39+08:00` rendered as `2013-05-21T03:02:39+08:00`
- `posts/github-hiring-story.html` (3 of 4 diffs) -- `2013-07-25T06:34:00+08:00` rendered as `2013-07-24 14:34:00 PST`, also wrong date format

### Category 4: Iframe rendering (4 pages, medium)

Iframes in markdown source are rendered as `<p>` instead of `<iframe>`. Jekyll preserves HTML iframe elements as-is; rustkyll wraps them in paragraph tags or strips them.

Affected pages:
- `posts/acceptance.html` (1 diff) -- iframe becomes `<p>`
- `posts/details-on-details.html` (1 diff) -- iframe becomes `<p>`
- `posts/leaving-github.html` (1 of 3 diffs) -- iframe becomes `<p>`
- `posts/mission-focused.html` (1 diff) -- iframe becomes `<p>`
- `posts/noise.html` (1 of 5 diffs) -- iframe becomes `<p>`

### Category 5: Image rendering -- standalone images as block elements (2 pages, medium)

Standalone `<img>` tags in markdown are wrapped in `<p>` instead of being rendered as block-level elements.

Affected pages:
- `posts/presence.html` (3 diffs) -- 3 images expected as direct `<img>` children, rendered as `<p>` wrappers
- `posts/github-hiring-story.html` (1 of 4 diffs) -- same issue with 1 image

### Category 6: URL encoding -- non-ASCII characters in href (2 pages, easy)

Links with non-ASCII characters in URLs are not being percent-encoded to match Jekyll output.

Affected pages:
- `posts/reparations.html` (1 of 2 diffs) -- `href='https://zh.wikipedia.org/zh-tw/轉型正義'` should be percent-encoded
- `notes/2024-11-25-cc.html` (1 diff) -- Chinese characters in Yahoo News URL not percent-encoded

### Category 7: `<details>` content rendering in excerpts (2 pages, medium)

Content inside `<details>` blocks still gets `<p>` wrappers in the notes index page excerpts. Issue 330 fixed this for individual pages but the notes listing (notes.html) still shows the issue.

Affected pages:
- `notes.html` (5 diffs) -- details content has `<p>` wrapping in note excerpts
- `notes/2025-09-24-ee.html` (1 diff) -- `<pre>` expected but `<div>` rendered

### Category 8: Meta description truncation with unmatched quotes (1 page, hard)

Unmatched double quote in meta description causes HTML attribute parsing to break. Jekyll truncates before the problematic quote; rustkyll includes it.

Affected page:
- `notes/2023-01-25-mm.html` (14 diffs) -- content has `"I'm tentatively scheduled to die."` and the unmatched quote breaks the meta content attribute

### Category 9: Hardbreak rendering and smart punctuation (partial diffs in photos.html)

`photos.html` has `<br>` vs no `<br>` diffs and `...` vs `…` (smart ellipsis) diffs.

### Category 10: Syntax highlighting class differences (pages/hacking-with-swift)

24 of 26 diffs in `pages/hacking-with-swift/index.html` are syntax highlighting span class differences (`class='p'` vs `class='o'`, etc.). This is a rouge/syntect tokenization difference.

### Category 11: Complex HTML rendering (posts/border-box-in-github)

34 diffs from inline HTML in markdown being processed differently. This page mixes raw HTML with markdown in complex ways.

### Category 12: Misc rendering issues

- `posts/depression.html` (2 diffs) -- `<br>` and `<hr>` rendering differences
- `posts/noise.html` (remaining diffs) -- `**text **` with trailing space before `**` parsed as bold vs literal
- `posts/thoughts-on-reparations.html` (remaining 2 diffs) -- `<br>` and `\- text` rendering
- `posts/leaving-github.html` (1 of 3 diffs) -- `mailto:` link truncation
- `notes/2024-11-23-mm.html` (1 diff) -- datetime format `2024-11-23 20:16:52 +0800` vs `2024-11-23T20:16:52+08:00`
- `posts/first-pull-request.html` (8 diffs) -- syntax highlighting structure (table vs spans)

## Scope

Fix categories that are achievable and have cross-site value. Priority:

1. **Category 2: Permalink `:title` stripping date prefix** (fixes 3 pages, 6+ diffs) -- High value, likely a bug in permalink generation that affects other sites too.
2. **Category 6: URL percent-encoding for non-ASCII hrefs** (fixes 2 pages, 2 diffs) -- Small fix, generic.
3. **Category 1: Underscore emphasis suppression** (fixes up to 5 pages) -- Only fix the kaomoji `¯\_(ツ)_/¯` case and `*text*` inside backtick code spans if feasible without regressions.
4. **Category 3: Timezone application** (fixes 3-4 pages) -- Use configured timezone for date rendering.
5. **Category 4: Iframe preservation** (fixes 4-5 pages) -- HTML iframes should pass through markdown rendering unchanged.

**Explicitly out of scope for this issue** (tracked separately or accepted as known limitations):
- `pages/blogroll.html` -- Random `sample:` order, non-deterministic by nature. Skip.
- `pages/hacking-with-swift` -- 24 syntax highlighting class diffs are rouge/syntect tokenization differences. Tracked below.
- `posts/border-box-in-github.html` -- 34 diffs from complex inline HTML mixing. Tracked below.
- `posts/first-pull-request.html` -- Syntax highlighting structure differences. Tracked below.
- `notes/2023-01-25-mm.html` -- Unmatched quote in meta description. Tracked below.

## Dependencies

- Issue 330 must be `.done.md` first

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `./scripts/cargo-safe test` passes
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] muan-blog DOM match reaches at least 2200/2218 (up from 2190, fixing 10+ pages)
- [ ] Permalink `:title` pattern strips date prefix from post filenames (e.g., `2020-06-06-reparations` becomes `reparations` when permalink is `/posts/:title`)
- [ ] Non-ASCII characters in `href` attributes are percent-encoded to match Jekyll output
- [ ] Timezone from `_config.yml` is applied to post date rendering (datetime attributes and displayed text)
- [ ] HTML `<iframe>` elements in markdown source are preserved as-is, not wrapped in `<p>` tags
- [ ] No regressions on DTC (must remain at current level or better)
- [ ] No regressions on any site currently at 100%
- [ ] Each unfixed page from the 28 is documented with the reason it was not fixed
- [ ] Tests include non-ASCII/Unicode content where applicable
- [ ] At least 8 new test functions covering the fixes

## Test Scenarios

### Unit: Permalink `:title` strips date prefix

- Given `_config.yml` has `permalink: /posts/:title` and a post file named `2020-06-06-reparations.md`
- The generated URL must be `/posts/reparations` (not `/posts/2020-06-06-reparations`)
- Test with multiple date formats: `YYYY-MM-DD-title`, verify the date prefix is stripped
- Verify that posts with NO date prefix in filename are unaffected

### Unit: URL percent-encoding for non-ASCII hrefs

- Given a markdown link `[text](https://zh.wikipedia.org/zh-tw/轉型正義)`
- The rendered `href` must be `https://zh.wikipedia.org/zh-tw/%E8%BB%89%E5%9E%8B%E6%AD%A3%E7%BE%A9`
- Test with Chinese, Japanese, and mixed ASCII/CJK URLs
- Verify that already-encoded URLs (e.g., `%E8%BB%89`) are not double-encoded

### Unit: Timezone application to post dates

- Given `_config.yml` with `timezone: Asia/Taipei` and a post with front matter `date: 2013-05-05T20:38:50+00:00`
- The rendered datetime attribute must show the Taipei time: `2013-05-06T04:38:50+08:00`
- The displayed date text must be `May 6, 2013` (not `May 5, 2013`)
- Test with `timezone: America/New_York` to verify generic timezone support

### Unit: Iframe preservation in CommonMark/GFM

- Given markdown content containing `<iframe src="https://example.com" width="560"></iframe>`
- The rendered HTML must contain the `<iframe>` element, not `<p>&lt;iframe...&gt;</p>`
- Test with iframe on its own line (block-level) and iframe preceded/followed by text

### Unit: Kaomoji underscore handling (if addressed)

- Given markdown text containing `¯\_(ツ)_/¯`
- The underscores must NOT be parsed as emphasis markers
- The rendered text must be `¯_(ツ)_/¯` with literal underscores (no `<em>` tags)
- Test also: `¯\_(ツ)_/¯` at end of paragraph, in middle of sentence

### Integration: muan-blog full site DOM comparison

- Build muan-blog with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify at least 2200/2218 pages match
- Spot-check previously-failing pages:
  - `pages/goodies.html` -- link to `/posts/javascript` not `/posts/2024-11-02-javascript`
  - `posts/reparations.html` -- href percent-encoded, link to `/posts/thoughts-on-reparations`
  - `posts/scribble.html` -- datetime `2013-05-06T04:38:50+08:00`, text `May 6, 2013`
  - `posts/acceptance.html` -- `<iframe>` preserved, not `<p>`
  - `notes/2024-11-25-cc.html` -- Chinese URL percent-encoded

### Regression: Other sites

- Run `./scripts/cargo-safe test` full suite
- Verify DTC score does not regress
- Verify no regression on any currently-passing site

## Output Verification

```bash
./scripts/cargo-safe build --release
./target/release/rustkyll build \
  --source websites/muan-blog \
  --destination /tmp/muan_334

uv run scripts/dom_compare.py \
  --jekyll-dir websites/muan-blog/_site_jekyll_cached \
  --rustkyll-dir /tmp/muan_334
```

Expected: 2200+ files matched (up from 2190).

Spot-checks:
```bash
# Permalink: should link to /posts/javascript (no date prefix)
grep 'href.*javascript' /tmp/muan_334/pages/goodies.html

# URL encoding: should have percent-encoded Chinese characters
grep 'wikipedia' /tmp/muan_334/posts/reparations.html

# Timezone: should show May 6, 2013 (Taipei time)
grep 'datetime' /tmp/muan_334/posts/scribble.html

# Iframe: should have <iframe>, not <p>
grep 'iframe\|<p>' /tmp/muan_334/posts/acceptance.html | head -5
```

## Notes

- This is a CommonMarkGhPages site, NOT kramdown. All fixes must respect the GFM rendering pipeline.
- The permalink `:title` fix is likely the highest-leverage change since it affects 3 pages and is a correctness bug that could affect any Jekyll site using `permalink: /posts/:title`.
- Timezone fixes must be generic (use the timezone from `_config.yml`), not hardcoded to Asia/Taipei.
- URL percent-encoding should follow RFC 3986 for non-ASCII characters, matching Jekyll's behavior.
- The kaomoji fix is tricky -- `\_` is a valid backslash-escaped underscore in CommonMark that should suppress emphasis. If the GFM parser already handles this, the issue may be elsewhere in the pipeline.
- `pages/blogroll.html` (99 diffs from random `sample:` order) is explicitly excluded and will remain a known non-deterministic difference.

## Log

### [SWE] 2026-03-24

TDD cycle for 5 fixes:

**Fix 1: Permalink :title date prefix stripping in {% link %} tag**
- Wrote tests: test_link_tag_strips_date_prefix_for_posts, test_link_tag_strips_date_prefix_for_posts_with_hyphens_in_title, test_link_tag_non_post_collection_unchanged
- Ran tests: FAILS as expected -- got "/posts/2024-11-02-javascript", expected "/posts/javascript"
- Implemented: Added strip_post_date_prefix_from_link() in src/template/engine.rs to strip YYYY-MM-DD- prefix from post filenames in {% link %} tag resolution
- Ran tests: PASSES -- all 3 tests pass

**Fix 2: URL percent-encoding for non-ASCII hrefs (CommonMarkGhPages)**
- Wrote tests: test_commonmark_non_ascii_url_percent_encoded, test_commonmark_non_ascii_url_already_encoded_not_double_encoded
- Ran tests: FAILS as expected -- got raw Chinese chars, expected percent-encoded
- Implemented: Added restore_non_ascii_in_urls_percent_encoded() in src/frontmatter.rs; for CommonMarkGhPages (add_code_classes=false), percent-encode instead of restoring raw chars
- Ran tests: PASSES -- both tests pass

**Fix 3: Timezone handling for YYYY-MM-DD HH:MM:SS (no timezone)**
- Wrote tests: test_naive_datetime_with_seconds_treated_as_utc_and_converted, test_naive_datetime_with_seconds_no_tz_becomes_utc, test_naive_datetime_america_new_york
- Ran tests: FAILS as expected -- got "2013-05-05 20:38:50" unchanged, expected "2013-05-06 04:38:50 +0800"
- Implemented: In expand_date_only_string_with_tz() (src/template/context.rs), added handling for 19-char YYYY-MM-DD HH:MM:SS format; treats as UTC per Ruby YAML convention and converts to site timezone
- Ran tests: PASSES -- all 3 tests pass

**Fix 4: Iframe preservation in GFM**
- Wrote test: test_iframe_preserved_as_block_html
- Ran test: FAILS as expected -- got "<p><iframe...></iframe></p>", expected iframe without p wrapper
- Implemented: Added unwrap_p_around_iframes() in src/kramdown.rs to strip <p> wrapping; also added "iframe" to CONTAINER_TAGS and BLOCK_TAGS lists in wrap_bare_text_in_paragraphs() to prevent re-wrapping
- Ran test: PASSES

**Fix 5: Kaomoji underscore emphasis suppression (CommonMarkGhPages)**
- Wrote tests: test_kaomoji_underscores_not_emphasis, test_kaomoji_at_end_of_sentence
- Ran tests: FAILS as expected -- got "¯<em>(ツ)</em>/¯", expected literal underscores
- Root cause: fix_literal_underscore_emphasis() and fix_literal_asterisk_emphasis() (kramdown-specific postprocessing) were running for CommonMarkGhPages sites, converting literal _text_ to <em>text</em> where pulldown-cmark had correctly left it as plain text
- Implemented: Guard fix_literal_asterisk_emphasis and fix_literal_underscore_emphasis with indent_lists flag (only run for kramdown mode)
- Ran tests: PASSES

**Results:**
- muan-blog DOM match: 2202/2218 (up from 2190, +12 pages)
- DTC DOM match: 765/790 (up from 751, +14 pages -- timezone fix helped)
- All tests pass: 2823 unit + integration tests, 0 failures
- clippy clean (also fixed 3 pre-existing clippy warnings)
- cargo fmt clean

**Files modified:**
- src/template/engine.rs -- permalink date prefix stripping in {% link %} tag
- src/frontmatter.rs -- URL percent-encoding for CommonMarkGhPages, iframe test, kaomoji test
- src/template/context.rs -- timezone handling for YYYY-MM-DD HH:MM:SS
- src/kramdown.rs -- iframe unwrapping, kaomoji fix (guard emphasis postprocessing by mode)

**Remaining 16 pages with diffs (documented):**
- pages/blogroll.html (99 diffs) -- random sample: order, non-deterministic by nature
- pages/hacking-with-swift/index.html (26 diffs) -- syntax highlighting class differences (rouge vs syntect)
- posts/border-box-in-github.html (34 diffs) -- complex inline HTML mixing
- posts/first-pull-request.html (8 diffs) -- syntax highlighting structure
- notes/2023-01-25-mm.html (14 diffs) -- unmatched quote in meta description
- posts/depression.html (2 diffs) -- br/hr rendering differences
- posts/noise.html (5 diffs) -- trailing space emphasis, timezone, iframe
- posts/thoughts-on-reparations.html (2 diffs) -- br and \- text rendering
- posts/leaving-github.html (3 diffs) -- mailto: link truncation, iframe
- notes/2024-11-23-mm.html (1 diff) -- datetime format difference
- notes/2025-09-24-ee.html (1 diff) -- pre vs div rendering
- notes.html (5 diffs) -- details content p-wrapping in excerpts
- photos.html -- smart punctuation ellipsis and hardbreak
- posts/presence.html (3 diffs) -- standalone img block elements
- posts/github-hiring-story.html (4 diffs) -- timezone, img, date format
- posts/emoji-code.html (2 diffs) -- *other* in code span

### [QA] 2026-03-24

**Test Results:**
- `./scripts/cargo-safe test`: 2823 passed, 0 failed (1 flaky perf test passed on retry, unrelated to issue 334)
- `./scripts/cargo-safe clippy -- -D warnings`: clean (only renamed lint warnings from liquid-lib dep)
- `cargo fmt --check`: clean

**DOM Regression Check:**
- muan-blog: 2202/2218 (up from 2190, +12 pages) -- exceeds 2200 threshold
- DTC: 765/790 (up from 751, +14 pages -- timezone fix helped DTC too)

**Spot Checks:**
- Permalink date prefix stripping: `/posts/javascript` in goodies.html (PASS)
- URL percent-encoding: Chinese chars encoded as `%E8%BD%89%E5%9E%8B%E6%AD%A3%E7%BE%A9` in reparations.html (PASS)
- Timezone handling: `2013-05-06T04:38:50+08:00` in scribble.html (PASS)
- Iframe preservation: `<iframe>` as block element in acceptance.html, no `<p>` wrapper (PASS)

**Acceptance Criteria:**
1. `cargo build` compiles: PASS
2. `./scripts/cargo-safe test` passes: PASS
3. `./scripts/cargo-safe clippy -- -D warnings`: PASS
4. `cargo fmt --check`: PASS
5. muan-blog DOM >= 2200/2218: PASS (2202)
6. Permalink `:title` strips date prefix: PASS
7. Non-ASCII URL percent-encoding: PASS
8. Timezone from _config.yml applied: PASS
9. Iframe elements preserved: PASS
10. No DTC regressions: PASS (765, improved from 751)
11. No regressions on 100% sites: PASS
12. Unfixed pages documented: PASS (16 pages with reasons)
13. Tests include non-ASCII/Unicode: PASS
14. At least 8 new test functions: PASS (11 new tests)

**VERDICT: PASS**

### [PM] 2026-03-24

**Acceptance Review**

Reviewed code diff (1135 lines added across 12 files) and QA report.

**Acceptance Criteria Verification:**

1. cargo build compiles: PASS
2. cargo-safe test passes (2823 tests): PASS
3. clippy clean: PASS
4. cargo fmt clean: PASS
5. muan-blog DOM >= 2200/2218: PASS (2202, +12 from baseline 2190)
6. Permalink :title strips date prefix: PASS -- strip_post_date_prefix_from_link() in engine.rs correctly strips YYYY-MM-DD- from post filenames in link tag resolution
7. Non-ASCII URL percent-encoding: PASS -- restore_non_ascii_in_urls_percent_encoded() correctly percent-encodes for CommonMarkGhPages mode only
8. Timezone from _config.yml applied: PASS -- expand_date_only_string_with_tz() now handles 19-char naive datetimes, treating as UTC per Ruby YAML convention, converting to site timezone
9. Iframe elements preserved: PASS -- unwrap_p_around_iframes() strips p-wrappers around iframe elements
10. No DTC regressions: PASS (765/790, improved +14 from 751 due to timezone fix)
11. No regressions on 100% sites: PASS
12. Unfixed pages documented: PASS (16 pages with root causes in SWE log)
13. Tests include non-ASCII/Unicode: PASS (Chinese character URL tests)
14. At least 8 new test functions: PASS (11 test functions for issue 334)

**Code Quality Notes:**
- Each fix is properly scoped: emphasis postprocessing guarded by indent_lists flag (kramdown-only), percent-encoding guarded by add_code_classes flag (CommonMarkGhPages-only)
- Permalink fix is generic (works for any site using :title permalink pattern)
- Timezone fix is generic (tested with both Asia/Taipei and America/New_York)
- No unwrap() in library code paths

**Note:** The diff also includes changes for issues 335 and 336 which are being developed in parallel. These are additive and do not affect the 334 acceptance review.

**VERDICT: ACCEPT**
