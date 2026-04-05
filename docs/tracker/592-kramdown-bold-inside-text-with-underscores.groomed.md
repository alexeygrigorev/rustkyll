# Issue #592: Kramdown bold `**word**` rendered as `<em>*word*</em>` in certain contexts

## Problem

In kramdown mode, `**word**` (bold) is sometimes rendered as `<em>*word*</em>`
(italic with literal asterisks) instead of `<strong>word</strong>`. This happens
when bold markers appear near other emphasis markers or in complex inline formatting.

**Source markdown (chirpy getting-started):**
```markdown
In the **Source** section (under _Build and deployment_), select [**GitHub Actions**]...
```

**Jekyll output (correct):**
```html
In the <strong>Source</strong> section (under <em>Build and deployment</em>), select <a href="..."><strong>GitHub Actions</strong></a>...
```

**Rustkyll output (broken):**
```html
In the <em>*Source*</em> section (under <em>Build and deployment</em>), select <a href="..."><strong>GitHub Actions</strong></a>...
```

Note: `[**GitHub Actions**]` renders correctly as `<strong>`, but standalone
`**Source**` in the same paragraph renders as `<em>*Source*</em>`. This suggests
the double-asterisk parser is consuming only one `*` as emphasis opener, leaving
the other `*` as literal text.

## Affected Sites

- **chirpy** (14/17): getting-started page has this as 1 of 12 diffs
  (`<strong>` vs `<em>` mismatch on `**Source**`)
- Potentially any site with `**bold**` near `_italic_` in kramdown mode

## Root Cause

The kramdown emphasis parser likely has a precedence/nesting issue. When it
encounters `**Source**` in a paragraph that also contains `_italic_` markers,
it may be:
1. Treating the first `*` as an emphasis opener (for `_..._` style matching)
2. Then treating `*Source*` as italic content
3. Leaving `*` as literal around the word

The fix should ensure that `**` is always parsed as strong emphasis when it
appears as a matched pair, regardless of surrounding `_` emphasis markers.

## Acceptance Criteria

- [ ] `**word**` renders as `<strong>word</strong>` in all contexts
- [ ] `**word**` adjacent to `_italic_` text renders correctly
- [ ] `_italic **bold** italic_` renders as `<em>italic <strong>bold</strong> italic</em>`
- [ ] Existing emphasis handling not regressed (single `*`, `_`, `***` combos)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM match count must not drop below 788/790
- [ ] Chirpy getting-started page `**Source**` renders as `<strong>Source</strong>`

## Test Scenarios

### Unit: Bold near italic
- `In the **Source** section` -> `<strong>Source</strong>`
- `_Settings_ tab, **Source** section` -> `<em>Settings</em> tab, <strong>Source</strong> section`
- `_italic **bold** italic_` -> `<em>italic <strong>bold</strong> italic</em>`

### Unit: Bold in complex inline formatting
- `In the **Source** section (under _Build and deployment_)` -> both strong and em correct
- `[**GitHub Actions**](url)` -> link with `<strong>` (verify no regression)

### Unit: Edge cases
- `**bold** _italic_ **bold**` -> two `<strong>` and one `<em>`
- `*italic* **bold** *italic*` -> correct nesting
- `***bold italic***` -> `<strong><em>bold italic</em></strong>` or equivalent

### Integration: Chirpy getting-started
- Build chirpy site
- Verify posts/getting-started/index.html has `<strong>Source</strong>` (not `<em>*Source*</em>`)

## Dependencies

None.

## DOM Baseline

- DTC: 788/790 matched
- Chirpy: 14/17 matched, 101 total diffs

## Log
