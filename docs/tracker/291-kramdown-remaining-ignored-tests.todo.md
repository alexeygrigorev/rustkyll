# Issue 291: Fix 11 remaining ignored kramdown conformance tests

## Problem

Kramdown conformance reaches 643/643 (100%) on non-ignored tests. However, 15 tests are marked `#[ignore]`, of which 11 still fail. These are edge cases deferred during the kramdown rewrite (issues 281c, 281d, 282, 287, 288, 289).

## Failing ignored tests (11)

| Test | Category | What it tests |
|------|----------|---------------|
| `block_03_paragraph_standalone_image` | Paragraph | Standalone images in paragraphs get special wrapping |
| `block_03_paragraph_with_html_to_native` | Paragraph | html_to_native conversion for paragraph content |
| `block_04_header_with_auto_ids` | Headers | Auto-generated IDs on headers |
| `block_04_header_with_auto_id_prefix` | Headers | Auto ID with custom prefix |
| `block_04_header_with_auto_id_stripping` | Headers | Auto ID stripping of special characters |
| `block_04_header_with_header_links` | Headers | Header links (anchor links on headers) |
| `block_06_codeblock_whitespace` | Code blocks | Whitespace handling in code blocks |
| `block_06_codeblock_guess_lang_css_class` | Code blocks | Guessing language from CSS class |
| `block_06_codeblock_rouge_multiple` | Code blocks | Multiple Rouge-highlighted code blocks |
| `block_06_codeblock_rouge_simple` | Code blocks | Simple Rouge highlighting in code blocks |
| `block_14_table_errors` | Tables | Error handling in malformed tables |

## Passing ignored tests (4, can be un-ignored)

| Test | Category |
|------|----------|
| `block_03_paragraph_line_break_last_line` | Paragraph |
| `block_14_table_table_with_footnote` | Tables |
| `block_06_codeblock_highlighting` | Code blocks |
| `block_06_codeblock_highlighting_opts` | Code blocks |

## Dependencies

None -- kramdown 643/643 is stable. These are incremental improvements.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions)
- [ ] At least 8 of the 11 failing ignored tests pass
- [ ] The 4 already-passing ignored tests are un-ignored (so they run in the normal suite)
- [ ] Any tests that cannot be fixed are documented with root cause
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean
