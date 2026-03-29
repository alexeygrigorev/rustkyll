# Issue 505: Kramdown block IAL not applied in markdown="1" contexts

## Problem

Kramdown block-level Inline Attribute Lists (IALs) like `{: .label }` on a line after a paragraph are rendered as literal text instead of being applied as CSS classes to the preceding element. This was partially fixed in issue #496 (for inline IALs on images with dot-concatenated classes), but block IALs with space-separated classes inside `markdown="1"` divs are still broken.

### Example

Source markdown:
```markdown
<div class="code-example" markdown="1">
Default label
{: .label }

Blue label
{: .label .label-blue }
</div>
```

**Jekyll** (correct):
```html
<div class="code-example">
  <p class="label">Default label</p>
  <p class="label label-blue">Blue label</p>
</div>
```

**Rustkyll** (broken):
```html
<div class="code-example label">
  <p class="label label-red">Default label {: .label } Blue label {: .label .label-blue } ...</p>
</div>
```

### Affected Pages

- docs/ui-components/labels/index.html (22 diffs, 15 from this bug)

## Root Cause

Block IALs (`{: .class }` on their own line after a block element) are not being parsed and applied to the preceding element. The existing fix from #496 only handles inline IALs attached to img elements.

## Dependencies

- Issue #496 (done -- provides base IAL parsing)

## Baseline

- just-the-docs: 1/47 (or higher if #501-#504 are fixed first)
- DTC: 790/790 (must not regress)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `{: .label }` after a paragraph applies class `label` to the `<p>` element
- [ ] `{: .label .label-blue }` applies both classes
- [ ] Block IALs work inside `markdown="1"` div contexts
- [ ] DTC DOM baseline remains at 790/790
- [ ] Does not regress the inline IAL fix from #496

## Test Scenarios

### Unit: Block IAL parsing
- Paragraph followed by `{: .highlight }` -- verify class applied to `<p>`
- Paragraph followed by `{: .a .b .c }` -- verify all three classes applied
- IAL with id: `{: #my-id }` -- verify id attribute applied
- IAL inside `<div markdown="1">` -- verify it works in nested markdown context

### Integration: just-the-docs labels page
- Build just-the-docs, check labels page has `<p class="label">` elements
- Verify `{: .label }` text does not appear literally in output
