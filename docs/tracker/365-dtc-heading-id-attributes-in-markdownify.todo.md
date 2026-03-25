# Issue 365: DTC missing heading id attributes in markdownify output

## Parent

Follow-up from #363 (RC-D).

## Problem

When markdownify produces `<h1>` or `<h3>` headings, Jekyll/kramdown adds `id` attributes (e.g., `id='then-do-your-stuff-with-the-pos-tags'`). Rustkyll's markdownify does not generate heading IDs.

## Affected Pages

- `books/20211213-mastering-spacy.html` (1 diff) -- `<h1>` missing `id='then-do-your-stuff-with-the-pos-tags'`
- `books/20241017-build-large-language-model-from-scratch.html` (partial of 8 diffs) -- `<h3>` missing `id='user'`

## Acceptance Criteria

- [ ] Markdownify output includes `id` attributes on headings matching Jekyll/kramdown behavior
- [ ] DTC DOM match count does not regress
- [ ] No site-specific hardcoding

## Priority

LOW
