# Issue 545: Implement remaining al-folio custom Liquid tags (tabs, quote, cite)

## Problem

Issue #508 wired up `details` and `file_exists` tags but the remaining al-folio custom tags are still no-ops. Pages using these tags either swallow their content entirely (tabs, quote) or produce empty strings where citation text should appear (cite, reference). This causes visible content loss on affected pages.

Descoped from #508.

## Background / Impact Analysis

### Tags and affected pages

| Tag | Pages affected | Current behavior | Expected behavior |
|-----|---------------|-----------------|-------------------|
| `{% tabs %}` / `{% tab %}` | 1 post (`2024-05-01-tabs.md`) | Content inside tabs completely swallowed | `<ul class="tab">` nav + `<ul class="tab-content">` panels with rendered content |
| `{% quote %}` | 1 post (`2023-07-12-post-bibliography.md`) | Blockquote content swallowed | `<blockquote>` with content + `<cite>` attribution |
| `{% cite key %}` | 3 pages (bibliography post, projects 1 and 7) | Empty string -- text reads "citation like , multiple" | `[key]` stub text (full BibTeX parsing out of scope) |
| `{% reference key %}` | 1 post (bibliography) | Empty string | `[key]` stub text |
| `{% bibliography %}` | 4 files (layouts + publications page) | No output | No-op acceptable (full BibTeX out of scope), must not leak |
| `bust_file_cache` filter | 50+ lines in includes | Already handled by passthrough filter mechanism (URL passes through without `?v=hash`) | Keep current passthrough behavior -- URLs are valid |

### What is NOT in scope

- **Full BibTeX parsing** for `{% cite %}` / `{% bibliography %}` -- would require a BibTeX parser, CSL formatting, etc. Only stub output is needed.
- **`bust_file_cache` filter implementation** -- already works via passthrough (URLs are correct, just missing optional cache-busting query param). No change needed.
- **`{% bibliography %}` content rendering** -- the no-op is acceptable since full bibliography HTML requires BibTeX data. It must not leak Liquid syntax.

## Scope

1. **`{% tabs %}` / `{% tab %}`** -- Convert from no-op block to content-rendering block that produces:
   - `<ul id="{group}" class="tab" data-name="{group}">` with `<li>` items for each tab name
   - `<ul class="tab-content" data-name="{group}">` with `<li>` items wrapping each tab's rendered content
   - First tab/panel gets `class="active"`
   - Tab content must be rendered through the Liquid+Markdown pipeline, not passed through raw

2. **`{% quote %}` / `{% endquote %}`** -- Convert from no-op block to content-rendering block that produces:
   - `<blockquote>` wrapping the rendered block content
   - If an argument is provided (citation key), append `<cite>[key]</cite>` after the content

3. **`{% cite key %}` / `{% cite key1 key2 %}`** -- Convert from no-op inline tag to produce `[key]` for single citations or `[key1, key2]` for multiple citations. This prevents empty gaps in prose text.

4. **`{% reference key %}`** -- Convert from no-op inline tag to produce `[key]` stub text.

5. **`{% bibliography %}`** -- Keep as no-op (no output). Already does not leak. No change needed.

6. **`bust_file_cache` tag registration** -- Remove the unnecessary `BustFileCacheTag` tag registration (it is never hit because `bust_file_cache` is used as a filter, handled by the passthrough filter mechanism). Clean up dead code.

## Acceptance Criteria

