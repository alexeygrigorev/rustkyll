# Issue 144: Fix accordion script tag placement/attributes

## Problem

Some pages include `<script src='/assets/accordion.js'>` for FAQ accordion functionality. In rustkyll, the script tag is either missing, in the wrong position, or has different attributes than Jekyll's output. 9 instances across 4 files.

The issue manifests as:
- `missing_attribute: src='/assets/accordion.js'` (script exists but without src)
- `extra_attribute: type='application/ld+json'` (script has wrong type)

This suggests rustkyll is confusing the accordion script with a JSON-LD script.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Accordion script tags match Jekyll placement and attributes
- No regressions

## Log

### [SWE] 2026-03-16 14:00
- Root cause: `wrap_bare_text_in_paragraphs()` in kramdown.rs did not recognize HTML comments (`<!-- ... -->`) or `<script>` tags as block-level elements. When the faq-accordion.html include expanded, the `<!-- FAQ Accordion Component -->` comment and `<script>` tags were treated as bare text between block elements and wrapped in `<p>` tags. This shifted the DOM child ordering, causing the DOM diff tool to report mismatched attributes (the accordion.js script was matched against the JSON-LD script instead).
- Fix: Added `script` to both `CONTAINER_TAGS` and `BLOCK_TAGS` in `wrap_bare_text_in_paragraphs()`, and added HTML comment detection (`<!--`) to `is_block_line()`.
- Tests added: 7 new tests covering HTML comments, script tags, accordion include patterns, course structured data, and full pipeline.
- Verified fix on all 9 affected pages: `<p><!--` wrapping eliminated, `accordion.js` script tags present and correctly placed.
- Build: 1250 unit tests pass, 0 fail; all integration tests pass; clippy clean; fmt clean.
- Files modified: src/kramdown.rs
