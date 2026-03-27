# Issue 390: Replace pulldown-cmark with kramdown parser in markdownify

## Problem

The `markdownify` Liquid filter uses `markdown_to_html_for_filter()` which
routes through pulldown-cmark (CommonMark). This causes emphasis boundary
mismatches with Jekyll/kramdown on 3 remaining DTC books pages (24+ diffs).

We already have a full kramdown parser rewrite in `src/kramdown_parser/` that
handles emphasis correctly. The pulldown-cmark dependency in the markdownify
path should be eliminated entirely.

## Goal

Replace the pulldown-cmark-based `markdown_to_html_for_filter()` with the
kramdown parser for ALL markdownify rendering. This eliminates the root cause
of all emphasis boundary mismatches and removes the need for many preprocessing
workarounds that exist only to patch over pulldown-cmark/kramdown differences.

## Affected Pages

- `books/20221121-reliable-machine-learning.html` (13 diffs -- URL asterisk emphasis)
- `books/20231106-analytics-engineering-with-sql-and-dbt.html` (11 diffs -- intra-word emphasis)
- `books/20220425-natural-language-processing-with-transformers.html` (3 diffs -- pipe/autolink)

## Scope

1. Route `markdown_to_html_for_filter()` through the kramdown parser (`kramdown_parser::to_html` or `to_html_with_options`) instead of pulldown-cmark
2. Remove or simplify the many preprocessing functions that only exist to work around pulldown-cmark differences (emphasis boundary fixes, list nesting workarounds, etc.)
3. Verify ALL DTC pages still match -- the kramdown parser must handle `newline_to_br`-preprocessed text (text containing `<br />` tags from the `newline_to_br` filter applied before `markdownify`)
4. Must not regress DTC DOM (785/790) -- should improve toward 788/790
5. Clean up any dead code from the pulldown-cmark path

## Architecture Notes

### Current markdownify pipeline (`markdown_to_html_for_filter`)

Located at `src/frontmatter.rs:747`. The function currently:

1. Configures pulldown-cmark `Options` (tables, strikethrough, smart punctuation)
2. Runs ~20 preprocessing functions to work around pulldown-cmark/kramdown differences
3. Invokes `pulldown_cmark::Parser::new_ext` + `html::push_html`
4. Runs ~15 postprocessing functions to fix up the HTML output

### Target: kramdown parser

Located at `src/kramdown_parser/`. Entry point is `kramdown_parser::to_html(input)` or `kramdown_parser::to_html_with_options(input, &options)`. Already used for main page rendering in the `markdown_to_html()` pipeline.

### Preprocessing functions ONLY used in the markdownify filter path

These are called from `markdown_to_html_for_filter` but NOT from `markdown_to_html`. They exist solely to patch pulldown-cmark behavior for `newline_to_br`-preprocessed text. Once the kramdown parser handles this text natively, they should become dead code:

- `escape_fenced_code_after_br()` (line 3220) -- prevents fenced code blocks after `<br />\n`
- `merge_blockquote_continuations_after_br()` (line 3148) -- adds `> ` prefix to continuation lines
- `insert_paragraph_break_before_numbered_list()` (line 2912) -- inserts paragraph break before numbered lists
- `strip_br_from_empty_numbered_list_markers()` (line 3030) -- strips `<br />` from empty list markers

### Post-processing functions ONLY used in the markdownify filter path

These fix HTML output issues caused by pulldown-cmark's structure differing from kramdown's:

- `renest_heading_after_list()` (line 2217) -- re-nests headings pulled out of list items
- `renest_leaked_paragraph_and_ol_into_bullet_item()` (line 2346)
- `renest_leaked_paragraph_and_ul_into_bullet_item()` (line 2484)
- `renest_sibling_list_into_parent_li()` (line 2672)
- `convert_definition_list_in_html()` (line 2814) -- converts definition list patterns in HTML

### Preprocessing functions shared with the main pipeline

These are called from BOTH `markdown_to_html_for_filter` and `markdown_to_html`. They work around pulldown-cmark behavior in both paths. When the kramdown parser replaces pulldown-cmark in the filter path, evaluate whether each is still needed there:

- `escape_non_standard_autolink_schemes()` -- kramdown parser should handle this natively
- `preprocess_kramdown_dashes()` -- kramdown parser handles dashes natively
- `escape_paren_list_markers()` -- kramdown only uses `.` as list delimiter, not `)`, so parser should handle this
- `normalize_zwsp_for_emphasis()` -- may not be needed since kramdown handles ZWSP boundaries
- `fix_kramdown_emphasis_patterns()` -- exists to make pulldown-cmark parse emphasis at word boundaries; kramdown parser handles this natively
- `protect_consecutive_single_quotes()` + `restore_consecutive_single_quotes()` -- kramdown parser has its own smart quote handling
- `protect_liquid_quotes()` + `restore_liquid_quotes()` -- may still be needed if Liquid tags appear in markdownify input
- `protect_non_ascii_in_link_urls()` + `restore_non_ascii_in_urls()` -- pulldown-cmark percent-encodes; kramdown preserves
- `decode_pulldown_url_encoding()` -- purely pulldown-cmark artifact
- `strip_mailto_from_display_text()` -- pulldown-cmark autolink behavior
- `strip_emphasis_boundary_placeholder()` -- only needed for `fix_kramdown_emphasis_patterns`
- `protect_preexisting_curly_quotes()` + `restore_preexisting_curly_quotes()` -- smart quote direction fix support

