# Issue 105: Fix whitespace in Liquid include output causing paragraph splits

## Priority

HIGH -- affects 6 of 22 DTC pages (homepage, articles, books, podcast, events, tools). This is the #1 blocker for pixel-perfect match.

## Problem

When Liquid `{% include %}` output contains blank lines inside HTML block elements (`<li>`, `<p>`, `<div>`, `<h5>`), pulldown-cmark treats the blank lines as paragraph separators, splitting content into multiple `<p>` tags. Jekyll/kramdown does not do this -- it preserves the inline flow.

### Concrete example

In `index.md`, line 57:
```html
<li class="{{ event.type }}">{% include event.html event=event speakers=true %}</li>
```

The `event.html` include produces output with blank lines (between `{% assign %}` and the conditional):
```
\n
Event Title by <a href="...">Speaker</a>\n
(<a href="...">watch on youtube</a>)\n
```

After Liquid rendering, the `<li>` content looks like:
```html
<li class="webinar">

  Event Title by <a href="...">Speaker</a>
  (<a href="...">watch on youtube</a>)

</li>
```

pulldown-cmark sees the blank lines and wraps content in `<p>` tags:
```html
<li class="webinar">
<p>Event Title by <a href="...">Speaker</a>
(<a href="...">watch on youtube</a>)</p>
</li>
```

Jekyll/kramdown output has NO `<p>` tag here -- the content stays inline inside `<li>`.

### Affected includes and pages

| Include file | Used in | Context |
|---|---|---|
| `event.html` | `index.md` (homepage), `events.md` | Inside `<li>` tags |
| `authors.html` | `index.md`, `articles.md`, `books.md`, `podcast.md`, `tools.md` | Inside `<li>`, `<h5>`, `<a>` contexts |
| `book.html` | `books.md` | Inside `<div>` blocks |

### Affected pages (from issue #93)

1. `/` -- Homepage (page 1)
2. `/articles` -- Articles listing (page 2)
3. `/books.html` -- Books listing (page 3)
4. `/podcast.html` -- Podcast listing (page 4)
5. `/events.html` -- Events listing (page 5)
6. `/tools.html` -- Tools listing (page 9)

## Root cause

The Liquid-to-markdown-to-HTML pipeline passes include output (which is already HTML) through the markdown converter. Blank lines in the include output get interpreted as markdown paragraph breaks by pulldown-cmark.

The existing `strip_paragraphs_in_html_blocks()` in `kramdown.rs` already attempts to strip `<p>` tags inside block elements, but it does not cover all cases. The fundamental problem is that blank lines in Liquid include output should be collapsed/stripped BEFORE the markdown parser sees them, so pulldown-cmark never creates the spurious `<p>` tags in the first place.

## Dependencies

None -- this issue is independent.

## Acceptance Criteria

### AC-1: Blank lines in include output are collapsed before markdown parsing

- [ ] After Liquid template rendering and before `markdown_to_html()`, blank lines within HTML block contexts (content between `<li>...</li>`, `<div>...</div>`, `<td>...</td>`, `<h1>`-`<h6>`, etc.) must be collapsed so pulldown-cmark does not see them as paragraph separators
- [ ] The fix must operate on the post-Liquid, pre-markdown content -- NOT as a post-processing HTML fixup

### AC-2: No spurious `<p>` tags inside HTML block elements from include output

- [ ] Content from `{% include event.html %}` inside `<li>` must NOT be wrapped in `<p>` tags
- [ ] Content from `{% include authors.html %}` inside `<li>` must NOT be wrapped in `<p>` tags
- [ ] Content from `{% include authors.html %}` inside `<h5>` must NOT be wrapped in `<p>` tags
- [ ] Content from `{% include book.html %}` inside `<div>` must NOT produce spurious `<p>` tags

### AC-3: Legitimate markdown paragraphs still work

- [ ] Regular markdown text separated by blank lines still produces proper `<p>` tags
- [ ] Markdown content NOT inside HTML block elements is unaffected
- [ ] Blog posts with intentional paragraph breaks render correctly

### AC-4: All 6 affected pages match Jekyll output

