# Issue 265: GFM table before non-pipe text should not render as table (kramdown compat)

## Problem

Descoped from issue 248. In kramdown, a pipe table (even with a `|---|---|` separator row) is NOT rendered as a table if it is not followed by a block boundary. For example:

```markdown
| A | B |
|---|---|
| 1 | 2 |
not a pipe
```

Kramdown renders this as a paragraph (plain text), not a table. But pulldown-cmark with the GFM tables extension renders it as a `<table>` followed by a `<p>`.

This causes false-positive table rendering on sites converted from kramdown.

## Root Cause

The `|---|---|` separator row triggers pulldown-cmark's built-in GFM table parsing, which happens AFTER the kramdown preprocessor runs. The preprocessor's `is_standard_pipe_table_context()` function correctly detects this as a GFM table and skips it, but pulldown-cmark then parses it as a table regardless of what follows.

Fixing this requires the preprocessor to detect GFM tables that are NOT followed by a block boundary and escape or remove the separator row so pulldown-cmark does not parse them as tables.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Input `| A | B |\n|---|---|\n| 1 | 2 |\nnot a pipe\n` does NOT produce `<table>` in output
- [ ] Input `| A | B |\n|---|---|\n| 1 | 2 |\n\ntext\n` (blank line after) still produces `<table>`
- [ ] Input `| A | B |\n|---|---|\n| 1 | 2 |\n` (EOF after) still produces `<table>`
- [ ] No regression on existing table tests (issues 200, 212, 248)

## Dependencies

- Issue 248 (done)

## Notes

This is a higher-risk change because it requires modifying content that pulldown-cmark's built-in GFM parser would normally handle. The separator row may need to be escaped (e.g., replace `|` with `\|`) or transformed to prevent GFM table detection while preserving the text content.
