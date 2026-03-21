# Issue 297: muan-blog remaining ~110 DOM diff pages

## Problem

muan-blog matches ~2108/2218 (95%). ~110 pages have diffs. The layout uses
`{{ page.content | strip_html | truncate: 240 }}` for meta description and
og:description tags. The diffs fall into distinct categories:

### Category A: List item indentation in strip_html output (~65 pages)

The muan-blog layout generates meta content by running
`page.content | strip_html | truncate: 240`. When the page content has list
items (`<ul><li>...</li></ul>`), the strip_html output differs:

- **Jekyll kramdown** renders: `<ul>\n<li>Item</li>\n</ul>` (no indentation)
- **Rustkyll kramdown** renders: `<ul>\n  <li>Item</li>\n</ul>` (2-space indent)

After `strip_html`, Jekyll gives `\nItem\n` while rustkyll gives `\n  Item\n`.
The extra spaces propagate into the meta content attribute.

This is NOT a `strip_html` filter bug. It is a **kramdown HTML output
whitespace** issue: rustkyll adds indentation to `<li>`, `<ol>`, `<dd>`, etc.
that Jekyll's kramdown does not.

**Fix**: Ensure kramdown HTML output for list items matches Jekyll's: no
indentation between `<ul>` and `<li>`, no indentation before `<li>` content.

### Category B: Truncation character counting for Unicode (~25 pages)

Ruby's `truncate` filter counts characters (Unicode codepoints), not bytes.
For CJK content, each character is 3+ bytes in UTF-8. If rustkyll counts bytes
instead of codepoints, it hits the 240 limit much sooner, adding "..." where
Jekyll does not.

Example: A string of ~80 CJK characters is ~240 bytes but only ~80 chars.
- Jekyll: Content fits in 240 chars, no truncation, ends with `\n`
- Rustkyll: Content is ~240 bytes, triggers truncation, ends with `\n...`

Additionally, the truncation cutoff position differs slightly for some ASCII
content (cutting at `don'` vs `don't`), suggesting the character boundary
handling or the counting logic has edge cases.

**Fix**: Ensure `truncate` filter counts Unicode codepoints (via
`.chars().count()`), not byte length (`.len()`).

### Category C: Bare URL auto-linking (~15 pages)

Bare URLs in markdown (like `https://example.com`) should be converted to
`<a href="...">...</a>` links by kramdown. Rustkyll outputs them as plain text.

Example:
- Jekyll: `<a href="https://example.com">https://example.com</a>`
- Rustkyll: `https://example.com` (plain text)

This affects pages where the markdown content has bare URLs without any link
syntax.

### Category D: Meta attribute quoting/escaping (~5 pages)

One page (`notes/2023-01-25-mm.html`) has content with double quotes that
break the HTML meta attribute, causing phantom attributes to appear in the
DOM parse.

### Category E: Miscellaneous body diffs (~10 pages)

Various body-level diffs including:
- `pages/blogroll.html` (99 diffs) -- large structured page
- `pages/hacking-with-swift/index.html` (30 diffs) -- code content
- Several posts with link/emphasis parsing differences

## Scope

This issue focuses on **Categories A and B** (highest impact, ~90 pages).
Categories C-E should be tracked as follow-up issues.

## Dependencies

- None. The build currently has compile errors from in-progress work that must
  be resolved first (type mismatch in `kramdown.rs` and `layout.rs`).

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests below
- [ ] Kramdown HTML output for unordered lists: `<li>` elements are NOT
      indented relative to `<ul>` -- output matches Jekyll format:
      `<ul>\n<li>content</li>\n</ul>` (not `<ul>\n  <li>content</li>\n</ul>`)
- [ ] Same for ordered lists: `<ol>\n<li>content</li>\n</ol>` (no indentation)
- [ ] `truncate` filter counts Unicode codepoints, not bytes -- a string of 240
      CJK characters is NOT truncated (each is 1 char despite being 3 bytes)
- [ ] `truncate: 240` on a 239-char ASCII string does not append "..."
- [ ] `truncate: 240` on a 241-char ASCII string truncates to 237 chars + "..."
- [ ] muan-blog DOM match improves to 2180+/2218 (from current ~2108/2218)
- [ ] No regressions on any of the 13+ sites currently at 100%
- [ ] No regressions on DTC, mlwiki match counts

## Test Scenarios

### Unit: Kramdown list output whitespace

- Parse `- item 1\n- item 2` through kramdown
- Verify output is `<ul>\n<li>item 1</li>\n<li>item 2</li>\n</ul>\n`
  (no indentation before `<li>`)
- Parse `1. first\n2. second` through kramdown
- Verify output is `<ol>\n<li>first</li>\n<li>second</li>\n</ol>\n`

### Unit: strip_html + truncate pipeline with list content

