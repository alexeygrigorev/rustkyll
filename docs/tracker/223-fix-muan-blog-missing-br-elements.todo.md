# Issue 223: Fix muan-blog missing br elements

## Problem

~5 muan-blog note pages have missing `<br>` elements. Jekyll outputs `<br>` for line breaks in note content, but rustkyll doesn't. This may be from CommonMark hard line break handling or from how `\n` in content is rendered.

## Scope

1. Identify the specific pages and content patterns that produce `<br>` in Jekyll
2. Determine if this is a hard line break issue (two trailing spaces or backslash) or a different mechanism
3. Fix line break rendering to match Jekyll output

## Acceptance Criteria

- [ ] Hard line breaks in markdown produce `<br>` elements matching Jekyll
- [ ] ~5 affected muan-blog pages match Jekyll output
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Tests include content with hard line breaks

## Log

- 2026-03-18: Created from muan-blog comparison analysis.
