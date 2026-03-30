# Issue 369: DTC blockquote + list continuation

## Parent

Follow-up from #363 (RC-H).

## Problem

Two distinct but related problems in how markdownify handles blockquote and list continuation with `<br>` tags:

### Sub-problem A: Extra `<blockquote>` elements (analytics-engineering page)

In the `books/20231106-analytics-engineering-with-sql-and-dbt.html` page, a comment contains alternating blockquote markers (`> *quoted text*`) and list items (`- list text`), all connected by `<br />` from `newline_to_br`. Jekyll/kramdown renders this as a single `<blockquote>` containing both `<p>` (for quoted text) and `<ul>` (for list items). Rustkyll instead produces multiple separate `<blockquote>` elements with the `<ul>` rendered outside the blockquote.

**Jekyll output** (lines 206-230 of built page):
```html
<blockquote>
<p><em>Is there any tool comparable to dbt?</em><br /></p>
<ul>
  <li>Matilion is more of fully-fledged ETL ...</li>
  <li>An alternative to dbt ...</li>
  ...
</ul>
</blockquote>
```

**Rustkyll (incorrect) output** produces:
```html
<blockquote><p><em>Is there any tool comparable to dbt?</em></p></blockquote>
<ul><li>Matilion ...</li>...</ul>
<blockquote><p><em>Have you tested dbt vault?</em></p></blockquote>
<ul><li>Nope</li>...</ul>
```

The DOM diff shows:
- `blockquote > ul: missing_element` (list should be inside blockquote)
- `ul: extra_element` (list incorrectly outside blockquote)
- `blockquote: extra_element` (spurious extra blockquotes)

The source YAML text contains patterns like:
```
> *Is there any tool comparable to dbt?*\n- Matilion is ...\n- An alternative ...\n> *Have you tested dbt vault?*\n- Nope
```

After `newline_to_br`, this becomes `> ` lines alternating with `- ` lines connected by `<br />`. The existing `merge_blockquote_continuations_after_br()` in `src/frontmatter.rs` (line ~3724) handles some cases but does not merge list items (`- `) that follow blockquote markers into the same blockquote context.

### Sub-problem B: Missing `<ol>` inside `<li>` with `<br>` continuation (business-skills page)

In the `books/20210823-business-skills-for-data-scientists.html` page, a comment contains the pattern:
```
- \nHere are a few tips\n1. Identify ...\n2. Actively ...\n3. Work to produce ...
```

Jekyll/kramdown renders this as:
```html
<ul>
  <li><br />
    Here are a few tips<br />
    <ol>
      <li>Identify what's currently important ...</li>
      <li>Actively grow your internal network ...</li>
      <li>Work to produce value quickly ...</li>
    </ol>
  </li>
  ...
</ul>
```

The `<li>` contains continuation text ("Here are a few tips"), a `<br>`, and a nested `<ol>`. Rustkyll is missing the continuation text, the `<br>`, and the entire nested `<ol>`.

The DOM diff shows:
- `ul > li: missing_text` - expected: 'Here are a few tips'
- `ul > li > br: missing_element`
- `ul > li > ol: missing_element`
- `ul > li: missing_element` (3x for the `<li>` items inside the `<ol>`)

### Root Cause

**Sub-problem A**: The `merge_blockquote_continuations_after_br()` function in `src/frontmatter.rs` merges non-`>` continuation lines into blockquotes when they are connected by `<br />`. However, it does not handle the case where a `- ` list marker line follows a `> ` blockquote line. Kramdown treats these list items as belonging to the blockquote context when they are in the same "paragraph" (connected by `<br />`).

**Sub-problem B**: The `insert_paragraph_break_before_numbered_list()` function in `src/frontmatter.rs` and/or the `strip_br_from_empty_numbered_list_markers()` function may be interfering with list continuation inside a `<li>` that has `<br>` followed by text and then a numbered list. The numbered list (`1. 2. 3.`) should become a nested `<ol>` inside the parent `<li>`, but instead the text and nested list are being dropped or misrendered.

### Files to Modify

- `src/frontmatter.rs` -- `merge_blockquote_continuations_after_br()` (line ~3724) for Sub-problem A
- `src/frontmatter.rs` -- `insert_paragraph_break_before_numbered_list()` (line ~3408) and/or `strip_br_from_empty_numbered_list_markers()` (line ~3526) for Sub-problem B
- `src/template/filters/markdownify.rs` -- may need test additions

## Affected Pages

- `books/20231106-analytics-engineering-with-sql-and-dbt.html` -- extra `<blockquote>` elements, `<ul>` outside blockquote (partial of 18 diffs in old DOM report; current state at 790/790 means some may have been resolved)
- `books/20210823-business-skills-for-data-scientists.html` -- missing text/`<br>`/`<ol>` inside `<li>` (9+ diffs in old DOM report)

## Dependencies

None. Self-contained changes to the markdownify preprocessing pipeline.

## Acceptance Criteria

### Sub-problem A: Blockquote + list nesting