### Key challenge: `<br />` tags in input

The markdownify filter often receives text that has already been through `newline_to_br`, which converts newlines to `<br />` tags. The kramdown parser must handle this correctly:
- `<br />` inside paragraphs should be preserved as-is
- `<br />` should not break list detection, blockquote continuation, or code blocks
- The kramdown parser's `parse_span_html` option (default: true) should handle inline `<br />` tags

## Baseline

- DTC DOM: 785/790

## Dependencies

- None (kramdown parser already exists and is battle-tested on main pipeline)

## Acceptance Criteria

- [ ] `markdown_to_html_for_filter()` routes through `kramdown_parser::to_html` (or `to_html_with_options`) instead of `pulldown_cmark::Parser`
- [ ] The pulldown-cmark `Options` configuration (ENABLE_TABLES, ENABLE_STRIKETHROUGH, ENABLE_SMART_PUNCTUATION) and `Parser::new_ext` + `html::push_html` calls are removed from `markdown_to_html_for_filter`
- [ ] All existing `markdown_to_html_for_filter` tests in `src/frontmatter.rs` still pass (50+ test calls)
- [ ] All existing tests in `tests/test_issue_385.rs`, `tests/test_issue_386.rs`, `tests/test_issue_378.rs`, `tests/test_issue_367.rs` still pass
- [ ] `./scripts/cargo-safe test` passes with zero failures
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] DTC DOM match count does not drop below 785/790 (must not regress)
- [ ] DTC DOM match count improves (target: 788/790 fixing the 3 affected books pages)
- [ ] The 3 affected books pages listed above produce correct HTML matching Jekyll output
- [ ] Dead preprocessing functions that are no longer called from ANY code path are identified in the issue log (they do not need to be deleted in this issue, but must be catalogued)
- [ ] Preprocessing functions that remain in `markdown_to_html_for_filter` are justified in the issue log (explain why each surviving pre/post-processing step is still needed with the kramdown parser)
- [ ] No site-specific hardcoding -- the change must work for any Jekyll site using the `markdownify` filter, not just DTC

## Test Scenarios

### Unit: Existing markdownify behavior preservation

- All ~50 existing `markdown_to_html_for_filter` test cases in `src/frontmatter.rs` must produce identical output
- If any test's expected output changes, the new output must be verified as MORE correct (matching Jekyll/kramdown), and the test updated with a comment explaining the change

### Unit: Emphasis boundary matching (the root cause)

- Input with intra-word asterisks (e.g., `word*X*`) must produce `<em>` tags matching kramdown behavior
- Input with URL-containing asterisks in link text must handle emphasis correctly
- Input with pipe characters adjacent to emphasis markers must match kramdown

### Unit: `<br />` tag handling

