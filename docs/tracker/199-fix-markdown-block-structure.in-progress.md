# Issue 199: Fix markdown block structure differences (335 pages)

## Checklist Category

**Markdown block structure differences** -- 335 pages

## Problem

335 pages have HTML element structure differences from markdown rendering. Elements appear in different order, text that should be in `<p>` tags appears as raw text, or block-level elements are wrapped differently.

Breakdown by site:
- alexeygrigorev-mlwiki.org (235): Definition lists (`dl`), block structure from MediaWiki-exported content, element ordering cascades
- muan-blog (53): Pages where layout fails cause structural diffs (overlaps with issue 196)
- DTC (26): Image references with markdown links not parsed, block structure shifts
- theme sites (1 each, ~10 total): Minor block structure differences
- government-github (2), opensource-guide (2), mojombo-blog (2), mlbookcamp-page (4), aihero (2): Various block patterns

## Goal

Fix markdown block structure to match kramdown output.

## Dependencies

- Issue 196 (layout not applied): muan-blog's 53 block structure diffs may largely be caused by missing layouts. Fix 196 first and recount.
- Issue 84 (kramdown compatibility) -- done
- Issue 92 (paragraph wrapping in HTML elements) -- done
- Issue 114 (kramdown bare text wrapping) -- done
- Issue 124 (kramdown loose list wrapping) -- done
- Issue 148 (misc markdown rendering edge cases) -- done
- Issue 152 (kramdown paragraph cascade) -- done

## Sub-tasks

### Sub-task 1: Investigation (do this FIRST)

1. Read `docs/comparison/dom-details/DataTalksClub-datatalksclub.github.io.txt` and extract the 26 block structure diffs. Categorize:
   - Image with markdown link not parsed (e.g., `Photo by [Name](url)` appearing as text instead of `<p>Photo by <a>Name</a></p>`)
   - FAQ page structure (child element ordering with `<script>` tags)
   - `<figure>/<figcaption>` ordering
   - Other block patterns

2. Read `docs/comparison/dom-details/alexeygrigorev-mlwiki.org.txt` and categorize the 235 block diffs:
   - Definition lists (`<dl>`) that kramdown generates but pulldown-cmark does not
   - Element ordering cascades from upstream structural differences
   - Other patterns

3. Check a theme site dom-details file for the specific block diff pattern.

4. For DTC, actually compare the Jekyll and rustkyll HTML output for 2-3 affected pages to see the full context.

### Sub-task 2: Fix image-link markdown pattern in DTC

The pattern `Photo by [Name](url) on [Site](url)` appears as raw text instead of being parsed as a paragraph with links. This is likely a kramdown vs pulldown-cmark difference in how markdown links with `{:target="_blank"}` attributes are handled.

### Sub-task 3: Fix definition list rendering for mlwiki.org

kramdown supports definition lists (`: definition` syntax). pulldown-cmark does not natively. This needs post-processing in `src/kramdown.rs` or a decision to document as a known limitation.

### Sub-task 4: Fix FAQ page block ordering in DTC

The FAQ pages show `<p>` and `<div>` elements in swapped positions, likely from how the FAQ schema script tag interacts with surrounding content.

## TDD Test Scenarios

### Test 1: Markdown link with kramdown attribute not breaking parsing (write FIRST, verify it fails)

```rust
#[test]
fn test_markdown_link_with_kramdown_target_attribute() {
    // Setup: Markdown input:
    //   Photo by [Kane](https://example.com){:target="_blank"} on [Unsplash](https://unsplash.com)
    //
    // Assert: Produces HTML with <p> containing:
    //   "Photo by " text, <a href="...">Kane</a>, " on ", <a href="...">Unsplash</a>
    //   NOT raw text with [Kane](url) visible.
    //
    // Verify it FAILS before implementing.
}
```

### Test 2: Definition list rendering

```rust
#[test]
fn test_kramdown_definition_list() {
    // Setup: Markdown input (kramdown definition list syntax):
    //   Term
    //   : Definition text here
    //
    //   Another term
    //   : Another definition
    //
    // Assert: Produces HTML with <dl><dt>Term</dt><dd>Definition text here</dd>...</dl>
    //
    // Verify it FAILS (pulldown-cmark does not support definition lists natively).
}
```

### Test 3: Block element ordering around script tags

```rust
#[test]
fn test_block_ordering_with_embedded_script() {
    // Setup: HTML content that contains a <div> followed by a <script> tag
    //   within markdown content.
    //
    // Assert: The <p>, <div>, and <script> elements appear in the same order
    //   as kramdown would produce.
    //
    // Verify it FAILS before implementing.
}
```

