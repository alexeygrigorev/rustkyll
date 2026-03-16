# Issue 156: Fix figcaption <p> wrapping (196 diffs)

## Problem

Jekyll wraps text inside `<figcaption>` in `<p>` tags. Rustkyll doesn't in some cases. 196 DOM diffs.

Note: Issue #152 partially fixed this but some cases remain.

## Acceptance criteria

- All figcaption content matches Jekyll's <p> wrapping
- 196 DOM diffs eliminated
- TDD: failing test, fix, test passes

## Log

### [SWE] 2026-03-16

**Investigation:**

Built the DTC site with both Jekyll and rustkyll and compared all 348 figcaption elements across 787 HTML files. Result: 0 diffs. All figcaption content already matches between Jekyll and rustkyll byte-for-byte.

**Root cause analysis:**

The issue description was based on stale data. Issue #152 fully fixed figcaption `<p>` wrapping. The key mechanism:
- `figcaption` is in `STRIP_P_PARENT_TAGS`, so `strip_p_in_tag` strips auto-generated `<p>` tags that pulldown-cmark inserts inside `<figcaption>`.
- Jekyll/kramdown does NOT wrap figcaption text in `<p>` tags (verified: all 348 figcaptions in DTC Jekyll output have zero `<p>` tags).
- The stripping behavior is correct and produces output matching Jekyll.

Initially attempted removing `figcaption` from `STRIP_P_PARENT_TAGS` (thinking Jekyll preserves `<p>`), but this introduced 194 regressions -- confirming that stripping is the correct behavior.

**What was done:**
- Verified figcaption output matches Jekyll (0 diffs across 348 elements)
- Added 5 unit tests confirming the correct `<p>` stripping behavior for figcaption
- Tests cover: standalone figcaption, figure+figcaption combo, postprocess end-to-end, no-p passthrough, and figcaption with inline links

**Tests:** 5 new tests added, all passing. 1297 total lib tests pass, 0 fail.
- `test_issue156_figcaption_p_tags_stripped`
- `test_issue156_figure_with_figcaption_strips_all_p`
- `test_issue156_figcaption_p_stripped_in_postprocess`
- `test_issue156_figcaption_without_p_unchanged`
- `test_issue156_figcaption_with_link_p_stripped`

**Build:** clippy clean, fmt clean. All integration tests pass.

**Files modified:**
- `src/kramdown.rs` -- added 5 tests confirming figcaption `<p>` stripping behavior

**Note:** The 196 DOM diffs mentioned in the issue title are not figcaption-related. The actual remaining DOM diffs are: 193 `text_differs` in `<script>` tags (JSON-LD content differences) and 215 `attribute_differs` (code element class attributes). These are separate issues.
