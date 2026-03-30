# Issue 443: jekyll-vitepress-theme rendering issues (0/17 pages match)

## Problem

The jekyll-vitepress-theme site has 0/17 pages matching Jekyll output. Every single page
has at least 6 common diffs, and content pages have many more (up to 86 diffs per page).
There are four distinct root causes:

### 1. Missing `<style>` tag -- Ruby hook generates Rouge CSS (all 17 pages, 4 diffs each)

The theme uses a Ruby hook (`lib/jekyll/vitepress_theme/hooks.rb`, `RougeStyles.apply`)
that generates syntax highlighting CSS at build time using the Rouge gem. This produces a
`<style>` tag in the `<head>` with light/dark theme CSS. Rustkyll does not execute Ruby
hooks, so this `<style>` tag is missing.

Additionally, the `<head>` contains 3 `<script>` tags that are present in Jekyll but
missing in rustkyll output. These come from includes that reference theme-generated data.

**DOM diff pattern (every page):**
```
head > child[15]: tag_name_differs - expected: 'style', actual: 'script'
head > script: missing_element (x3)
```

### 2. Version string 'auto' not resolved to 'v1.1.1' (all 17 pages, 2 diffs each)

The theme's `_data/versions.yml` has `current: auto`. A Ruby hook
(`VersionLabel.apply` in `hooks.rb`) resolves `auto` to the gem version string
(`v#{Jekyll::VitePressTheme::VERSION}` = `v1.1.1`). Since rustkyll cannot execute
Ruby code, the version stays as the literal string `auto`.

**DOM diff pattern (every page):**
```
nav > button > span > span: text_differs - expected: 'v1.1.1', actual: 'auto'
button > span: text_differs - expected: 'v1.1.1', actual: 'auto'
```

### 3. Content structural diffs -- elements out of order (content pages, ~10-80 diffs each)

On pages with markdown content, the child elements of `<main>` are in wrong order.
Pattern is consistently: `expected: 'h1', actual: 'p'` then `expected: 'p', actual: 'h2'`
etc. This suggests the first content element is being rendered as a paragraph when it
should be a heading, causing all subsequent elements to shift.

**DOM diff pattern:**
```
main > div > child[1]: tag_name_differs - expected: 'h1', actual: 'p'
main > div > child[2]: tag_name_differs - expected: 'p', actual: 'h2'
main > div > child[3]: tag_name_differs - expected: 'h2', actual: 'p'
...
```

This may be related to how the theme's layout handles page titles vs content headings,
or an issue with the markdown rendering pipeline for this theme.

### 4. Syntax highlighting token class differences (code-heavy pages)

Code block `<span>` classes differ between Jekyll/Rouge and rustkyll/syntect token mapping.
This is partially covered by #471 (in-progress) but may have theme-specific aspects since
vitepress uses custom Rouge theme names (`github`, `github.dark`).

## Root Cause Analysis

**Root cause 1 and 2** are Ruby hook behaviors that rustkyll fundamentally cannot replicate
without special theme-specific support. Possible approaches:
- (a) Accept these diffs as "theme requires Ruby hooks" and exclude from matching
- (b) Implement a generic "theme hooks" shim that reads gem version from `Gemfile.lock`
  and injects it into data
- (c) Implement a `_data` preprocessing step that resolves known patterns like `auto`

**Root cause 3** needs investigation. The structural shift pattern (h1->p, p->h2, etc.)
suggests either:
- A Liquid include is generating content that pushes elements
- The page title is being rendered differently
- An unresolved include is generating raw text that becomes a paragraph

**Root cause 4** is partially covered by #471 syntax highlighting work.

**Key files:**
- `src/template/engine.rs` -- Liquid template rendering
- `src/template/layout.rs` -- page rendering pipeline
- `src/kramdown_parser/parser.rs` -- markdown parsing
- `websites/jekyll-vitepress-theme/_includes/` -- theme includes
- `websites/jekyll-vitepress-theme/lib/jekyll/vitepress_theme/hooks.rb` -- Ruby hooks (reference)

## Scope

Investigate and fix the content structural diffs (root cause 3). Document the Ruby-hook
dependent issues (root causes 1 and 2) and decide whether they should be accepted as
theme limitations or addressed via a generic mechanism.

For root cause 4, depend on #471.

## Dependencies

- Issue #471 (syntax highlighting token mismatches) -- for root cause 4

## Baseline

- DTC: 790/790 (must not regress)
- jekyll-vitepress-theme: 0/17 pages match, 354+ total diffs

## Acceptance Criteria

- [ ] Root cause 3 (content structural diffs) investigated and root cause identified
- [ ] If fixable: structural diffs fixed, child elements in correct order on content pages
- [ ] If not fixable: documented why, with follow-up issue created
- [ ] Root causes 1 and 2 (Ruby hooks) documented with clear decision:
  - Either accepted as theme limitation with rationale, OR
  - Follow-up issue created for generic version/style resolution
- [ ] jekyll-vitepress-theme match count improved from 0/17 (target: at least 1/17 for index.html after fixing structural issues)
- [ ] DTC DOM baseline remains at 790/790
- [ ] No regression on any other test site
- [ ] `cargo test` passes with new tests
- [ ] Tests include non-ASCII content

## Test Scenarios

### Investigation: content structural diffs
- Compare Jekyll vs rustkyll output for `index.html` (simplest page, 9 diffs)
- Identify what generates the `<p>` where `<h1>` is expected on content pages
- Check if an unresolved Liquid include produces raw text
- Check if the theme's layout file has special title handling

### Unit tests (if structural fix identified)
- Render a simple page with the vitepress default layout
- Verify heading elements appear in correct order
- Verify no spurious `<p>` elements wrapping headings

### Integration: site build
- Build jekyll-vitepress-theme with `./scripts/cargo-safe build`
- Run DOM comparison
- Document diff count per page before and after fix
- Compare `index.html` output line-by-line with Jekyll cached output
- Verify DTC still at 790/790
- Verify no regression on other test sites (check at least chirpy, muan-blog)
