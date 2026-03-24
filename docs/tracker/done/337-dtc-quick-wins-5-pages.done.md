# Issue 337: DTC quick win fixes (3 pages)

## Problem

Three DTC pages have small, well-understood diffs that are easy to fix individually.

## Sub-issues

### A: FAQ accordion whitespace and related-posts ordering (1 page -- free-machine-learning-courses.html, 4 DOM diffs)

The 4 DOM diffs on this page are caused by whitespace/structural differences in the FAQ accordion include output. The `faq-accordion.html` include template produces slightly different whitespace in rustkyll vs Jekyll (extra blank lines between closing `</div>` tags). This causes the DOM comparator to report `tag_name_differs` (p vs div) and `missing_element` diffs.

Additionally (not captured in DOM diffs), the related posts appear in a different order than Jekyll. Jekyll sorts `site.related_posts` by date descending, and for same-date posts the tiebreak is by **path ascending** (alphabetical). Rustkyll currently tiebreaks by **slug descending**. The fix is to change the tiebreak in `build_related_posts()` in `src/generator.rs` from `b.slug.cmp(&a.slug)` to match Jekyll's behavior.

**Root cause (FAQ whitespace):** The Liquid template engine renders include content with different indentation. This is a whitespace-only issue -- the HTML structure is semantically identical. The DOM comparator misaligns elements due to extra whitespace nodes.

**Root cause (related posts order):** In `src/generator.rs`, function `build_related_posts()`, the sort tiebreaker uses `b.slug.cmp(&a.slug)` (slug descending). Jekyll uses source path ascending for tiebreaking.

### B: Autolink tel: false positive (1 page -- transfer-learning-in-action.html, 5 DOM diffs)

The pattern `<tel:100-1000|100-1000>` in a list item is being autolinked as a telephone URL, producing `<a href="tel:100-1000%7C100-1000">tel:100-1000|100-1000</a>`. Jekyll does not autolink `tel:` scheme URIs -- it treats the angle-bracketed content as escaped text (`&lt;tel:100-1000|100-1000&gt;`).

**Root cause:** In `src/kramdown_parser/span_parser.rs`, function `try_parse_autolink()`, only `http://`, `https://`, and `ftp://` schemes are recognized as URL autolinks. However, upstream of that, the function `process_raw_html_content()` (or a similar path) detects `://` in the content and may route `tel:` URIs into the autolink path. The fix should ensure that only `http://`, `https://`, `ftp://`, and `mailto:` schemes produce autolinks, and all other schemes (including `tel:`) are escaped as `&lt;...&gt;`.

Looking at the code: `try_parse_autolink()` at line ~1932 only matches `http://`, `https://`, `ftp://`. But `process_raw_html_content()` at line ~2253 checks `rest.contains("://")` which matches `tel:` URIs since the full content is `<tel:100-1000|100-1000>` -- but wait, `tel:` does not contain `://`. Let me re-examine: the actual text in the markdown is `<tel:100-1000|100-1000>`. The `try_parse_autolink()` function extracts the content between `<` and `>`, checks if it starts with http/https/ftp (no), then checks if it's a mailto: (no), then checks for email pattern (no). So it should return `None` and the `<` should be escaped. The issue may be in how the YAML front matter content is processed before reaching the span parser -- check whether the text arrives as raw `<tel:...>` or whether `newline_to_br` or other preprocessing changes it.

### D: Zero-width space in paragraph text (1 page -- guidelines-to-get-data-engineer-job-against-odds.html, 5 DOM diffs)

Rustkyll outputs a zero-width space character (U+200B) in the paragraph text: `straightforward\u200b <em>.</em>`. Jekyll strips or does not include this character. The source markdown has `straightforward​ _._` where the zero-width space exists between "straightforward" and the space before the emphasis.

The 5 DOM diffs are:
1. `text_differs` -- the zero-width space in the paragraph text
2. `expected_element_got_text` -- a list item has `<p>` wrapping in Jekyll but not in rustkyll
3. `tag_name_differs` -- child element ordering differs due to missing `<p>` wrapper
4. `extra_text` -- extra text fragment from unwrapped list content
5. A structural diff from the `<p>` wrapping difference

**Root cause (zero-width space):** The source markdown contains a literal U+200B character. Jekyll/kramdown strips zero-width spaces during markdown processing. Rustkyll preserves them. The fix should strip U+200B characters from text content during kramdown processing (either in preprocessing or during text node output).

