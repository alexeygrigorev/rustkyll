# Issue 274: Kramdown wraps standalone HTML comments in `<p>` tags

## Problem

The DOM comparison reports 3-diff shifts on 5 DTC zoomcamp pages and missing `<p>` elements on 2 more pages. The original issue title ("script embed pass-through") is misleading -- `<script>` elements already pass through correctly. The actual root cause is that kramdown wraps standalone HTML comments in `<p>` tags, but pulldown-cmark (following CommonMark spec) treats them as HTML blocks and leaves them unwrapped.

### Root Cause

The `related-posts.html` include contains HTML comments like `<!-- Use manually specified posts -->` inside Liquid control flow (`{% if %}`, `{% for %}`). After Liquid processing strips the control flow tags, these comments are left as standalone lines surrounded by blank lines. Kramdown treats such standalone comments as inline content and wraps them in `<p>` tags. Pulldown-cmark treats them as HTML block type 2 and passes them through unwrapped.

**Jekyll output (kramdown):**
```html
<p><!-- Use manually specified posts --></p>

<div class="related-posts-section">
```

**Rustkyll output (pulldown-cmark):**
```html
<!-- Use manually specified posts -->

<div class="related-posts-section">
```

This missing `<p>` element shifts subsequent child indices in the DOM tree, causing the DOM comparator to report the `<div>` and `<script>` as mismatched and the final `<script>` as "missing" (when it is actually present but at a different index).

### Interaction with Issue 144

Issue 144 added a rule that HTML comments between block elements should NOT be wrapped in `<p>` by `wrap_bare_text_in_paragraphs`. That rule is correct for comments adjacent to block-level elements (e.g., `<h2>...\n<!-- comment -->\n<div>`). The new fix must be selective: only wrap comments that appear as standalone lines surrounded by blank lines (i.e., not adjacent to other HTML block elements on the immediately preceding/following non-blank lines). The existing issue 144 tests must continue to pass.

## Affected Pages (7 total)

### Pages with 3-diff script-shift pattern (comment + structured data include):
- `blog/ai-dev-tools-zoomcamp-2025-free-course-to-master-coding-assistants-agents-and-automation.html` (3 diffs -> 0)
- `blog/data-engineering-zoomcamp.html` (3 diffs -> 0)
- `blog/llm-zoomcamp.html` (3 diffs -> 0)
- `blog/machine-learning-zoomcamp.html` (3 diffs -> 0)

### Pages with script-shift pattern among other diffs:
- `blog/mlops-zoomcamp.html` (7 diffs, 3 from this issue -> reduces by 3)

### Pages with missing `<p>` comment diffs only (no structured data include):
- `blog/slack-communities.html` (3 diffs -> 0; all 3 are missing `<p>`-wrapped comments)
- `blog/free-machine-learning-courses.html` (4 diffs, 3 from this issue -> reduces by 3)

### Specific comments that need wrapping (from `related-posts.html` include output):
- `<!-- Use manually specified posts -->` (on pages using `manual_posts` parameter)
- `<!-- Auto-generate based on tags - simplified approach -->` (on pages without `manual_posts`)
- `<!-- Find posts with matching tags -->` (on auto-generate pages)
- `<!-- Sort by date (most recent first) -->` (on auto-generate pages)

## Dependencies

- None. This is a standalone kramdown compatibility fix.

## Approach

The fix should go in the kramdown postprocessor pipeline (likely in `wrap_bare_text_in_paragraphs` or as a new preprocessing step before it). The logic:

1. After pulldown-cmark converts markdown to HTML, identify standalone HTML comments that are:
   - On their own line(s)
   - Surrounded by blank lines (or at the start/end of content)
   - NOT immediately adjacent to block-level HTML elements on the preceding or following non-blank line
2. Wrap those comments in `<p>` tags: `<!-- comment -->` becomes `<p><!-- comment --></p>`

