# Issue 535: GFM task list checkboxes not rendered as HTML input elements

## Problem

Markdown task lists (GFM extension) are not rendered as interactive checkbox elements.
Instead, the `[ ]` and `[x]` markers are left as plain text.

### Example

Source markdown:
```markdown
- [ ] Milk
- [x] Cookies
  - [x] Classic Choco-chip
```

Jekyll (correct):
```html
<ul class="task-list">
  <li class="task-list-item"><input type="checkbox" class="task-list-item-checkbox" disabled="disabled" />Milk</li>
  <li class="task-list-item"><input type="checkbox" class="task-list-item-checkbox" disabled="disabled" checked="checked" />Cookies
    <ul class="task-list">
      <li class="task-list-item"><input type="checkbox" class="task-list-item-checkbox" disabled="disabled" checked="checked" />Classic Choco-chip</li>
    </ul>
  </li>
</ul>
```

Rustkyll (wrong):
```html
<ul>
  <li>[ ] Milk</li>
  <li>[x] Cookies
<ul>
  <li>[x] Classic Choco-chip</li>
</ul>
</li>
</ul>
```

### Affected page

`junk/2016/05/20/this-post-demonstrates-post-content-styles.html` in minima

## Root Cause

Rustkyll's kramdown/markdown processor does not implement the GFM task list extension.
Jekyll uses kramdown with the GFM parser which includes task list support via the
`kramdown-parser-gfm` gem. This converts `[ ]` to unchecked checkboxes and `[x]` to
checked checkboxes, and adds `task-list` / `task-list-item` CSS classes.

## Dependencies

None.

## Scope

- Implement GFM task list checkbox rendering in the markdown processor
- Convert `[ ]` to `<input type="checkbox" class="task-list-item-checkbox" disabled="disabled" />`
- Convert `[x]` to `<input type="checkbox" class="task-list-item-checkbox" disabled="disabled" checked="checked" />`
- Add `class="task-list"` to parent `<ul>`
- Add `class="task-list-item"` to parent `<li>`

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes
- [ ] DTC DOM baseline: 790/790 (must not regress)
- [ ] Task list items render as `<input type="checkbox">` elements
- [ ] Checked items have `checked="checked"` attribute
- [ ] All checkboxes have `disabled="disabled"` attribute
- [ ] Parent `<ul>` has `class="task-list"`
- [ ] Parent `<li>` has `class="task-list-item"`
- [ ] Nested task lists work correctly
- [ ] At least 4 new unit tests

## Test Scenarios

### Unit: task list parsing
- `- [ ] unchecked` -> unchecked checkbox input
- `- [x] checked` -> checked checkbox input
- Mixed task list and regular list items
- Nested task lists

### Integration: minima build
- Build minima, verify task list in `this-post-demonstrates-post-content-styles.html`
  has checkbox inputs with correct classes

## Baselines

- DTC: 790/790
- Minima: this fix should eliminate ~10 diffs on the affected page
