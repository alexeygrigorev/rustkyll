# Issue 547: Liquid capture tag whitespace preservation

## Problem

The `{% capture %}` tag in rustkyll trims leading and trailing whitespace from the captured content. Jekyll preserves all whitespace inside `{% capture %}...{% endcapture %}` blocks verbatim.

This causes mismatches on any site that captures multi-line HTML and then applies `strip_newlines` to produce a single-line string. The most visible case is chirpy's `search-loader.html`:

```liquid
{% capture result_elem %}
  <article class="px-1">hello</article>
{% endcapture %}
searchResultTemplate: '{{ result_elem | strip_newlines }}'
```

**Jekyll output:** `searchResultTemplate: '  <article class="px-1">hello</article>  '`
(newlines removed, but the leading/trailing spaces on lines are preserved)

**Rustkyll output:** `searchResultTemplate: '<article class="px-1">hello</article>'`
(all leading/trailing whitespace stripped by capture, then strip_newlines is a no-op)

## Impact

This single bug causes 12 out of 17 chirpy pages to show a 1-diff mismatch (the search script text). Fixing it would push chirpy from 0/17 matched to 12/17 matched.

Also likely affects other sites that use `{% capture %}` with multi-line content.

## Root Cause

The liquid-rust library or rustkyll's Liquid engine trims whitespace from `{% capture %}` block content. Jekyll's Ruby Liquid implementation does not trim -- it captures the raw string between the tags.

## Scope

- Fix the `{% capture %}` tag to preserve all whitespace (including leading/trailing spaces and newlines) inside the block, matching Jekyll's behavior
- This is a Liquid engine fix, not a filter fix -- `strip_newlines` likely works correctly already

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all existing tests plus new ones
- [ ] `{% capture foo %}  hello  {% endcapture %}{{ foo }}` outputs `  hello  ` (with leading/trailing spaces preserved)
- [ ] `{% capture foo %}\n  <article>hello</article>\n{% endcapture %}{{ foo | strip_newlines }}` outputs `  <article>hello</article>` (newlines removed but spaces preserved)
- [ ] Chirpy DOM comparison: at least 12/17 pages match (up from 0/17)
- [ ] DTC DOM match count must not drop below 790/790

## Test Scenarios

### Unit: Capture whitespace preservation
- Capture block with leading/trailing spaces: verify spaces preserved in output
- Capture block with multi-line content: verify newlines preserved before any filter
- Capture block with `strip_newlines` filter: verify only `\n`/`\r` removed, spaces kept
- Capture block with content on same line as tags: `{% capture x %} hi {% endcapture %}` preserves ` hi `

### Integration: Chirpy search template
- Build chirpy site, verify `searchResultTemplate` value starts with `  <article` (two leading spaces)
- Verify chirpy 404.html matches Jekyll output for the script block
- Run DOM comparison on chirpy, verify at least 12/17 pages match

## Dependencies

None.

## DTC Baseline

790/790 matched (must not regress)

## Log
