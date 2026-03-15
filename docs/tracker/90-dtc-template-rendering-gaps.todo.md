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

## Specific Differences Found (from Issue 87 audit)

See `docs/audit/87-visual-parity-report.md` for full details.

### High Priority
- **D8**: Include output within markdown is being re-processed through markdown converter. Author links get wrapped in `<p>` tags, indented text becomes code blocks. Affects books.html, podcast.html, events.html, articles.html, tools.html.
- **D10**: `date_to_string` filter off by 1 day due to timezone handling. Affects books listing and book detail pages.
- **D18-D22**: feed.xml differences -- entry count (20 vs 10), missing `<subtitle>`, entity encoding vs CDATA, timezone differences.

### Medium Priority
- **D1**: Headings inside includes get auto-generated `id` attributes that Jekyll does not add.
- **D5**: Smart quote (curly vs straight) conversion differences.
- **D11**: `<ol start="N">` attribute added for non-1 list starts (Jekyll does not add this).
- **D13**: Podcast timestamp format for sub-minute times: `0.0` in Jekyll vs `0:00` in rustkyll.
- **D17**: HTML entity encoding differences (`&amp;` vs `&` in some contexts).

### Low Priority (no visual impact)
- **D2,D3,D12**: Boolean attribute formatting (`required=""` vs `required`, `<input />` vs `<input>`, `itemscope=""` vs `itemscope`).
- **D4,D6,D7,D16**: Whitespace, indentation, and blank line differences.
- **D14,D15**: JSON-LD date metadata uses different values.

## Acceptance criteria

- All Liquid templates produce identical output to Jekyll
- All include files render correctly
- All data file lookups work correctly
- No raw Liquid tags in any output
- No missing template-generated content