- [ ] When a `> *quoted text*` line is followed by `- list item` lines (connected by `<br />`), the list renders inside the `<blockquote>`, not as a separate element outside it
- [ ] No extra `<blockquote>` elements are generated -- the alternating `> ` and `- ` lines within one `<br />`-connected block produce a single `<blockquote>` containing both `<p>` and `<ul>`
- [ ] The fix is generic: any blockquote followed by list items in `<br />`-connected text should be handled

### Sub-problem B: Nested `<ol>` inside `<li>` with continuation text

- [ ] When a list item (`- \n`) is followed by continuation text ("Here are a few tips") and then a numbered list (`1. 2. 3.`), the output contains: `<li><br />\nHere are a few tips<br />\n<ol><li>...</li></ol></li>`
- [ ] The continuation text appears as text content of the `<li>`, not dropped
- [ ] The `<br>` element appears between the list marker and the continuation text
- [ ] The `<ol>` is nested inside the parent `<li>`, not rendered as a sibling

### General

- [ ] No site-specific hardcoding
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes with no regressions
- [ ] DTC DOM baseline: 790/790 -- match count must not drop below this

## Test Scenarios

### Unit: Sub-problem A -- blockquote with list continuation

- Input to markdownify: `"> *Is there any tool comparable to dbt?*<br />\n- Matilion is a fully-fledged ETL tool<br />\n- An alternative is Dataform<br />\n> *Have you tested dbt vault?*<br />\n- Nope"`
- Expected: single `<blockquote>` containing `<p>` with the quoted text AND `<ul>` with the list items
- Verify: no extra `<blockquote>` elements in output
- Verify: `<ul>` is inside `<blockquote>`, not a sibling

### Unit: Sub-problem A -- blockquote without list (no regression)

- Input: `"> This is a quote<br />\n> continued quote"`
- Expected: single `<blockquote>` with the quoted text, no `<ul>`
- Verify: existing blockquote behavior is preserved

### Unit: Sub-problem B -- list item with continuation text and nested `<ol>`

- Input to markdownify: `"- <br />\nHere are a few tips<br />\n1. First tip<br />\n2. Second tip<br />\n3. Third tip"`
- Expected: `<ul><li><br />\nHere are a few tips<br />\n<ol><li>First tip</li><li>Second tip</li><li>Third tip</li></ol></li></ul>`
- Verify: "Here are a few tips" appears as text content inside the `<li>`
- Verify: `<ol>` is nested inside the `<li>`

### Unit: Sub-problem B -- list item with continuation text only (no nested list)

- Input: `"- <br />\nSome continuation text"`
- Expected: `<ul><li><br />\nSome continuation text</li></ul>`
- Verify: continuation text is preserved

### Unit: Unicode content

- Input: `"> *Gibt es ein vergleichbares Tool?*<br />\n- Ja, es gibt Alternativen"`
- Expected: correct blockquote + list rendering with non-ASCII characters

### Integration: DTC output verification

- Build the DTC site
- Inspect `books/20231106-analytics-engineering-with-sql-and-dbt.html`: verify single `<blockquote>` elements with `<ul>` inside (not outside)
- Inspect `books/20210823-business-skills-for-data-scientists.html`: verify "Here are a few tips" text, `<br>`, and `<ol>` are present inside the `<li>`
- Run DOM comparison, verify 790/790 is maintained

## DOM Baseline

- Current: 790/790 matched
- Expected after fix: 790/790 maintained (these pages currently match at 790/790; the diffs described in RC-H may have been partially resolved by prior fixes or the current DOM comparison may count them differently)

## Priority

MEDIUM

## Log

### [SWE] 2026-03-30

- Analyzed the two sub-problems described in the issue
- Sub-problem A (blockquote + list continuation): The existing `merge_blockquote_continuations_after_br()` in src/frontmatter.rs already handles this case. The function adds `> ` prefix to non-blockquote lines in blockquote runs, and the post-processing at line 3243 merges `</blockquote>\n<ul>` into the blockquote. Test confirms `<ul>` renders inside `<blockquote>`.
- Sub-problem B (nested `<ol>` inside `<li>` with continuation): The existing `strip_br_from_empty_numbered_list_markers()` already joins continuation text onto the list marker line, and `insert_paragraph_break_before_numbered_list()` recognizes the numbered list. Test confirms "Here are a few tips" text, `<br>`, and `<ol>` all appear nested inside the `<li>`.
- Both sub-problems were already resolved by prior fixes (issues 370, 381, 379, etc.)
- Wrote 5 unit tests to lock in correct behavior as regression guards:
  1. test_issue369_blockquote_with_list_continuation - verifies `<ul>` inside `<blockquote>`, no extra blockquotes
  2. test_issue369_blockquote_without_list_no_regression - verifies pure blockquote still works
  3. test_issue369_list_item_with_continuation_text_and_nested_ol - verifies text, `<ol>` nested in `<li>`, all 3 tips present
  4. test_issue369_list_item_continuation_text_only - verifies continuation text without nested list
  5. test_issue369_unicode_blockquote_list - verifies non-ASCII content in blockquote+list
- All 5 tests PASS
- Full test suite: all tests pass, no regressions
- clippy: clean (no warnings with -D warnings)
- fmt: clean (no changes)
- DTC DOM baseline: 790/790 maintained
- Files modified: src/template/filters/markdownify.rs (5 new tests added)
