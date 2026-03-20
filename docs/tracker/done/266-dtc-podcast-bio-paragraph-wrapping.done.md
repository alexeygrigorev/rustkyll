# Issue 266: Fix collection item `content` field to return rendered HTML

## Problem

On 174 DTC podcast pages, guest bio descriptions are rendered as bare text nodes instead of being wrapped in `<p>` elements. Jekyll wraps each paragraph of the guest bio in `<p>` tags, but rustkyll outputs raw markdown text directly inside the container `<div>`.

This is the single largest source of DOM diffs on DTC (420+ occurrences across 174 pages).

## Example

The podcast layout (`_layouts/podcast.html` line 436-441) renders guest bios like this:

```liquid
<div class="guest-bio-description">
  {% if guest.bio_short %}
    {{ guest.bio_short }}
  {% else %}
    {{ guest.content }}
  {% endif %}
</div>
```

When `bio_short` is absent, `{{ guest.content }}` is used. The same `.content` property is also used in JSON-LD (line 115): `{{ guest.content | strip_html | jsonify }}`.

**Jekyll output (display, `{{ guest.content }}`):**
```html
<div class="guest-bio-description">
  <p>Andrey Cheptsov is the founder and CEO of dstack...</p>
</div>
```

**Rustkyll output (display, `{{ guest.content }}`):**
```html
<div class="guest-bio-description">
  Andrey Cheptsov is the founder and CEO of dstack...
</div>
```

**Jekyll output (JSON-LD, `{{ guest.content | strip_html | jsonify }}`):**
```json
,"description": "Andrey Cheptsov is the founder...tools.\n"
```

**Rustkyll output (JSON-LD):**
```json
,"description": "Andrey Cheptsov is the founder...tools."
```

Both the display and JSON-LD outputs differ from Jekyll.

## Root Cause

Issue 217 changed `collection_item_to_liquid_slim()` in `src/generator.rs` (line 580-583) to set the `content` field to raw markdown (`item.content.trim_start()`) instead of rendered HTML (`item.html_content`). This was done to fix JSON-LD author descriptions in blog posts, where `strip_html | jsonify` on raw markdown preserves markdown link syntax.

However, in Jekyll, `Document#content` accessed through `site.<collection>` returns **rendered HTML** (markdown converted to HTML with `<p>` tags). The blog post case from issue 217 where raw markdown appeared in Jekyll output was a rendering-order artifact, not the standard behavior.

The current code at `src/generator.rs` lines 573-590:

```rust
// Issue 217: Use raw markdown for the content field
obj.insert(
    "content".into(),
    LiquidValue::scalar(item.content.trim_start().to_string()),
);

// Also store rendered HTML as `output`
obj.insert(
    "output".into(),
    LiquidValue::scalar(item.html_content.clone()),
);
```

The `output` field was added as a fallback but is never used by any template (templates use `.content`, not `.output`).

## Fix

Change `collection_item_to_liquid_slim()` to set `content` to rendered HTML (`item.html_content`):

```rust
obj.insert(
    "content".into(),
    LiquidValue::scalar(item.html_content.clone()),
);
```

The `output` field can be removed since it becomes redundant.

### Impact Analysis

**Podcast pages (positive):**
- Display: `{{ guest.content }}` will output `<p>...</p>` matching Jekyll -- fixes ~174 pages
- JSON-LD: `{{ guest.content | strip_html | jsonify }}` will produce `"...text...\n"` with trailing `\n` -- matches Jekyll

**Blog posts (minor regression):**
- Only 1 blog post author (David Gates, appearing on 2 blog pages) has markdown links in their bio content
- Those 2 pages already have other diffs and do not currently match Jekyll
- The JSON-LD description for David Gates will change from `"...of [Accents Welcome](https://url)..."` to `"...of Accents Welcome,..."` (links stripped by `strip_html` on HTML content)
- This is a known trade-off; the 2 blog posts were an artifact of Jekyll's rendering order, not intentional behavior

**Net impact:** Fix ~174 podcast pages, introduce minor description change on 2 already-diffing blog pages.

## Dependencies

- Issue 217 (done) -- this issue partially reverts that change

## Acceptance Criteria

- [ ] `collection_item_to_liquid_slim()` sets `content` to `item.html_content` (rendered HTML), not raw markdown
- [ ] The redundant `output` field is removed from `collection_item_to_liquid_slim()`
- [ ] Guest bio descriptions on podcast pages render with `<p>` tags wrapping paragraphs, matching Jekyll
- [ ] Podcast JSON-LD `description` fields for guests without `bio_short` include trailing `\n`, matching Jekyll
- [ ] Guests with `bio_short` are unaffected (bio_short is a YAML string, not markdown content)
- [ ] Existing tests that assumed raw markdown for `content` are updated to expect rendered HTML
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] Tests include non-ASCII/Unicode content
- [ ] No hardcoded site-specific logic -- the fix must be generic

## Test Scenarios

All tests follow strict TDD: write the test FIRST, verify it FAILS, implement the fix, verify it PASSES.

### Unit: `collection_item_to_liquid_slim` content field

**Test 1: Content field returns rendered HTML**
- Create a `CollectionItem` with `content = "Test Person is a developer."` and `html_content = "<p>Test Person is a developer.</p>\n"`
- Call `collection_item_to_liquid_slim()` and check `obj["content"]`
- Expected: `"<p>Test Person is a developer.</p>\n"` (rendered HTML)
- FIRST RUN: Expect FAIL (current code returns raw markdown)

