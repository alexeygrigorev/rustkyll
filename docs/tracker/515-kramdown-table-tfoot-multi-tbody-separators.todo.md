# Issue 515: Kramdown table tfoot and multi-tbody full-width separators

## Problem

Kramdown supports table body and footer separator rows that span the full width
without per-column pipes. When a kramdown table contains:

```markdown
| Header1 | Header2 | Header3 |
|:--------|:-------:|--------:|
| cell1   | cell2   | cell3   |
| cell4   | cell5   | cell6   |
|-----------------------------|
| cell1   | cell2   | cell3   |
| cell4   | cell5   | cell6   |
|=============================|
| Foot1   | Foot2   | Foot3   |
```

Jekyll/kramdown produces:
- A `<thead>` for the header row
- Two separate `<tbody>` sections (split by the `|-----|` separator)
- A `<tfoot>` section (after the `|=====|` separator)

Rustkyll renders the separator rows as literal cell content (`-----` and `=====`),
producing extra `<tr>` rows with dashes/equals signs instead of structural elements.

### Rustkyll output (wrong)

```html
<tbody>
  <tr><td>cell4</td><td>cell5</td><td>cell6</td></tr>
  <tr><td>-----</td><td></td><td></td></tr>
  <tr><td>cell1</td><td>cell2</td><td>cell3</td></tr>
  ...
  <tr><td>=====</td><td></td><td></td></tr>
  <tr><td>Foot1</td><td>Foot2</td><td>Foot3</td></tr>
</tbody>
```

### Expected output (Jekyll)

```html
<tbody>
  <tr><td>cell4</td><td>cell5</td><td>cell6</td></tr>
</tbody>
<tbody>
  <tr><td>cell1</td><td>cell2</td><td>cell3</td></tr>
  ...
</tbody>
<tfoot>
  <tr><td>Foot1</td><td>Foot2</td><td>Foot3</td></tr>
</tfoot>
```

## Affected Pages

- hydeout: `markup/2012/01/11/markup-html-elements-and-formatting.html` (8 of 77 diffs are from this)
- Potentially any kramdown site using full-width table separators

## Root Cause

The kramdown table parser (implemented in issue #281b) handles per-column separator
rows (`|---|---|---| `) but not full-width separators that span the entire table
width without individual column pipes (`|-----|` or `|=====|`). The full-width
format is valid kramdown syntax and must be recognized as a body/footer separator.

The key difference:
- Per-column: `|---|---|---|` (pipes between columns) -- already works
- Full-width: `|-----------------------------|` (single long dash/equals run) -- not recognized

## Scope

Fix the kramdown table separator detection to recognize full-width separator rows
(those matching `|[-]+|` or `|[=]+|` pattern) as body/footer separators, producing
the correct `<tbody>` splits and `<tfoot>` wrapping.

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] A kramdown table with `|----|` full-width separator produces two `<tbody>` sections
- [ ] A kramdown table with `|====|` full-width separator produces a `<tfoot>` section
- [ ] A kramdown table with both `|----|` and `|====|` produces correct multi-tbody + tfoot
- [ ] Per-column separator rows (`|---|---|---|`) continue to work (no regression)
- [ ] The `th` empty cell renders as `\xa0` (non-breaking space) matching Jekyll's `<th> </th>`
- [ ] DTC DOM match count must not drop below 790/790
- [ ] Hydeout DOM match count improves from 20/30

## Test Scenarios

### Unit: Full-width body separator

- Parse table with `|----|` between rows, verify two `<tbody>` sections produced
- Parse table with `|-----|` (longer), verify same result
- Parse table with leading/trailing spaces: `| --------- |`, verify recognized

### Unit: Full-width footer separator

- Parse table with `|====|` before last row, verify `<tfoot>` wrapping
- Parse table with `|=============================|` (long), verify same result

### Unit: Combined separators

- Parse the exact hydeout table (header + body separator + footer separator)
- Verify: 1 `<thead>`, 2 `<tbody>`, 1 `<tfoot>`

### Unit: No regression on per-column separators

- Parse `|---|---|---|` separator, verify it still works as header/body separator
- Parse `|:--|:--:|--:|` alignment separator, verify alignment preserved

### Unit: Empty header cell renders as non-breaking space

- Parse table with empty header cell, verify `<th> </th>` (contains `\xa0`)

### Integration: Hydeout site

- Build hydeout site, verify `markup-html-elements-and-formatting.html` table diff count decreases
- Run DOM comparison, verify no regression on other pages
