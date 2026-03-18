# Issue 219: Fix DTC author description leading newline

## Problem

Issue 217 switched `collection_item_to_liquid_slim()` from `html_content` to `content` (raw markdown) for the `content` field. However, the raw markdown stored in `CollectionItem.content` has a leading `\n` because most Jekyll files have a blank line between the closing `---` front matter delimiter and the body text.

**Root cause:** In `src/frontmatter.rs`, `split_front_matter()` returns the body starting immediately after the closing `---\n` line (line 73-77). For a typical file like:

```
---
short: alexeygrigorev
---
                          <-- this blank line becomes a leading \n in body
Alexey Grigorev is the founder of DataTalks.Club
```

The body returned is `"\nAlexey Grigorev is the founder of DataTalks.Club"`. This propagates through `parse_document()` (line 152: `content = body.to_string()`) into `CollectionItem.content` and then into the Liquid `content` field in `collection_item_to_liquid_slim()` (line 336).

**Effect:** JSON-LD author descriptions render as `"\nAlexey Grigorev is the founder..."` instead of `"Alexey Grigorev is the founder..."`. This affects ~200+ DTC blog pages.

**Fix location:** The trim should happen in `collection_item_to_liquid_slim()` in `src/generator.rs` (line 336), trimming leading whitespace from `item.content` before inserting it. Do NOT change `parse_document()` or `split_front_matter()` because other code paths (excerpt extraction, markdown rendering) may depend on the current content format.

## Scope

1. In `collection_item_to_liquid_slim()` (src/generator.rs, line ~336), trim leading whitespace from `item.content` before inserting into the Liquid object
2. Verify no regressions in other collection item content usage (excerpt, html_content, output field)
3. Verify the fix works for files with and without a blank line after front matter

## Dependencies

- Issue 217 (fix DTC JSON-LD author descriptions) -- done

## Acceptance Criteria

- [ ] `collection_item_to_liquid_slim()` content field has no leading `\n` or whitespace -- e.g., for alexeygrigorev.md the content is `"Alexey Grigorev is the founder of DataTalks.Club"` not `"\nAlexey Grigorev is the founder of DataTalks.Club"`
- [ ] Content with no leading whitespace in the source file is unaffected (trim_start on a string with no leading whitespace is a no-op)
- [ ] The `output` field (html_content) in the slim representation is NOT trimmed -- it should remain as-is since HTML rendering handles its own whitespace
- [ ] DTC author description JSON-LD matches Jekyll output exactly (no leading `\n`)
- [ ] No regressions in other collection item fields (url, slug, front_matter, excerpt)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Tests include non-ASCII/Unicode author description content (e.g., accented characters)
- [ ] No hardcoded site-specific logic -- the fix must be generic

## Test Scenarios

All tests follow strict TDD: write the test FIRST, run it to verify it FAILS, THEN implement the fix, THEN re-run to verify it PASSES.

### Unit: Leading whitespace trimmed from slim content field

**Test 1: Content with leading newline is trimmed**
- Create a `CollectionItem` with `content = "\nAlexey Grigorev is the founder of DataTalks.Club"` (leading `\n`, matching what `split_front_matter` produces for typical files)
- Call `collection_item_to_liquid_slim()` and assert `obj["content"]` equals `"Alexey Grigorev is the founder of DataTalks.Club"` (no leading `\n`)
- FIRST RUN: Expect FAIL -- current code inserts `item.content` as-is, so the leading `\n` is preserved
- After fix: Expect PASS

**Test 2: Content with multiple leading newlines is trimmed**
- Create a `CollectionItem` with `content = "\n\n\nSome bio text"` (multiple leading newlines, possible with extra blank lines in source)
- Call `collection_item_to_liquid_slim()` and assert `obj["content"]` equals `"Some bio text"`
- FIRST RUN: Expect FAIL
- After fix: Expect PASS

**Test 3: Content with no leading whitespace is unchanged**
- Create a `CollectionItem` with `content = "Already trimmed content"` (no leading whitespace)
- Call `collection_item_to_liquid_slim()` and assert `obj["content"]` equals `"Already trimmed content"`
- FIRST RUN: Expect PASS (trim_start is a no-op)
- After fix: Must still PASS

**Test 4: Content with leading newline and non-ASCII characters**
- Create a `CollectionItem` with `content = "\nRene Descartes est un philosophe francais"` (use actual accented characters: e-acute in Rene, c-cedilla in francais)
- Call `collection_item_to_liquid_slim()` and assert `obj["content"]` starts with `"Ren"` (not `"\nRen"`) and contains the accented characters
- FIRST RUN: Expect FAIL (leading `\n` preserved)
- After fix: Expect PASS

**Test 5: Content with leading newline and markdown links**
- Create a `CollectionItem` with `content = "\nDavid Gates is the founder of [Accents Welcome](https://accentswelcome.com)"` (leading `\n` plus markdown link)
- Call `collection_item_to_liquid_slim()` and assert `obj["content"]` equals `"David Gates is the founder of [Accents Welcome](https://accentswelcome.com)"` (leading `\n` removed, markdown link preserved)
- FIRST RUN: Expect FAIL (leading `\n` preserved)
- After fix: Expect PASS

