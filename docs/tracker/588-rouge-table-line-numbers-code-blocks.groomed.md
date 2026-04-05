# Issue #588: Rouge-style table line numbers in code blocks

## Problem

When kramdown is configured with `syntax_highlighter_opts.block.line_numbers: true`,
Jekyll/Rouge wraps code blocks in a `<table class="rouge-table">` structure with line
numbers in one column and code in the other. Rustkyll ignores this setting and outputs
code blocks without line number tables.

**Jekyll output (with line_numbers: true):**
```html
<div class="highlight"><code><table class="rouge-table"><tbody><tr>
  <td class="rouge-gutter gl"><pre class="lineno">1
2
3
</pre></td>
  <td class="rouge-code"><pre><span class="k">if</span> ...
</pre></td>
</tr></tbody></table></code></div>
```

**Rustkyll output (line_numbers ignored):**
```html
<div class="highlight"><code><span class="k">if</span> ...
</code></div>
```

## Affected Sites

- **chirpy**: 3 pages with code block diffs (getting-started, text-and-typography,
  customize-the-favicon). The text-and-typography page alone has 28 DOM differences,
  most from missing line number tables.
- Any site using kramdown `block.line_numbers: true` configuration

## Configuration

The setting comes from `_config.yml`:
```yaml
kramdown:
  syntax_highlighter_opts:
    block:
      line_numbers: true
      start_line: 1
    span:
      line_numbers: false
```

When `block.line_numbers` is true, ALL fenced code blocks and `{% highlight %}` blocks
get the table structure. When `span.line_numbers` is true (rare), inline code also gets
line numbers.

## Implementation Notes

The table structure is:
```html
<table class="rouge-table">
  <tbody>
    <tr>
      <td class="rouge-gutter gl">
        <pre class="lineno">1\n2\n3\n</pre>
      </td>
      <td class="rouge-code">
        <pre>...highlighted code...</pre>
      </td>
    </tr>
  </tbody>
</table>
```

The line count is derived from the number of newlines in the code. The `start_line`
config (default 1) sets the first line number.

This wrapping should be applied in the kramdown postprocessing or code block rendering
stage, after syntax highlighting has been applied.

## Acceptance Criteria

- [ ] When `kramdown.syntax_highlighter_opts.block.line_numbers` is true, code blocks are wrapped in rouge-table
- [ ] Line numbers match the actual number of lines in the code
- [ ] `start_line` config is respected (defaults to 1)
- [ ] When `block.line_numbers` is false or unset, code blocks render without tables (current behavior)
- [ ] `span.line_numbers` setting does NOT affect block code (and vice versa)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM match count must not drop below 789/790
- [ ] Chirpy DOM match count improves from 13/17

## Test Scenarios

### Unit: Table wrapping with line numbers
- Code block with 3 lines, line_numbers=true: produces rouge-table with lineno "1\n2\n3\n"
- Code block with 1 line: produces rouge-table with lineno "1\n"
- Code block with start_line=5 and 3 lines: produces lineno "5\n6\n7\n"

### Unit: No table when disabled
- Code block with line_numbers=false: no rouge-table, code rendered directly
- Code block with no line_numbers config: no rouge-table (default is false)

### Unit: Span vs block distinction
- Inline code with span.line_numbers=true, block.line_numbers=false: inline unaffected
- Block code with span.line_numbers=false, block.line_numbers=true: block gets table

### Integration: Chirpy code blocks
- Build chirpy site
- Verify text-and-typography page has rouge-table wrapped code blocks
- Verify line numbers match Jekyll output
- DOM match count improves from 13/17

## Dependencies

None.

## DOM Baseline

- DTC: 789/790 matched
- chirpy: 13/17 matched, 46 total diffs

## Log

### [PM] 2026-04-02 10:00
- Created from analysis of chirpy DOM diffs
- 28 of 46 total diffs on text-and-typography page are from missing line number tables
- Feature gap: kramdown line_numbers config is completely ignored
