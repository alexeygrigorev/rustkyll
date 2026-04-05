# Issue #589: Nested task-list missing class attribute on inner `<ul>`

## Problem

When a task list contains nested task list items, Jekyll adds `class="task-list"` to
both the outer `<ul>` and the inner `<ul>`. Rustkyll only adds the class to the
outermost `<ul>`, leaving nested `<ul>` elements without the `task-list` class.

**Jekyll output:**
```html
<ul class="task-list">
  <li class="task-list-item"><input type="checkbox" disabled />Milk</li>
  <li class="task-list-item"><input type="checkbox" disabled checked />Cookies
    <ul class="task-list">
      <li class="task-list-item"><input type="checkbox" disabled checked />Classic</li>
      <li class="task-list-item"><input type="checkbox" disabled checked />Sourdough</li>
    </ul>
  </li>
</ul>
```

**Rustkyll output (broken):**
```html
<ul class="task-list">
  <li class="task-list-item"><input type="checkbox" disabled />Milk</li>
  <li class="task-list-item"><input type="checkbox" disabled checked />Cookies
    <ul>
      <li class="task-list-item"><input type="checkbox" disabled checked />Classic</li>
      <li class="task-list-item"><input type="checkbox" disabled checked />Sourdough</li>
    </ul>
  </li>
</ul>
```

Note: the inner `<li>` elements correctly have `task-list-item` class, but the
inner `<ul>` wrapper is missing `task-list`.

## Affected Sites

- **chirpy** (14/17): text-and-typography page has 1 DOM diff from this
- **minima** (6/9): junk/this-post-demonstrates-post-content-styles has 1 DOM diff from this
- Any site with nested checkbox/task lists

## Root Cause

The task-list class addition logic likely only processes the first/outermost `<ul>`
that contains task list items, rather than recursively processing all `<ul>` elements
that contain `<li class="task-list-item">` children.

## Acceptance Criteria

- [ ] Nested `<ul>` elements containing task-list-item children get `class="task-list"`
- [ ] Outermost `<ul>` still gets the class (no regression)
- [ ] Non-task-list `<ul>` elements are not affected
- [ ] Deeply nested task lists (3+ levels) all get the class
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM match count must not drop below 788/790
- [ ] Chirpy DOM total diffs decrease by at least 1
- [ ] Minima junk/this-post-demonstrates-post-content-styles matches

## Test Scenarios

### Unit: Nested task list class
- Task list with one level of nesting: both outer and inner `<ul>` get `class="task-list"`
- Task list with two levels of nesting: all three `<ul>` elements get the class
- Mixed: outer task-list with inner non-task `<ul>`: inner `<ul>` does NOT get the class
- Non-task `<ul>` with no checkboxes: no `task-list` class added

### Integration: Chirpy task list
- Build chirpy site, verify text-and-typography page nested task-list has correct classes

### Integration: Minima task list
- Build minima site, verify post-content-styles page nested task-list matches Jekyll

## Dependencies

None.

## DOM Baseline

- DTC: 788/790 matched
- Chirpy: 14/17 matched, 101 total diffs
- Minima: 6/9 matched, 210 total diffs

## Log