**Test 2: Content field returns multi-paragraph HTML**
- Create a `CollectionItem` with multi-paragraph markdown and corresponding html_content with multiple `<p>` tags
- Verify `obj["content"]` contains the multi-paragraph HTML
- FIRST RUN: Expect FAIL

**Test 3: Content field with markdown links returns HTML anchor tags**
- Create a `CollectionItem` with `content = "Founded [Company](https://example.com) in 2020."` and `html_content = "<p>Founded <a href=\"https://example.com\">Company</a> in 2020.</p>\n"`
- Verify `obj["content"]` contains `<a href=...>` (rendered HTML)
- FIRST RUN: Expect FAIL (current code returns raw markdown with `[link](url)`)

**Test 4: Content field with non-ASCII/Unicode characters**
- Create a `CollectionItem` with content containing accented characters (e.g., `Rene Descartes est un philosophe francais` with proper cedilla)
- Verify `obj["content"]` has rendered HTML with Unicode preserved
- FIRST RUN: Expect FAIL

**Test 5: Output field is removed**
- Create a `CollectionItem` and call `collection_item_to_liquid_slim()`
- Verify there is no `output` key in the returned object
- FIRST RUN: Expect FAIL (current code adds `output` field)

### Unit: strip_html | jsonify on HTML content

**Test 6: strip_html on rendered HTML produces expected JSON-LD output**
- Set up Liquid context with `content = "<p>Andrey Cheptsov is the founder...tools.</p>\n"`
- Render template: `{{ content | strip_html | jsonify }}`
- Expected: `"Andrey Cheptsov is the founder...tools.\n"` (trailing `\n` from HTML `<p>` block)
- Purpose: Validates that HTML content + strip_html + jsonify matches Jekyll JSON-LD output

### Integration: Update existing issue 217 tests

**Test 7: Update existing tests**
- The following existing tests must be updated to expect rendered HTML instead of raw markdown:
  - `test_collection_item_content_uses_raw_markdown` -- rename and change expectation to HTML
  - `test_collection_item_content_preserves_markdown_links` -- change expectation to HTML with `<a>` tags
  - `test_collection_item_content_no_trailing_html_newline` -- change to expect trailing `\n` (from HTML)
  - `test_collection_item_content_unicode_preserved` -- change expectation to HTML
  - `test_collection_item_slim_has_output_field` -- change to verify output field is ABSENT
  - `test_slim_content_leading_newline_trimmed` -- update expectation

### Output verification

**Test 8: Build DTC site and verify podcast bio**
- Build the DTC site with `./scripts/cargo-safe run -- datatalksclub.github.io`
- Check `_site/podcast/ai-infrastructure-hybrid-cloud-on-prem-distributed-training.html`
- Verify the `guest-bio-description` div contains `<p>Andrey Cheptsov...</p>` (not bare text)
- Verify the JSON-LD description has trailing `\n` matching Jekyll

**Test 9: Build DTC site and verify bio_short guest is unaffected**
- Check `_site/podcast/ab-testing-and-product-experimentation.html`
- Verify the `guest-bio-description` div contains bare text for Jakob Graff (who has `bio_short`, no `<p>` tags expected)

## Implementation Notes

- The fix is a 2-line change in `src/generator.rs` in `collection_item_to_liquid_slim()`:
  1. Change `item.content.trim_start().to_string()` to `item.html_content.clone()` for the `content` field
  2. Remove the `output` field insertion (lines 587-590)
- Update the comment block (lines 573-579) to explain the new behavior
- Update or remove the 6 existing tests from issue 217 that test the old raw-markdown behavior
- Do NOT change `collection_item_to_liquid_full()` in `src/pagination.rs` -- it already uses `html_content` for `content`

## Log

### [SWE] 2026-03-20
- TDD Step 1: Wrote 7 failing tests (updated 6 existing issue 217/219 tests + 1 new multi-paragraph test)
  - `test_collection_item_content_uses_rendered_html` -- expects HTML, got raw markdown: FAILS
  - `test_collection_item_content_renders_markdown_links_as_html` -- expects `<a>` tags, got `[link](url)`: FAILS
  - `test_collection_item_content_has_html_paragraph_wrapping` -- expects `<p>` wrapping: FAILS
  - `test_collection_item_content_unicode_rendered_html` -- expects HTML with unicode: FAILS
  - `test_collection_item_slim_no_output_field` -- expects `output` absent, got Some: FAILS
  - `test_slim_content_uses_html_content_regardless_of_raw_newlines` -- expects HTML: FAILS
  - `test_slim_content_multi_paragraph_html` -- new test, expects multi-paragraph HTML: FAILS
- TDD Step 2: Applied fix in `collection_item_to_liquid_slim()` (src/generator.rs line ~573-590):
  1. Changed `item.content.trim_start().to_string()` to `item.html_content.clone()` for the `content` field
  2. Removed the redundant `output` field insertion
  3. Updated the comment to reference issue 266
- TDD Step 3: All 7 tests now PASS
- Full test suite: 1984 lib + all integration tests pass, 0 failures
- Clippy: pre-existing failure in `liquid-core` dependency (mismatched_lifetime_syntaxes), not caused by our changes
- Format: clean (`cargo fmt --check` passes)
- Files modified: src/generator.rs, docs/tracker/266-dtc-podcast-bio-paragraph-wrapping.in-progress.md
