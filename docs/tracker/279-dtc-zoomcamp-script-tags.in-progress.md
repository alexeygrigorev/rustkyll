# Issue 279: Standalone HTML comments not wrapped in p tags (zoomcamp script tag cascade)

## Problem

Four DTC zoomcamp blog pages show a 3-diff cascade pattern in DOM comparison:

```
child[N]:   tag_name_differs - expected: 'p', actual: 'div'
child[N+1]: tag_name_differs - expected: 'div', actual: 'script'
script:     missing_element  - expected: '<script>', actual: '(none)'
```

Affected pages:
- `blog/ai-dev-tools-zoomcamp-2025-free-course-to-master-coding-assistants-agents-and-automation.html`
- `blog/data-engineering-zoomcamp.html`
- `blog/llm-zoomcamp.html`
- `blog/machine-learning-zoomcamp.html`

The `<script>` tags themselves are NOT missing. The issue is an off-by-one shift in child element positions caused by a missing `<p>` wrapper on an HTML comment.

## Root Cause

All four pages use `{% include related-posts.html manual_posts=page.related_posts %}` near the end of the post. When Liquid processes the include, it outputs lines like:

```
<!-- Get related posts -->
  <!-- Use manually specified posts -->
<!-- Limit to max_related posts -->
```

These are standalone HTML comments surrounded by blank lines (from Liquid control flow whitespace).

**Jekyll/kramdown behavior:** kramdown treats standalone HTML comments that appear between blank lines as inline content and wraps them in `<p>` tags:

```html
<p><!-- Use manually specified posts --></p>
```

**Rustkyll/pulldown-cmark behavior:** CommonMark treats HTML comments as HTML block elements (type 2 in the CommonMark spec), so they pass through as-is without `<p>` wrapping:

```html
  <!-- Use manually specified posts -->
```

This causes rustkyll output to have one fewer element at this position. Everything after shifts up by one, producing the 3-diff cascade: the `<div>` for related posts appears where the `<p>` was expected, the `<script>` for structured data appears where the `<div>` was expected, and the final `<script>` appears to be missing.

## Scope

This affects not just the 4 zoomcamp pages but at least 7 pages total (confirmed via `grep -rn '<p><!--' _site_jekyll/blog/`):

- 5 pages with `<!-- Use manually specified posts -->` (the 4 zoomcamp pages + mlops-zoomcamp)
- 2 pages with `<!-- Auto-generate based on tags -->`, `<!-- Find posts with matching tags -->`, `<!-- Sort by date -->` (slack-communities, free-machine-learning-courses)

All originate from `_includes/related-posts.html` template comments.

## Technical Details

The rendering pipeline in `layout.rs` `render_markdown_page_with_cached_site()` is:

1. Liquid processing (includes expanded)
2. `dedent_html_lines()` -- unindents HTML
3. `mark_existing_html_headings()` -- protects headings from ID generation
4. `collapse_blank_lines_in_html_blocks()` -- collapses blank lines WITHIN block tags
5. `markdown_to_html_with_options()` -- pulldown-cmark converts to HTML

The fix belongs in the kramdown postprocessing layer (step 5 or a new step between 4 and 5). The approach should be one of:

**Option A (recommended): Pre-markdown transform.** Before passing to pulldown-cmark, detect standalone HTML comments (lines containing only `<!-- ... -->` surrounded by blank lines) and wrap them in `<p>` tags. This matches what kramdown would do.

**Option B: Post-markdown transform.** After pulldown-cmark rendering, find bare HTML comments at the top level (not inside any element) and wrap them in `<p>` tags.

**Option C: Strip include-generated comments entirely.** Since these are debugging/documentation comments that carry no semantic value, they could be stripped during include expansion. However, this would diverge from Jekyll's output and might affect other comparison diffs.

Option A is preferred because it operates at the same pipeline stage as other kramdown-compatibility transforms.

## Dependencies

