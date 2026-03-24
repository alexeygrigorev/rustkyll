# Issue 335: beautiful-jekyll sample-markdown page kramdown parity (5th page)

## Problem

Issue 331 brought beautiful-jekyll from 0/5 to 4/5 pages matching. The remaining page (`2020-02-28-sample-markdown/index.html`) has 30 DOM differences caused by deep kramdown features that rustkyll does not yet handle:

1. **Inline IAL on images**: `{: .mx-auto.d-block :}` -- kramdown applies CSS classes to the preceding `<img>` element via inline IAL. Rustkyll leaves the IAL as raw text in the output instead of applying it as a `class` attribute on the image.

2. **LaTeX display math**: kramdown converts `$$...$$` to `\(...\)` inline math notation. Rustkyll leaves `$$...$$` as-is.

3. **Syntax highlighting tables**: kramdown/Rouge emit syntax-highlighted code blocks as `<table>` elements with line numbers. Rustkyll emits `<span>` elements instead, producing a completely different DOM structure.

4. **Line break differences in paragraphs**: Text after `<br>` tags is structured differently -- Jekyll puts continuation text as children of the `<br>` element while rustkyll puts it as sibling text nodes.

## Source

Descoped from issue 331, acceptance criterion 5 ("beautiful-jekyll DOM match reaches 5/5").

## Scope

Fix the 30 remaining DOM differences in `websites/beautiful-jekyll/_site_jekyll_cached/2020-02-28-sample-markdown/index.html` to bring beautiful-jekyll to 5/5.

## Dependencies

- Issue 331 must be done first (provides the 4/5 baseline)
- May overlap with issue 294 (kramdown block-level interactions)

## Acceptance Criteria

- [ ] beautiful-jekyll DOM match reaches 5/5
- [ ] `2020-02-28-sample-markdown/index.html` has 0 DOM differences
- [ ] No regressions on any other site
- [ ] `./scripts/cargo-safe test` passes
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
