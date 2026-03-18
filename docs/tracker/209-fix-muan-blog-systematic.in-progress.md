# Issue 209: Fix muan-blog systematic differences (2189 pages)

## Problem

muan-blog has 29/2218 (1%) DOM match rate. Five systematic issues affect nearly every page. This issue addresses the four that are fixable with targeted code changes.

### 1. Collection URL has `.html` extension (affects every page)

**Root cause:** In `src/collection.rs` line 431, the default permalink for non-post collections is `/:collection/:title.html`. Jekyll's actual default for collections is `/:collection/:path` (no `.html`). This causes every `page.url` and `note.url` to end in `.html`, which cascades into:
- `og:url` meta tag: `https://muan.co/notes/2018-06-04-aa.html` instead of `https://muan.co/notes/2018-06-04-aa`
- `open-heart` widget: `href='https://likes.muan.dev/?id=/notes/2018-06-04-aa.html'` instead of without `.html`
- `notes.html` listing page links

Additionally, the `{% link _pages/banners.md %}` preprocessing in `src/template/engine.rs` (line 1054) always converts `.md` to `.html`. In Jekyll, `{% link %}` resolves to the document's actual URL, which for collections without an explicit `.html` permalink pattern would be `/pages/banners` (no extension). The footer on every page has `{% link _pages/banners.md %}`, so this affects all 2218 pages.

**Note:** The site's `_config.yml` has `permalink: /posts/:title` which only applies to posts, NOT to other collections (notes, pages, stories). The collections use the default permalink pattern.

### 2. Empty meta description (2000+ notes pages)

**Root cause:** The template uses `{{ page.content | strip_html | truncate: 240 }}` for the description meta tag (NOT `page.excerpt`). Jekyll's `page.content` in the template context contains the rendered HTML content of the page. If rustkyll's `page.content` is empty or not populated in the Liquid context when rendering the layout, `strip_html` on empty string produces empty string. The comparison data confirms: Jekyll outputs the note's text content while rustkyll outputs empty `content=''`.

### 3. Datetime format in `<time>` tag (all notes pages)

**Root cause:** Notes have front matter like `date: 2018/06/04 00:00` (slashes, no timezone). Jekyll's Ruby YAML parser converts this to a Time object and `{{ page.date }}` renders as `2018-06-04 00:00:00 +0800` (using the site timezone `Asia/Taipei`). Rustkyll's `expand_date_only_string_with_tz` in `src/template/context.rs` only handles the `YYYY-MM-DD` pattern (exactly 10 chars). The `2018/06/04 00:00` format (16 chars, slashes) is not recognized and passed through as-is.

The fix needs to: (a) recognize `YYYY/MM/DD HH:MM` format dates, (b) normalize slashes to dashes, and (c) apply the site timezone to produce `YYYY-MM-DD HH:MM:SS +HHMM`.

### 4. `map` filter on nested arrays concatenates instead of flattening (notes.html tags)

**Root cause:** The template `{% assign tags = notes | map: "tags" | uniq | sort %}` expects `map: "tags"` on a collection of notes (each having a tags array like `["Book", "Mental health"]`) to produce a flat array of all tags. In Jekyll/Ruby, `Array#map` followed by `flatten` produces `["Book", "Mental health", "Hobby", ...]`. The `liquid` crate's built-in `map` filter maps each item's `tags` property, but since each `tags` is itself an array, the result is an array of arrays: `[["Book", "Mental health"], ["Hobby", "Life"], ...]`. When `uniq` or iteration then stringifies each inner array, it concatenates the elements: `"BookMental health"`.

The fix needs to flatten the result of `map` when the mapped values are arrays, matching Jekyll's behavior.

### 5. Smart quotes differ (minor, text_differs)

Jekyll uses kramdown's smart quotes (Unicode right single quotation mark U+2019). Rustkyll uses pulldown-cmark's smart punctuation which also produces U+2019 but the comparison shows differences on some pages. This may be a subset of pages where the markdown processor choice affects the specific quote character used.

**This issue is descoped** to a separate follow-up since it is cosmetic and requires deeper investigation into which specific characters differ and why.

## Goal

Fix issues 1-4 to bring muan-blog from 29/2218 to 2000+/2218 DOM match rate.

## Dependencies

- Issue 196 (fix layout not applied) should ideally be done first -- muan-blog's index.html, de-DE/, film.html have layout issues. However, the 4 fixes in this issue are independent of layout resolution and can proceed in parallel.
- No other blocking dependencies.

## Sub-tasks

### Sub-task 1: Fix default collection permalink pattern