None. This is an independent fix in the kramdown/markdown processing pipeline.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests plus new tests for this issue
- [ ] For all 4 affected zoomcamp pages, the 3-diff cascade (`p` vs `div`, `div` vs `script`, missing `script`) is eliminated
- [ ] Standalone HTML comments between blank lines in markdown content are wrapped in `<p>` tags, matching Jekyll/kramdown behavior
- [ ] HTML comments that are INSIDE block-level elements (e.g., inside `<div>`, `<table>`) are NOT affected by this change
- [ ] The fix applies generically (not hardcoded to specific pages or comment text)
- [ ] Building the DTC site and comparing output for `blog/data-engineering-zoomcamp.html` shows the `<p><!-- Use manually specified posts --></p>` line is present in rustkyll output
- [ ] The mlops-zoomcamp page (`blog/mlops-zoomcamp.html`) also has the comment wrapped in `<p>` tags

## Test Scenarios

### Unit: HTML comment wrapping

- Input: standalone `<!-- comment -->` between blank lines -> output wraps in `<p><!-- comment --></p>`
- Input: indented `  <!-- comment -->` between blank lines -> output wraps in `<p><!-- comment --></p>`
- Input: `<!-- comment -->` inside a `<div>` block -> output does NOT wrap in `<p>`
- Input: `<!-- comment -->` on the same line as text (e.g., `text <!-- comment --> more`) -> NOT wrapped separately (inline content)
- Input: multiple consecutive comment lines between blank lines -> each wrapped in its own `<p>` tag
- Input: content with non-ASCII/Unicode characters in comment `<!-- Kommentar -->` -> correctly handled

### Integration: DTC zoomcamp pages

- Build DTC site with rustkyll
- Verify `blog/data-engineering-zoomcamp.html` contains `<p><!-- Use manually specified posts --></p>`
- Verify the child element count at the post content container level matches between Jekyll and rustkyll output for `blog/ai-dev-tools-zoomcamp-2025-free-course-to-master-coding-assistants-agents-and-automation.html`
- Verify the `<script type="application/ld+json">` for course structured data is at the same child index in both outputs

### Regression: existing behavior preserved

- Verify HTML comments inside `<div>` blocks are not double-wrapped
- Verify the `collapse_blank_lines_in_html_blocks` function still operates correctly
- Verify other pages with HTML comments (non-zoomcamp) are not broken

## Output Verification

After building the site, inspect the generated HTML:

1. `grep '<p><!-- Use manually specified posts --></p>' /tmp/dtc-test/blog/data-engineering-zoomcamp.html` should return a match
2. `grep '<p><!-- Use manually specified posts --></p>' /tmp/dtc-test/blog/ai-dev-tools-zoomcamp-2025-free-course-to-master-coding-assistants-agents-and-automation.html` should return a match
3. The total number of `<script type="application/ld+json">` tags should be identical between Jekyll and rustkyll output for all 4 pages
4. DOM comparison of the 4 affected pages should no longer show the `tag_name_differs` / `missing_element` cascade for the related-posts/structured-data region

## Log

### [SWE] 2026-03-20
- Root cause: `wrap_standalone_comments_in_paragraphs()` in src/kramdown.rs only checked immediate neighbors (prev/next line) for blank-or-start/end. Consecutive HTML comment lines (e.g., from Liquid include output) had other comment lines as neighbors, so none were recognized as "standalone".
- Wrote 7 tests (test_279_*): 3 for consecutive comment wrapping, 1 for indented, 1 for Unicode, 1 for adjacent-to-block (negative), 1 for inline (negative), 1 for inside-div (negative)
- Ran tests: 3 FAILED as expected (consecutive comments not wrapped)
- Fixed `wrap_standalone_comments_in_paragraphs`: when checking prev/next neighbors, skip past adjacent HTML comment lines to find the first non-comment neighbor. If that neighbor is blank (or start/end), the comment group is standalone and each gets wrapped.
- Ran tests: all 7 new tests PASS, all 9 existing issue-274 tests PASS
- Full test suite: 2108 passed, 198 failed (all failures pre-existing in kramdown_parser::tests)
- Clippy: pre-existing dependency failure (liquid-core), no new warnings from my code
- Fmt: clean
- Files modified: src/kramdown.rs
