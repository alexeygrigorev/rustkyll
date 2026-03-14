# Issue 73: Kramdown compatibility gaps

## Problem

Visual comparison (issue #72) identified several markdown rendering differences between Jekyll (kramdown) and rustkyll (pulldown-cmark) that cause 1.8-2.9% pixel differences on DTC site pages.

## Root Causes

### 1. Inline attribute syntax `{:target="_blank"}`
Jekyll/kramdown supports `{:target="_blank"}` after links to add HTML attributes. Rustkyll outputs the raw syntax as visible text.

Affected pages: homepage, books, events (all pages using `{:target="_blank"}` in markdown)

### 2. Auto-generated heading IDs
Jekyll/kramdown generates `id` attributes on headings (e.g., `<h2 id="upcoming-books">`). Pulldown-cmark does not generate IDs by default.

Pulldown-cmark supports heading IDs via the `HEADING_ATTRS` extension. Alternatively, a post-processing step could add IDs based on heading text (slugified).

### 3. Code element class differences
Jekyll/kramdown adds `class="language-plaintext highlighter-rouge"` to inline `<code>` elements. Pulldown-cmark uses plain `<code>`.

This is mostly a cosmetic HTML difference but can affect CSS styling if styles target these classes.

### 4. Paragraph spacing
Jekyll/kramdown outputs extra blank lines between `<p>` tags. Pulldown-cmark does not. This causes minor visual spacing differences.

## Scope

This is a markdown parser compatibility issue. Solutions may include:
- Adding pulldown-cmark extensions for heading attributes
- Post-processing HTML to add heading IDs
- Implementing a kramdown attribute parser for `{:attr}` syntax
- These are significant changes that should be done carefully

## Dependencies

- Issue 72 (visual comparison investigation) -- done

## Reference

See `docs/comparison/visual-results.md` for detailed pixel diff data and screenshots.
