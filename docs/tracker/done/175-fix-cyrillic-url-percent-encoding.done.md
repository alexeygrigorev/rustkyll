# Issue 175: Fix Cyrillic URL percent-encoding in collection/page URLs

## Problem

On alexeygrigorev/little-book-of-metals-ru, all 43 common files have URL attribute diffs because rustkyll outputs raw Cyrillic characters in `href` attributes while Jekyll percent-encodes them.

Additionally, 5 section index pages with Cyrillic collection names are missing entirely from rustkyll output.

## Root cause

Jekyll percent-encodes non-ASCII characters in URLs. For example:
- Jekyll: `href='/little-book-of-metals-ru/%D1%87%D0%B0%D1%81%D1%82%D1%8C_1_%D0%B8%D1%81%D1%82%D0%BE%D1%80%D0%B8%D1%8F/'`
- Rustkyll: `href='/little-book-of-metals-ru/часть_1_история/'`

Issue #143 (fix-url-percent-encoding) was completed but may not have covered collection URLs with Cyrillic characters generated from `page.url` or `site.pages` iteration in templates.

## Affected sites

| Site | Files affected | Diffs |
|------|---------------|-------|
| alexeygrigorev/little-book-of-metals-ru | 43/43 (all) + 5 missing | 2,130 |
| alexeygrigorev/mlwiki.org | some pages | partial (URL encoding in wiki links) |

## Acceptance criteria

- [ ] Cyrillic characters in page/collection URLs are percent-encoded in href attributes
- [ ] Section index pages with Cyrillic names are generated (5 missing files)
- [ ] little-book-of-metals-ru index.html nav links match Jekyll's percent-encoded URLs
- [ ] Existing tests continue to pass

## Dependencies

Extends issue #143 (fix-url-percent-encoding) which is already done.

## Log

### [SWE] 2026-03-17 14:00
- Root cause: `encode_url_spaces()` in `relative_url.rs` only replaced spaces with `%20` but did not encode non-ASCII characters (Cyrillic, etc.). URLs stored in `CollectionItem.url` and `Page.url` contained raw UTF-8 Cyrillic.
- Fix: Replaced `encode_url_spaces()` with `encode_url_path()` that percent-encodes all non-ASCII bytes as UTF-8 sequences (e.g., Cyrillic char -> `%D1%87`). Preserves already-encoded `%XX` sequences to prevent double-encoding.
- Added `decode_url_path()` for converting percent-encoded URLs back to filesystem paths in `url_to_output_path()`.
- Applied `encode_url_path()` at URL generation time in `collection.rs` for both collection items (line 635) and standalone pages (line 930).
- Updated `url_to_output_path()` in `generator.rs` to decode percent-encoded URLs before constructing filesystem paths.
- Tests added: 11 new tests (6 in relative_url.rs for encode/decode, 4 in collection.rs for Cyrillic pages/collections, 2 in generator.rs for percent-encoded output paths)
- Build: 1392 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: `src/template/filters/relative_url.rs`, `src/collection.rs`, `src/generator.rs`
