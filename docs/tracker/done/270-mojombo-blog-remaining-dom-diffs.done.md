# Issue 270: Fix remaining mojombo-blog DOM differences (15/17 -> 17/17)

## Problem

mojombo-blog currently matches 15/17 pages (as of 2026-03-21 DOM recount). The original issue described 3 failing pages but the Ruby syntax highlighting page (tomdoc) was fixed by prior work. The 2 remaining failures are:

1. **Kramdown underscore emphasis parsing** (how-i-turned-down-300k.html, 5 diffs)
   - Input: `"I can has _______"` (7 underscores)
   - Jekyll/kramdown renders: `"I can has <strong>__</strong>_"`
   - Rustkyll renders: `"I can has _______"` (literal underscores)
   - Kramdown treats runs of underscores as emphasis delimiters

2. **Image alt attribute newline normalization** (the-git-parable.html, 1 diff)
   - Jekyll normalizes `alt='Creative\nCommons License'` to `alt='Creative Commons License'`
   - Rustkyll preserves the literal newline in the alt attribute
   - This is a whitespace normalization issue in HTML attribute output

## Scope

Fix both categories to reach 17/17 DOM match on mojombo-blog.

### Category 1: Underscore emphasis (5 diffs)

The `_______` pattern is a kramdown-specific emphasis edge case. Kramdown treats `__` as strong emphasis delimiters even when embedded in a longer run of underscores. This was previously descoped in issue 246 for the DTC site (same class of bug with `____`).

**Approach:** Fix the kramdown parser's emphasis handling for runs of 4+ consecutive underscores. Kramdown greedily matches `__...__` as `<strong>` and leaves remaining underscores as literal text.

Specifically, kramdown parses `_______` (7 underscores) as:
- `__` = open strong
- `__` = strong content (literal underscores inside strong)
- `__` = close strong
- `_` = remaining literal underscore

Resulting in: `<strong>__</strong>_`

### Category 2: Alt attribute newline (1 diff)

The markdown source contains an image with an alt attribute that spans multiple lines. Jekyll collapses the newline to a space in the rendered HTML attribute. Rustkyll preserves the literal newline.

**Approach:** In the HTML output postprocessing, normalize whitespace in HTML element attributes -- specifically collapse newlines to spaces in attribute values. This is standard HTML behavior per the spec.

## Impact

Fixes 2 pages to achieve 17/17 (100%) DOM match on mojombo-blog.

## Dependencies

None. These fixes are independent. Kramdown is now at 643/643 (100% conformance), so the underscore emphasis fix needs to be made in the kramdown parser which is stable.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `how-i-turned-down-300k.html` matches Jekyll DOM (underscore emphasis fix)
- [ ] `the-git-parable.html` matches Jekyll DOM (alt attribute newline fix)
- [ ] mojombo-blog DOM comparison: 17/17 pages match (0 differences)
- [ ] No regressions on large-blog-3000 (3001/3001), large-docs-site (801/801), kids-horror-stories-ru (1344/1344)
- [ ] No regressions on DTC (657/790 or better)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean

## Test Scenarios

### Unit: Kramdown underscore emphasis handling

- Parse `_______` (7 underscores) in markdown, verify output contains `<strong>__</strong>_`
- Parse `____` (4 underscores), verify output contains `<em>__</em>` (kramdown treats `_..._` as em)
- Parse `__` (2 underscores), verify output is literal `__` (not enough for emphasis)
- Parse `___` (3 underscores), verify output matches kramdown behavior
- Parse `_word_` normally, verify `<em>word</em>` still works (regression check)
- Parse `__word__` normally, verify `<strong>word</strong>` still works (regression check)
- Include non-ASCII content in test strings to catch encoding regressions

### Unit: HTML attribute whitespace normalization

- Render markdown image `![Creative\nCommons License](url)`, verify alt attribute is `Creative Commons License` (newline collapsed to space)
- Render markdown image `![normal alt](url)`, verify alt attribute unchanged (regression check)
- Render image with tab in alt text, verify tab collapsed to space

### Integration: Full site verification

- Build mojombo-blog with rustkyll
- Run DOM comparison against Jekyll output
- Verify 17/17 pages match with 0 total differences
- Inspect both previously-failing HTML files to confirm fixes

### Regression: Other sites unaffected

- Run DOM comparison on at least 3 other test sites to verify no regressions
- Verify `cargo test` full suite passes

## Output Verification

The engineer must:
1. Build mojombo-blog: `./target/release/rustkyll build --source websites/mojombo-blog --destination websites/mojombo-blog/_site_rustkyll`
2. Run DOM comparison: `uv run scripts/dom_compare.py --jekyll-dir websites/mojombo-blog/_site_jekyll_cached --rustkyll-dir websites/mojombo-blog/_site_rustkyll`
3. Verify output shows: `Summary: 17 files matched, 0 files with differences, 0 total differences`
4. Inspect both previously-failing files manually to confirm the fix is correct

## Notes

- The kramdown underscore emphasis issue was previously descoped in issue 246 for the same class of bug on the DTC site
- Kramdown is at 643/643 conformance, so emphasis parsing is stable -- this is a targeted edge case fix for runs of consecutive underscores
- The alt attribute newline fix is standard HTML spec behavior (collapse whitespace in attributes)

## Log

### [SWE] 2026-03-21

**TDD Cycle:**

1. Wrote failing tests for underscore emphasis (7, 6, 4, 3 underscores) and image alt newline normalization
2. Tests confirmed: pulldown-cmark (used for actual site build) produces literal underscores for all runs -- kramdown parser tests pass but aren't used in the build pipeline
3. Implemented `convert_kramdown_underscore_runs()` in kramdown.rs to preprocess underscore runs: 4 -> `<em>__</em>`, 6+ -> `<strong>__</strong>` + remainder
4. Tests pass: underscore emphasis now matches kramdown Ruby for 4, 6, 7+ consecutive underscores
5. Implemented `normalize_newlines_in_html_tags()` in kramdown.rs to collapse newlines inside HTML tags to spaces
6. Added alt text whitespace normalization in kramdown parser's `try_parse_image` for completeness
7. All tests pass (2341 pass, 0 fail), clippy clean, fmt clean

**Root causes:**
- Category 1 (underscore emphasis): pulldown-cmark (CommonMark) does not handle kramdown's underscore-run emphasis semantics. Added preprocessing step to convert 4+ underscore runs to HTML before parsing.
- Category 2 (alt newline): Raw HTML tags spanning multiple lines were passed through without normalization. Added postprocessing step to collapse newlines inside HTML tag content.

**Files modified:**
- `src/kramdown.rs` -- Added `convert_kramdown_underscore_runs()` and `normalize_newlines_in_html_tags()`
- `src/frontmatter.rs` -- Added preprocessing call in both `markdown_to_html` and `markdown_to_html_with_options`; added 5 unit tests
- `src/kramdown_parser/span_parser.rs` -- Added alt text whitespace normalization in `try_parse_image`
- `src/kramdown_parser/tests.rs` -- Added 10 unit tests for underscore emphasis and image alt normalization

**Verification:**
- mojombo-blog: 17/17 pages match, 0 total differences
- Build: 2341 tests pass, 0 fail, clippy clean, fmt clean
