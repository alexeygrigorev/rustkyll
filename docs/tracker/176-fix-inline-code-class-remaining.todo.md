# Issue 176: Fix remaining inline code class attribute differences

## Problem

Rustkyll adds `class='highlighter-rouge language-plaintext'` to inline `<code>` elements, while Jekyll only adds `class='highlighter-rouge'` (without `language-plaintext`). Issue #145 and #157 addressed this for DTC, but the fix is incomplete for other sites.

Additionally, some sites have `<div class='highlighter-rouge'>` wrappers where rustkyll adds `language-plaintext` but Jekyll does not.

## Affected sites

| Site | Occurrences | Context |
|------|------------|---------|
| mojombo-blog | 15 (across 3 posts) | `<code>` inline elements |
| muan-blog | 2 | `<code>` inline elements |
| alexeygrigorev/mlbookcamp-page | 5+ | `<div class='highlighter-rouge'>` wrappers |

## Acceptance criteria

- [ ] Inline `<code>` elements without a language specifier get `class='highlighter-rouge'` only (no `language-plaintext`)
- [ ] `<div>` code block wrappers without language get `class='highlighter-rouge'` only
- [ ] mojombo-blog blogging-like-a-hacker.html has zero extra_attribute diffs
- [ ] Existing tests continue to pass

## Dependencies

Extends issues #145 and #157 (fix-inline-code-class) which are already done.