- Input HTML: `<ul>\n<li>Jeff Bridges</li>\n<li>Julianne Moore</li>\n</ul>`
- After strip_html: `\nJeff Bridges\nJulianne Moore\n` (no leading spaces)
- After truncate: 240: same (under limit)

### Unit: truncate filter Unicode character counting

- Input: 80 CJK characters (e.g., repeat of U+4E16 "shi" 80 times)
- `truncate: 240` should NOT truncate (80 chars < 240)
- Input: 241 ASCII characters
- `truncate: 240` should produce 237 chars + "..."
- Input: 240 ASCII characters
- `truncate: 240` should produce exactly the 240 characters (no "...")
- Input: mixed ASCII + CJK totaling 241 codepoints
- `truncate: 240` should truncate at codepoint 237 + "..."

### Integration: muan-blog page rendering

- Build muan-blog with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify match count is >= 2180/2218
- Spot-check `notes/2022-04-05-zz.html` meta content attribute: must contain
  `Jeff Bridges` without leading spaces (not `  Jeff Bridges`)
- Spot-check a CJK-heavy note page: meta content must NOT have spurious "..."

## Output Verification

- Build muan-blog: `rustkyll build --source websites/muan-blog/ --destination /tmp/muan_test`
- Compare: `python3 scripts/dom_compare.py --jekyll-dir websites/muan-blog/_site_jekyll_cached --rustkyll-dir /tmp/muan_test`
- Summary line must show >= 2180 files matched

## Follow-up Issues to Create

After this issue is done, create separate issues for:
- Bare URL auto-linking in kramdown (Category C) -- ~15 pages
- Meta attribute escaping for content with double quotes (Category D)
- Miscellaneous body diffs in blogroll/hacking-with-swift pages (Category E)

## Log

### [SWE] 2026-03-21

**TDD Cycle - Fix 1: List item indentation (Category A)**

- Wrote failing tests: `test_issue297_commonmark_list_no_indent`, `test_issue297_commonmark_ordered_list_no_indent` (src/frontmatter.rs)
- Ran tests: FAILS as expected -- got `<ul>\n  <li>` instead of `<ul>\n<li>`
- Root cause: `indent_list_items()` postprocessor in `kramdown.rs` unconditionally adds 2-space indent to `<li>` tags. This is correct for kramdown mode but wrong for CommonMarkGhPages (Jekyll's CommonMark renderer does NOT indent).
- Fix: Added `postprocess_with_options(html, is_kramdown)` function in `src/kramdown.rs`. When `indent_lists=false`, skips `indent_list_items()`. Updated `markdown_to_html_with_options` in `src/frontmatter.rs` to pass `add_code_classes` (kramdown mode flag) to control list indentation.
- Wrote `test_issue297_kramdown_list_still_indented` to verify no regression for kramdown sites.
- Wrote `test_issue297_strip_html_truncate_pipeline` for end-to-end pipeline verification.
- Ran tests: ALL 4 PASS

**TDD Cycle - Fix 2: Unicode truncation (Category B)**

- Wrote failing test: `unit_truncate_cjk_counts_chars_not_bytes` (vendor/liquid-lib truncate.rs)
- Ran test: FAILS as expected -- 81 CJK chars (243 bytes) being truncated at `truncate: 240`
- Root cause: `TruncateFilter::evaluate()` uses `input_string.len()` (byte count) and `truncate_string.len()` (byte count) for comparisons, but Ruby's `truncate` counts codepoints.
- Fix: Changed length comparison to use `.chars().count()` (codepoints) instead of `.len()` (bytes). Changed truncation to use `.chars().take(l)` instead of grapheme clusters. Changed ellipsis length to use `.chars().count()`. Used `saturating_sub` for clippy compliance.
- Updated pre-existing `unit_truncate_unicode_codepoints_examples` test to match codepoint-based behavior (was testing grapheme cluster behavior).
- Added 4 new tests: CJK 81-char, 241 ASCII, 240 ASCII (no truncation), mixed ASCII+CJK
- Ran tests: ALL 16 PASS

**Full test suite:**
- 2434 lib tests pass, 0 fail, 3 ignored
- All integration tests pass (41+4+12+17+4+20+9+12+22+15+2+30+8+6+7+20+13+5+23+19 = ~334)
- Clippy clean (only pre-existing renamed lint warnings)
- cargo fmt clean

**Files modified:**
- `src/kramdown.rs` -- Added `postprocess_with_options()` with `indent_lists` parameter
- `src/frontmatter.rs` -- Updated `markdown_to_html_with_options` to use `postprocess_with_options`, added 4 tests
- `vendor/liquid-lib/src/stdlib/filters/string/truncate.rs` -- Changed byte counting to codepoint counting, updated/added tests
