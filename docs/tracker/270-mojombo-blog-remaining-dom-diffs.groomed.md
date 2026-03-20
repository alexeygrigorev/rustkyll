# Issue 270: Fix remaining mojombo-blog DOM differences (14/17 -> 17/17)

## Problem

mojombo-blog currently matches 14/17 pages. The original issue description (prev/next navigation) was incorrect -- that was already fixed in issue 186 (root cause was `site.related_posts` not excluding the current post). The 3 remaining failures are unrelated to navigation and are caused by:

1. **Kramdown underscore emphasis parsing** (how-i-turned-down-300k.html, 5 diffs)
   - Input: `"I can has _______"` (7 underscores)
   - Jekyll/kramdown renders: `"I can has <strong>__</strong>_"`
   - Rustkyll renders: `"I can has _______"` (literal underscores)
   - Kramdown treats runs of underscores as emphasis delimiters; pulldown-cmark (CommonMark) does not

2. **Image alt attribute newline normalization** (the-git-parable.html, 1 diff)
   - Jekyll normalizes `alt='Creative\nCommons License'` to `alt='Creative Commons License'`
   - Rustkyll preserves the literal newline in the alt attribute
   - This is a whitespace normalization issue in HTML attribute output

3. **Ruby syntax highlighting token differences** (tomdoc-reasonable-ruby-documentation.html, 8 diffs)
   - Token classification differences between rustkyll's syntect and Jekyll's Rouge
   - Affects `<span>` class attributes in highlighted Ruby code blocks
   - Example: `class='n'` vs `class='o'`, `class='o'` vs `class='k'`

## Scope

Fix all 3 categories to reach 17/17 DOM match on mojombo-blog.

### Category 1: Underscore emphasis (5 diffs)

The `_______` pattern is a kramdown-specific emphasis edge case. Kramdown treats `__` as strong emphasis delimiters even when embedded in a longer run of underscores. This was previously descoped in issue 246 for the DTC site (same class of bug with `____`).

**Approach:** Add a postprocessing step (or pre-processing markdown normalization) that handles runs of 4+ consecutive underscores the way kramdown does. Kramdown greedily matches `__...__` as `<strong>` and leaves remaining underscores as literal text.

Specifically, kramdown parses `_______` (7 underscores) as:
- `__` = open strong
- `__` = strong content (literal underscores inside strong)
- `__` = close strong
- `_` = remaining literal underscore

Resulting in: `<strong>__</strong>_`

### Category 2: Alt attribute newline (1 diff)

The markdown source contains an image with an alt attribute that spans multiple lines. Jekyll collapses the newline to a space in the rendered HTML attribute. Rustkyll preserves the literal newline.

**Approach:** In the HTML output postprocessing, normalize whitespace in HTML element attributes -- specifically collapse newlines to spaces in attribute values. This is standard HTML behavior per the spec.

### Category 3: Ruby syntax highlighting (8 diffs)

Token classification differences between syntect and Rouge for Ruby code. The differences are in how tokens are classified (e.g., `n` vs `o`, `o` vs `k`).

**Approach:** Add or adjust token mapping rules in the syntect-to-Rouge compatibility layer for Ruby-specific tokens. Check `src/syntax.rs` for existing mappings.

## Impact

Fixes 3 pages to achieve 17/17 (100%) DOM match on mojombo-blog.

## Dependencies

None. These fixes are independent and can each be implemented without waiting for the kramdown rewrite (issue 282). The kramdown rewrite will eventually supersede fix #1, but a targeted fix is appropriate now.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `how-i-turned-down-300k.html` matches Jekyll DOM (underscore emphasis fix)
- [ ] `the-git-parable.html` matches Jekyll DOM (alt attribute newline fix)
- [ ] `tomdoc-reasonable-ruby-documentation.html` matches Jekyll DOM (syntax highlighting fix)
- [ ] mojombo-blog DOM comparison: 17/17 pages match (0 differences)
- [ ] No regressions on other test sites (run full DOM comparison suite)

## Test Scenarios

### Unit: Kramdown underscore emphasis handling

- Parse `_______` (7 underscores) in markdown, verify output contains `<strong>__</strong>_`
- Parse `____` (4 underscores), verify output contains `<em>__</em>` (kramdown treats `_..._` as em)
- Parse `__` (2 underscores), verify output is literal `__` (not enough for emphasis)
- Parse `___` (3 underscores), verify output contains `<em></em>` or is treated as horizontal rule depending on context
- Parse `_word_` normally, verify `<em>word</em>` still works (regression check)
- Parse `__word__` normally, verify `<strong>word</strong>` still works (regression check)
- Include non-ASCII content in test strings to catch encoding regressions

### Unit: HTML attribute whitespace normalization

- Render markdown image `![Creative\nCommons License](url)`, verify alt attribute is `Creative Commons License` (newline collapsed to space)
- Render markdown image `![normal alt](url)`, verify alt attribute unchanged (regression check)
- Render image with tab in alt text, verify tab collapsed to space

### Unit: Ruby syntax highlighting token mapping

- Highlight a Ruby code block containing the patterns from tomdoc (method definitions, `*` operator, `end` keyword)
- Verify token classes match Rouge output for the specific patterns that differ
- Test that existing Python/JavaScript highlighting is unaffected (regression check)

### Integration: Full site verification

- Build mojombo-blog with rustkyll
- Run DOM comparison against Jekyll output
- Verify 17/17 pages match with 0 total differences
- Inspect all 3 previously-failing HTML files to confirm fixes

### Regression: Other sites unaffected

- Run DOM comparison on DTC site (or other test sites) to verify no regressions
- Verify `cargo test` full suite passes

## Output Verification

The engineer must:
1. Build mojombo-blog: `./target/release/rustkyll build --source websites/mojombo-blog --destination websites/mojombo-blog/_site_rustkyll`
2. Run DOM comparison: `python3 scripts/dom_compare.py --jekyll-dir websites/mojombo-blog/_site_jekyll --rustkyll-dir websites/mojombo-blog/_site_rustkyll`
3. Verify output shows: `Summary: 17 files matched, 0 files with differences, 0 total differences`
4. Inspect each of the 3 previously-failing files manually to confirm the fix is correct

## Notes

- The original issue title referenced "previous/next post navigation" which was already fixed in issue 186 (the actual root cause was `site.related_posts` excluding the current post)
- The kramdown underscore emphasis issue was previously descoped in issue 246 for the same class of bug
- Issue 282 (kramdown phase 3 spans) will eventually provide a comprehensive fix for emphasis parsing, but this issue provides a targeted fix for the specific pattern seen in mojombo-blog
