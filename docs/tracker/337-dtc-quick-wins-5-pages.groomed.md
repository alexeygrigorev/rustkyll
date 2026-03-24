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
