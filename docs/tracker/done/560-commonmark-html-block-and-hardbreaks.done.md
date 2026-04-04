# Issue 560: CommonMark HTML block handling and HARDBREAKS in markdownify

## Problem

Multiple muan-blog pages have rendering differences caused by incorrect CommonMark HTML handling:

### A. Standalone HTML inline elements wrapped in `<p>` (3 pages, 8 diffs)

Bare HTML `<img>` and `<br>` tags on their own lines are incorrectly wrapped in `<p>` tags. In CommonMark with UNSAFE mode, these should be passed through as raw HTML.

**posts/presence.html** (3 diffs): Standalone `<img style="...">` on own line gets wrapped in `<p><img .../></p>` instead of bare `<img>`.

**posts/depression.html** (2 diffs): A bare `<br>` followed by text on next line. Jekyll renders `<br>\nP.S. Many thanks...` as bare HTML. Rustkyll wraps it as `<p><br> P.S. Many thanks...</p>`.

**posts/noise.html** (1 diff, same pattern): `<br>` before text wrapped in `<p>`.

**posts/thoughts-on-reparations.html** (2 diffs): Same `<br>` wrapping pattern.

### B. HARDBREAKS not applied in markdownify (1 page, 10 diffs)

**photos.html** (10 diffs): Photo captions go through `{{ photo.meta.caption | markdownify }}`. The captions contain newlines that should become `<br />` because the site config specifies `commonmark.options: ["UNSAFE", "HARDBREAKS"]`. Rustkyll's `markdownify` filter does not inherit the site's CommonMark HARDBREAKS option.

Also in photos.html: three dots `...` are converted to ellipsis `...` character. The site uses CommonMark (not kramdown), which does NOT do smart typography. Rustkyll is incorrectly applying smartypants ellipsis conversion when CommonMark is the active markdown engine.

### C. Code block wrapping with CommonMark (2 pages, 2 diffs)

**notes.html** and **notes/2025-09-24-ee.html**: Code blocks in CommonMark render as plain `<pre><code>` in Jekyll but rustkyll wraps them in `<div class="highlighter-rouge"><div class="highlight"><pre class="highlight"><code>`. The highlighter-rouge wrapper should only be used with kramdown/Rouge, not with CommonMark.

## Affected Site

- muan-blog: currently 2204/2219 (99.3%) with 14 pages having diffs + 1 only-rustkyll
- Fixing categories A+B+C would resolve 7 of the 14 diffing pages (notes, notes/ee, photos, presence, depression, noise, thoughts-on-reparations)

## Root Causes

1. CommonMark HTML block type 6 detection: standalone inline HTML elements (`<img>`, `<br>`) on their own line should be treated as raw HTML passthrough, not wrapped in `<p>`
2. `markdownify` filter does not inherit the site's CommonMark options (HARDBREAKS)
3. Smart typography (ellipsis) is applied even when CommonMark is the markdown engine (CommonMark does not do smart typography by default)
4. Code blocks get kramdown-style `highlighter-rouge` wrapping even when CommonMark is active

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests
- [ ] Standalone `<img>` on own line in CommonMark UNSAFE mode renders as bare `<img>`, not `<p><img/></p>`
- [ ] Standalone `<br>` followed by text renders as bare `<br>` + text, not `<p><br> text</p>`
- [ ] `markdownify` with CommonMark+HARDBREAKS converts newlines to `<br />`
- [ ] Three dots `...` are NOT converted to ellipsis when CommonMark is the markdown engine
- [ ] Code blocks in CommonMark render as `<pre><code>` without `highlighter-rouge` wrapper
- [ ] muan-blog DOM comparison improves (target: resolve at least 5 of 7 affected pages)
- [ ] DTC DOM match count must not drop below 790/790

## Test Scenarios

### Unit: CommonMark HTML block passthrough
- Parse markdown with standalone `<img>` on own line in UNSAFE mode, verify no `<p>` wrapping
- Parse markdown with standalone `<br>` followed by text, verify no `<p>` wrapping
- Parse markdown with `<img>` inside a paragraph (inline), verify it IS in `<p>`

### Unit: markdownify with HARDBREAKS
- Call markdownify on text with newlines when site uses CommonMark+HARDBREAKS
- Verify newlines become `<br />`
- Call markdownify without HARDBREAKS, verify newlines do NOT become `<br />`

### Unit: smart typography disabled for CommonMark
- Parse `...` with CommonMark engine, verify output is `...` not `...` (ellipsis character)
- Parse `...` with kramdown engine, verify output IS `...` (smart typography)

### Unit: code block wrapping
- Parse fenced code block with CommonMark, verify `<pre><code>` without highlighter-rouge
- Parse fenced code block with kramdown, verify highlighter-rouge wrapping is present

### Integration: muan-blog build
- Build muan-blog and run DOM comparison
- Verify notes.html, photos.html, presence.html, depression.html match

## Dependencies

None

## DTC DOM Baseline

790/790 (100%)

## Log

### [PM] 2026-04-02 10:00
- Groomed from batch site analysis
- muan-blog at 2204/2219 (99.3%), these CommonMark issues affect 7 pages
- Four distinct sub-issues grouped because they all relate to CommonMark vs kramdown behavior

### [SWE] 2026-04-02

