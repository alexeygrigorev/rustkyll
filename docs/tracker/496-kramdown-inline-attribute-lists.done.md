# Issue 496: Kramdown inline attribute lists {: .class :}

## Problem

Kramdown inline attribute lists (IALs) like `{: .mx-auto.d-block :}` are
rendered as literal text instead of being applied as HTML class attributes
on the preceding element.

Jekyll/kramdown: `<img class="d-block mx-auto" ...>`
Rustkyll: `<img ...>\n{: .mx-auto.d-block :}`

## Affected Sites

- beautiful-jekyll (1 page, sample-markdown)
- Potentially any site using kramdown IALs

## Scope

Parse `{: .class#id attr=val :}` patterns after HTML elements and apply
them as attributes. This is a kramdown-specific feature.

## Baseline

DTC 790/790. beautiful-jekyll 4/5. Must not regress.

## Log

### [SWE] 2026-03-29
- TDD: wrote 6 failing tests first (parse_ial_trailing_colon, parse_ial_dot_concatenated_classes, parse_ial_dot_concatenated_three_classes, inline_ial_on_img_element, inline_ial_on_img_classes_sorted, block_ial_with_trailing_colon)
- 5 tests FAILED as expected; 1 (block_ial_with_trailing_colon) already passed (trailing colon was handled by existing code for block IALs)
- Root causes found in src/kramdown.rs:
  1. `parse_ial_attributes`: `.mx-auto.d-block` parsed as single class instead of two; trailing `:` in `{: ... :}` not stripped
  2. `apply_attributes_to_last_tag`: only handled closing tags (`</tag>`), not self-closing void elements (`<img ... />`)
  3. `insert_attributes_at`: inserted attributes before `>` but not before ` />` for self-closing tags; classes not sorted alphabetically
- Implemented fixes:
  1. `parse_ial_attributes`: strip trailing `:` before parsing; split dot-concatenated classes (`.a.b.c` -> three separate classes)
  2. `apply_attributes_to_last_tag`: added fallback for self-closing/void elements (`<img />`, `<br />`)
  3. `insert_attributes_at`: sort class names alphabetically (matching kramdown); insert before ` />` for self-closing tags
- All 6 tests PASS
- Full lib test suite: 3118 passed, 0 failed, 2 ignored
- Clippy: clean (no warnings from our code)
- DOM: DTC 790/790, beautiful-jekyll 4/5 (no regression)
- Verified beautiful-jekyll sample-markdown page: `class="box-success"`, `class="d-block mx-auto"`, `class="box-note"`, `class="box-warning"`, `class="box-error"` all correctly applied
- Files modified: src/kramdown.rs
