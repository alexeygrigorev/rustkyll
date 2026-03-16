# Issue 152: Fix kramdown paragraph wrapping cascade (~1184 diffs)

## Problem

When a structural diff exists (missing/extra <p> tag), all downstream text nodes appear shifted in the DOM comparison, creating cascade diffs. ~1184 diffs are secondary effects.

Also: Jekyll wraps content in <p> inside <li>, <figcaption>, <blockquote> where rustkyll doesn't in some cases.

## Goal

Fix the remaining paragraph wrapping mismatches. The cascade diffs will resolve automatically.

## Acceptance criteria

- Paragraph wrapping matches Jekyll for li, figcaption, blockquote
- Cascade diffs eliminated
- DOM diff count drops significantly

## Log

### [SWE] 2026-03-16

**Root cause analysis:**
1. **Figcaption `<p>` stripping (~194 diffs):** `figcaption` was in `STRIP_P_PARENT_TAGS`, causing `strip_paragraphs_in_html_blocks` to remove `<p>` tags inside `<figcaption>`. Jekyll/kramdown preserves these. Additionally, when processing `<figure>` (which IS in `STRIP_P_PARENT_TAGS`), `maybe_strip_p_tags` was stripping `<p>` inside nested `<figcaption>` elements.
2. **Blockquote indentation (~53 diffs):** Jekyll/kramdown indents content inside `<blockquote>` with 2 spaces. rustkyll did not indent at all.

**Fixes applied:**
1. Removed `figcaption` from `STRIP_P_PARENT_TAGS` so `<p>` tags inside standalone `<figcaption>` are preserved.
2. Added figcaption-awareness to `maybe_strip_p_tags`: when stripping `<p>` inside `<figure>`, it now skips `<p>` tags that are inside a nested `<figcaption>`.
3. Added `indent_blockquote_content` function that indents tag-start lines inside `<blockquote>` by 2 spaces. Only lines starting with `<` get indented; continuation text lines within multi-line `<p>` content are left as-is (matching Jekyll behavior).
4. Fixed pre-existing compile errors: added missing `id` field to `CollectionItem` test initializers in feed.rs, generator.rs, pagination.rs, sitemap.rs. Removed erroneous `id` field from `Page` initializers in pagination.rs (Page struct does not have `id`).

**Results after fix:**
- Figcaption `<p>` missing: 0 (was 194) -- fully fixed
- Blockquote diffs: 2 (was 53) -- remaining 2 are smart quote (curly quote U+201C vs U+201D) differences, unrelated to paragraph wrapping
- Total lines with `<p>` diffs: 4088 (was 4263) -- reduction of 175 direct diffs, with cascade reduction as well

**Tests:** 5 new tests added, all passing. 1265 total lib tests pass, 0 fail.
- `test_issue152_figcaption_preserves_p_in_figure`
- `test_issue152_figcaption_p_preserved_in_postprocess`
- `test_issue152_figcaption_without_p_unchanged`
- `test_issue152_blockquote_content_indented`
- `test_issue152_blockquote_multi_paragraph_indented`

**Build:** clippy clean, fmt clean.

**Files modified:**
- `src/kramdown.rs` -- removed figcaption from STRIP_P_PARENT_TAGS, added figcaption guard in maybe_strip_p_tags, added indent_blockquote_content function, added 5 tests
- `src/feed.rs` -- added missing `id` field in test CollectionItem
- `src/generator.rs` -- added missing `id` field in test CollectionItems
- `src/pagination.rs` -- added missing `id` field in test CollectionItem, removed erroneous `id` from Page initializers
- `src/sitemap.rs` -- added missing `id` field in test CollectionItem
- `src/collection.rs` -- fmt auto-fix (pre-existing formatting issue)