In `src/collection.rs`, change the default permalink for non-post collections from `/:collection/:title.html` to `/:collection/:path` (matching Jekyll's default). Verify this does not break other sites by checking that sites which explicitly set collection permalinks are unaffected.

### Sub-task 2: Fix `{% link %}` tag URL generation

In `src/template/engine.rs` `preprocess_jekyll_tags`, the `{% link _pages/banners.md %}` conversion currently always produces `.html` extension. It needs to respect the collection's permalink pattern. Since the `link` tag preprocessing happens before template rendering (no access to site config), the simplest correct approach is to generate URLs without `.html` for collection documents (i.e., `{% link _pages/banners.md %}` -> `/pages/banners`), matching Jekyll's behavior when no explicit `.html` permalink is configured.

### Sub-task 3: Fix date normalization for `YYYY/MM/DD HH:MM` format

In `src/template/context.rs`, extend `expand_date_only_string_with_tz` (or add a companion function) to also recognize and normalize dates in `YYYY/MM/DD HH:MM` format:
- Convert slashes to dashes: `2018/06/04` -> `2018-06-04`
- Add seconds: `00:00` -> `00:00:00`
- Apply site timezone offset: append `+0800` for `Asia/Taipei`
- Result: `2018-06-04 00:00:00 +0800`

### Sub-task 4: Fix `page.content` availability in layout template context

Investigate why `{{ page.content | strip_html | truncate: 240 }}` produces empty output. In Jekyll, `page.content` in the layout context contains the rendered HTML. If rustkyll is not populating `page.content` in the template context or populating it after layout rendering, fix the ordering so `page.content` is available when the layout template is rendered.

### Sub-task 5: Fix `map` filter to flatten nested arrays

The `liquid` crate's built-in `map` filter does not flatten results. Implement a custom `map` filter override (or a post-processing step) that flattens the result when `map` produces an array of arrays, matching Jekyll/Ruby's behavior where `array.map(&:property).flatten` is the effective operation.

## TDD Test Scenarios

### Test 1: Default collection permalink has no .html extension (write FIRST, verify it fails)

```rust
#[test]
fn test_default_collection_permalink_no_html() {
    // Setup: A site with a "pages" collection (no explicit permalink in collection config)
    // Load a collection item from _pages/banners.md
    // Assert: The generated URL is "/pages/banners" (no .html)
    // NOT "/pages/banners.html"
    //
    // This tests the default permalink pattern for non-post collections.
    // The default should be "/:collection/:path" matching Jekyll.
}
```

### Test 2: Link tag preprocessing produces extensionless URLs for collection docs (write FIRST)

```rust
#[test]
fn test_link_tag_no_html_for_collection_pages() {
    // Setup: preprocess_jekyll_tags with input:
    //   r#"<a href="{% link _pages/banners.md %}">Link</a>"#
    // Assert: output is:
    //   r#"<a href="/pages/banners">Link</a>"#
    // NOT "/pages/banners.html"
    //
    // Include a non-collection file test too:
    //   {% link about.md %} -> /about.html (root pages keep .html as before)
    //
    // Include Unicode filename test:
    //   {% link _pages/uber-uns.md %} -> /pages/uber-uns
}
```

### Test 3: Date normalization for slash-format dates with site timezone (write FIRST)

```rust
#[test]
fn test_date_normalization_slash_format_with_timezone() {
    // Setup: Front matter date value "2018/06/04 00:00"
    //        Site timezone: Asia/Taipei (+0800)
    // Run through expand_date_only_string_with_tz (or the new normalization)
    // Assert: output is "2018-06-04 00:00:00 +0800"
    //
    // Additional cases:
    //   "2024-02-23 19:17" with Asia/Taipei -> "2024-02-23 19:17:00 +0800"
    //   "2018/06/04 00:00" with no timezone -> "2018-06-04 00:00:00 +0000"
}
```

### Test 4: page.content available in layout context with strip_html (write FIRST)

```rust
#[test]
fn test_page_content_available_in_layout_for_strip_html() {
    // Setup: Create a minimal site with:
    //   _layouts/default.html containing:
    //     <meta content="{{ page.content | strip_html | truncate: 240 }}" name="description">
    //     <body>{{ content }}</body>
    //   A note (collection item) with content:
    //     "pretty sure my dad is the biggest winner here"
    //     (Include non-ASCII: "Мой блог ~ pretty sure my dad")
    //
    // Build the page through layout rendering
    // Assert: The <meta> description contains the note's text content
    //   NOT content=''
}
```

### Test 5: map filter flattens nested arrays (write FIRST)

```rust
#[test]
fn test_map_filter_flattens_nested_arrays() {
    // Setup: Create Liquid template:
    //   {% assign all_tags = items | map: "tags" | uniq | sort %}
    //   {% for tag in all_tags %}{{ tag }},{% endfor %}
    //
    // Context: items = [
    //   { tags: ["Book", "Mental health"] },
    //   { tags: ["Hobby", "Life"] },
    //   { tags: ["Book", "Life"] },
    // ]
    //
    // Assert output contains: "Book,Hobby,Life,Mental health,"
    // NOT: "BookMental health,HobbyLife,BookLife,"
    //
    // Include Unicode tag test:
    //   { tags: ["Buch", "Gesundheit"] }
}
```

### Test 6: map filter on flat property still works normally

```rust
#[test]
fn test_map_filter_flat_property_unchanged() {
    // Setup: items | map: "title" where each item has a scalar title
    // Assert: produces array of strings (no flattening needed, no regression)
}
```

### Test 7 (integration, #[ignore]): Build muan-blog and verify fixes

```rust
#[test]
#[ignore]
fn test_muan_blog_systematic_fixes() {
    // Build muan-blog site
    // Check notes/2018-06-04-aa.html:
    //   - og:url contains /notes/2018-06-04-aa (no .html)
    //   - description meta is NOT empty
    //   - datetime attribute is "2018-06-04 00:00:00 +0800"
    //   - footer link to banners has no .html
    // Check notes.html:
    //   - Tag filter CSS has individual tags like "Book", "Mental health"
    //   - NOT concatenated tags like "BookMental health"
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new tests (at least 6 unit tests)
- [ ] Default collection permalink pattern is `/:collection/:path` (no `.html`), matching Jekyll's default
- [ ] `{% link _pages/banners.md %}` produces `/pages/banners` (no `.html` extension) for collection documents
- [ ] Date values in `YYYY/MM/DD HH:MM` format are normalized to `YYYY-MM-DD HH:MM:SS +HHMM` with the site timezone applied
- [ ] `page.content | strip_html | truncate: 240` produces non-empty description for notes with content
- [ ] `map` filter on arrays of arrays produces a flattened result matching Jekyll behavior
- [ ] Tests include non-ASCII/Unicode content (per project convention)
- [ ] No regressions: existing tests for other sites (DTC, choosealicense, etc.) still pass
- [ ] Building muan-blog and inspecting output HTML confirms:
  - `og:url` values have no `.html` suffix for collection items
  - `datetime` attributes use `YYYY-MM-DD HH:MM:SS +HHMM` format
  - Meta description is populated (not empty) for notes pages
  - Tags on notes.html page are individual (not concatenated)

## Descoped to follow-up issues

- **Smart quotes difference (issue 5):** Needs investigation into which specific Unicode characters differ between kramdown and pulldown-cmark smart punctuation. Low impact (cosmetic text difference only). Will be tracked separately if not already covered by an existing issue.

## Log

### [SWE] 2026-03-18

- Implemented all 5 sub-tasks following TDD (wrote failing tests first, then implemented fixes)
- **Sub-task 1: Default collection permalink** - Changed default from `/:collection/:title.html` to `/:collection/:path` in `src/collection.rs:431`. Sites with explicit permalink config (like DTC) are unaffected.
- **Sub-task 2: Link tag preprocessing** - Modified `preprocess_jekyll_tags` in `src/template/engine.rs` to produce extensionless URLs for collection docs (paths starting with `_`). Root pages still get `.html`.
- **Sub-task 3: Date normalization** - Rewrote `expand_date_only_string_with_tz` in `src/template/context.rs` to handle `YYYY/MM/DD`, `YYYY/MM/DD HH:MM`, and `YYYY-MM-DD HH:MM` formats, normalizing to `YYYY-MM-DD HH:MM:SS +HHMM` with site timezone.
- **Sub-task 4: page.content in layout** - Added `page.content` injection in both `build_render_context` and `build_render_context_page_only` in `src/template/layout.rs`. The rendered HTML is now available as `page.content` in layout templates.
- **Sub-task 5: Map filter flattens nested arrays** - Created custom `Map` filter in `src/template/filters/map.rs` that flattens results when mapped values are arrays, matching Jekyll/Ruby behavior. Registered in parser builder.
- Tests added: 17 new tests (5 date normalization, 4 collection permalink, 4 link tag, 2 page.content, 2 map filter)
- All tests include non-ASCII/Unicode content (Cyrillic, German)
- Build: 1621 lib tests pass, 0 fail; all integration tests pass; clippy clean; fmt clean
- Files created: `src/template/filters/map.rs`
- Files modified: `src/collection.rs`, `src/template/context.rs`, `src/template/engine.rs`, `src/template/layout.rs`, `src/template/filters/mod.rs`