Alternatively, the fix could happen before pulldown-cmark processing by converting standalone comments into something pulldown-cmark will paragraph-ify, but post-processing is simpler and more consistent with existing kramdown compatibility logic.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (including all existing issue 144 tests for comments adjacent to block elements)
- [ ] Standalone HTML comments surrounded by blank lines in pulldown-cmark output are wrapped in `<p>` tags, matching kramdown behavior
- [ ] HTML comments adjacent to block-level elements (e.g., `<h2>...\n<!-- comment -->\n<div>`) are NOT wrapped in `<p>` tags (preserving issue 144 behavior)
- [ ] Building the DTC site and inspecting output shows `<p><!-- Use manually specified posts --></p>` on pages using `related-posts.html` with `manual_posts`
- [ ] Building the DTC site and inspecting output shows `<p><!-- Auto-generate based on tags - simplified approach --></p>`, `<p><!-- Find posts with matching tags --></p>`, and `<p><!-- Sort by date (most recent first) --></p>` on pages using `related-posts.html` without `manual_posts`
- [ ] DOM comparison diff count for `blog/ai-dev-tools-zoomcamp-*.html` drops from 3 to 0
- [ ] DOM comparison diff count for `blog/data-engineering-zoomcamp.html` drops from 3 to 0
- [ ] DOM comparison diff count for `blog/llm-zoomcamp.html` drops from 3 to 0
- [ ] DOM comparison diff count for `blog/machine-learning-zoomcamp.html` drops from 3 to 0
- [ ] DOM comparison diff count for `blog/slack-communities.html` drops from 3 to 0
- [ ] DOM comparison diff count for `blog/mlops-zoomcamp.html` reduces by 3 (from 7 to 4)
- [ ] DOM comparison diff count for `blog/free-machine-learning-courses.html` reduces by 3 (from 4 to 1)

## Test Scenarios

### Unit: Standalone comment wrapping
- Input: `<p>text</p>\n\n<!-- comment -->\n\n<div>block</div>` -- comment surrounded by blank lines, should be wrapped in `<p>`
- Input: `<!-- comment1 -->\n\n<!-- comment2 -->\n\n<div>block</div>` -- multiple standalone comments, each should be wrapped in `<p>`
- Input: `<h2>Heading</h2>\n<!-- comment -->\n<div>block</div>` -- comment adjacent to block elements, should NOT be wrapped (issue 144 regression test)
- Input: `</div>\n<script src="x.js"></script>\n<div>` -- script tag adjacent to block elements, should NOT be wrapped (issue 144 regression test)
- Input: `<!-- FAQ Accordion Component -->\n<div class="faq">` -- comment immediately before block element, should NOT be wrapped
- Input: content with no HTML comments -- should pass through unchanged

### Unit: Real include output patterns
- Simulate the Liquid-processed output of `related-posts.html` with `manual_posts` parameter: verify `<!-- Use manually specified posts -->` is wrapped in `<p>`
- Simulate the Liquid-processed output of `related-posts.html` without `manual_posts`: verify all 3 comments (`Auto-generate`, `Find posts`, `Sort by date`) are each wrapped in `<p>`

### Integration: Full page rendering
- Create a test post with `{% include %}` that expands to HTML containing standalone comments followed by a `<script>` block. Verify the output matches the expected kramdown-style wrapping.

### Integration: DTC site output verification (manual/CI)
- Build the DTC site with rustkyll
- Verify `<p><!-- Use manually specified posts --></p>` appears in `blog/data-engineering-zoomcamp.html`
- Verify `<script type="application/ld+json">` appears in `blog/data-engineering-zoomcamp.html` (regression: scripts must still pass through)

## Log

### [SWE] 2026-03-20
- Wrote 9 TDD tests in src/kramdown.rs (test_274_*)
- Ran tests: 5 FAIL as expected (standalone comments not wrapped), 4 PASS (adjacent-to-block comments correctly not wrapped)
- Implemented `wrap_standalone_comments_in_paragraphs()` function in src/kramdown.rs
  - Runs after `wrap_bare_text_in_paragraphs` in postprocess pipeline
  - Wraps HTML comments that are on their own line AND surrounded by blank lines (or start/end of content)
  - Leaves comments adjacent to non-blank lines untouched (preserving issue 144 behavior)
- Ran tests: all 9 new tests PASS
- Ran full test suite: 2042 lib tests + all integration tests PASS, 0 failures
- Issue 144 regression tests all PASS (accordion, structured data, comment-before-block patterns)
- cargo fmt: clean
- clippy: pre-existing dependency error in liquid-core (not caused by this change, confirmed by testing on clean main)
- Files modified: src/kramdown.rs
