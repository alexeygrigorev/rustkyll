# Issue 271: DTC loose list item paragraph wrapping

## Status: Already Resolved -- Verify and Close

## Problem (Original)

On 17 DTC pages, list items with blank lines between them (loose lists per CommonMark spec) were reported as not getting their content wrapped in `<p>` tags. Jekyll/kramdown wraps loose list item content in `<p>`, but rustkyll allegedly did not.

## Investigation Findings

**This issue is already fixed in the current codebase.** A fresh build of the DTC site and comparison against Jekyll output shows zero `<li><p>` wrapping differences across all 551 matched HTML files.

The fix is spread across several mechanisms in `src/kramdown.rs`:

1. **`collapse_blank_lines_between_list_items` (line 653):** Pre-markdown step that identifies fully loose lists (blank lines between ALL consecutive items) and preserves their blank lines so pulldown-cmark sees them. Partially loose lists are collapsed to tight, matching kramdown behavior.

2. **`strip_paragraphs_in_html_blocks` / `strip_p_in_tag` (line 1306):** Post-markdown step that strips auto-inserted `<p>` from HTML block elements. Has an explicit `is_bare_li` check (line 1376) that skips stripping for bare `<li>` elements (from markdown list syntax), preserving the `<p>` tags pulldown-cmark correctly inserts for loose lists. Only strips `<p>` from `<li>` with attributes (e.g., `<li class="podcast">`), which come from raw HTML/Liquid where pulldown-cmark erroneously adds `<p>`.

3. **pulldown-cmark:** Correctly distinguishes tight vs loose lists. When a list is loose (blank lines between items), it emits `Start(Paragraph)` inside the list item. When tight, it does not.

Existing tests already cover this (5 tests matching `loose_list`):
- `test_loose_list_preserves_p_tags_in_li`
- `test_tight_list_no_p_tags_in_li`
- `test_loose_list_multi_paragraph_item`
- `test_html_li_with_attributes_still_strips_p`
- `test_readme_driven_development_loose_list`

## Scope

The engineer must verify the fix is complete and close the issue. No new code is expected unless gaps are found during verification.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (including the 5 loose list tests)
- [ ] Fresh DTC site build (`rustkyll build --source datatalksclub.github.io --destination /tmp/dtc_verify`) produces zero `<li><p>` wrapping differences compared to Jekyll output
- [ ] Loose list items (blank lines between ALL items) wrap content in `<p>` tags matching Jekyll
- [ ] Tight list items (no blank lines between items) remain unwrapped (no `<p>`)
- [ ] Partially loose lists (some blank lines but not all) are collapsed to tight, matching kramdown behavior
- [ ] HTML `<li>` elements with attributes (from Liquid includes) still have their auto-inserted `<p>` stripped

## Test Scenarios

### Verification: Existing tests pass
- Run `cargo test loose_list` and confirm all 5 tests pass
- Run `cargo test tight_list` and confirm tight list tests pass

### Verification: DTC site output
- Build the DTC site with rustkyll
- Compare `<li>` content wrapping against Jekyll output across all pages
- Confirm zero differences in `<li><p>` wrapping

### Unit: Edge cases to add if missing
- Loose ordered list (`1. item\n\n2. item`) wraps items in `<p>`, including non-ASCII content
- Loose list with inline formatting (`- **bold item**\n\n- *italic item*`) preserves `<p>` wrapping
- Loose list inside a blockquote preserves `<p>` wrapping
- Mixed list markers (`-` then `*`) with blank lines between -- verify kramdown-compatible behavior
- Single-item list with trailing blank line does NOT get `<p>` wrapping (not loose)

## Dependencies

None. This issue is self-contained verification work.
