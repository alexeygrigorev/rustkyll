# Issue #587: markdown="1" not processed on some translated aside blocks

## Problem

Some translated pages in opensource-guide have `<aside markdown="1">` blocks where the
`markdown="1"` attribute is NOT being stripped from the output and the content is NOT being
processed as markdown. The `<img>` and text content appear as raw HTML children of the
aside instead of being wrapped in `<p>` elements.

**Affected pattern (Bengali maintaining-balance page):**
```html
<aside markdown="1" class="pquote">
  <img src="..." class="pquote-avatar" alt="avatar">
  Bengali text here...

-- [@thisisnic](https://github.com/thisisnic) , description
</aside>
```

**Jekyll output:**
```html
<aside class="pquote">
  <p><img src="..." class="pquote-avatar" alt="avatar">
  Bengali text here...</p>
  <p>-- @thisisnic, description</p>
</aside>
```

**Rustkyll output:**
```html
<aside markdown="1" class="pquote">
  <img src="..." class="pquote-avatar" alt="avatar">
  Bengali text here...
<p>-- <a href="...">@thisisnic</a>, description</p>
</aside>
```

Key symptoms:
1. `markdown="1"` attribute is retained in HTML output (should be stripped)
2. Content is not processed as markdown (img and text not wrapped in `<p>`)
3. Markdown links within the block are partially processed but paragraph structure is wrong

**Note:** The English and some other translations work correctly -- the issue is specific
to certain translations (Bengali bn, possibly others) where the aside content structure
differs slightly (no nested `<p markdown="1">` wrapper for the credit line).

## Affected Sites

- **opensource-guide**: ~13+ pages in Bengali (bn) and similar translations with
  `<aside markdown="1">` blocks that are not processed. The DOM comparison shows 132
  instances of `aside > img` (actual) vs `aside > p > img` (expected) across multiple
  language translations.

## Root Cause

The `markdown="1"` block processing has an edge case where certain content patterns
inside `<aside>` blocks are not recognized for markdown processing. This may be related
to how the content is structured -- some translations don't have a nested
`<p markdown="1">` for the credit line, which may cause the parser to skip the block.

## Acceptance Criteria

- [ ] `markdown="1"` attribute is stripped from output HTML on `<aside>` elements
- [ ] Content inside `<aside markdown="1">` is processed as markdown
- [ ] `<img>` followed by text gets wrapped in `<p>` inside markdown="1" blocks
- [ ] Pages where markdown="1" already works correctly are not regressed
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM match count must not drop below 789/790

## Test Scenarios

### Unit: markdown="1" attribute stripping
- Input: `<aside markdown="1" class="test">content</aside>`
- Output must NOT contain `markdown="1"` in the aside tag

### Unit: Content processing inside markdown="1" aside
- Input: `<aside markdown="1"><img src="x" alt=""> Some text</aside>`
- Expected: `<aside><p><img src="x" alt=""> Some text</p></aside>`

### Unit: Markdown links processed inside aside
- Input: `<aside markdown="1">[@user](https://github.com/user)</aside>`
- Expected: `<aside><p><a href="...">@user</a></p></aside>`

### Integration: opensource-guide Bengali pages
- Build opensource-guide site
- Verify bn/maintaining-balance page aside blocks are processed
- Verify markdown="1" attribute is stripped from output

## Dependencies

None.

## DOM Baseline

- DTC: 789/790 matched
- opensource-guide: 212/390 matched, 1580 total diffs

## Log

### [PM] 2026-04-02 10:00
- Created from analysis of opensource-guide DOM diffs
- Root cause: markdown="1" processing edge case on certain translation patterns
- Bengali maintaining-balance page is a clear reproduction case
- The English version works correctly; only certain translations are affected
