# Issue #590: Markdown HTML block `<style>` tag incorrectly wrapped in `<p>`

## Problem

When a markdown file ends with (or contains) a standalone `<style>` block,
rustkyll wraps it in `<p>` tags. Jekyll correctly treats `<style>` as an HTML
block element and does NOT wrap it in `<p>`.

**Source markdown:**
```markdown
Sure `tuple` can exist.

<style>
  h2 + p { margin-top: -1.2em; font-size: .8em; }
  article ul { list-style: square; }
</style>
```

**Jekyll output (correct):**
```html
<p>Sure <code>tuple</code> can exist.</p>

<style>
  h2 + p { margin-top: -1.2em; font-size: .8em; }
  article ul { list-style: square; }
</style>
```

**Rustkyll output (broken):**
```html
<p>Sure <code>tuple</code> can exist.</p>

<p><style> h2 + p { margin-top: -1.2em; font-size: .8em; } article ul { list-style: square; } </style></p>
```

The `<style>` tag is a block-level HTML element per the CommonMark and kramdown specs.
It should never be wrapped in `<p>`.

## Affected Sites

- **muan-blog** (2214/2218): pages/hacking-with-swift has 1 DOM diff from this
  (`<style>` rendered as `<p>` instead of block element)
- Any site with `<style>` blocks in markdown content

## Root Cause

The markdown parser does not recognize `<style>` as an HTML block element that
should interrupt paragraph processing. The list of recognized HTML block tags
likely includes `<div>`, `<table>`, `<pre>`, etc. but is missing `<style>`.

Per CommonMark spec (section 4.6), the following tags start HTML blocks:
`<script>`, `<pre>`, `<style>`, `<textarea>` (type 1 blocks). These should
never be wrapped in paragraphs.

## Acceptance Criteria

- [ ] `<style>` blocks in markdown are NOT wrapped in `<p>` tags
- [ ] `<style>` blocks render as standalone HTML block elements
- [ ] `<script>` blocks similarly are not wrapped in `<p>` (if not already handled)
- [ ] Inline `<style>` references in text still work (e.g., "the style tag")
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM match count must not drop below 788/790
- [ ] Muan-blog hacking-with-swift page DOM diff decreases by 1

## Test Scenarios

### Unit: Style block recognition
- Markdown with `<style>...</style>` after a paragraph: style block is NOT in `<p>`
- Markdown with `<style>` on its own line: renders as block element
- Markdown with `<style>` at end of file: renders as block element, no trailing `<p>`

### Unit: Script block recognition
- Markdown with `<script>...</script>` after a paragraph: script block is NOT in `<p>`

### Unit: No false positives
- Text mentioning "style" without tags: normal paragraph processing
- Inline HTML like `<span style="color:red">`: not treated as block element

### Integration: Muan-blog hacking-with-swift
- Build muan-blog site
- Verify pages/hacking-with-swift/index.html has `<style>` as block element, not in `<p>`

## Dependencies

None.

## DOM Baseline

- DTC: 788/790 matched
- Muan-blog: 2214/2218 matched, 52 total diffs

## Log
