# Issue 573: Kramdown tight list rendering -- single-paragraph items wrapped in `<p>`

## Problem

Kramdown renders list items that contain a single paragraph as "tight" (no `<p>` wrapping), while commonmark renders them as "loose" (with `<p>` wrapping) when there are blank lines between items or certain structural patterns. This causes widespread diffs across multiple sites.

### Concrete example

From `websites/homebrew-site/_posts/2024-07-30-homebrew-security-audit.md`, a nested list:

```markdown
1. Path traversal during file caching
   - Status: [Fixed](https://...)
```

**Jekyll/kramdown output:**
```html
<ol>
  <li>Path traversal during file caching
    <ul>
      <li>Status: <a href="...">Fixed</a></li>
    </ul>
  </li>
</ol>
```

**Rustkyll output:**
```html
<ol>
  <li>Path traversal during file caching
    <ul>
      <li><p>Status: <a href="...">Fixed</a></p></li>
    </ul>
  </li>
</ol>
```

The inner `<li>` in rustkyll has an extra `<p>` wrapper that kramdown does not produce.

### Also affects homebrew blog list items

From the 4.3.0 blog post, list items with links are wrapped in `<p>` in rustkyll but not in Jekyll, causing the `<a>` element to appear as a child of `<li>` directly in Jekyll but nested inside `<p>` in rustkyll.

## Affected Sites

- homebrew-site: 49 of 134 pages have diffs, many include this pattern
  - `2024/07/30/homebrew-security-audit/index.html` -- 37 differences (nested lists)
  - `2024/05/14/homebrew-4.3.0/index.html` -- 28 differences (list items with links)
  - Multiple blog posts with similar list structures
- hydeout: Some list rendering diffs on post pages
- Potentially any site with kramdown-style tight lists

## Root Cause

Issue 392 (kramdown br-aware list tightening) addressed list tightening for the `markdownify` pipeline, and issue 343 (kramdown partial loose list p-wrapping) handled some cases. However, the general case of kramdown tight list detection still differs from commonmark.

In kramdown, a list is tight (no `<p>` wrapping) when:
- List items contain only inline content (no block-level children)
- Even if there are blank lines between items (kramdown still renders tight in some cases where commonmark renders loose)

The current tightening heuristic in rustkyll may not cover nested list contexts or certain patterns that kramdown considers tight.

## Scope

- Analyze the specific patterns from homebrew-site and hydeout that produce loose lists in rustkyll but tight in kramdown
- Extend the kramdown list tightening logic to handle:
  - Single-line nested list items (inner `<li>` with just inline content)
  - List items where the only child is a paragraph containing inline elements
- Must not break DTC or other sites that currently match

## Baseline

- DTC: 789/790 matched (163 total diffs). Must not regress.
- Homebrew: 85/134 matched (4068 total diffs). Must improve.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new ones
- [ ] Nested list items with single inline content render without `<p>` wrapping
- [ ] Homebrew security-audit page nested lists match Jekyll output (no extra `<p>`)
- [ ] DTC DOM match count does not drop below 789/790
- [ ] Homebrew DOM match count improves from 85/134

## Test Scenarios

### Unit: Tight nested list rendering
- Input: `"1. Item\n   - Sub-item with [link](url)"` produces tight inner `<li>` (no `<p>`)
- Input: `"- Item\n  - Sub: [Fixed](url)"` produces tight inner `<li>`
- Input with blank lines between items: verify correct tight/loose determination

### Unit: Regression checks
- Verify DTC list rendering is unchanged
- Verify lists that SHOULD be loose (multiple paragraphs per item) remain loose

### Integration: Homebrew security audit page
- Build homebrew-site, check the security audit page nested list structure
- Verify `<li>Status: <a ...>Fixed</a></li>` (no `<p>` wrapper)

## Dependencies

Issues 343, 392 (kramdown list tightening) are done. This extends that work for additional patterns.