**Test 6: output field is NOT trimmed**
- Create a `CollectionItem` with `html_content = "<p>Some bio</p>\n"` (trailing newline from HTML rendering) and `content = "\nSome bio"`
- Call `collection_item_to_liquid_slim()` and assert `obj["output"]` equals `"<p>Some bio</p>\n"` (output field unchanged)
- FIRST RUN: Expect PASS (output field is not affected by this change)
- After fix: Must still PASS

### Note on existing tests

The 5 tests added in issue 217 (`test_collection_item_content_uses_raw_markdown`, `test_collection_item_content_preserves_markdown_links`, etc.) use `CollectionItem` content values WITHOUT a leading `\n`. They should continue to pass unchanged. The new tests in this issue specifically cover the leading-newline case.

## Implementation Notes

- The fix is a single-line change: in `collection_item_to_liquid_slim()`, change `item.content.clone()` to `item.content.trim_start().to_string()` (or equivalent)
- Use `trim_start()` not `trim()` -- trailing whitespace in content should be preserved as-is (it may be intentional in some markdown files, though unlikely)
- Do NOT modify `split_front_matter()`, `parse_document()`, or `CollectionItem.content` -- the raw content should remain unchanged in the struct; only the Liquid representation should be trimmed
- Do NOT modify `collection_item_to_liquid_full()` in `src/pagination.rs` -- that uses `html_content` which handles its own whitespace via HTML rendering

## Log

- 2026-03-18: Created. Regression from issue 217.
- 2026-03-18: Groomed by PM. Root cause traced to `split_front_matter()` preserving blank line after front matter closing delimiter. Fix is targeted to `collection_item_to_liquid_slim()` only. Added 6 TDD test scenarios covering leading newline, multiple newlines, no-op case, non-ASCII, markdown links, and output field non-regression.

### [SWE] 2026-03-18
- TDD Step 1: Wrote 6 tests in src/generator.rs (test_slim_content_leading_newline_trimmed, test_slim_content_multiple_leading_newlines_trimmed, test_slim_content_no_leading_whitespace_unchanged, test_slim_content_leading_newline_unicode, test_slim_content_leading_newline_markdown_links, test_slim_output_field_not_trimmed)
- TDD Step 2: Ran tests: 4 FAIL as expected (leading newline cases), 2 PASS as expected (no-op and output field cases)
  - test_slim_content_leading_newline_trimmed: FAIL -- got "\nAlexey Grigorev...", expected "Alexey Grigorev..."
  - test_slim_content_multiple_leading_newlines_trimmed: FAIL -- got "\n\n\nSome bio text", expected "Some bio text"
  - test_slim_content_leading_newline_unicode: FAIL -- got "\nRene..." instead of "Rene..."
  - test_slim_content_leading_newline_markdown_links: FAIL -- got "\nDavid Gates..." instead of "David Gates..."
  - test_slim_content_no_leading_whitespace_unchanged: PASS (trim_start is no-op)
  - test_slim_output_field_not_trimmed: PASS (output field unaffected)
- TDD Step 3: Implemented fix in src/generator.rs line 336: changed `item.content.clone()` to `item.content.trim_start().to_string()`
- TDD Step 4: Ran tests: all 6 PASS
- Full test suite: 1725 lib + all integration tests pass, 0 failures
- Clippy: clean (no warnings)
- Format: clean
- Files modified: src/generator.rs (1 line fix + 6 tests)

### [QA] 2026-03-18
- cargo build: compiles without errors
- cargo test: all tests pass (lib + integration)
- cargo clippy -- -D warnings: clean (only vendored liquid-core warnings, no project warnings)
- cargo fmt --check: clean
- Code review: 1-line fix in collection_item_to_liquid_slim() at line 336, changes `item.content.clone()` to `item.content.trim_start().to_string()`. Correct and minimal.
- 6 new tests cover: leading newline, multiple leading newlines, no-op case, non-ASCII/Unicode, markdown links, output field non-regression
- TDD verified: SWE log shows tests written first, 4 failed as expected, 2 passed as expected, then fix applied, all 6 pass
- AC 1 (no leading newline in content): PASS
- AC 2 (no-op for already trimmed): PASS
- AC 3 (output field NOT trimmed): PASS
- AC 4 (JSON-LD matches Jekyll): PASS
- AC 5 (no regressions in other fields): PASS
- AC 6 (cargo build): PASS
- AC 7 (cargo test): PASS
- AC 8 (non-ASCII/Unicode test): PASS
- AC 9 (no hardcoded site-specific logic): PASS
- VERDICT: PASS

### [PM] 2026-03-18 -- Acceptance Review
- All 9 acceptance criteria verified against code diff and QA report
- Fix is a single `trim_start().to_string()` call in `collection_item_to_liquid_slim()` -- minimal and correct
- 6 tests cover: leading newline, multiple newlines, no-op, non-ASCII/Unicode, markdown links, output field non-regression
- TDD verified: SWE wrote tests first, 4 failed / 2 passed as expected, then applied fix, all 6 pass
- No silent descoping: all criteria met, no follow-up issues needed
- Note: unstaged diff also contains issue 220 changes (smart punctuation) which are separate and not reviewed here
- VERDICT: **ACCEPT**
