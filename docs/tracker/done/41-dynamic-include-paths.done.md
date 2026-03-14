# Issue 41: Support Dynamic Include Paths

## Problem

Complex site testing (Issue 35) revealed that some Jekyll sites use dynamic include paths with Liquid expressions:
```
{% include {{ page.form | append: '.html' }} %}
```
This is parsed as a syntax error because the `{% include %}` tag expects a literal filename (or a quoted string). The current `LenientIncludeTag::parse` method in `src/template/include_tag.rs` calls `expect_identifier()` or handles quoted strings, but it cannot handle a Liquid expression (variable reference or filter chain) as the include path.

## Affected Sites

- government.github.com -- `{% include {{ page.form | append: '.html' }} %}`

## Requirements

- Support dynamic include paths where the filename is a Liquid expression (variable reference, optionally with filters)
- Evaluate the expression at render time and include the resolved filename
- Continue supporting all existing include syntax: literal filenames, quoted paths, and parameters
- The `preprocess_include_paths` function must not corrupt dynamic include tags

## Scope

This issue is limited to:
- `src/template/include_tag.rs` -- the `LenientIncludeTag` parser and `LenientInclude` renderer, plus `preprocess_include_paths`
- `src/template/engine.rs` -- only if changes to how templates are parsed or preprocessed are needed

No changes to the generator, layout engine, or other template components.

## Dependencies

None. The existing include tag infrastructure (Issue 08) is already done.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `{% include {{ page.form }} %}` parses without error and resolves `page.form` at render time to determine the include filename
- [ ] `{% include {{ page.form | append: '.html' }} %}` parses without error and evaluates the filter chain at render time
- [ ] `{% include {{ some_var }} param1="value" %}` works -- dynamic path with parameters still supported
- [ ] `{% include simple.html %}` (literal filename) continues to work unchanged
- [ ] `{% include "subdir/file.html" %}` (quoted path) continues to work unchanged
- [ ] `{% include subdir/file.html %}` (unquoted path with slash) continues to work via `preprocess_include_paths`
- [ ] `{% include simple.html param="val" %}` (literal with params) continues to work unchanged
- [ ] If the dynamic expression resolves to a partial name that does not exist, a clear error is returned (not a panic)
- [ ] If the dynamic expression resolves to an empty or nil value, a clear error is returned

## Test Scenarios

### Unit: Preprocessing does not corrupt dynamic includes

- Input `{% include {{ page.form | append: '.html' }} %}` -- verify `preprocess_include_paths` does not mangle the double-brace expression (the `{{` inside `{% %}` should not be treated as a subdirectory path)
- Input `{% include {{ var }} %}` -- verify preprocessing leaves it unchanged
- Input with both a dynamic include and a static subdirectory include on separate lines -- verify the static one is quoted and the dynamic one is untouched

### Unit: Parsing dynamic include tags

- Parse `{% include {{ page.form }} %}` -- verify it creates a `LenientInclude` with a variable expression for the path (not a literal)
- Parse `{% include {{ page.form | append: '.html' }} %}` -- verify it creates a renderable without error
- Parse `{% include {{ page.form }} param="value" %}` -- verify both the dynamic path and the parameter are captured

### Integration: Rendering dynamic includes

- Set up a template engine with a partial named `contact.html` containing `<p>Contact</p>`. Render `{% include {{ page.form }} %}` with `page.form = "contact.html"` in the context. Verify output contains `<p>Contact</p>`.
- Set up a partial named `survey.html`. Render `{% include {{ page.form | append: '.html' }} %}` with `page.form = "survey"`. Verify the `survey.html` partial is included.
- Render `{% include {{ page.form }} %}` where `page.form` is not set (nil). Verify a descriptive error is returned, not a panic.
- Render `{% include {{ page.form }} %}` where `page.form = "nonexistent.html"` and no such partial exists. Verify a descriptive error is returned.

### Regression: Existing include functionality

- Verify all existing include tag tests still pass (literal filenames, quoted paths, subdirectory paths, parameters, lenient parameter access)
- Render `{% include header.html %}` with a partial named `header.html` -- verify it still works as before

## Notes

- The key challenge is at the parser level: `LenientIncludeTag::parse` currently expects an identifier or quoted string as the first argument. For dynamic paths, the parser needs to detect the `{{` token pattern and capture the inner expression (variable + optional filters) instead.
- At render time, `LenientInclude::render_to` already evaluates `self.partial` as an expression. The fix may involve making `partial` hold a more complex expression type when the path is dynamic.
- Consider whether the `{{ }}` wrapper is stripped during parsing (i.e., the expression inside `{{ page.form | append: '.html' }}` is just `page.form | append: '.html'`) or whether it needs special handling.
- Look at how Jekyll itself handles this: Jekyll's `IncludeTag` supports both static filenames and variable filenames via the `{% include {{ var }} %}` syntax.

## References

- `src/template/include_tag.rs` -- `LenientIncludeTag`, `LenientInclude`, `preprocess_include_paths`
- `src/template/engine.rs` -- template parsing and include registration
- Jekyll include tag documentation: https://jekyllrb.com/docs/includes/