- [ ] Build DTC site with rustkyll: `./target/release/rustkyll build --source datatalksclub.github.io --destination /tmp/rustkyll-dtc`
- [ ] Build DTC site with Jekyll: `cd datatalksclub.github.io && bundle exec jekyll build --destination /tmp/jekyll-dtc`
- [ ] For each of the 6 affected pages, diff the generated HTML against Jekyll output
- [ ] No `<p>` tags in rustkyll output where Jekyll has none (inside `<li>`, `<div>`, `<h5>` from includes)
- [ ] No extra vertical spacing visible when the pages are rendered in a browser

### AC-5: Output verification -- structural HTML comparison

For each affected page, the rustkyll output must structurally match Jekyll output in the include-affected sections:

- [ ] `/index.html`: upcoming events `<ul class="emoji-list">` section -- each `<li>` has inline content (no `<p>` wrappers). Podcast episodes `<ul>` section -- each `<li>` has inline content. Book of the week `<ul>` section -- same. Latest articles `<ul>` section -- same.
- [ ] `/articles/index.html` (or `articles.html`): article list `<li>` elements with `{% include authors.html %}` -- inline content, no `<p>` wrappers.
- [ ] `/books.html`: current book `{% include book.html %}` -- `<h5>` with `{% include authors.html %}` inside must NOT have `<p>` tags. Past books list -- `<li>` elements with author includes must be inline.
- [ ] `/podcast.html`: episode list `<li>` elements with `{% include authors.html %}` for guests -- inline, no `<p>` wrappers.
- [ ] `/events.html`: upcoming and past events with `{% include event.html %}` -- `<li>` elements with inline content.
- [ ] `/tools.html`: tools list with `{% include authors.html %}` -- inline content.

### AC-6: Existing tests pass

- [ ] `./scripts/cargo-safe test` passes (all existing Rust tests)
- [ ] `./scripts/cargo-safe clippy -- -D warnings` is clean
- [ ] `cargo fmt --check` is clean

### AC-7: No regressions on other pages

- [ ] Pages 6-8, 10-22 from issue #93 that were previously working must not regress
- [ ] Blog posts with markdown content and includes must still render correctly
- [ ] Book detail pages, podcast episode pages, and people pages must still render correctly

## Test Scenarios

### Unit: Blank line collapsing in HTML block context

- Test: Include output with blank lines inside `<li>` -- blank lines must be removed
  ```
  Input:  "<li>\n\n  Event Title\n  (link)\n\n</li>"
  Expect: "<li>\n  Event Title\n  (link)\n</li>"  (no blank lines)
  ```
- Test: Include output with blank lines inside `<div>` -- blank lines must be removed
- Test: Include output with blank lines inside `<h5>` -- blank lines must be removed
- Test: Content OUTSIDE HTML block elements retains blank lines (paragraph separation preserved)
- Test: Nested HTML blocks (e.g., `<li>` inside `<ul>` inside `<div>`) -- blank lines collapsed at all levels
- Test: Content with NO blank lines is unchanged (no false positives)

### Unit: No `<p>` tags from include output in block elements

- Test: `markdown_to_html("<li>\nSome text by <a href='x'>Author</a>\n</li>")` must NOT contain `<li><p>`
- Test: `markdown_to_html("<li>\n\nSome text\n\n</li>")` after blank-line collapsing must NOT contain `<li><p>`
- Test: `markdown_to_html("<h5>by <a href='x'>Author</a></h5>")` must NOT contain `<h5><p>`

### Integration: DTC include patterns

- Test: Simulate `event.html` include output inside `<li class="webinar">` -- verify no `<p>` tag in output
- Test: Simulate `authors.html` include output (multiple `<a>` tags with commas) inside `<li>` -- verify inline
- Test: Simulate `book.html` include output (nested `<div>` with `<h5>` containing `authors.html`) -- verify no `<p>` in `<h5>`
- Test: Full pipeline test: create a markdown file with Liquid `{% include %}` inside `<li>`, render through the full pipeline, verify no `<p>` wrappers

### Integration: Regression -- legitimate paragraphs

