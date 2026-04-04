# Issue 570: Kramdown block IALs not separating paragraphs

## Problem

In kramdown, a block-level inline attribute list (IAL) like `{: .label }` on a line by itself acts as BOTH a paragraph separator AND an attribute applicator. The preceding text becomes a paragraph with the specified class. Rustkyll's IAL implementation (issue 496) handles IALs that follow a blank line or are attached to HTML elements, but does NOT handle IALs that separate consecutive lines of text into distinct paragraphs.

### Concrete example

Source markdown (from `websites/just-the-docs/docs/index-test.md`):
```markdown
blue
{: .label .label-blue }
green
{: .label .label-green }
purple
{: .label .label-purple }
```

**Jekyll output:**
```html
<p class="label label-blue">blue</p>
<p class="label label-green">green</p>
<p class="label label-purple">purple</p>
```

**Rustkyll output:**
```html
<p class="label label-red">blue
{: .label .label-blue }
green
{: .label .label-green }
purple</p>
```

Rustkyll combines all lines into a single paragraph (since commonmark doesn't recognize `{: }` as paragraph separators), applies only the last IAL's classes, and renders the IAL syntax as literal text.

### Also affects font-size utilities

```markdown
Font size 1
{: .fs-1 }
Font size 2
{: .fs-2 }
```

Jekyll: separate `<p>` elements with different font size classes.
Rustkyll: single `<p>` with all text concatenated and `{: .fs-X }` as literal text.

## Affected Sites

- just-the-docs: 31 of 47 pages have diffs, many related to this pattern
  - `docs/index-test/index.html` -- 319 differences (labels, font sizes, colors)
  - `docs/ui-components/labels/index.html` -- 15 differences
  - `docs/ui-components/typography/index.html` -- 40 differences
  - `docs/utilities/typography/index.html` -- 28 differences
  - `docs/utilities/color/index.html` -- 4 differences
  - `docs/utilities/layout/index.html` -- 23 differences
- Any site using kramdown block IALs to style consecutive paragraphs

## Root Cause

The IAL processing in `src/kramdown.rs` runs AFTER markdown-to-HTML conversion. By then, commonmark has already merged the consecutive lines into a single paragraph. The `{: .class }` patterns inside the merged paragraph are then partially processed (the last one gets applied), but the paragraph splitting cannot happen post-conversion.

The fix must happen BEFORE markdown conversion: detect `{: .class }` patterns on their own lines and either:
1. Insert blank lines before them to force paragraph separation, then apply classes post-conversion
2. Pre-process the markdown to convert the pattern into HTML with correct classes directly

## Scope

- Pre-process kramdown block IALs that appear on their own line between text lines
- Split the text into separate paragraphs at IAL boundaries
- Apply the IAL classes to the preceding paragraph element
- Handle multiple consecutive IAL-separated paragraphs
- Handle IALs with multiple classes (e.g., `{: .label .label-blue }`)
- Handle IALs with other attributes (e.g., `{:style="counter-reset:none"}`)

## Baseline

- DTC: 789/790 matched (163 total diffs). Must not regress.
- JTD: 16/47 matched (2063 total diffs). Must improve.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new ones
- [ ] Markdown with `text\n{: .class }\n` produces `<p class="class">text</p>`
- [ ] Consecutive `text\n{: .class }` blocks produce separate `<p>` elements with correct classes
- [ ] IALs with multiple classes work: `{: .label .label-blue }` produces `class="label label-blue"`
- [ ] IALs with style attributes work: `{:style="counter-reset:none"}` produces `style="counter-reset:none"`
- [ ] JTD `docs/ui-components/labels/index.html` renders separate colored label paragraphs
- [ ] DTC DOM match count does not drop below 789/790
- [ ] JTD DOM match count improves from 16/47

## Test Scenarios

### Unit: Block IAL paragraph separation
- Input: `"blue\n{: .label .label-blue }\ngreen\n{: .label .label-green }"` produces two `<p>` with correct classes
- Input: `"text\n{: .fs-1 }"` produces `<p class="fs-1">text</p>`
- Input: `"text\n{:style=\"color:red\"}"` produces `<p style="color:red">text</p>`
- Input: `"no IAL here\n\nregular paragraph"` is unchanged (no false positives)

### Unit: Mixed IAL and regular paragraphs
- Input with blank-line-separated paragraphs mixed with IAL-separated ones
- Verify each paragraph gets correct treatment

### Integration: JTD label rendering
- Build JTD site, check `docs/ui-components/labels/index.html`
- Verify `<p class="label label-blue">blue</p>` exists in output

## Dependencies

Issue 496 (kramdown IALs) is done. This extends that work to handle the paragraph-separation case.
