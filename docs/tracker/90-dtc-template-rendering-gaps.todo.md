# Issue 90: Fix DTC template rendering gaps

## Problem

Some Liquid templates or includes on the DTC site may not render correctly with rustkyll. This could include:
- Complex Liquid logic (nested for loops, complex conditionals)
- Template variables not resolved correctly
- Include parameters not passed correctly
- Data file access patterns not supported
- Filters producing different output

## Goal

Every Liquid template and include used by the DTC site must produce identical output to Jekyll.

## Approach

1. Diff HTML output file-by-file between Jekyll and rustkyll
2. Identify template rendering differences
3. Trace to the specific Liquid code causing the difference
4. Fix the rendering engine

## Dependencies

- Issue 87 (visual parity audit) will identify the specific rendering gaps

## Acceptance criteria

- All Liquid templates produce identical output to Jekyll
- All include files render correctly
- All data file lookups work correctly
- No raw Liquid tags in any output
- No missing template-generated content