**Root cause (list paragraph wrapping):** Jekyll wraps certain list item content in `<p>` tags (loose list items). Rustkyll does not wrap this particular list item. The trigger is likely that this is a "loose" list (list items separated by blank lines or containing block-level content), where Jekyll wraps each `<li>` content in `<p>`. This may be related to the specific markdown structure of this post.

## Descoped (tracked separately)

### Sub-issue C: Canvas data-attribute stripping -- Covered by issue 339

The `how-do-professionals-use-llm-tools-and-frameworks.html` page (9 DOM diffs) is fully covered by issue 339 (DTC blog canvas data attributes and LLM tools page). Removed from this issue to avoid duplicate work.

### Sub-issue E: Code block in list rendered as heading (mastering-spacy.html) -- New issue 341

The mastering-spacy page has **24 DOM diffs** (not 2 as originally estimated). The core problem is that backtick-delimited code blocks inside YAML comment text (processed via `newline_to_br | markdownify`) are parsed as fenced code block boundaries, causing:
- All code content to be swallowed into a `<pre><code>` block with mangled language attributes
- The `# Then do your stuff with the pos tags` line to be rendered as `<h1>` instead of plain text
- 22+ DOM diffs from content being restructured

This is NOT a quick win -- it requires changes to how the `markdownify` filter handles backtick sequences inside inline content. Issue 336 already covers 2 of the 24 diffs (the br-sublist nesting pattern). The remaining 22 diffs need their own issue. See issue 341.

## Dependencies

- None (these are independent fixes)
- Issue 339 must be done separately for sub-issue C
- Issue 336 covers the br-sublist portion of mastering-spacy
- Issue 341 covers the code-block-in-list portion of mastering-spacy

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with no regressions
- [ ] **Sub-issue A (FAQ whitespace):** The FAQ accordion include output matches Jekyll's whitespace structure -- no extra blank lines between closing `</div>` tags inside `faq-item` elements
- [ ] **Sub-issue A (related posts order):** `site.related_posts` tiebreaking for same-date posts matches Jekyll's behavior (path ascending, not slug descending)
- [ ] **Sub-issue A:** Build the DTC site and verify `blog/free-machine-learning-courses.html` has 0 DOM diffs (down from 4)
- [ ] **Sub-issue A:** The related posts on `blog/free-machine-learning-courses.html` appear in the same order as Jekyll output: (1) data-engineering-zoomcamp, (2) building-discipline-in-machine-learning-with-ml-zoomcamp, (3) how-to-build-blood-cell-classifier
- [ ] **Sub-issue B:** The pattern `<tel:100-1000|100-1000>` in markdown is NOT autolinked -- it is escaped to `&lt;tel:100-1000|100-1000&gt;` matching Jekyll
- [ ] **Sub-issue B:** Existing autolinks for `http://`, `https://`, `ftp://`, and `mailto:` schemes continue to work correctly
- [ ] **Sub-issue B:** Build the DTC site and verify `books/20211004-transfer-learning-in-action.html` has 0 DOM diffs (down from 5)
- [ ] **Sub-issue D:** The zero-width space (U+200B) is stripped from paragraph text, matching Jekyll output
- [ ] **Sub-issue D:** The list item in `guidelines-to-get-data-engineer-job-against-odds.html` is wrapped in `<p>` tags matching Jekyll's loose list rendering
- [ ] **Sub-issue D:** Build the DTC site and verify `blog/guidelines-to-get-data-engineer-job-against-odds.html` has fewer DOM diffs than before (target: 0, acceptable: reduction from 5)
- [ ] All fixes are generic Jekyll-compatible behavior, not hardcoded to specific DTC pages
- [ ] Tests include non-ASCII/Unicode content (per project convention)

## Test Scenarios

### Unit: Related posts tiebreaking (Sub-issue A)

- Create 3 posts with the same date but different slugs/paths. Verify `build_related_posts()` returns them in the order matching Jekyll (by source path ascending for same-date tiebreaking)
- Create posts with different dates. Verify date-descending ordering is unchanged (no regression)
- Verify that changing the tiebreak does not affect posts with distinct dates

### Unit: Autolink tel: rejection (Sub-issue B)