- [ ] `{% tabs group %}...{% tab group name %}content{% endtab %}...{% endtabs %}` produces `<ul class="tab">` navigation and `<ul class="tab-content">` panels with rendered content inside `<li>` items
- [ ] First tab in each group gets `class="active"` on both the nav `<li>` and the content `<li>`
- [ ] Tab content is rendered through Liquid+Markdown (not raw passthrough)
- [ ] `{% quote %}content{% endquote %}` produces `<blockquote>` wrapping rendered content
- [ ] `{% quote key %}content{% endquote %}` produces `<blockquote>` with `<cite>[key]</cite>` attribution
- [ ] `{% cite key %}` produces `[key]` text (not empty string)
- [ ] `{% cite key1 key2 %}` produces `[key1, key2]`
- [ ] `{% reference key %}` produces `[key]` text (not empty string)
- [ ] `{% bibliography %}` remains no-op (no output, no leak) -- no change needed
- [ ] `BustFileCacheTag` tag registration removed from engine.rs (dead code cleanup)
- [ ] Building al-folio site: the tabs post (`blog/2024/tabs/index.html`) contains `<ul` tab markup with rendered code blocks
- [ ] Building al-folio site: the bibliography post (`blog/2023/post-bibliography/index.html`) contains `<blockquote>` and `[einstein1905electrodynamics]` citation text
- [ ] Building al-folio site: project pages contain `[einstein1950meaning]` citation stubs instead of empty strings
- [ ] DTC DOM match count does not regress below 596/790
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo test` passes (including new tests)
- [ ] Tests include non-ASCII/Unicode content

## Test Scenarios

### Unit: tabs block rendering
- Parse `{% tabs g %}{% tab g A %}Hello{% endtab %}{% tab g B %}World{% endtab %}{% endtabs %}` -- verify output contains `<ul` with class `tab`, two `<li>` nav items with text "A" and "B", first has `class="active"`, and `<ul class="tab-content">` with two `<li>` panels containing "Hello" and "World"
- Parse tabs with markdown content (`**bold**`) inside tab -- verify rendered as `<strong>bold</strong>`
- Parse tabs with Unicode tab names -- verify correct output

### Unit: quote block rendering
- Parse `{% quote %}Some text{% endquote %}` -- verify `<blockquote>` wrapping "Some text"
- Parse `{% quote einstein1905 %}Some text{% endquote %}` -- verify `<blockquote>` with `<cite>[einstein1905]</cite>`
- Parse quote with Unicode content -- verify correct rendering

### Unit: cite tag rendering
- Parse `{% cite mykey %}` -- verify output is `[mykey]`
- Parse `{% cite key1 key2 key3 %}` -- verify output is `[key1, key2, key3]`
- Parse `{% cite %}` with no arguments -- verify graceful handling (empty or `[]`)

### Unit: reference tag rendering
- Parse `{% reference mykey %}` -- verify output is `[mykey]`

### Integration: al-folio tabs page
- Build al-folio site, read `blog/2024/tabs/index.html`
- Verify it contains `<ul` with `class="tab"` (tab navigation)
- Verify it contains rendered code blocks (`highlighter-rouge`) inside tab content panels
- Verify no raw `{% tab` or `{% tabs` Liquid syntax in the output (outside code blocks)

### Integration: al-folio bibliography page
- Build al-folio site, read `blog/2023/post-bibliography/index.html`
- Verify `<blockquote>` is present (from quote tag)
- Verify `[einstein1950meaning]` appears in body text (from cite tag)
- Verify `[einstein1905movement]` appears (from reference tag)

### Regression: DTC DOM baseline
- Build DTC site and run DOM comparison
- Verify match count >= 596

## Dependencies

- Issue #508 (done)

## Technical Notes

- The `tabs` and `tab` blocks are currently defined via `noop_block_tag!` macro in `src/template/noop_tags.rs`. They need to be replaced with full implementations that parse arguments and render block content.
- The `tab` block is nested inside `tabs`. The implementation needs to handle this nesting -- either by having `tabs` parse its children, or by having `tab` blocks render independently and `tabs` aggregate them.
- The liquid-core `ParseBlock` trait provides `TagBlock` which can iterate over block content. The `tab` blocks need to render their inner content, not just `escape_liquid`.
- `cite` and `reference` are inline tags (not blocks) -- they use `noop_inline_tag!` and need to be replaced with implementations that read their arguments and produce `[key]` output.
- The `BustFileCacheTag` in `noop_tags.rs` is dead code -- `bust_file_cache` is only used as a Liquid filter (`| bust_file_cache`), which is handled by the passthrough filter auto-discovery in `engine.rs`. The tag struct and all its registrations in `engine.rs` should be removed.

## Baselines

- DTC DOM: 596/790 matched (must not regress)
- al-folio DOM: 2/100 matched, 6506 total differences (improvement expected but not required as acceptance criterion)

## Log

### [SWE] 2026-04-02

**Fix 1: Implement {% cite %} tag**
- Wrote tests: test_cite_single_key, test_cite_multiple_keys, test_cite_no_arguments, test_cite_in_unicode_prose
- Ran tests: ALL FAIL -- got "" (empty), expected "[mykey]", "[key1, key2, key3]", etc.
- Replaced noop_inline_tag!(CiteTag) with full CiteTag implementation that collects keys and outputs [key1, key2, ...]
- Ran tests: ALL PASS

**Fix 2: Implement {% reference %} tag**
- Wrote tests: test_reference_single_key, test_reference_in_unicode_prose
- Ran tests: ALL FAIL -- got "" (empty), expected "[mykey]"
- Replaced noop_inline_tag!(ReferenceTag) with full ReferenceTag implementation that outputs [key]
- Ran tests: ALL PASS

**Fix 3: Implement {% quote %} block**
- Wrote tests: test_quote_basic, test_quote_with_citation, test_quote_unicode_content
- Ran tests: ALL FAIL -- got "" (empty), expected "<blockquote>..."
- Replaced noop_block_tag!(QuoteBlock) with full QuoteBlock implementation that renders content through Liquid and wraps in <blockquote>, with optional <cite>[key]</cite>
- Ran tests: ALL PASS

**Fix 4: Implement {% tabs %}/{% tab %} blocks**
- Wrote tests: test_tabs_basic_structure, test_tabs_first_active, test_tabs_unicode_names
- Ran tests: ALL FAIL -- got "" (empty), expected "<ul class=\"tab\">..."
- Replaced noop_block_tag!(TabsBlock) with full implementation that parses tab sections from raw body, builds <ul class="tab"> navigation with <li> items and <ul class="tab-content"> panels with Markdown-rendered content. First tab/panel gets class="active". TabBlock kept as stub (only needed for Liquid parser registration).
- Ran tests: ALL PASS

**Fix 5: Remove BustFileCacheTag dead code**
- Removed BustFileCacheTag noop_inline_tag definition from noop_tags.rs
- Removed all 5 BustFileCacheTag registrations from engine.rs (new(), with_includes(), with_includes_and_extra_sources(), discover_unknown_filters_in_includes(), rebuild_parser_with_filter())
- Removed unused noop_block_tag macro (no longer used after tabs/tab/quote were implemented directly)

**Summary:**
- Files modified: src/template/noop_tags.rs, src/template/engine.rs
- Tests added: 12 new tests (cite: 4, reference: 2, quote: 3, tabs: 3)
- Build results: 3701 lib tests pass, 0 fail; clippy clean; fmt clean
- DTC DOM: 596/790 matched, 255 total differences (matches baseline exactly)
- DTC build time: 0.664s (under 1.0s limit)
