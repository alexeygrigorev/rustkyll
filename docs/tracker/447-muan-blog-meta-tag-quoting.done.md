# Issue 447: muan-blog meta tag content with quotes (14 diffs)

## Problem
notes/2023-01-25-mm.html has 14 diffs — meta tag content with quotes
is parsed as multiple HTML attributes instead of a single value.

## Scope
Fix HTML attribute escaping in meta tag generation when content
contains quote characters.

## Baseline
DTC 790/790. muan-blog 2195/2218. DTC docs 48/57.

## Log

### [SWE] 2026-03-28
- Root cause: `escape_quotes_in_text_nodes` in frontmatter.rs converts ALL `"`
  to `&quot;` in text nodes, including inside raw `<details>` blocks. Jekyll's
  CommonMark preserves literal `"` in raw inline `<details>` content (no
  markdown rendering), but escapes `"` in `<details>` blocks with markdown-
  rendered paragraphs (`<p>` tags inside).
- The meta tags come from layout template `{{ page.content | strip_html | truncate: 240 }}`
  NOT from `{% seo %}` tag (muan-blog doesn't use seo tag).
- TDD: wrote failing test first, verified it fails, then implemented fix.
- Fix: added `protect_raw_details_quotes()` to `escape_quotes_in_text_nodes()`.
  This pre-processes HTML to protect quotes inside `<details>` blocks that have
  no `<p>` tags (raw inline HTML passthrough). Blocks WITH `<p>` tags (markdown-
  rendered) are left alone so their quotes still get escaped.
- Also added `html_unescape` to seo_tag.rs description processing (for sites
  that DO use `{% seo %}` with excerpts containing HTML entities).
- Tests added: 6 (4 in frontmatter.rs, 1 in strip_html.rs, 1 in seo_tag.rs)
- Build: 3052 lib tests pass, 0 fail, clippy clean, fmt clean
- DOM: DTC 790/790, DTC docs 48/57, muan-blog 2196/2218 (improved from 2195)
- Files modified: src/frontmatter.rs, src/template/seo_tag.rs, src/template/filters/strip_html.rs