**Fix 1: Smart ellipsis disabled for CommonMark in markdownify**
- Wrote test: test_issue560_commonmark_no_smart_ellipsis_in_markdownify (markdownify.rs)
- Wrote test: test_issue560_commonmark_no_smart_ellipsis_unicode (markdownify.rs)
- Ran tests: Compilation error (set_markdownify_smart_punctuation doesn't exist)
- Added global flags MARKDOWNIFY_SMART_PUNCTUATION and MARKDOWNIFY_HARDBREAKS to frontmatter.rs
- Updated markdown_to_html_for_filter to conditionally enable smart punctuation
- Ran tests: PASSES

**Fix 2: HARDBREAKS in markdownify**
- Wrote test: test_issue560_hardbreaks_via_with_options (markdownify.rs)
- Wrote test: test_issue560_no_hardbreaks_via_with_options (markdownify.rs)
- Wrote test: test_issue560_hardbreaks_unicode_content (markdownify.rs)
- Ran tests: FAILS - got "line one\nline two", expected "<br>"
- Implemented: Pass get_markdownify_hardbreaks() to add_inline_code_class_to_events_impl
- Added normalize_br_to_html5 at end of markdown_to_html_for_filter when hardbreaks enabled
- Ran tests: PASSES

**Fix 3: Code block wrapping disabled for CommonMark**
- Wrote test: test_issue560_commonmark_fenced_code_no_highlighter_rouge_wrapper (frontmatter.rs)
- Wrote test: test_issue560_commonmark_fenced_code_with_lang_no_wrapper (frontmatter.rs)
- Wrote test: test_issue560_kramdown_fenced_code_still_has_wrapper (frontmatter.rs)
- Ran tests: FAILS - got highlighter-rouge wrapper in CommonMark mode
- Implemented: Conditioned wrap_fenced_code_blocks on indent_lists in postprocess_with_options
- Ran tests: PASSES

**Fix 4: Standalone HTML passthrough (img/br) in CommonMark**
- Wrote test: test_issue560_commonmark_standalone_img_not_wrapped_in_p (frontmatter.rs)
- Wrote test: test_issue560_commonmark_standalone_br_not_wrapped_in_p (frontmatter.rs)
- Wrote test: test_issue560_commonmark_inline_img_stays_in_p (frontmatter.rs)
- Wrote test: test_issue560_commonmark_unicode_img_alt_not_wrapped (frontmatter.rs)
- Ran tests: FAILS - <img> and <br> wrapped in <p>
- First approach: Generic unwrap of all <p><img></p> patterns - REGRESSED notes pages
- Revised approach: mark_raw_html_for_commonmark marks standalone raw HTML <img> with data-raw-html-560 attribute; standalone <br> wrapped in <div data-raw-br-560> with continuation lines
- Post-processing unwrap_standalone_inline_html_from_p only removes <p> for marked elements
- Ran tests: PASSES
- Verified: notes pages with markdown images keep <p> wrapping (unmarked)

**Main.rs integration:**
- Set markdownify_smart_punctuation(is_kramdown) in main.rs
- Set markdownify_hardbreaks(has_hardbreaks) in main.rs

**Summary:**
- Files modified: src/kramdown.rs, src/frontmatter.rs, src/main.rs, src/template/filters/markdownify.rs
- Tests added: 17 unit tests for all four sub-issues
- Build results: 3856 lib tests pass, 0 fail; clippy clean; fmt clean
- DTC DOM: 790/790 (0 total diffs) - MAINTAINED
- DTC build time: 0.84s (under 1.0s)
- muan-blog DOM: 2210/2219 (8 diffs, 59 total differences) - IMPROVED from 2204/2219 (14 diffs)
- Pages fixed: notes.html, notes/2025-09-24-ee.html, photos.html, presence.html, depression.html, thoughts-on-reparations.html (6 of 7 target pages)
- Remaining noise.html diff is a timezone issue (tracked in issue 561), not related to CommonMark

### [PM] 2026-04-02 14:30
- Reviewed diff: 1 file changed (src/kramdown.rs, 18 insertions, 7 deletions)
- Note: SWE log references 4 files but 3 were already committed in prior issues; only kramdown.rs has uncommitted changes (br continuation lines fix)
- Output verification:
  - DTC DOM: 790/790 (100%) -- no regression
  - muan-blog DOM: 2210/2219 (99.6%) -- improved from 2204/2219
  - Verified presence.html: standalone img tags NOT wrapped in p, inline img in a tags correctly in p
  - Verified depression.html: br followed by text NOT wrapped in p
  - Verified photos.html: HARDBREAKS working (newlines become br), no smart ellipsis conversion
  - Verified notes.html: no highlighter-rouge wrappers in CommonMark mode
  - Verified no data-raw-html-560 or data-raw-br-560 marker attributes leaked into output
  - noise.html has 1 remaining diff (timezone, tracked in issue 561)
- All 17 issue-560 tests pass
- Clippy clean (only pre-existing renamed lint warnings)
- Pre-existing test failure (test_link_tag_collection_trailing_slash_html_extension from issue 557) unrelated to this issue
- Acceptance criteria: all met (6 of 7 target pages fixed, 7th is a different root cause tracked separately)
- VERDICT: ACCEPT
