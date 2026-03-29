# Issue 520: jekyll-docs kramdown table cells missing paragraph wrapping

## Problem

On 9 jekyll-docs pages, kramdown table cells that contain multi-line or complex
content are missing `<p>` tag wrapping inside `<td>` elements. Jekyll's kramdown
renderer wraps table cell content in `<p>` tags when the cell contains block-level
content (paragraphs, multiple lines, inline code mixed with text). Our markdown
parser renders the cell content directly without paragraph wrapping.

This is the single largest fixable pattern by page count and diff count in
jekyll-docs, accounting for an estimated 1,300+ of the 10,481 total differences.

### Affected pages (9)

- docs/variables/index.html (282 diffs, all table-related)
- docs/liquid/filters/index.html (257 diffs, all table-related)
- docs/plugins/hooks/index.html (223 diffs, mostly table-related)
- docs/pagination/index.html (369 diffs, partially table-related)
- docs/structure/index.html (98 diffs, all table-related)
- docs/configuration/options/index.html (79 diffs, all table-related)
- docs/static-files/index.html (68 diffs, all table-related)
- docs/plugins/your-first-plugin/index.html (24 diffs, partially table-related)
- docs/security/index.html (6 diffs, partially table-related)

### Example

Markdown table cell:
```markdown
| `site` | Site wide information + configuration settings from `_config.yml`. See below for details. |
```

Expected (Jekyll/kramdown):
```html
<td><p>Site wide information + configuration settings from <code>_config.yml</code>. See below for details.</p></td>
```

Actual (rustkyll):
```html
<td><code>site</code></td>
<td>Site wide information + configuration settings from <code>_config.yml</code>. See below for details.</td>
```

The expected output wraps the entire cell content in a `<p>` tag. The actual output
places `<code>` and text as direct children of `<td>`.

## Root Cause

Kramdown wraps table cell content in `<p>` tags when the cell contains inline
elements mixed with text (what kramdown calls "block-level" cell content). Our
markdown table renderer always outputs cell content as inline children of `<td>`
without checking whether paragraph wrapping is needed.

The specific kramdown rule: if a table cell's content would be parsed as a
paragraph in a normal block context (i.e., it contains text that would form a
paragraph), it gets wrapped in `<p>`.

## Scope

Update the markdown table cell renderer to wrap cell content in `<p>` tags when
the content contains mixed inline elements and text, matching kramdown's behavior.

Key considerations:
- Simple cells with just a single code span should NOT get `<p>` wrapping
- Cells with text + inline code + more text SHOULD get `<p>` wrapping
- Cells with multiple paragraphs should get multiple `<p>` tags
- Empty cells should remain empty (no empty `<p>`)

## Dependencies

None.

## DTC DOM Baseline

- Current: 790/790
- Must not drop below: 790/790

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` produces no changes
- [ ] Table cells with mixed text + inline code are wrapped in `<p>` tags
- [ ] Simple table cells (single word, single code span) are NOT wrapped in `<p>`
- [ ] Empty table cells remain empty
- [ ] Multi-paragraph table cells produce multiple `<p>` tags
- [ ] DTC DOM match count must not drop below 790/790
- [ ] jekyll-docs configuration/options page table diffs significantly reduced
- [ ] jekyll-docs variables page table diffs significantly reduced
- [ ] jekyll-docs liquid/filters page table diffs significantly reduced

## Test Scenarios

### Unit: Table cell paragraph wrapping

- Cell with `Code text more` -> `<td><p><code>Code</code> text more</p></td>`
- Cell with single `code` -> `<td><code>code</code></td>` (no wrapping)
- Cell with just text `hello` -> `<td>hello</td>` or `<td><p>hello</p></td>` depending on kramdown rules
- Empty cell -> `<td></td>`

### Unit: Multi-paragraph table cells

- Cell with two paragraphs separated by blank line -> two `<p>` tags inside `<td>`

### Integration: jekyll-docs site

- Build jekyll-docs, compare docs/variables/index.html diff count (should drop significantly)
- Build jekyll-docs, compare docs/configuration/options/index.html diff count
- Run full DOM comparison, verify no regression on other pages
