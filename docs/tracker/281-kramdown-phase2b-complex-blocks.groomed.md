# Issue 281: Kramdown parser Phase 2b - Complex block elements (SPLIT)

## Status: Split into sub-issues

This issue was too large for a single implementation pass. It has been split into 4 sub-issues that should be implemented in order:

### Sub-issues

1. **281a - Lists** (`281a-kramdown-phase2b-lists.groomed.md`)
   - Ordered and unordered lists, nesting, lazy continuation, compact vs loose, block content in items, item IAL
   - 13 conformance tests in `block/08_list/`
   - No dependencies beyond Phase 2a (#280)

2. **281b - Tables** (`281b-kramdown-phase2b-tables.groomed.md`)
   - Pipe tables, headers, footers, alignment, escaping, code spans in cells, IAL
   - 6-8 conformance tests in `block/14_table/`
   - Independent of 281a (can be done in parallel)

3. **281c - HTML blocks, definition lists, math blocks** (`281c-kramdown-phase2b-html-blocks.groomed.md`)
   - Raw HTML blocks, parse_block_html option, markdown attribute, HTML comments, script/style
   - Definition lists with block content, IAL, auto IDs
   - Math blocks (display/inline, no-engine mode, XSS prevention)
   - 17 + 12 + 3 = 32 conformance tests
   - Depends on 281a (definition list `with_blocks` needs lists)

4. **281d - ALD, enhanced IAL, block extensions, TOC** (`281d-kramdown-phase2b-ald-ial-extensions-toc.groomed.md`)
   - ALD definitions and resolution, IAL with ALD references, class/id merging
   - Block extensions: comment, nomarkdown, options
   - TOC generation with auto IDs, level filtering, no_toc exclusion
   - 1 + 3 + 4 + 5 = 13 conformance tests
   - Depends on 281a (TOC replaces a list) and 281c (IAL on HTML blocks, parse_block_html option)

### Recommended implementation order

```
281a (Lists) ──────────┐
                       ├──> 281c (HTML/DefList/Math) ──> 281d (ALD/IAL/Ext/TOC)
281b (Tables) ─────────┘
```

281a and 281b can be done in parallel. 281c depends on 281a. 281d depends on 281a and 281c.

### Test cases with missing .html files (skip these)

- `block/08_list/brackets_in_item` -- no .html file
- `block/08_list/nested_compact` -- no .html file
- `block/09_html/standalone_image_in_div` -- no .html file
- `block/09_html/table` -- no .html file

### Test cases requiring features from later phases

- `block/14_table/table_with_footnote` -- requires footnote support (Phase 3 spans or later)
- `block/12_extension/options2` -- requires footnote support
- `block/12_extension/options3` -- requires syntax highlighting
- `block/16_toc/toc_with_footnotes` -- requires footnote support
- `block/04_header/` deferred tests -- auto IDs covered in 281d, header links need span-level link support

These should be tracked as deferred items. If not implementable in Phase 2b, follow-up issues must be created.

## Original scope (for reference)

All `.text`/`.html` pairs in categories 08-16 of `block/` test cases.

## Dependencies

Depends on Issue #280 (Phase 2a) being complete.
