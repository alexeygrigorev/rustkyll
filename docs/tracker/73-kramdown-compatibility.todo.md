# Issue 73: Fix kramdown compatibility gaps

## Priority

HIGH — these differences mean rustkyll output does NOT match Jekyll. Every difference must be fixed, not tolerated.

## Problem

Visual comparison (issue #72) found markdown rendering differences between Jekyll (kramdown) and rustkyll (pulldown-cmark) causing 1.8-2.9% pixel diffs on DTC pages. These are NOT acceptable — if Jekyll produces `target="_blank"` on a link, rustkyll must produce the same.

## What must be fixed

### 1. Inline attribute syntax `{:target="_blank"}`

Jekyll/kramdown supports `{:target="_blank"}` after links to add HTML attributes. Rustkyll currently outputs the raw `{:target="_blank"}` as visible text — this is broken, not just cosmetic.

Must support at minimum:
- `{:target="_blank"}` on links
- `{:.class-name}` for CSS classes
- `{:#id-name}` for IDs
- Multiple attributes: `{:target="_blank" rel="noopener"}`

### 2. Auto-generated heading IDs

Jekyll/kramdown generates `id` attributes on headings (e.g., `<h2 id="upcoming-books">`). Rustkyll does not.

Must generate slugified IDs matching kramdown's algorithm (lowercase, spaces to hyphens, strip non-alphanumeric).

### 3. Code element class differences

Jekyll/kramdown adds `class="language-plaintext highlighter-rouge"` to inline `<code>` elements. This affects CSS styling.

Must match kramdown's class output.

### 4. Paragraph spacing

Jekyll/kramdown outputs extra blank lines between `<p>` tags. Must match to achieve pixel-perfect output.

## Goal

After fixing all 4 issues, re-run the Playwright visual comparison. Target: 0% pixel diff on all pages (or document why specific differences remain).

## Dependencies

- Issue 72 (visual comparison) done

## Acceptance criteria

- `{:target="_blank"}` renders as `target="_blank"` attribute on the link (not visible text)
- `{:.class-name}` renders as `class="class-name"` on the element
- `{:#id-name}` renders as `id="id-name"` on the element
- All headings have auto-generated `id` attributes matching kramdown's slugification
- Inline code has `class="language-plaintext highlighter-rouge"`
- Paragraph spacing matches Jekyll output
- Playwright visual comparison re-run with updated pixel diff numbers
- DTC pages that were 1.8-2.9% must be <0.5% after fixes
- All existing tests still pass
- Results documented in docs/comparison/visual-results.md (updated)

## Reference

See `docs/comparison/visual-results.md` for pixel diff data and screenshots.
