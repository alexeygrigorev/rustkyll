# Issue 139: Fix extra JSON-LD script tags in books and other pages

## Problem

Rustkyll emits extra `<script type="application/ld+json">` tags that Jekyll does not produce. This affects ~100 files, primarily book detail pages. Jekyll either does not emit JSON-LD for these pages, or emits it inline (without `type="application/ld+json"`).

Also, some pages have FAQ JSON-LD scripts appearing at different DOM positions (inside content div vs in head/body).

Discovered in issue #119 DOM diff audit.

## Example

Jekyll book page: no JSON-LD script tag
Rustkyll book page: `<script type="application/ld+json">{ "@context": "https://schema.org", ... }</script>`

## Acceptance criteria

- Book pages only emit JSON-LD if Jekyll also emits it
- No extra `<script>` elements in rustkyll output compared to Jekyll
- No regressions

## Log

### [SWE] 2026-03-16

- Investigation: Ran `analyze_jsonld_full.py` and `analyze_jsonld_raw.py` comparing Jekyll vs rustkyll output
- Found 0 extra JSON-LD script tags (count mismatch = 0) -- the "extra tags" issue was likely resolved by prior fixes
- Found 193 files with JSON-LD content differences, all in podcast pages
- Root cause: `yaml_to_liquid()` in `context.rs` was expanding ALL date-only strings (`YYYY-MM-DD` -> `YYYY-MM-DD 00:00:00 +0000`), not just the special `date` key
  - In Ruby YAML, all `YYYY-MM-DD` values become Date objects. Ruby's `Date#to_s` returns just `YYYY-MM-DD`
  - Jekyll only converts the special `date` frontmatter field to a Time object (renders as `YYYY-MM-DD 00:00:00 +0000`)
  - Fields like `dateadded`, `start`, etc. should remain as plain date strings
  - This caused podcast JSON-LD `datePublished` to be `"2022-02-27 00:00:00 +0000"` instead of `"2022-02-27"`
  - Also caused `uploadDate` to be `"2022-02-27 00:00:00 +0000T00:00:00Z"` instead of `"2022-02-27T00:00:00Z"`
- Fix: Modified `yaml_to_liquid()` to not expand date strings, and `yaml_mapping_to_object()` to only expand when key is exactly `"date"`
- Updated 2 tests to match new behavior
- After fix: All 193 remaining JSON-LD diffs are build-timestamp-only (dateModified, endDate) -- expected
- Build: 1462 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: `src/template/context.rs`
