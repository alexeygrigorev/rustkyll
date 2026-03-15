# Issue 107: Fix where_exp date comparison for course/conference pages

## Problem

Course and conference pages use where_exp with date comparisons to filter tracks/events. rustkyll produces empty results where Jekyll shows content.

Affected pages from issue #93:
- /courses/2021-winter-ml-zoomcamp.html (4.12% pixel diff)
- /conferences/2021-feb.html (2.21% pixel diff)

## Acceptance criteria

- where_exp date comparisons produce same results as Jekyll
- Course page shows syllabus sections correctly
- Conference page shows "Past days" tracks
- Both pages achieve 0% pixel diff
- All existing tests pass

## Log

### [SWE] 2026-03-15

- Root cause: Non-post collection items (conferences, courses) had markdown converted to HTML BEFORE Liquid processing. This caused operators like `>=` and `<` inside Liquid expressions (e.g., `where_exp: "track", "track.date >= site.time"`) to become HTML entities (`&gt;=`, `&lt;`), which the where_exp evaluator did not recognize.
- Fix 1 (primary): In `src/generator.rs`, changed collection item rendering to use the Liquid-first-then-markdown pipeline (`render_markdown_page_with_cached_site`) for any markdown-sourced collection item containing Liquid tags. This matches Jekyll's processing order.
- Fix 2 (defense-in-depth): In `src/template/filters/where_exp.rs`, added `decode_html_entities()` to the expression evaluator so HTML-encoded operators are recognized even if they arrive through other code paths.
- Tests added: 7 new unit tests for HTML entity decoding and date string comparisons in where_exp
- Build: 1268 tests pass, 0 fail, clippy clean, fmt clean
- Verified: Conference page "Past days" section now shows all 4 tracks. Course page dates render correctly.
- Files modified: `src/generator.rs`, `src/template/filters/where_exp.rs`
