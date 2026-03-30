# Issue 400: muan-blog -- multiple rendering issues (2199/2218)

## Problem

muan-blog is at 2199/2218 (99.1%) DOM match with 19 pages differing. Multiple
root causes affect different pages. This is a parent/tracking issue to coordinate
fixes across sub-issues.

### Known sub-categories

1. **iframe/img wrapped in `<p>` tags** (9 diffs across ~7 pages) -- tracked as #449
2. **Heading ID generation** (multiple pages) -- heading IDs differ from Jekyll output
   (e.g., `id='codeltdetailsgtcode-basics'` vs `id='details-basics'`)
3. **Datetime formatting** -- ISO 8601 format vs Jekyll's `datetime` attribute format
   (e.g., `2013-05-21T11:02:39+08:00` vs `2013-05-21T03:02:39+08:00`, timezone offset)
4. **URL encoding of Unicode** -- Unicode characters in URLs handled differently
5. **Text content differences** -- some link text or paragraph text split differently
   (e.g., mailto: links with special characters)
6. **`notes.html` list page** (24 diffs) -- structural differences in notes listing
7. **`posts/border-box-in-github.html`** (34 diffs) -- extra `<br>` elements in
   blockquotes, structural reordering

### Pages with diffs (from DOM comparison, partial list)

- `notes.html` (24 diffs)
- `posts/border-box-in-github.html` (34 diffs)
- `posts/presence.html` (3 diffs -- img in p)
- `posts/acceptance.html` (1 diff -- iframe in p)
- `posts/mission-focused.html` (1 diff -- iframe in p)
- `posts/details-on-details.html` (3 diffs -- iframe in p + heading IDs)
- `posts/leaving-github.html` (4 diffs -- iframe in p + link text)
- `posts/noise.html` (3 diffs -- iframe in p + datetime + missing br)
- Various `notes/` pages (2-14 diffs each)

## Scope

This is a **tracking issue**. The engineer must:
1. Rebuild muan-blog with current code
2. Run DOM comparison and categorize all remaining diffs
3. For each category, either fix it directly (if small) or create a focused sub-issue
4. The iframe/img-in-p subset is already tracked as #449

## Dependencies

- Issue #449 (iframe/img in p) should be completed first to reduce diff count

## Baseline

- DTC: 790/790 (must not regress)
- muan-blog: 2199/2218 (19 pages differ)

## Acceptance Criteria

