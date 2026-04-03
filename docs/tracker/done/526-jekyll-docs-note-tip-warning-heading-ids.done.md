# Issue 526: jekyll-docs note/tip/warning box h5 headings have spurious id attributes

## Problem

On 7 jekyll-docs pages, `<h5>` elements inside note/tip/warning boxes have
extra `id` attributes that Jekyll does not produce. Jekyll's kramdown renderer
does NOT add `id` attributes to headings inside certain block-level containers
(like `.note`, `.warning` divs), but our renderer adds them unconditionally.

### Affected pages (at least 7 instances across multiple pages)

Specific h5 IDs found:
- `id='be-aware-of-directory-paths'`
- `id='there-are-several-unsupported-kramdown-options'`
- `id='remember-your-front-matter'`
- `id='diving-in'`
- `id='absolute-permalinks-will-be-required-in-v30-and-on'`
- `id='drafts-dont-have-dates'`
- `id='stay-up-to-date'`

### Example

Expected (Jekyll):
```html
<h5>Be aware of directory paths</h5>
```

Actual (rustkyll):
```html
<h5 id='be-aware-of-directory-paths'>Be aware of directory paths</h5>
```

## Root Cause

Kramdown's auto-ID generation for headings has exceptions. Headings inside
certain block-level elements (like `{: .note}` or `{: .warning}` annotated
blocks) do not get auto-generated IDs in Jekyll's kramdown. Our implementation
generates IDs for all headings unconditionally.

Alternatively, these `<h5>` elements may be generated from markdown syntax like
`##### Title` where kramdown strips the ID because it is a "ProTip" or "Note"
heading pattern in the jekyll-docs theme.

## Scope

Investigate when kramdown suppresses heading ID generation and replicate that
behavior. This may be:
1. Headings at certain levels (h5, h6) don't get auto-IDs
2. Headings inside block IAL-annotated containers don't get auto-IDs
3. A theme-specific pattern

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] h5 headings inside note/warning blocks do NOT have auto-generated id attributes
- [ ] Regular h1-h4 headings continue to have auto-generated id attributes
- [ ] DTC DOM match count must not drop below 790/790
- [ ] The 7 extra_attribute id diffs in jekyll-docs are resolved

## Test Scenarios

### Unit: Heading ID suppression

- h2 heading -> has auto-generated id attribute
- h5 heading inside note block -> no id attribute
- h5 heading outside note block -> investigate if it should have id

### Integration: jekyll-docs site

- Build jekyll-docs, verify troubleshooting page h5 elements have no id
- Run DOM comparison, verify improvement and no regression
