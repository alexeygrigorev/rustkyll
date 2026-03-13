# Issue 26: Include Parameters

## Problem

Jekyll's `{% include file.html param="value" %}` creates an `include` object accessible inside the included file via `include.param` (dot notation) or `include["param"]` (bracket notation). Rustkyll uses the `liquid-lib` crate's `jekyll::IncludeTag` which already supports basic `include.param` access with string-quoted parameters. However, there are gaps:

1. **Bracket notation (`include["param"]`)** -- The Liquid parser may not handle `include["max_posts"]` style access. Jekyll treats this identically to `include.max_posts`. This syntax is commonly used in the wild (mentioned in issue #22 research).
2. **Numeric parameter values** -- Parameters like `max_posts=5` (unquoted integer) should be available as numbers, not strings, so that Liquid's `default` filter and arithmetic work correctly.
3. **Boolean parameter values** -- Parameters like `show_header=true` or `show_header=false` should be available as booleans.
4. **Variable references** -- `{% include file.html authors=page.authors %}` should resolve `page.authors` from the current context and pass it into the include. This appears to work already based on existing tests in `layout.rs`.

The DTC site uses `{% include related-posts.html manual_posts=page.related_posts max_posts=5 %}` which accesses `include.max_posts` and `include.manual_posts` inside `related-posts.html`.

## Scope

### In scope

- Ensure `include.param` dot-notation access works for all parameter types (string, number, boolean, variable reference)
- Ensure `include["param"]` bracket-notation access works identically to dot notation
- Ensure unquoted numeric values (`max_posts=5`) are passed as numbers
- Ensure unquoted boolean values (`show=true`, `show=false`) are passed as booleans
- Ensure variable references (`key=page.variable`) resolve from the calling context
- Ensure parameters with special characters in values work (`msg="hello & goodbye"` -- already tested)
- Ensure nested includes can pass parameters through (`{% include outer.html x=include.x %}`)
- All existing tests must continue to pass

### Out of scope

- Dynamic include filenames (`{% include {{ page.include_name }} %}`) -- advanced feature, very rare
- Include-relative paths (`{% include_relative file.html %}`) -- separate tag
- Parameters with expressions (`key={{ foo | upcase }}`) -- not valid Jekyll syntax

## Dependencies

- Issue #08 (layout and includes) -- DONE

## Implementation Notes

- The `liquid-lib` crate (v0.26) with the `jekyll` feature provides `IncludeTag` which handles parameter parsing. The current tests in `layout.rs` show that `include.param` works for string and variable params.
- The `LenientObject` wrapper in `engine.rs` already handles `include` as a special case for lenient key access (lines 204-205, 301).
- The main work may be:
  1. Verifying that the `liquid` crate's `IncludeTag` already passes numeric/boolean values correctly or if we need to handle conversion.
  2. Testing and fixing bracket notation access (`include["param"]`). The `liquid` crate treats bracket access on objects the same as dot access, so this may already work if the `include` object is properly structured.
  3. If the `liquid-lib` `IncludeTag` does not support all parameter types correctly, a custom include tag implementation may be needed.
- Check whether the `push` filter works with includes (used in `related-posts.html`), as the DTC site uses `{% assign related_posts = related_posts | push: post %}`.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests plus new tests
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `include.param` works for string-quoted values (`param="value"`)
- [ ] `include.param` works for unquoted numeric values (`max_posts=5`) and the value is numeric (not string `"5"`)
- [ ] `include.param` works for boolean values (`show=true`, `show=false`) and the values are boolean
- [ ] `include.param` works for variable references (`authors=page.authors`)
- [ ] `include["param"]` bracket notation works identically to `include.param` dot notation
- [ ] Missing include parameters resolve to nil/empty (not an error) -- the `LenientValue` wrapper should handle this
- [ ] Nested includes can forward parameters (`{% include inner.html x=include.x %}`)
- [ ] The `related-posts.html` include from the DTC site renders without errors when called with `manual_posts=page.related_posts max_posts=5`
- [ ] The DTC site builds with zero warnings related to include parameter access (the 4 `include["max_posts"]` warnings from issue #22 research should be eliminated)

## Test Scenarios

### Unit: String parameters (existing -- verify still pass)

- `{% include sub.html subscribe="true" %}` with `sub.html` containing `{{ include.subscribe }}` renders `true`
- `{% include ev.html event="test event" speakers=false %}` renders both parameters correctly

### Unit: Numeric parameters

- `{% include counter.html max_posts=5 %}` with `counter.html` containing `{{ include.max_posts }}` renders `5`
- `{% include counter.html max_posts=5 %}` with `counter.html` containing `{% assign x = include.max_posts | default: 3 %}{{ x }}` renders `5` (not `3`, confirming value is not nil)

### Unit: Boolean parameters

- `{% include toggle.html show=true %}` with `toggle.html` containing `{% if include.show %}YES{% endif %}` renders `YES`
- `{% include toggle.html show=false %}` with `toggle.html` containing `{% if include.show %}YES{% else %}NO{% endif %}` renders `NO`

### Unit: Variable reference parameters

- `{% include auth.html authors=page.authors %}` where `page.authors` is `["alice", "bob"]`, and `auth.html` contains `{% for a in include.authors %}{{ a }}{% endfor %}` renders `alicebob` (existing test)

### Unit: Bracket notation access

- `{% include data.html count=5 %}` with `data.html` containing `{{ include["count"] }}` renders `5`
- `{% include data.html name="test" %}` with `data.html` containing `{{ include["name"] }}` renders `test`
- `{% include data.html flag=true %}` with `data.html` containing `{% if include["flag"] %}OK{% endif %}` renders `OK`

### Unit: Missing parameters (lenient access)

- `{% include simple.html %}` with `simple.html` containing `{{ include.missing_param }}` renders empty string (not error)
- `{% include simple.html %}` with `simple.html` containing `{% assign x = include.max | default: 3 %}{{ x }}` renders `3`

### Unit: Nested includes with parameter forwarding

- Outer include passes `x=include.x` to inner include; inner include accesses `include.x` correctly (existing test)

### Unit: Multiple parameters

- `{% include card.html title="Hello" count=3 show=true %}` with include accessing all three parameters via dot notation renders all correctly
- Same include accessing via bracket notation renders all correctly

### Integration: DTC site related-posts include

- Build the DTC site and verify posts that use `{% include related-posts.html manual_posts=page.related_posts max_posts=5 %}` render without warnings
- Verify the rendered HTML contains the related posts section when related posts exist

## References

- Issue #22 compatibility research, gap #9
- Jekyll include documentation: https://jekyllrb.com/docs/includes/
- Current include tests in `src/template/layout.rs` (lines 454-593)
- `LenientObject`/`LenientValue` wrappers in `src/template/engine.rs` (lines 24-306)
