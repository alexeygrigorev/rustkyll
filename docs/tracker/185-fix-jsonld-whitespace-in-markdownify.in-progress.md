# Issue 185: Fix JSON-LD FAQ/transcript whitespace in markdownify output

## Checklist Categories

This issue covers:
- **JSON-LD FAQ answer text differences** -- 8 pages
- **JSON-LD author description trailing whitespace/markdown** -- 21 pages

Both categories involve whitespace/formatting issues in JSON-LD field values.

## Problem

JSON-LD FAQ `acceptedAnswer.text` and podcast `transcript` fields have trailing whitespace and newline differences compared to Jekyll. The markdownify filter produces slightly different whitespace when its output ends up inside JSON-LD. Additionally, author description fields have trailing `\n` or unprocessed markdown links.

Sample diffs:
- Trailing space: `'<p>...fees.</p>'` vs `'<p>...fees.</p> '`
- Trailing newline in description: `'...DataTalks.Club'` vs `'...DataTalks.Club\n'`
- Markdown links not rendered: `'[Accents Welcome](https://...)'` vs rendered `'Accents Welcome'`

## Goal

Match Jekyll's whitespace handling in markdownify output when used inside JSON-LD. Ensure author descriptions are properly trimmed and markdown-processed.

## Affected Sites

- DataTalksClub/datatalksclub.github.io: ~29 pages (8 FAQ + 21 author description)

## Dependencies

None.

## Approach (TDD)

1. Write tests for markdownify output whitespace trimming in JSON-LD context
2. Write tests for author description trailing newline stripping
3. Verify tests fail
4. Fix whitespace handling in `src/template/filters/markdownify.rs` or `src/jsonld.rs`
5. Verify tests pass

## Acceptance Criteria

- [ ] JSON-LD `acceptedAnswer.text` has no trailing spaces after closing `</p>` tags
- [ ] JSON-LD author `description` has no trailing `\n` characters
- [ ] Markdown links in author descriptions are rendered to plain text (matching Jekyll's `strip_html | strip_newlines` behavior for that field)
- [ ] JSON-LD FAQ page `acceptedAnswer.text` matches Jekyll's output exactly (HTML content with no trailing whitespace)
- [ ] `cargo test` passes
- [ ] DTC FAQ and author-description pages show improved DOM match

## Test Scenarios

### Unit: Whitespace trimming (write FIRST, must fail before fix)

- **Test `test_jsonld_faq_answer_no_trailing_space`**: Create a FAQ page with an answer containing a paragraph. Render JSON-LD. Assert `acceptedAnswer.text` ends with `</p>` and has no trailing whitespace.
- **Test `test_jsonld_author_description_no_trailing_newline`**: Create a page with author description `"John is a developer\n"`. Render JSON-LD. Assert `author.description` is `"John is a developer"` (no trailing newline).
- **Test `test_jsonld_author_description_markdown_links_stripped`**: Author description contains `"Founded [Company](https://example.com)"`. Assert JSON-LD output is `"Founded Company"` (link rendered to text, HTML stripped).
- **Test `test_jsonld_faq_multiple_answers_all_trimmed`**: Multiple FAQ entries -- assert all are trimmed consistently.

### Regression: Non-JSON-LD markdownify unaffected

- **Test `test_markdownify_filter_in_template_unchanged`**: The `| markdownify` Liquid filter used in regular templates should not have its output changed (whitespace trimming is only for JSON-LD embedding).

### Integration: Output verification

- Build DTC site and inspect FAQ pages (e.g., `ai-dev-tools-zoomcamp-2025-...html`) to verify JSON-LD content.
- Verify author descriptions in blog post JSON-LD have no trailing newlines.

## Log

### [SWE] 2026-03-18 11:30
- Started implementation
- Root cause: `add_block_spacing()` in `src/kramdown.rs` was modifying content inside `<script type="application/ld+json">` blocks. It found `</p>` tags inside JSON-LD string values and added extra newlines after them, corrupting the JSON string (turning escaped `\n` into literal newlines).
- Fix: Modified `add_block_spacing()` to detect and skip `<script>` blocks entirely, preserving their content verbatim.
- Tests added: 10 new tests across 2 files
  - `src/template/engine.rs`: 7 tests (FAQ answer no trailing space, multi-paragraph valid JSON, multiple answers trimmed, survives markdown pipeline, author description no trailing newline, author description markdown links stripped, markdownify filter unchanged)
  - `src/template/filters/markdownify.rs`: 3 tests (multi-paragraph output, script block preservation, indented script block preservation)
- Verified: All 9 FAQ pages now produce valid JSON-LD. No trailing whitespace in `acceptedAnswer.text`. Author page descriptions have no trailing `\n`.
- Build: 1491 lib tests + integration tests pass, 0 fail, clippy clean, fmt clean
- Files modified: `src/kramdown.rs`, `src/template/engine.rs`, `src/template/filters/markdownify.rs`
- Note: Blog post layout `author.content | strip_html | jsonify` (without `strip_newlines`) produces trailing `\n` in both Jekyll and Rust -- this matches Jekyll behavior and is by design (the template doesn't strip newlines). No markdown link issues found in current output.
