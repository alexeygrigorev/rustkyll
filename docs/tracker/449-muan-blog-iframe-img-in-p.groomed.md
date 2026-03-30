# Issue 449: Block-level HTML elements (iframe, img) incorrectly wrapped in `<p>` tags

## Problem

When standalone `<iframe>` or `<img>` elements appear on their own line with blank lines
before and after in markdown content, the kramdown parser wraps them in `<p>` tags instead
of treating them as block-level HTML elements.

**Rustkyll output:**
```html
<p><iframe style="border: 0;" src="https://example.com" seamless>
<a href="...">Link</a></iframe></p>
```

**Expected (Jekyll/kramdown) output:**
```html
<iframe style="border: 0;" src="https://example.com" seamless>
<a href="...">Link</a></iframe>
```

Same issue for `<img>`:
```html
<!-- Rustkyll: wrapped in <p> -->
<p><img src="..." alt="..." style="..." /></p>

<!-- Jekyll: standalone block -->
<img src="..." alt="..." style="..." />
```

## Affected Pages (muan-blog)

### iframe wrapped in `<p>` (5 pages):
- `posts/acceptance.html` -- `<iframe>` bandcamp embed
- `posts/details-on-details.html` -- `<iframe>` embed
- `posts/leaving-github.html` -- `<iframe>` embed
- `posts/mission-focused.html` -- `<iframe>` YouTube embed
- `posts/noise.html` -- `<iframe>` embed

### img wrapped in `<p>` (2 pages, 4 diffs):
- `posts/presence.html` -- 3 standalone `<img>` tags with style attributes
- `posts/noise.html` -- 1 standalone `<img>` tag (also has iframe issue)

## Root Cause Analysis

The kramdown parser in `src/kramdown_parser/parser.rs` has the correct infrastructure:
- `iframe` IS in `HTML_BLOCK_TAGS` (line ~2816)
- `img` IS in `HTML_VOID_TAGS` (line ~2899)
- `try_parse_html_block()` (line ~3084) should catch both before paragraph parsing
- `is_html_block_start()` (line ~2855) should return `true` for both tags

The bug is likely in one of these areas:
1. **Content preprocessing**: The markdown content may be modified by Liquid template
   processing before reaching the kramdown parser, altering line structure or adding
   whitespace that prevents block detection.
2. **Frontmatter stripping**: The content after frontmatter removal may not have the
   expected blank line/newline structure.
3. **`img` not in `HTML_BLOCK_TAGS`**: While `img` is in `HTML_VOID_TAGS`, it is NOT in
   `HTML_BLOCK_TAGS`. The `try_parse_html_block` function should still handle it (it
   falls through to `parse_html_block_element` which checks `is_html_void_tag`), but
   there may be an early-exit code path that prevents this.
4. **GFM vs standard kramdown mode**: muan-blog does NOT specify `input: GFM`, using
   default kramdown mode. Behavior may differ between modes for HTML block detection.

**Key files to investigate:**
- `src/kramdown_parser/parser.rs` -- `try_parse_html_block()`, `parse_blocks_with_lazy()`,
  `is_html_block_start()`
- `src/kramdown_parser/html.rs` -- HTML block rendering
- `src/template/layout.rs` -- content pipeline (Liquid -> kramdown)
- `src/frontmatter.rs` -- frontmatter stripping

## Scope

Fix the kramdown parser so that standalone block-level HTML elements (`<iframe>`, `<img>`,
and similar void/block tags) on their own lines are NOT wrapped in `<p>` tags. The fix
must be generic (not specific to muan-blog).

## Dependencies

None -- this is a standalone parser fix.

## Baseline

- DTC: 790/790 (must not regress)
- muan-blog: 2199/2218 (expect to gain ~7 pages after fix)

## Acceptance Criteria

- [ ] `<iframe>` on its own line (with blank lines before/after) renders as block element, not wrapped in `<p>`
- [ ] `<img>` on its own line (with blank lines before/after) renders as block element, not wrapped in `<p>`
- [ ] `<iframe>` with attributes (src, style, width, height, frameborder, etc.) handled correctly
- [ ] `<img>` with attributes (src, alt, style) handled correctly
- [ ] `<iframe>` with content between tags (e.g., `<iframe ...><a>fallback</a></iframe>`) handled correctly
- [ ] Fix works in both GFM mode and default kramdown mode
- [ ] muan-blog DOM match count improves from 2199 to at least 2206 (7 pages fixed)
- [ ] DTC DOM baseline remains at 790/790
- [ ] No regression on any other test site
- [ ] `cargo test` passes with new tests
- [ ] Tests include non-ASCII/Unicode content (per project convention)

## Test Scenarios

### Unit: kramdown parser (src/kramdown_parser/)

#### Test: iframe block-level rendering
```
Input:
<iframe style="border: 0; width: 100%;" src="https://example.com/embed" seamless><a href="https://example.com">Fallback</a></iframe>

Some text after.
```
Expected: `<iframe` NOT preceded by `<p>`, and `</iframe>` NOT followed by `</p>`

#### Test: img block-level rendering
```
Input:
Some text before.

<img src="https://example.com/photo.jpg" alt="A photo" style="max-height: 20em;">

Some text after.
```
Expected: `<img` NOT preceded by `<p>`, and `/>` NOT followed by `</p>`

#### Test: iframe with YouTube embed attributes
```
Input:
<iframe width="240" height="140" src="https://www.youtube.com/embed/test" frameborder="0" allow="accelerometer; autoplay" allowfullscreen></iframe>
```
Expected: standalone `<iframe>` without `<p>` wrapper

#### Test: img with Unicode alt text
```
Input:
<img src="https://example.com/pic.jpg" alt="Ein Bild mit Umlauten: aou" style="display: block;">
```
Expected: standalone `<img>` without `<p>` wrapper

#### Test: inline img within paragraph (should stay in p)
```
Input:
Here is an image <img src="x.jpg"> in a paragraph.
```
Expected: `<p>Here is an image <img src="x.jpg" /> in a paragraph.</p>` (img stays inline)

#### Test: default kramdown mode (no GFM)
```
Config: no `input: GFM` specified
Input: standalone <iframe> and <img> on own lines
```
Expected: same block-level behavior as GFM mode

### Integration: muan-blog site build
- Build muan-blog with `./scripts/cargo-safe build`
- Run DOM comparison
- Verify `posts/acceptance.html` matches Jekyll (iframe not in p)
- Verify `posts/presence.html` matches Jekyll (img not in p)
- Verify `posts/mission-focused.html` matches Jekyll (iframe not in p)
- Count total matching pages: must be >= 2206/2218
- DTC site: rebuild and verify still 790/790