- Input containing `<br />` between paragraph text must preserve the `<br />` in output
- Input containing `<br />\n> ` (blockquote after br) must produce correct blockquote HTML
- Input containing `<br />\n1. ` (numbered list after br) must produce correct list HTML
- Input containing `<br />\n- ` (bullet list after br) must produce correct list HTML
- Input containing `` <br />\n``` `` (fenced code after br) must handle code blocks correctly

### Unit: Smart punctuation

- Dash sequences (em-dash, en-dash) must match kramdown's greedy algorithm
- Smart quotes must match kramdown's direction rules
- Consecutive single quotes ('', ''') must be handled correctly

### Unit: Autolinks and URLs

- Non-standard URI schemes (`<tel:...>`, `<ssh:...>`) must not become autolinks
- `<mailto:user@host>` must produce correct link without `mailto:` in display text
- Non-ASCII characters in link URLs must be preserved (not percent-encoded)

### Integration: DTC site build

- Build the full DTC site with `./scripts/cargo-safe build`
- Run DOM comparison and verify >= 785/790 match
- Specifically inspect the 3 affected books pages for correct emphasis rendering
- Verify no other pages regressed by comparing before/after DOM counts

### Regression: Non-DTC sites

- If the codebase has tests for other Jekyll sites (al-folio, text-theme, etc.), verify those still pass
- The markdownify filter must work generically for any kramdown-processed content

## Risk Mitigation

This is a high-impact change affecting every use of the `markdownify` Liquid filter across all sites. The main risks are:

1. **kramdown parser does not handle `<br />`-preprocessed text correctly** -- The parser was built for clean markdown, not markdown with embedded HTML `<br />` tags from `newline_to_br`. If the parser chokes on this input, it may need options or preprocessing adjustments.

2. **Subtle output differences** -- Even though both paths target kramdown output, the preprocessing/postprocessing pipeline may have introduced compensating behaviors that the raw kramdown parser does not replicate. Every test failure must be individually analyzed to determine if the new output is more correct or a regression.

3. **Dead code removal risk** -- Functions that appear unused after the switch may still be called from `markdown_to_html` or `markdown_to_html_with_options` (the main page rendering pipeline, which still uses pulldown-cmark). Only remove functions that are truly dead across ALL call sites.

The SWE should proceed incrementally:
1. First, swap the parser and run all tests to identify failures
2. Then, analyze each failure to determine if the new output is better or worse
3. Only then remove preprocessing that is confirmed unnecessary
4. Run full DTC DOM comparison after each major change

## Log

### [SWE] 2026-03-27

#### Investigation: Full kramdown parser swap

**Step 1: Swap parser, minimal preprocessing**
- Replaced `pulldown_cmark::Parser::new_ext` + `html::push_html` with `kramdown_parser::to_html()`
- Removed all pulldown-cmark-specific pre/post processing
- Added `add_inline_code_class_to_kramdown_output()` to add `language-plaintext highlighter-rouge` to bare `<code>` tags (kramdown parser doesn't add these by default)
- Result: **37 test failures, DOM dropped to 704/790** (82 page regression)

**Step 2: Added preprocessing to restore blank lines**
- Added `restore_blank_lines_from_br()`: converts standalone `<br />` lines back to blank lines
- Added `insert_paragraph_breaks_before_blocks()`: inserts blank lines before list/heading markers after `<br />` lines
- Also restored `escape_fenced_code_after_br`, `merge_blockquote_continuations_after_br`, `strip_br_from_empty_numbered_list_markers`
- Result: **DOM still 704/790** -- improvements on some pages but new issues emerged

**Root cause analysis: 4 categories of differences**

1. **Paragraph/block boundaries (affects ~80 book pages)**
   - In kramdown, list markers (`1.`, `-`, `*`) do NOT interrupt paragraphs (confirmed by kramdown conformance test `block/08_list/escaping`)
   - In pulldown-cmark (CommonMark/GFM), list markers DO interrupt paragraphs (with restrictions: only `1.` interrupts, not other numbers)
   - DTC uses Jekyll with `kramdown-parser-gfm` (GFM mode), where list markers interrupt paragraphs
   - The previous preprocessing functions (`insert_paragraph_break_before_numbered_list`) were specifically designed for pulldown-cmark's behavior
   - The kramdown parser would need either GFM-mode paragraph interruption or more aggressive preprocessing

2. **Loose vs tight list items (cascading from #1)**
   - When the kramdown parser DOES create lists, it wraps content in `<p>` tags (loose lists)
   - The pulldown-cmark path produced tight lists with `<br />` inside `<li>` directly
   - Jekyll/kramdown output matches the tight list style for `newline_to_br | markdownify` content
   - This is a fundamental difference in how the two parsers handle list item content

3. **HTML tag parsing (affects ~10 pages)**
   - kramdown parser with `parse_span_html: true` treats angle brackets in prose as HTML tags
   - Example: `"come on <TensorFlow 2"` produces `<tensorflow>` element
   - Example: `"chef's kiss"` with angle brackets produces `<chef's>` element
   - pulldown-cmark only treats valid HTML tags as HTML, not arbitrary angle-bracket content

4. **Smart quote direction differences (minor, ~2 pages)**
   - kramdown parser's smart quote algorithm differs slightly from pulldown-cmark + fix_smart_quote_directions
   - Specifically after `<br />` tags, the quote direction context differs

**Conclusion: Full swap is not feasible in a single issue**

The kramdown parser was built for clean markdown input, not for `newline_to_br`-preprocessed text with `<br />\n` throughout. The `newline_to_br | markdownify` pipeline is fundamentally problematic because `newline_to_br` destroys the blank-line structure that the parser needs.

Making this work would require:
1. Adding GFM-mode paragraph interruption to the kramdown parser (significant parser change)
2. Adding `<br />`-aware list item tightening to the kramdown HTML converter
3. Making `parse_span_html` smarter about distinguishing real HTML tags from angle brackets in prose
4. Extensive preprocessing to reconstruct the original text structure from `<br />`-annotated input

Each of these is a separate issue. The emphasis boundary fix (the original goal) can be achieved more safely by targeted fixes to the existing pulldown-cmark preprocessing.

**Files modified during investigation (all reverted):**
- `src/frontmatter.rs` -- swapped parser, added preprocessing functions (REVERTED)
- `src/kramdown_parser/parser.rs` -- tested paragraph interruption change (REVERTED)
- `src/kramdown_parser/span_parser.rs` -- tested inline code class change (REVERTED)

**Recommendation:** Split this into smaller issues:
- Issue A: Fix the 3 specific emphasis boundary mismatches in the pulldown-cmark path
- Issue B: Add GFM paragraph interruption mode to the kramdown parser
- Issue C: Add `<br />` handling mode to the kramdown parser for markdownify
- Issue D: After B+C are complete, swap markdownify to kramdown parser
