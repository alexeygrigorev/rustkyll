# Issue 571: Kramdown TOC directive with ordered list marker not recognized

## Problem

The kramdown `{:toc}` directive generates a table of contents from page headings. It must follow a list item marker. Rustkyll's current implementation (issue 289) only recognizes the unordered list pattern `* TOC\n{:toc}` but does NOT recognize the ordered list pattern `1. TOC\n{:toc}` or the dash pattern `- TOC\n{:toc}`.

The just-the-docs theme exclusively uses the ordered list pattern:
```markdown
1. TOC
{:toc}
```

### Concrete example

Source (`websites/just-the-docs/docs/configuration.md`):
```markdown
## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}
```

**Jekyll output:** A properly generated `<ol id="markdown-toc">` with anchor links to all headings.

**Rustkyll output:** `<ol>\n <li>TOC\n    {:toc}\n</ol>` -- the `{:toc}` is rendered as literal text inside the list item.

## Root Cause

In `src/kramdown.rs`, `replace_toc_pattern_with_placeholder()` only matches the pattern when the list item starts with `*`:

```rust
if next_line.starts_with("{:toc") && next_line.ends_with('}') {
```

The preceding line check likely only matches `*` markers. The function needs to also match `1.`, `-`, and potentially other ordered list markers like `2.`, `3.`, etc.

## Affected Sites

- just-the-docs: Nearly every documentation page uses `1. TOC\n{:toc}` (at least 20 pages affected)
  - `docs/configuration/index.html` -- 159 differences
  - `docs/customization/index.html` -- 231 differences  
  - `docs/search/index.html` -- 105 differences
  - `docs/navigation/in-page/index.html` -- 31 differences (documents the TOC feature itself)
  - `docs/ui-components/code/index.html` -- 98 differences
  - Many more pages with TOC
- Any site using ordered list markers with `{:toc}`

## Scope

- Extend `replace_toc_pattern_with_placeholder()` to recognize `1. TOC\n{:toc}`, `- TOC\n{:toc}`, and `+ TOC\n{:toc}` patterns in addition to existing `* TOC\n{:toc}`
- The generated TOC should be an `<ol>` with `id="markdown-toc"` (matching Jekyll's output)
- Also handle the `{: .no_toc }` IAL on headings to exclude them from the generated TOC (this may already work from issue 289 but needs verification)

## Baseline

- DTC: 789/790 matched (163 total diffs). Must not regress.
- JTD: 16/47 matched (2063 total diffs). Must improve.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new ones
- [ ] `1. TOC\n{:toc}` produces a table of contents (same as `* TOC\n{:toc}`)
- [ ] `- TOC\n{:toc}` also produces a table of contents
- [ ] Generated TOC has `id="markdown-toc"` attribute
- [ ] TOC links use `#section-id` anchors matching heading IDs
- [ ] Headings with `{: .no_toc }` are excluded from the TOC
- [ ] DTC DOM match count does not drop below 789/790
- [ ] JTD pages with TOC render correctly (at least TOC structure present)

## Test Scenarios

### Unit: Ordered list TOC pattern recognition
- Input: `"1. TOC\n{:toc}"` is recognized and replaced with TOC placeholder
- Input: `"- TOC\n{:toc}"` is recognized and replaced with TOC placeholder
- Input: `"* TOC\n{:toc}"` still works (regression check)
- Input: `"1. TOC\n{:toc .custom-class}"` preserves custom class on generated TOC

### Unit: TOC generation from headings
- Content with `## Section A\n## Section B\n### Subsection B1` produces nested TOC with correct links
- Heading with `{: .no_toc }` is excluded from generated TOC

### Integration: JTD configuration page
- Build JTD site, check `docs/configuration/index.html`
- Verify `<ol id="markdown-toc">` exists with anchor links
- Verify `{:toc}` does not appear as literal text

## Dependencies

Issue 289 (kramdown TOC) is done. This extends it to handle ordered list markers.
Issue 570 (block IAL paragraph separation) is related but independent -- `{: .no_toc }` on headings may benefit from both fixes.
