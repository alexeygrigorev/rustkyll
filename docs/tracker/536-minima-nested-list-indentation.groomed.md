# Issue 536: Nested list sub-items break out of parent <li> element

## Problem

When rendering nested lists, rustkyll places the inner `<ul>` outside the parent `<li>`
instead of inside it. This breaks HTML structure and visual nesting.

### Example

Jekyll (correct -- nested `<ul>` inside `<li>`):
```html
<li>Do re mi
    <ul>
      <li>So la ti do</li>
      <li>Ba-da-bing!</li>
    </ul>
  </li>
```

Rustkyll (wrong -- nested `<ul>` after `<li>` closes):
```html
<li>Do re mi
<ul>
  <li>So la ti do</li>
  <li>Ba-da-bing!</li>
</ul>
</li>
```

Note: the indentation is also wrong (rustkyll does not indent the nested `<ul>` properly),
and the `<ul>` appears to break out of the `<li>` flow.

### Affected page

`junk/2016/05/20/this-post-demonstrates-post-content-styles.html` in minima (both
unordered and ordered nested lists affected).

## Root Cause

The kramdown/markdown list renderer does not properly nest inner lists within their
parent `<li>` elements. The closing of the parent `<li>` tag may be happening before
the nested list is rendered.

This may be related to existing nested list issues (362, 373) but is specifically about
the HTML nesting structure, not about block elements or paragraph wrapping.

## Dependencies

None.

## Scope

- Fix nested list HTML structure so inner `<ul>`/`<ol>` is inside parent `<li>`
- Fix indentation of nested list elements
- Verify both ordered and unordered nested lists

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes
- [ ] DTC DOM baseline: 790/790 (must not regress)
- [ ] Nested `<ul>` appears inside parent `<li>`, not after it
- [ ] Nested `<ol>` appears inside parent `<li>`, not after it
- [ ] Indentation matches Jekyll output (4-space indent for nested lists)
- [ ] At least 3 new unit tests

## Test Scenarios

### Unit: nested list structure
- Unordered list with nested unordered list -> inner `<ul>` inside `<li>`
- Ordered list with nested unordered list -> inner `<ul>` inside `<li>`
- Three levels of nesting -> all properly contained

### Integration: minima build
- Build minima, verify nested lists in `this-post-demonstrates-post-content-styles.html`

## Baselines

- DTC: 790/790
- Minima: this fix should eliminate ~10 diffs across 2 nested list sections
