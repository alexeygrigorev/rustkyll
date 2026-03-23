# Issue 301: choosealicense remaining diffs (17/72)

## Problem

choosealicense matches 17/72 (24%). 55 pages have diffs.

## Acceptance Criteria

- [x] choosealicense DOM match improves significantly (63/72 -> 72/72)
- [x] No regressions (2688 tests pass, clippy clean, fmt clean)

## Log

### [SWE] 2026-03-23

Three fixes implemented via TDD:

**Fix 1: Block IAL forward direction**
- Wrote test_block_ial_forward_direction_from_markdown: FAILS (class applied to preceding heading)
- Root cause: pulldown-cmark doesn't preserve blank-line info in HTML output
- Solution: Added `mark_forward_ial()` markdown preprocessor that inserts `<!-- IAL:FWD -->` marker when IAL has blank lines on both sides. Updated `apply_block_ial()` to detect marker and apply forward.
- Tests: PASS (14 block IAL tests including 3 new forward-direction tests)

**Fix 2: SEO description smartify + whitespace collapse**
- Wrote test_description_smartify_apostrophe: FAILS (straight apostrophes in descriptions)
- Wrote test_description_collapse_whitespace: FAILS (double spaces not collapsed)
- Applied `smartify()` and whitespace collapsing to description after `strip_html_tags()`
- Tests: PASS

**Fix 3: JSON-LD entity encoding**
- Wrote test_jsonld_description_no_html_entities: FAILS (&#39; in JSON-LD)
- Changed `json_escape(&html_escape(desc))` to `json_escape(desc)` for JSON-LD descriptions
- Tests: PASS

Files modified:
- src/kramdown.rs (mark_forward_ial preprocessor + apply_block_ial forward direction)
- src/frontmatter.rs (call mark_forward_ial in all 3 markdown pipelines)
- src/template/seo_tag.rs (smartify descriptions, collapse whitespace, raw UTF-8 in JSON-LD)

Build: 2688 tests pass, 0 fail, clippy clean, fmt clean
DOM compare: choosealicense.com 72/72 matched (was 63/72)
