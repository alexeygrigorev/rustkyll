# Issue 215: Graceful handling of unknown Liquid tags

## Origin

Descoped from issue 197 (fix Liquid comparison type errors). Sites using Jekyll plugins with custom tags (e.g., `octicon` from jekyll-octicons) cause hard parse errors.

## Problem

When a Jekyll site uses a plugin-provided custom tag like `{% octicon mark-github height:24 %}`, the Liquid parser produces an "Unknown tag" error and the page fails to render entirely. This affects government-github (8 pages using `octicon` in `_includes/footer.html`).

Rather than implementing every possible Jekyll plugin tag, rustkyll should handle unknown tags gracefully -- skip the tag content and emit a warning, allowing the rest of the page to render.

## Requirements

- When the Liquid parser encounters an unknown tag, emit a warning (not an error) and skip the tag
- The rest of the template should continue to render normally
- The warning should include the tag name and source file for debugging
- This applies to both inline tags (`{% tagname ... %}`) and block tags (`{% tagname %}...{% endtagname %}`)

## Dependencies

None.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with new tests
- [ ] Unknown inline tags are skipped with a warning (not a fatal error)
- [ ] Unknown block tags are skipped with a warning (not a fatal error)
- [ ] The rest of the template renders correctly around the skipped tag
- [ ] Warning message includes the unknown tag name
- [ ] government-github pages render (with warnings for `octicon` but no fatal errors)
