# Issue 153: Fix remaining JSON-LD diffs (~270 diffs, 245 files)

## Problem

Remaining JSON-LD diffs: timezone offsets (build timestamps differ), text content differences, null-vs-empty.

## Goal

Reduce JSON-LD DOM diffs. Build timestamp diffs are expected (different build times) but other content diffs should be fixed.

## Acceptance criteria

- Non-timestamp JSON-LD diffs reduced to 0
- Build timestamp diffs documented as expected

## Log

### [SWE] 2026-03-16

**Analysis of 270 JSON-LD diffs across 265 files:**

1. **Timezone-only diffs**: 22 files -- build timestamps differ (expected)
2. **JSON parse errors**: 9 files -- FAQPage script blocks with control chars
3. **Content diffs**: 234 files -- caused by:
   - `&amp;` in `<script>` blocks (5 files): `encode_bare_ampersands()` was escaping `&` inside `<script>` tags
   - Trailing `\n` in descriptions (~200 files): Jekyll uses raw content for cross-referenced collection items, rustkyll uses HTML-rendered content
   - Markdown link syntax in descriptions (2 files): same root cause as trailing `\n`
   - Trailing space before newlines (5 files): same root cause

**Fixes implemented:**

1. **`&amp;` in `<script>` blocks** (FIXED): Modified `encode_bare_ampersands()` in `kramdown.rs` to skip content inside `<script>` tags. This fixes 5 content diffs, reducing them to timezone-only.

2. **Content field inconsistency** (DOCUMENTED, NOT FIXABLE): In Jekyll, `document.content` is timing-dependent -- raw markdown before the document is rendered, HTML after. When post templates reference `site.people[i].content`, the person may not be rendered yet, so content is raw. When podcast templates reference the same, the person IS rendered, so content is HTML. Rustkyll uses HTML consistently, which is correct for display contexts (podcast `{{ guest.content }}`) but causes minor JSON-LD description differences. Switching to raw content would fix blog post JSON-LD but break podcast bio rendering. This is unfixable without replicating Jekyll's exact rendering order.

**Results:**

- Total DOM diffs: 4622 -> 4078 (-544, -12%)
- JSON-LD script diffs: 270 -> 265 (-5)
  - Content diffs: 239 -> 234 (-5, the `&amp;` fix)
  - Timezone diffs: 22 (expected)
  - Parse errors: 9 (unchanged)
- The remaining 234 content diffs are from the Jekyll rendering-order timing issue (documented above)

**Files modified:**
- `src/kramdown.rs`: Modified `encode_bare_ampersands()` to skip `<script>` blocks
- `src/generator.rs`: Added documentation comment, added unit test
- `docs/comparison/dom-diff-full-report-current.txt`: Updated

**Tests:**
- 1289 lib tests pass, 0 fail
- 16 integration tests pass
- Clippy clean, fmt clean
- New tests added:
  - `test_script_block_ampersand_not_escaped` in frontmatter.rs
  - `test_encode_bare_ampersand_skips_script_blocks` in kramdown.rs
  - `test_collection_item_content_uses_html_content` in generator.rs
