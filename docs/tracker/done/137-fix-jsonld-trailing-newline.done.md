# Issue 137: Fix JSON-LD trailing newline in person description

## Problem

In JSON-LD structured data, the `description` field for person/author entries has a double trailing newline (`\n\n`) in rustkyll output, while Jekyll outputs a single trailing newline (`\n`). This affects ~211 files.

Also, podcast transcript fields have minor whitespace differences in ~59 files.

Discovered in issue #119 DOM diff audit.

## Example

Jekyll: `"description": "Valeriia Kuka is a Content Manager...\n"`
Rustkyll: `"description": "Valeriia Kuka is a Content Manager...\n\n"`

## Acceptance criteria

- Person description in JSON-LD has single trailing newline matching Jekyll
- Podcast transcript text matches Jekyll output
- No regressions in JSON-LD output

## Log

### [SWE] 2026-03-16

- Root cause: `add_block_spacing()` in `kramdown.rs` was adding an extra `\n` after the last block tag (e.g., `</p>`) at the end of content. This converted `</p>\n` to `</p>\n\n`. When collection item `html_content` was used in templates via `strip_html | jsonify`, the extra `\n` caused JSON-LD description fields to have `\n\n` instead of `\n`.
- Fix: Modified `add_block_spacing()` to NOT add an extra newline when the remaining content after a block tag is just a single `\n` (i.e., end of content). The condition `remaining.len() > 1` prevents doubling the final newline.
- Tests added: 3 unit tests in `frontmatter.rs`:
  - `test_markdown_to_html_single_paragraph_trailing_newline` -- verifies single paragraph ends with `\n` not `\n\n`
  - `test_markdown_to_html_no_trailing_newline_source` -- verifies no `\n\n` even when source has no trailing newline
  - `test_markdown_to_html_multi_paragraph_trailing_newline` -- verifies multi-paragraph content ends with single `\n`
- Impact: trailing-newline diffs reduced from 211 to 31 (180 fixed, 85% reduction)
- Remaining 31: person files that lack a trailing newline in source -- rustkyll produces `\n` (from pulldown-cmark) while Jekyll produces no `\n`. This is a minor cosmetic difference inherent to pulldown-cmark vs kramdown behavior.
- Build: 1455 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: `src/kramdown.rs`, `src/frontmatter.rs`