- Parse `<tel:100-1000|100-1000>` through span parsing. Verify output is `&lt;tel:100-1000|100-1000&gt;` (escaped), NOT an `<a>` tag
- Parse `<http://example.com>` through span parsing. Verify output is `<a href="http://example.com">http://example.com</a>` (still works)
- Parse `<https://example.com>` -- verify autolink still works
- Parse `<ftp://files.example.com>` -- verify autolink still works
- Parse `<mailto:user@example.com>` -- verify autolink still works
- Parse `<ssh://server.com>` -- verify this is escaped (not autolinked), matching Jekyll
- Parse `<tel:+1-555-0100>` -- verify this is escaped (not autolinked)

### Unit: Zero-width space stripping (Sub-issue D)

- Parse markdown containing `straightforward\u200b _._` and verify the output does NOT contain U+200B
- Parse markdown with U+200B in various positions (start, middle, end of text, inside emphasis) and verify all are stripped
- Parse markdown with other Unicode characters (accented, CJK, emoji) and verify they are NOT stripped -- only U+200B is removed
- Parse markdown with U+200B between words and verify word boundaries are preserved correctly after stripping

### Unit: List paragraph wrapping (Sub-issue D)

- Parse a list where items are separated by content that triggers loose-list mode. Verify `<li>` content is wrapped in `<p>` matching Jekyll
- Parse a tight list (no blank lines between items). Verify `<li>` content is NOT wrapped in `<p>` (no regression)

### Integration: DTC site output verification

- Build the DTC site with `./scripts/cargo-safe run -- --source datatalksclub.github.io --destination _site`
- Run DOM comparison on `blog/free-machine-learning-courses.html` -- target 0 diffs
- Run DOM comparison on `books/20211004-transfer-learning-in-action.html` -- target 0 diffs
- Run DOM comparison on `blog/guidelines-to-get-data-engineer-job-against-odds.html` -- target 0 diffs (or fewer than 5)
- Verify no regressions on other DTC pages (total matched pages does not decrease)

## Implementation Notes

### Sub-issue A: Related posts tiebreak
File: `src/generator.rs`, function `build_related_posts()` (~line 738)
Change: `b.slug.cmp(&a.slug)` to use source path or slug in the direction matching Jekyll. Test by comparing the related-posts order on `free-machine-learning-courses.html` before and after.

For FAQ whitespace: investigate how the Liquid template engine renders include files. The extra blank lines may come from how `{% for %}` loop whitespace is handled. Check `src/template/engine.rs` or include rendering logic.

### Sub-issue B: Autolink scheme filtering
File: `src/kramdown_parser/span_parser.rs`, function `try_parse_autolink()` (~line 1910)
The function already only matches http/https/ftp. The `tel:` URI might be reaching a different code path. Check `process_raw_html_content()` and the `is_autolink` detection at ~line 2253 which checks for `://` or `mailto:` -- but `tel:` has neither `://` nor `mailto:`, so this path should not match. Debug by adding a test case that processes the exact markdown content from the DTC page and trace where the `<a href="tel:...">` is generated.

### Sub-issue D: Zero-width space
This could be handled as a preprocessing step (strip U+200B from input) or during text node output in kramdown processing. Preprocessing is simpler and matches Jekyll's behavior.

For list paragraph wrapping: this may require understanding when kramdown treats a list as "loose" vs "tight". Check the list parsing logic in `src/kramdown_parser/` or `src/kramdown.rs`.

## Log

### [SWE] 2026-03-24

**Sub-issue A: Related posts tiebreak**
- Wrote test `test_337a_related_posts_tiebreak_by_path_ascending` (src/generator.rs)
- Ran test: FAILS as expected -- got zzz-post first (slug descending), expected aaa-post first (path ascending)
- Implemented fix: changed `b.slug.cmp(&a.slug)` to `a.source_path.cmp(&b.source_path)` in `build_related_posts()`
- Ran test: PASSES
- Updated existing test `test_related_posts_tiebreaking_same_date_by_slug_descending` -> renamed to `test_related_posts_tiebreaking_same_date_by_path_ascending`
- Note: DTC free-machine-learning-courses page uses a custom template (related-posts.html) that iterates site.posts and sorts via Liquid `sort: "date" | reverse`. The Liquid sort filter uses slug/path tiebreak which differs from Jekyll's stable sort. This is a separate issue from `build_related_posts`. The 6 DOM diffs on this page are NOT from `site.related_posts` but from the Liquid sort filter behavior.

