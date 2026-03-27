# Issue 395: Auto-extract page title from first H1 heading

## Problem

When a markdown page has no `title` in its YAML frontmatter, Jekyll automatically
extracts the title from the first `<h1>` heading in the rendered content. Rustkyll
does not do this — `page.title` is undefined, causing templates that use
`{{ page.title | default: "fallback" }}` to show the fallback instead of the real title.

This affects the little-book-of-metals-ru site (43/48 DOM match, 5 pages differ)
where chapter README.md files have no frontmatter but start with `# Part Title`.

## Root Cause

`page_to_liquid()` in `src/generator.rs` (around line 757) only copies fields from
parsed frontmatter. It never inspects the rendered HTML content for an H1 heading
to use as the title.

## Scope

1. When a page has no `title` in frontmatter, extract the first `<h1>` text from
   the rendered content and set it as `page.title`
2. This must be generic Jekyll behavior, not site-specific

## Acceptance Criteria

- [ ] Pages without a `title` in frontmatter get their title from the first H1
- [ ] Pages WITH a `title` in frontmatter are NOT affected (frontmatter takes precedence)
- [ ] The extracted title is the text content of the H1, not the raw markdown or HTML tags
- [ ] `./scripts/cargo-safe test` passes with zero failures
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes clean
- [ ] little-book-of-metals-ru DOM improves from 43/48 toward 48/48
- [ ] DTC DOM does not regress (787/790 baseline)
- [ ] No other site regresses

## Test Scenarios

### Unit tests
- Markdown with `# My Title` and no frontmatter → `page.title` = "My Title"
- Markdown with frontmatter `title: "Explicit"` and `# Different` → `page.title` = "Explicit"
- Markdown with no H1 and no frontmatter title → `page.title` is nil/undefined
- H1 with inline formatting `# **Bold** Title` → `page.title` = "Bold Title" (text only)
- Multiple H1s → use the first one

### Integration
- Build little-book-of-metals-ru and verify DOM improvement
- Build DTC and verify no regression

## Log

### [SWE] 2026-03-27
- Added `extract_title_from_h1()` function in `src/generator.rs` (line ~755)
  - Uses regex to find first `<h1>` tag, strips inner HTML tags, decodes common entities
  - Uses `OnceLock` for compiled regex caching
- Integrated into `page_to_liquid()`: after content is set, if frontmatter has no "title" key, extracts title from first H1 in html_content
- Tests added: 10 unit tests
  - 7 tests for `extract_title_from_h1`: simple, inline formatting, no H1, multiple H1s, id attribute, unicode, whitespace trimming
  - 3 tests for `page_to_liquid`: no-frontmatter-title extracts H1, frontmatter title takes precedence, no H1 no title
- All 10 new tests pass; 17 pre-existing failures (frontmatter, kramdown, markdownify -- unrelated)
- `cargo fmt` clean
- Clippy: pre-existing compilation error in unrelated code (`add_inline_code_class_to_kramdown_output` -- not my change)
- little-book-of-metals-ru DOM: 43/48 (unchanged -- the 5 remaining diffs are about navigation link text using a different title from `_config.yml` defaults, not about missing page.title)
- DTC DOM: 744/790 (same as baseline without my changes -- no regression)
- Files modified: `src/generator.rs`
