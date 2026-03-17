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
