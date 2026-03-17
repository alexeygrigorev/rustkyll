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

## Log

### [SWE] 2026-03-17

- **Root cause**: `add_inline_code_classes()` in `kramdown.rs` was a post-processing
  step that added `class="language-plaintext highlighter-rouge"` to ALL `<code>` tags
  without a class attribute. This is incorrect for `<code>` tags that appear as raw
  HTML in the markdown source (e.g., mojombo-blog's the-git-parable.md). Jekyll/kramdown
  only adds the class to `<code>` generated from backtick markdown syntax.

- **Fix**: Moved inline code class addition from post-processing (`kramdown::postprocess`)
  to markdown rendering (`frontmatter::add_inline_code_class_to_events`). This function
  intercepts pulldown-cmark `Event::Code` events (backtick-generated) and emits them
  as `InlineHtml` with the class. Raw HTML `<code>` tags pass through untouched.

- **Files modified**:
  - `src/frontmatter.rs`: Added `add_inline_code_class_to_events()` and `html_escape_for_code()`;
    modified `markdown_to_html()` and `markdown_to_html_for_filter()` to use event transformation;
    added 5 new tests for issue 176
  - `src/kramdown.rs`: Removed `add_inline_code_classes()` function and its call from
    `postprocess()` and `postprocess_for_filter()`; updated 7 existing tests and doc comments

- **Verification**:
  - mojombo-blog: Raw HTML `<code>working</code>` passes through without class (matches Jekyll)
  - mojombo-blog: Backtick `@private` still gets `language-plaintext highlighter-rouge` class (matches Jekyll)
  - DTC site: All backtick inline code still gets the class correctly (matches Jekyll)

- **Tests**: 1419 passed, 0 failed, clippy clean, fmt clean
  - 5 new tests: backtick code gets class, raw HTML code does not, mixed, markdownify, special chars

- **Note on acceptance criteria**: The issue describes the problem as "rustkyll adds
  language-plaintext but Jekyll doesn't." Investigation revealed the actual behavior:
  Jekyll adds the class to backtick-generated code but NOT to raw HTML `<code>` in the
  source. The fix correctly handles both cases. The acceptance criteria mention
  `class='highlighter-rouge'` only (no language-plaintext), but for backtick code Jekyll
  actually outputs `language-plaintext highlighter-rouge`. The real issue was raw HTML
  `<code>` getting classes it shouldn't.