- [ ] muan-blog site rebuilt with latest code
- [ ] DOM comparison re-run and results documented
- [ ] Every differing page categorized by root cause
- [ ] For each root cause category, either:
  - (a) Fixed in this issue, OR
  - (b) Existing sub-issue referenced (e.g., #449), OR
  - (c) New sub-issue created in `docs/tracker/`
- [ ] muan-blog match count improved from 2199/2218 (target: 2205+ after #449 lands)
- [ ] DTC DOM baseline remains at 790/790
- [ ] `cargo test` passes
- [ ] No regression on any other test site

## Test Scenarios

### Investigation: DOM diff categorization
- Build muan-blog, run DOM comparison, list all differing pages
- For each differing page, identify root cause category
- Verify heading ID differences are consistent (same algorithm issue)
- Check if datetime diffs are timezone-related or format-related
- Count how many diffs would be resolved by #449 alone

### Output verification
- After any fixes, rebuild muan-blog and re-run DOM comparison
- Verify fixed pages now match Jekyll output exactly
- Verify no previously-matching pages regressed

## Log

### [SWE] 2026-03-30

**Starting baseline**: muan-blog 2161/2218 (57 pages differ), DTC 790/790

#### Fix 1: redirect_to pages with custom redirect layout

**Root cause**: Pages with `redirect_to` in front matter AND a custom `redirect` layout
(e.g., `_layouts/redirect.html`) were being overwritten by rustkyll's hardcoded redirect
template. The normal rendering pipeline correctly rendered these pages using the custom
layout, but the redirect_to post-processing step (step 10c2) replaced the output with a
full HTML page including `<title>`, `<script>`, etc. -- while Jekyll's output was just
`<meta http-equiv="refresh" content="0; url=...">`.

**Fix**: Added a check in the redirect_to override: if the page/collection item has a
`layout` that exists in the layout engine, skip the hardcoded redirect override since
the normal rendering pipeline already rendered it correctly.

**Files modified**: `src/main.rs` (2 locations in redirect_to handling)

**Result**: 2161 -> 2189 (+28 pages fixed)

#### Fix 2: URL percent-encoding of non-ASCII characters in CommonMarkGhPages mode

**Root cause**: The `restore_non_ascii_in_urls` function always restored non-ASCII
characters as raw Unicode. For kramdown mode this is correct, but for CommonMarkGhPages
mode, the `commonmarker` gem used by Jekyll percent-encodes non-ASCII characters in link
URLs.

**Fix**: Added a `keep_unicode` parameter to `restore_non_ascii_in_urls`. When false
(CommonMarkGhPages mode), non-ASCII characters are percent-encoded instead of restored
as raw Unicode.

**Files modified**: `src/frontmatter.rs` (function signature + 3 call sites)

**Test added**: `test_url_with_non_ascii_percent_encoded_commonmark` in frontmatter.rs

**Result**: 2189 -> 2191 (+2 pages fixed: `posts/reparations.html`, `notes/2024-11-25-cc.html`)

#### Final score: muan-blog 2191/2218, DTC 790/790

#### Remaining 27 differing pages categorized by root cause:

**Category A: img not wrapped in `<p>` tags (issue #449) -- 15 pages, ~18 diffs**
- notes/2022-03-07-oo.html, notes/2022-05-07-oo.html, notes/2022-06-27-vv.html,
  notes/2022-07-04-ss.html, notes/2022-07-27-ww.html, notes/2022-08-11-ee.html,
  notes/2023-07-03-oo.html, notes/2024-11-11-oo.html, notes/2025-10-16-ff.html
- pages/goodies.html (2 img-in-p diffs)
- posts/2-years.html, posts/2013-in-contributions.html, posts/2014-in-contributions.html,
  posts/hubberversary.html (1 img-in-p each)
- posts/noise.html (3 img-in-p + other issues)
- posts/border-box-in-github.html (1 img-in-p + syntax highlighting)
- notes.html (1 img-in-p + other issues)

**Category B: Syntax highlighting class differences -- 3 pages**
- pages/hacking-with-swift/index.html (23 diffs)
- posts/border-box-in-github.html (42 diffs -- Ruby/CSS syntax highlighting)
- posts/first-pull-request.html (9 diffs -- CSS syntax highlighting)
Root cause: Different syntax highlighting engines produce different token classes
(e.g., `class='c'` vs `class='cm'` for comments).

**Category C: Datetime/timezone issues -- 3 pages**
- posts/github-hiring-story.html (2 diffs -- `PST` timezone abbreviation not resolved)
- posts/scribble-the-jekyll-theme.html (2 diffs -- naive datetime not converted to site timezone)
- posts/scribble.html (2 diffs -- same as above)
Root cause: Dates without explicit timezone (e.g., `2013-05-05 20:38:50`) are treated
differently. Jekyll interprets them as UTC and converts to site timezone (Asia/Taipei +08:00),
while rustkyll appends the timezone offset without converting. Also, `PST` timezone
abbreviation is not resolved.

**Category D: CommonMarkGhPages `<br>` + text rendering -- 3 pages**
- posts/depression.html (2 diffs -- `<br>` followed by text wrapped in `<p>`)
- posts/thoughts-on-reparations.html (2 diffs -- same pattern + `\-` escape)
- posts/noise.html (1 diff -- `<br>` vs `<p>`)
Root cause: In CommonMarkGhPages with HARDBREAKS, `<br>` followed by text on the
next line should not be wrapped in `<p>`. Also, `\- text` (backslash-escaped dash)
handling differs.

**Category E: CommonMarkGhPages code block wrapping -- 2 pages**
- notes/2025-09-24-ee.html (1 diff -- `<pre>` vs `<div class="highlighter-rouge">`)
- notes.html (1 diff -- same)
Root cause: CommonMarkGhPages outputs bare `<pre><code>` for fenced code blocks,
but rustkyll wraps them in `<div class="highlighter-rouge"><div class="highlight">`.

**Category F: Mailto link with special characters -- 1 page**
- posts/leaving-github.html (2 diffs)
Root cause: Markdown link `[text](mailto:...?subject=...&body=...[reasons].)` with
`[reasons]` inside URL confuses the markdown parser.

**Category G: photos.html smart punctuation -- 1 page**
- photos.html (10 diffs -- `...` vs ellipsis, `<br>` handling in figcaption)
Root cause: Three periods being converted to ellipsis character, and line breaks
in figcaption content not being preserved as `<br>`.

**Category H: notes.html details element text wrapping -- 1 page**
- notes.html (4 diffs -- text inside `<details>` wrapped differently)
Root cause: Text directly inside `<details>` elements is wrapped in `<p>` tags
by rustkyll but kept as bare text by Jekyll's CommonMarkGhPages.

#### Build/lint results
- `cargo test`: all tests pass
- `cargo clippy -- -D warnings`: clean
- `cargo fmt --check`: clean
- DTC: 790/790 (no regression)
- Full DOM comparison on all sites: no regressions