**Sub-issue B: Autolink tel: false positive** (already committed in HEAD via issue 336 revert)
- Wrote 7 tests: tel escaped, http/https/ftp/mailto preserved, ssh escaped, tel+phone escaped
- Ran tests: 3 FAIL (tel, ssh, tel+phone), 4 PASS (http, https, mailto, ftp) -- as expected
- Implemented `escape_non_kramdown_autolinks()` in src/frontmatter.rs: escapes `<` in angle-bracketed URIs with non-kramdown schemes before pulldown-cmark processes them
- Added calls in all 3 markdown_to_html functions
- Ran tests: all 7 PASS
- DTC transfer-learning-in-action page: 5 diffs -> 0 diffs (FIXED)

**Sub-issue D: Zero-width space** (already committed in HEAD via issue 336 revert)
- Wrote 4 ZWSP tests
- Ran tests: 3 FAIL (ZWSP preserved in output), 1 PASS (other unicode preserved)
- Implemented: strip U+200B from HTML output after pulldown-cmark processing in all 3 markdown_to_html functions
- Ran tests: all 4 PASS
- DTC guidelines page: ZWSP text_differs diff eliminated (4 -> 3 diffs)
- Updated kramdown test `test_issue198_zwsp_preserved_without_emphasis` -> `test_issue198_zwsp_stripped_from_output`

**Sub-issue D: List paragraph wrapping** (DESCOPED)
- Attempted to implement kramdown partial-loose list behavior (items followed by blank lines get `<p>` wrapping)
- Used marker-based approach: insert `<!-- kramdown-loose-item -->` in collapse function, then post-process to add `<p>`
- This caused 22+ regressions across blog/book/podcast pages
- Root cause: kramdown's partial-loose behavior differs fundamentally from CommonMark's all-or-nothing loose/tight model
- REVERTED all changes. The 3 remaining DOM diffs on the guidelines page are from this unsupported partial-loose behavior.
- This should be tracked as a separate issue.

**Test results:** 2836 lib tests + integration/other tests all PASS, 0 FAIL, clippy clean, fmt clean
**DTC DOM comparison (vs clean HEAD baseline):** No regressions. Same as clean: 743 matched, 47 diffs, 808 total.

**Files modified:**
- `src/generator.rs` -- related posts tiebreak (path ascending instead of slug descending)
- `src/frontmatter.rs` -- removed partial-loose test (feature not implementable without regressions)
- `src/kramdown.rs` -- updated ZWSP test expectation
- `docs/tracker/337-dtc-quick-wins-5-pages.in-progress.md` -- this log

### [QA] 2026-03-24

**Checks:**
- `cargo test`: PASS (all tests pass, 0 failures)
- `cargo clippy -- -D warnings`: PASS (clean, only renamed lint warnings from liquid-lib dependency)
- `cargo fmt --check`: PASS (clean)
- DOM regression: 743 matched, 47 diffs, 808 total (same as baseline -- no regression)
- Build performance: ~1.1s (same as baseline; slightly over 1.0s target but not a regression from this issue)

**Acceptance criteria review:**

- [PASS] `cargo build` compiles without errors
- [PASS] `cargo test` passes with no regressions
- [N/A] Sub-issue A (FAQ whitespace): SWE determined the 6 diffs on free-machine-learning are from Liquid sort filter tiebreak, not FAQ accordion whitespace. Original issue root cause analysis was incorrect. Needs separate issue for Liquid sort stability.
- [PASS] Sub-issue A (related posts order): tiebreak changed from slug descending to path ascending, with 2 tests verifying the behavior
- [FAIL] Sub-issue A: free-machine-learning 0 DOM diffs -- still 6 diffs (caused by Liquid sort, not site.related_posts)
- [FAIL] Sub-issue A: related posts order on free-machine-learning -- page uses Liquid sort filter, not site.related_posts; the fix is correct but does not affect this page
- [PASS] Sub-issue B: tel: not autolinked (committed in 336 revert, verified in tests)
- [PASS] Sub-issue B: existing autolinks work (http/https/ftp/mailto tests pass)
- [PASS] Sub-issue B: transfer-learning 0 DOM diffs (confirmed)
- [PASS] Sub-issue D: ZWSP stripped (committed in 336 revert, test updated)
- [FAIL] Sub-issue D: list paragraph wrapping -- descoped, not implemented (caused 22+ regressions)
- [PASS] Sub-issue D: guidelines page reduced from 5 to 3 diffs (ZWSP fix; 3 remaining from partial-loose list)
- [PASS] All fixes are generic Jekyll behavior, not site-specific
- [PASS] Tests include non-ASCII/Unicode content

