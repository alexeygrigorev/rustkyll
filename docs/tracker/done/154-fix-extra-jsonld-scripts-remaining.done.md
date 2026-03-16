# Issue 154: Fix remaining extra JSON-LD script tags (100 diffs, 99 files)

## Problem

rustkyll emits JSON-LD on 99 pages where Jekyll doesn't. These are extra <script type="application/ld+json"> blocks.

## Goal

Only emit JSON-LD on pages where Jekyll emits it. Investigate which page types get JSON-LD in Jekyll and match.

## Acceptance criteria

- JSON-LD script count matches Jekyll on every page
- No extra JSON-LD on pages Jekyll doesn't have it
- No missing JSON-LD on pages Jekyll does have it

## Log

### [SWE] 2026-03-16

**Investigation:**
- Compared JSON-LD counts: Jekyll has 675 pages with JSON-LD, rustkyll had 773 (98 extra).
- All 98 extra pages were in the `books/` directory.
- Root cause: `src/jsonld.rs` injected Book+BreadcrumbList JSON-LD for all book layout pages as a post-processing step. But Jekyll's book layout (`_layouts/book.html`) does NOT include any `<script type="application/ld+json">` block.
- Jekyll's JSON-LD comes from the layout templates themselves (author.html, podcast.html, post.html), not from post-processing. The `inject_jsonld` function was adding JSON-LD that Jekyll never emitted.

**Fix:**
- Made `inject_jsonld()` a no-op that returns HTML unchanged. Jekyll does not inject post-processing JSON-LD for any layout.
- Moved all helper functions (`generate_book_jsonld`, `build_author_person`, etc.) behind `#[cfg(test)]` since they are only used by unit tests now.
- Updated `test_inject_jsonld_book_layout_adds_jsonld` -> `test_inject_jsonld_book_layout_unchanged` to assert book pages get NO JSON-LD.
- Updated `tests/integration_jsonld.rs`: replaced 8 book JSON-LD tests (expected Book schema) with 3 tests asserting books have NO JSON-LD.
- Also fixed pre-existing compilation errors: added missing `id` field to CollectionItem test initializers across generator.rs, collection.rs, sitemap.rs, feed.rs.

**TDD:** Wrote failing test first (`test_inject_jsonld_book_layout_unchanged`), confirmed red, then fixed, confirmed green.

**Test results:** All tests pass.
- lib: 1265 passed, 0 failed
- integration_jsonld: 22 passed, 0 failed
- Full suite: all suites green, 0 failures
- Clippy: clean (no warnings)
- Format: clean

**Files modified:**
- `src/jsonld.rs` -- Made `inject_jsonld` a no-op; moved helpers behind `#[cfg(test)]`
- `tests/integration_jsonld.rs` -- Replaced book JSON-LD tests with no-JSON-LD assertions
- `src/generator.rs` -- Added missing `id: String::new()` to test CollectionItem initializers (pre-existing fix)
- `src/collection.rs` -- Same pre-existing fix
- `src/sitemap.rs` -- Same pre-existing fix
- `src/feed.rs` -- Same pre-existing fix
