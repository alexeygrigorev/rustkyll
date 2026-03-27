# Issue 391: Add GFM paragraph interruption mode to kramdown parser

## Problem

The kramdown parser does not support GFM-style paragraph interruption by list
markers. In standard kramdown, list markers (`1.`, `-`, `*`) do NOT interrupt
paragraphs -- a blank line is required before a list. In GFM mode (used by
DTC's Jekyll via `kramdown-parser-gfm`), list markers DO interrupt paragraphs.

This behavioral difference blocks switching the markdownify pipeline to the
kramdown parser (#390).

## Scope

1. Add a `gfm_paragraph_interruption` option (bool, default false) to `Options` in `src/kramdown_parser/options.rs`
2. Thread the option through to `parse_paragraph_with_lazy` in `src/kramdown_parser/parser.rs`
3. When enabled, add `is_list_start(line)` and `is_horizontal_rule(line)` as paragraph-breaking conditions in `parse_paragraph_with_lazy` (lines ~1216-1243), matching the behavior already present in `parse_paragraph_in_list_context_with_lazy` (line ~324)
4. Support parsing `gfm_paragraph_interruption: true` from `.options` files
5. Existing behavior (option=false) must be unchanged -- all existing kramdown tests must continue to pass
6. This change must NOT affect the main rendering pipeline or DTC DOM baseline

## Key Code Locations

- `src/kramdown_parser/options.rs` -- add the new bool field to `Options` struct and `Default` impl, add parsing in `parse_options_str`
- `src/kramdown_parser/parser.rs:1216` -- the comment "In kramdown, HRs and list markers do NOT interrupt paragraphs" is where the GFM behavior diverges. When `gfm_paragraph_interruption` is true, add `is_list_start(line)` and `is_horizontal_rule(line)` checks here
- `src/kramdown_parser/parser.rs:324` -- `parse_paragraph_in_list_context_with_lazy` already breaks on list markers; use this as reference for the correct break conditions

## Dependencies

- No blocking dependencies (this adds a new opt-in mode)
- Prerequisite for #390 (kramdown parser in markdownify)

## DTC DOM Baseline

- Current: 787/790
- This change adds a new option defaulting to false, so DOM count must remain 787/790

## Acceptance Criteria

- [ ] `Options` struct has a `gfm_paragraph_interruption: bool` field, defaulting to `false`
- [ ] `Options::parse_options_str` parses `gfm_paragraph_interruption: true` from `.options` content
- [ ] With `gfm_paragraph_interruption: false` (default), list markers do NOT interrupt paragraphs (standard kramdown behavior preserved)
- [ ] With `gfm_paragraph_interruption: true`, unordered list markers (`-`, `*`, `+`) interrupt paragraphs
- [ ] With `gfm_paragraph_interruption: true`, ordered list markers (`1.`, `2)`) interrupt paragraphs
- [ ] With `gfm_paragraph_interruption: true`, horizontal rules (`---`, `***`, `___`) interrupt paragraphs
- [ ] All existing kramdown parser tests pass unchanged (the default is false, so no behavior change)
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] DTC DOM baseline remains at 787/790 or above

## Test Scenarios

### Unit: Option parsing
- Parse `.options` string with `gfm_paragraph_interruption: true`, verify the field is true
- Parse `.options` string without the option, verify default is false

### Unit: Paragraph interruption disabled (default)
- Input: `"Some text\n- list item\n"` with default options -- verify produces a single paragraph containing "Some text\n- list item" (list does NOT interrupt)
- Input: `"Some text\n1. ordered\n"` with default options -- verify single paragraph (ordered list does NOT interrupt)

### Unit: Paragraph interruption enabled
- Input: `"Some text\n- list item\n"` with `gfm_paragraph_interruption: true` -- verify produces a paragraph "Some text" followed by an unordered list
- Input: `"Some text\n* list item\n"` with `gfm_paragraph_interruption: true` -- verify paragraph then unordered list
- Input: `"Some text\n+ list item\n"` with `gfm_paragraph_interruption: true` -- verify paragraph then unordered list
- Input: `"Some text\n1. ordered\n"` with `gfm_paragraph_interruption: true` -- verify paragraph then ordered list
- Input: `"Some text\n---\n"` with `gfm_paragraph_interruption: true` -- verify paragraph then horizontal rule (not setext header)

### Unit: Edge cases
- Input starting with a list marker (no preceding paragraph) works the same in both modes
- Blank line before list works the same in both modes (blank line already separates)
- List inside blockquote with GFM mode enabled -- list interrupts paragraph inside the blockquote
- Indented code does NOT interrupt paragraphs in GFM mode (only list markers and HRs do)

### Integration: HTML output
- `to_html_with_options("Some text\n- item\n", &opts_with_gfm)` produces `<p>Some text</p>\n<ul>\n  <li>item</li>\n</ul>\n` (or equivalent kramdown HTML)
- `to_html("Some text\n- item\n")` (default options) produces a single `<p>` with both lines

### Regression: Existing tests
- All tests in `src/kramdown_parser/tests.rs` pass without modification
- All fixture-based kramdown tests pass without modification

## Log

### [SWE] 2026-03-27
- TDD red phase: wrote 15 tests covering option parsing, default behavior, GFM interruption for -, *, +, ordered lists, HRs, edge cases (list at start, blank line, indented code, setext underline, unicode)
- Ran tests: compilation fails as expected (field gfm_paragraph_interruption does not exist)
- Added `gfm_paragraph_interruption: bool` field to Options struct (default false)
- Added parsing support in `parse_options_str` for `gfm_paragraph_interruption: true`
- Added GFM paragraph interruption logic in `parse_paragraph_with_lazy`: when enabled, `is_list_start(line)` and `is_horizontal_rule(line) && !is_setext_underline(line)` break paragraphs
- Note: `---` after text is a setext heading underline (h2), not an HR, even in GFM mode. Test updated to use `***` for HR case and added explicit setext test.
- Ran tests: all 15 new tests PASS
- Full suite: 2906 lib tests pass, 0 fail; all integration tests pass
- Clippy: clean (no warnings on our code)
- Fmt: clean
- Files modified: src/kramdown_parser/options.rs, src/kramdown_parser/parser.rs, src/kramdown_parser/tests.rs

### [QA] 2026-03-27
- Code review: options.rs, parser.rs changes for issue 391 look correct
  - `gfm_paragraph_interruption` field added with default false
  - Option parsing added in `parse_options_str`
  - Parser logic correctly breaks paragraphs on `is_list_start` and `is_horizontal_rule && !is_setext_underline` when enabled
  - 15 GFM-specific tests cover option parsing, default behavior, all list marker types, HR, setext edge case, unicode, blank line, indented code
- **COMPILATION FAILURE**: `src/kramdown_parser/tests.rs` contains 14 additional `tight_lists` tests (lines 2679-2863) that reference `opts.tight_lists` -- a field that does not exist on `Options`. These tests belong to issue 392 (kramdown-br-aware-list-tightening), not issue 391. The lib test binary fails to compile, so no lib tests run at all.
- Clippy: passes (clippy does not compile test code by default)
- Fmt: clean
- Acceptance criteria check:
  - Options struct field: PASS (code is correct)
  - Option parsing: PASS (code is correct)
  - Default false behavior: CANNOT VERIFY (tests don't compile)
  - GFM interruption behavior: CANNOT VERIFY (tests don't compile)
  - All existing tests pass: FAIL (compilation error)
  - cargo build: PASS (lib compiles, only test target fails)
  - clippy: PASS
  - fmt: PASS
  - DOM baseline: NOT CHECKED (blocked by test failure)
- VERDICT: **FAIL**
  - The SWE must remove the `tight_lists` tests (lines 2679-2863) from `src/kramdown_parser/tests.rs`. Those tests belong to issue 392 and should be added when that issue implements the `tight_lists` field on `Options`.
  - After removing those tests, re-run `./scripts/cargo-safe test` and confirm all tests pass.

### [QA] 2026-03-27 (re-review with both issues present)
- Both issues 391 and 392 are now implemented together; previous compilation failure resolved
- `./scripts/cargo-safe test`: all tests pass (0 failures)
- `./scripts/cargo-safe clippy -- -D warnings`: clean (only upstream lint rename warnings)
- `cargo fmt --check`: clean
- DOM baseline: 787/790 (matches required baseline exactly)
- Acceptance criteria:
  - Options struct has `gfm_paragraph_interruption: bool` field: PASS
  - `parse_options_str` parses `gfm_paragraph_interruption: true`: PASS
  - Default false preserves standard kramdown (list does NOT interrupt): PASS (test_gfm_disabled_unordered_list_does_not_interrupt, test_gfm_disabled_ordered_list_does_not_interrupt)
  - GFM mode: unordered markers (-, *, +) interrupt paragraphs: PASS (3 tests)
  - GFM mode: ordered list markers interrupt paragraphs: PASS
  - GFM mode: horizontal rules interrupt paragraphs: PASS (*** case)
  - Setext underline (---) correctly treated as h2, not HR: PASS
  - All existing kramdown tests pass unchanged: PASS
  - cargo build: PASS
  - clippy: PASS
  - fmt: PASS
  - DTC DOM baseline 787/790: PASS
- 15 issue-391-specific tests all pass, covering option parsing, default behavior, all marker types, edge cases (setext, unicode, indented code, blank lines)
- Code quality: clean, idiomatic Rust, no unwrap in library code, well-documented comments
- VERDICT: **PASS**

### [PM] 2026-03-27 -- Final Acceptance Review

Reviewed the code diff and QA report for issue 391.

**Code review summary:**
- `options.rs`: `gfm_paragraph_interruption: bool` field added to `Options` struct with default `false`. Parsing added in `parse_options_str`. Clean and minimal.
- `parser.rs`: In `parse_paragraph_with_lazy`, when `gfm_paragraph_interruption` is true, `is_list_start(line)` and `is_horizontal_rule(line) && !is_setext_underline(line)` break the paragraph loop. The setext guard is correct -- `---` after text must remain a setext h2, not an HR.
- `tests.rs`: 15 tests covering option parsing (true/false/default), default behavior (dash and ordered lists do NOT interrupt), GFM behavior (dash/asterisk/plus/ordered lists and HR DO interrupt), edge cases (setext underline, list at start, blank line, indented code, unicode).

**Acceptance criteria verification:**
- [x] `Options` struct has `gfm_paragraph_interruption: bool`, default false
- [x] `parse_options_str` parses `gfm_paragraph_interruption: true`
- [x] Default false preserves standard kramdown (lists do not interrupt paragraphs)
- [x] GFM true: unordered markers (-, *, +) interrupt paragraphs
- [x] GFM true: ordered list markers interrupt paragraphs
- [x] GFM true: horizontal rules (***) interrupt paragraphs
- [x] Setext underline (---) correctly not treated as HR even in GFM mode
- [x] All existing kramdown tests pass unchanged
- [x] cargo build: PASS
- [x] cargo clippy: PASS
- [x] cargo fmt: PASS
- [x] DTC DOM baseline 787/790: unchanged

**Tests are meaningful:** Each test validates specific behavior with clear assertions and informative failure messages. Unicode coverage included. Edge cases well covered.

**No descoping:** All acceptance criteria from the groomed spec are met. No items dropped.

VERDICT: **ACCEPT**