**Key findings:**
1. Sub-issues B and D (tel: fix, ZWSP stripping) were already committed in the issue 336 revert (commit 6ea0493). The uncommitted changes for this issue are: (a) related posts tiebreak fix, (b) test cleanup for descoped partial-loose feature, (c) ZWSP test expectation update.
2. Sub-issue A's free-machine-learning page diffs are NOT from site.related_posts tiebreaking but from Liquid sort filter stability. The SWE correctly identified this but the acceptance criteria targets (0 diffs, specific post order) cannot be met by this fix.
3. Sub-issue D's partial-loose list behavior was reasonably descoped -- it caused 22+ regressions.
4. No DOM regressions from these changes.

**VERDICT: PASS** (with notes)

The uncommitted code changes are correct and well-tested. The related posts tiebreak fix is a real improvement matching Jekyll behavior. The descopes are well-documented and justified:
- FAQ whitespace / Liquid sort stability: needs separate issue (original root cause analysis was wrong)
- Partial-loose list behavior: needs separate issue (fundamentally incompatible with CommonMark model)

The acceptance criteria for free-machine-learning 0 diffs and specific related post ordering cannot be met by this issue because the root cause is different from what was originally analyzed. This should be tracked in a follow-up issue for Liquid sort filter stability.

### [PM] 2026-03-24

**Acceptance criteria review:**

| # | Criterion | Verdict |
|---|-----------|---------|
| 1 | `cargo build` compiles without errors | PASS |
| 2 | `cargo test` passes with no regressions | PASS |
| 3 | Sub-issue A (FAQ whitespace): FAQ accordion matches Jekyll | N/A -- root cause misdiagnosed; diffs are from Liquid sort, not FAQ whitespace. Tracked in issue 342. |
| 4 | Sub-issue A (related posts order): tiebreak matches Jekyll | PASS -- changed from slug descending to path ascending, verified by 2 tests |
| 5 | Sub-issue A: free-machine-learning 0 DOM diffs | FAIL -- still 6 diffs, caused by Liquid sort filter stability (not site.related_posts). Tracked in issue 342. |
| 6 | Sub-issue A: specific related post order on page | FAIL -- page uses Liquid sort filter, not site.related_posts. Tracked in issue 342. |
| 7 | Sub-issue B: tel: not autolinked | PASS |
| 8 | Sub-issue B: existing autolinks work | PASS |
| 9 | Sub-issue B: transfer-learning 0 DOM diffs | PASS |
| 10 | Sub-issue D: ZWSP stripped | PASS |
| 11 | Sub-issue D: list paragraph wrapping | FAIL -- descoped due to 22+ regressions. Tracked in issue 343. |
| 12 | Sub-issue D: guidelines page fewer diffs | PASS -- reduced from 5 to 3 (criteria: "target 0, acceptable: reduction from 5") |
| 13 | All fixes generic Jekyll behavior | PASS |
| 14 | Tests include non-ASCII/Unicode | PASS |

**Code review:**

- The related posts tiebreak change in `src/generator.rs` is clean: single-line change from `b.slug.cmp(&a.slug)` to `a.source_path.cmp(&b.source_path)`, well-commented.
- New test `test_337a_related_posts_tiebreak_by_path_ascending` is thorough: 3 posts with mixed dates, verifies both date ordering and path tiebreak.
- Existing test renamed and expectations updated correctly.
- Removed partial-loose test that tested unimplemented feature -- correct cleanup.
- ZWSP test expectation flipped from "preserved" to "stripped" -- matches the implementation.
- No over-engineering. Changes are minimal and focused.

**Descoped items -- follow-up issues created (no silent descoping):**

1. **Issue 342** (`docs/tracker/342-liquid-sort-filter-stable-tiebreak.todo.md`): Liquid sort filter does not match Jekyll's stable sort. Covers the 6 remaining diffs on free-machine-learning-courses page and the unmet criteria for sub-issue A items 3, 5, 6.
2. **Issue 343** (`docs/tracker/343-kramdown-partial-loose-list-p-wrapping.todo.md`): Kramdown partial-loose list paragraph wrapping. Covers the 3 remaining diffs on guidelines page and the unmet criterion for sub-issue D item 11.

**VERDICT: ACCEPT**

All implemented changes are correct, well-tested, and improve Jekyll compatibility. Three acceptance criteria could not be met because the original root cause analysis was wrong (Liquid sort stability) or the fix caused regressions (partial-loose lists). Both gaps are tracked in new issues 342 and 343. No DOM regressions. No silent descoping.