### Test 4 (integration, #[ignore]): Build DTC and verify block structure

```rust
#[test]
#[ignore]
fn test_dtc_block_structure_matches() {
    // Build DTC site
    // Parse essentials-of-public-speaking-for-career-in-data-science.html
    // Verify first content element after heading is a <p> tag, not raw text
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with new tests for each block structure pattern
- [ ] Investigation documents categorization of all 335 page diffs by specific markdown pattern
- [ ] DTC block structure diffs fixed: image-link pattern parsed correctly, FAQ page ordering correct (26 pages)
- [ ] muan-blog: after issue 196 (layout) is fixed, recount remaining block structure diffs; fix or create sub-issue
- [ ] mlwiki.org definition lists: document as known limitation if pulldown-cmark cannot support, OR implement post-processing; create sub-issue if complex
- [ ] Theme site block diffs addressed
- [ ] No regressions in existing kramdown compatibility tests

## Log

### [SWE] 2026-03-18 Implementation

#### Investigation results

Categorized all 335 page diffs by specific markdown pattern:

**DTC (26 pages):**
- Image-link after HTML block: 1 page (`essentials-of-public-speaking`) -- `</figure>Photo by [Name](url){:target="_blank"}` not parsed as markdown because pulldown-cmark treats text after closing HTML block tag as part of the HTML block
- FAQ page `<p>`/`<div>`/`<script>` ordering: 5 pages (zoomcamp pages) -- kramdown wraps bare HTML comments in `<p>` which shifts element ordering by +1 position
- URL with parentheses + IAL: 2 pages (`how-to-run-postgresql`, `how-to-setup-lightweight-local-version-for-airflow`) -- `Docker_(software){:target="_blank"}` causes nesting issues with markdown link syntax
- Syntax highlighting class differences: ~8 pages -- `class='k'` vs `class='n'` (keyword vs identifier) from different syntect vs rouge classification
- JSON-LD value differences: ~10 pages -- minor whitespace/newline differences in JSON-LD structured data

**mlwiki.org (235 pages):**
- kramdown definition lists (`: definition` syntax): ~148 occurrences -- pulldown-cmark doesn't support this natively
- MediaWiki triple/double quote formatting differences
- Table rendering inside list items
- Syntax highlighting differences for XML/code blocks

**Other sites (~10 pages):**
- Minor block structure differences from theme-specific patterns

#### Fixes implemented

1. **`split_text_after_html_block_close`** (pre-processing): When markdown text immediately follows a closing HTML block tag on the same line (e.g., `</figure>Photo by [Name](url)`), inserts a blank line so pulldown-cmark parses the text as markdown instead of treating it as part of the HTML block. Integrated into all 4 markdown processing pipeline paths (layout.rs x3, pagination.rs x1).

2. **`convert_definition_lists`** (post-processing): Detects kramdown-style definition list patterns in pulldown-cmark output (`<p>Term\n: Definition</p>`) and converts them to proper `<dl>`/`<dt>`/`<dd>` HTML. Handles multiple terms, multiple definitions per term, and Unicode content.

#### Test results
- 11 new tests added (6 for split_text, 2 for full pipeline, 3 for definition lists)
- All include non-ASCII/Unicode content
- 1543 lib tests pass, 0 fail
- Clippy clean, fmt clean
- Integration tests cannot compile due to pre-existing errors from issues 198/200 (missing functions in frontmatter.rs from parallel work)

#### Files modified
- `src/kramdown.rs`: Added `split_text_after_html_block_close()`, `convert_definition_lists()`, and 11 tests
- `src/template/layout.rs`: Integrated split_text in 3 pipeline paths
- `src/pagination.rs`: Integrated split_text in pagination pipeline

#### Known limitations
- FAQ page ordering diffs (5 pages): caused by kramdown wrapping HTML comments in `<p>` -- not fixed, would need HTML comment wrapping in post-processing
- URL parentheses + IAL pattern (`Docker_(software){:target="_blank"}`): not fixed, would need URL-aware IAL extraction pre-processing
- mlwiki definition lists inside list items (`<ul><li>` context): the current implementation only handles `<p>` wrapped definition lists, not those inside list items where kramdown generates nested `<dl>` elements
- Syntax highlighting class differences: different issue (syntect vs rouge keyword classification)
- muan-blog: 53 diffs likely overlap with issue 196 (layout not applied), pending recount after 196 fix