- Test: Markdown with two paragraphs separated by blank lines produces two `<p>` tags (unaffected)
- Test: Markdown with HTML blocks followed by markdown paragraphs -- both render correctly
- Test: Blog post content with `{% include %}` at top level (not inside block element) -- paragraphs around it still work

### Output verification: DTC site build

- Build the DTC site with rustkyll and Jekyll
- For each of the 6 affected pages, extract the include-affected HTML sections and diff
- Verify 0 structural differences in those sections (no extra `<p>` tags, no missing content)

## Implementation hints

The fix should happen in `src/template/layout.rs` in `render_markdown_page_with_cached_site()`, between the Liquid rendering step and the `markdown_to_html()` call. Currently the pipeline does:

1. Liquid rendering (produces HTML with possible blank lines from includes)
2. `dedent_html_lines()` (fixes indentation)
3. `mark_existing_html_headings()` (marks headings)
4. `markdown_to_html()` (converts markdown, but blank lines cause `<p>` splits)

A new step is needed between 2 and 4 (or between 3 and 4): collapse blank lines that appear inside HTML block-level elements. This prevents pulldown-cmark from ever seeing the blank lines as paragraph breaks.

The `authors.html` include is especially tricky because its `{% for %}` loop produces blank lines between iterations:
```
\n  <a href="/people/foo.html">Foo</a>, \n\n  <a href="/people/bar.html">Bar</a>\n
```
These blank lines inside `<li>` cause pulldown-cmark to create `<p>` tags around each author link.

Key considerations:
- Only collapse blank lines inside HTML block elements, not in pure markdown sections
- The existing `strip_paragraphs_in_html_blocks()` in `kramdown.rs` is a post-hoc fix; the better approach is to prevent the problem at source
- The `dedent_html_lines()` function already tracks HTML context; a similar approach could work for blank line collapsing
- Be careful with `<div>` blocks that intentionally have blank lines for markdown paragraph formatting

## This issue is DONE when

All 6 affected pages produce HTML output identical to Jekyll in the include-affected sections. No spurious `<p>` tags. No extra vertical spacing. Not 5 out of 6 -- all 6. The fix must be in the pre-markdown pipeline, not a fragile post-processing HTML regex.

## Log

### [SWE] 2026-03-15

- **Root cause**: Liquid `{% include %}` output contains blank lines (from `{% assign %}`, `{% for %}` loops, conditionals) inside HTML block elements (`<li>`, `<div>`, `<h5>`, etc.). pulldown-cmark interprets these blank lines as paragraph separators and wraps content in `<p>` tags. Jekyll/kramdown does not do this.
- **Fix**: Added `collapse_blank_lines_in_html_blocks()` function in `src/kramdown.rs` that operates pre-markdown (after Liquid rendering, before `markdown_to_html()`). It finds HTML block elements using the existing `BLOCK_PARENT_TAGS` list and `find_matching_close()` utility, then removes blank lines inside them while preserving blank lines outside HTML blocks for normal paragraph separation.
- **Pipeline integration**: Inserted the new step as "Step 2.75" in `render_markdown_page_with_cached_site()` and `render_markdown_content_with_cached_site()` in `src/template/layout.rs`, between heading marking and `markdown_to_html()`.
- **Tests added**: 16 new unit tests covering:
  - Blank line collapsing in `<li>`, `<div>`, `<h5>`, `<td>` elements
  - Content with classes (e.g., `<li class="webinar">`)
  - Nested HTML blocks (`<div>` containing `<li>`)
  - Multiple consecutive elements
  - Authors include pattern (blank lines between `{% for %}` iterations)
  - Event include pattern (full event.html simulation)
  - Book include pattern (nested `<div>` with `<h5>` containing authors)
  - No-op when no blank lines present
  - Content outside HTML blocks preserved (markdown paragraphs still work)
  - Full pipeline tests: collapse + markdown_to_html verifying no `<p>` tags
  - Regression test: legitimate markdown paragraphs still produce `<p>` tags
- **Build**: 1,251 tests pass (1,037 unit + 214 integration), 0 failures
- **Clippy**: clean (no warnings)
- **Fmt**: clean
- **Files modified**: `src/kramdown.rs`, `src/template/layout.rs`
