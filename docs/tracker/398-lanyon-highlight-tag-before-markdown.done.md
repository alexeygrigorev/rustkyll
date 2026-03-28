# Issue 398: Lanyon — {% highlight %} Liquid tags processed after markdown conversion

## Problem

Lanyon site is at 5/6 DOM match. The remaining page (`index.html`) has 19 diffs
because `{% highlight %}...{% endhighlight %}` Liquid block tags are passed to
the markdown processor as plain text, wrapped in `<p>` tags, and never executed.

Jekyll processes Liquid tags BEFORE markdown conversion. Rustkyll currently converts
markdown to HTML first in `src/collection.rs` (line ~771), then processes Liquid later.

## Root Cause

`src/collection.rs` line 771-778: `markdown_to_html_with_options()` is called on raw
markdown that still contains Liquid tags. The tags become plain text in `<p>` tags.

## Scope

Ensure Liquid tags in collection item content are processed before markdown conversion,
matching Jekyll's order of operations. This affects any site using `{% highlight %}`
or other Liquid block tags in markdown content.

## Affected Sites

- lanyon (5/6 → target 6/6)
- Potentially any site with Liquid tags in markdown posts

## Log

### [SWE] 2026-03-28

- Wrote test `test_collection_item_html_content_processes_highlight` -- FAILS as expected: html_content has raw `{% highlight %}` tags wrapped in `<p>` tags
- Root cause: `collection.rs` pre-computes `html_content` by running markdown conversion BEFORE Liquid processing. Individual post pages are fine (generator.rs already routes them through Liquid-first), but `{{ post.content }}` on index/listing pages uses the pre-computed `html_content`
- Implemented `pre_render_highlight_blocks()` in `collection.rs` -- processes `{% highlight lang %}...{% endhighlight %}` blocks into `<figure>` HTML before markdown conversion, using the same syntect highlighting as the Liquid tag
- Applied to both collection items and standalone pages in `load_pages()`
- First attempt: blank lines inside syntect output caused markdown parser to insert `<p>` tags. Added blank-line collapsing (`\n\n` -> `\n`)
- Second issue: lanyon files use CRLF line endings. Blank line pattern was `\r\n\r\n`, not `\n\n`. Added CRLF collapsing too
- Tests: TDD cycle verified -- test_lanyon_example_content_html_content FAILS before fix, PASSES after
- DOM: lanyon 5/6 -> 6/6, DTC stays at 790/790
- Build: 3037 lib tests + all integration tests pass, 0 failures, clippy clean, fmt clean
- Files modified: `src/collection.rs`
