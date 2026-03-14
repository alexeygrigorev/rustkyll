# Issue 54: Fix so-simple-theme site build

## Problem

so-simple-theme (66 pages) builds with Jekyll in 1.5s but fails with rustkyll. The build fails at template parse time with:

```
template parse error: liquid:  --> 5:27
  |
5 |   {% assign postsInYear = site.posts | group_by_exp: 'post', 'post.date | date: "%Y"' %}
  |                           ^---------------------------------------------------------^
  = unexpected FilterChain; expected FilterChain
```

The root cause is that the `group_by_exp` filter is not implemented. This filter takes a variable name and a Liquid expression (which may itself contain filter pipes), evaluates the expression for each array element, and groups elements by the result.

## Root Cause Analysis

### Primary blocker: missing `group_by_exp` filter

Used in `_layouts/posts.html` (lines 8 and 18):
```liquid
{% assign postsInYear = site.posts | group_by_exp: 'post', 'post.date | date: "%Y"' %}
```

This groups posts by year. The expression argument (`'post.date | date: "%Y"'`) contains a pipe character inside a quoted string, which is part of the expression to evaluate -- not a Liquid filter chain. The parser currently chokes because it tries to parse the inner pipe as a filter separator.

The existing `group_by` filter (in `src/template/filters/group_by.rs`) groups by a simple property name. `group_by_exp` is the expression-based variant, similar to how `where_exp` relates to `where`.

### Potential secondary blockers (investigate after fixing primary)

1. **`site.collections` iteration in `assets/js/search-data.js`**: Iterates `site.collections` as an array of collection objects with `.docs` property. Verify rustkyll exposes `site.collections` in this format.

2. **`c.docs | where_exp: 'doc', 'doc.search != false'`** in the same file: Uses `where_exp` (already implemented) but on collection docs. Depends on `site.collections` working.

3. **`slugify` filter** used in `_layouts/default.html`: `{{ page.title | slugify }}`. Verify this is registered.

## Scope

1. Implement the `group_by_exp` filter in `src/template/filters/`
2. Register it in the template engine
3. Fix any secondary blockers that emerge once the build gets past the parse error
4. Verify the site builds and output is correct

## Approach

### Implementing `group_by_exp`

The filter takes two string arguments: a variable name and a Liquid expression string. For each element in the input array, it:
1. Binds the element to the variable name
2. Evaluates the expression (which may contain Liquid filters like `date: "%Y"`)
3. Uses the result as the grouping key

The expression evaluation is the tricky part. The expression `post.date | date: "%Y"` must be evaluated as a mini Liquid template. The `where_exp` filter already has expression-evaluation infrastructure in `src/template/filters/where_exp.rs` (the `evaluate_expression` function), but `group_by_exp` needs to evaluate an expression that returns a value (not a boolean). This means the expression evaluation needs to handle filter chains (pipes), not just comparison operators.

Two implementation approaches:
- **Option A**: Render the expression as a Liquid output tag (`{{ expression }}`), using a temporary runtime context with the variable bound. This reuses the full Liquid engine for expression evaluation.
- **Option B**: Parse the expression manually, handling dotted paths and filter chains. More code, but avoids the overhead of a full template parse per element.

Option A is recommended -- it is simpler, more correct (handles all Liquid filters), and the performance cost is acceptable since `group_by_exp` is typically called on small arrays (dozens of posts, not thousands).

## Dependencies

None. All prerequisite filters (`where_exp`, `group_by`, `date`) are already implemented.

## Acceptance Criteria

- [ ] `group_by_exp` filter is implemented and registered in the template engine
- [ ] `group_by_exp` correctly handles expressions containing Liquid filter pipes (e.g., `'post.date | date: "%Y"'`)
- [ ] `group_by_exp` returns an array of objects with `name`, `items`, and `size` keys (same structure as `group_by`)
- [ ] `cargo run --release -- build --source websites/so-simple-theme` completes without errors
- [ ] Output page count is within 10% of Jekyll's 66 pages (allow variance from pagination/feed differences)
- [ ] No generated HTML file is empty (0 bytes)
- [ ] No raw Liquid tags (e.g., `{{`, `{%`) appear in any generated HTML file
- [ ] The posts-by-year page (`/posts/` or equivalent) renders with posts grouped by year, with year headings and post links
- [ ] `cargo build` compiles without errors or warnings (`-D warnings`)
- [ ] `cargo test` passes with all existing tests plus new tests for `group_by_exp`
- [ ] No regressions: all previously passing sites still build correctly

## Test Scenarios

### Unit: `group_by_exp` filter

- **Simple property grouping**: Group an array of objects by a direct property (e.g., `group_by_exp: 'item', 'item.category'`). Verify groups have correct `name`, `items`, and `size`.
- **Expression with filter chain**: Group by `'item.date | date: "%Y"'`. Verify items are grouped by year string.
- **All same group**: All items evaluate to the same key. Verify single group with all items.
- **Empty array**: Input is `[]`. Verify output is `[]`.
- **Non-array input**: Input is a scalar. Verify output is `[]`.
- **Missing property**: Some items lack the property referenced in the expression. Verify they are grouped under an empty-string key (or handled gracefully).
- **Multiple filter pipes**: Expression like `'item.name | downcase | strip'` with chained filters.

### Integration: so-simple-theme build

- **Full site build**: Run `cargo run --release -- build --source websites/so-simple-theme` and assert exit code 0.
- **Page count**: Count HTML files in output directory; verify at least 50 files generated.
- **No empty pages**: Assert every generated HTML file has non-zero size.
- **No raw Liquid**: Grep all generated HTML files for `{{` and `{%`; assert zero matches (excluding JavaScript that may contain these characters in string literals -- use a heuristic like checking outside `<script>` tags).
- **Posts-by-year structure**: Read the generated posts page HTML and verify it contains year headings (e.g., `<h2` elements with 4-digit year text) and post links within each year section.

### Regression: existing sites

- Run the full test suite (`cargo test`) and verify no failures.
- If there are integration tests for other sites, verify they still pass.

## Output Verification

After building, the engineer and tester must:

1. **File tree comparison**: Run `diff` or equivalent on the list of generated HTML files vs Jekyll output (if available). Same files should be generated.
2. **Structural spot-check**: For at least 3 representative pages (home, a post, the posts-by-year archive), verify the HTML contains expected elements: page title in `<title>`, navigation links, post content, year groupings on the archive page.
3. **Search data**: Verify `assets/js/search-data.js` (or equivalent) is generated and contains valid JSON-like data (not raw Liquid).
4. **No broken includes**: No "include file not found" errors in the build output.
