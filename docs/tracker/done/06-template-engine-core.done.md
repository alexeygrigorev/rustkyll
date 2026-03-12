# Issue 06: Template Engine Core

## Description

Integrate the `liquid` crate (v0.26) as the template engine for rustkyll. The crate already supports all the core Liquid tags, operators, and most standard filters needed by the Jekyll site. This issue sets up the crate integration, provides a wrapper API (`TemplateEngine`), implements YAML-to-Liquid value conversion (needed to pass front matter and data files as template context), and verifies that the crate's built-in features work correctly for our use cases.

Custom Jekyll-specific filters (`jsonify`, `date_to_string`, `date_to_xmlschema`, `slugify`, `markdownify`, `relative_url`, `where_exp`) are deferred to Issue 07. Layout wrapping, includes loading from `_includes/`, and Jekyll include-syntax adaptation are deferred to Issue 08.

## Dependencies

- Issue 01 (project setup) -- DONE

## Research: `liquid` crate (v0.26) capabilities

Verified by running test programs against the crate. The following features all work:

### Tags (all supported)
- `for` (with `limit` parameter), `if`/`elsif`/`else`/`endif`, `unless`/`endunless`, `assign`, `capture`/`endcapture`, `break`, `include` (with quoted filenames and colon-separated named parameters)

### Operators (all supported)
- `==`, `!=`, `<`, `<=`, `>`, `>=`, `contains`, `and`, `or`

### Forloop variables (all supported)
- `forloop.first`, `forloop.last`, `forloop.index`

### Other features (all supported)
- Dot notation: `page.title`, `site.data.events`, `page.links.youtube`
- Whitespace control: `{%- -%}`, `{{- -}}`
- `.size` property on arrays and strings

### Built-in filters from stdlib
`where`, `sort`, `reverse`, `map`, `first`, `last`, `size`, `join`, `uniq`, `compact`, `slice`, `append`, `prepend`, `default`, `strip`, `strip_html`, `strip_newlines`, `truncate`, `newline_to_br`, `split`, `plus`, `minus`, `times`, `divided_by`, `modulo`, `replace`, `remove`, `date`, `escape`, `downcase`, `upcase`, `concat`

### Additional filters from `liquid-lib` with `jekyll` feature
`slugify`, `push`, `pop`, `unshift`, `array_to_sentence_string`

### NOT in the crate (deferred to Issue 07)
`where_exp`, `jsonify`, `date_to_string`, `date_to_xmlschema`, `markdownify`, `relative_url`

### Jekyll include-syntax differences (deferred to Issue 08)
- Jekyll: `{% include file.html param=value %}` -- no quotes around filename, `=` separator, accessed as `include.param` inside the partial
- Crate: `{% include "file.html" param: value %}` -- quotes required, `:` separator, params are accessible as direct variables (not under `include.`)
- Issue 08 will handle preprocessing templates to bridge this syntax gap

### Undefined variables
The `liquid` crate errors on undefined variables by default. Jekyll silently renders empty strings. The engine must handle this -- either by pre-populating all expected variables or by catching/suppressing undefined-variable errors.

## Scope

### In scope
- Add `liquid = "0.26"`, `liquid-core = "0.26"`, `liquid-lib = { version = "0.26", features = ["jekyll"] }` to `Cargo.toml`
- Create `src/template/mod.rs` -- public API re-exports
- Create `src/template/engine.rs` -- `TemplateEngine` struct wrapping `liquid::Parser`
  - `new() -> Result<Self>` -- builds parser with stdlib + Jekyll filters
  - `parse(template_str) -> Result<Template>` -- parse a template string
  - `render(template, context) -> Result<String>` -- render a parsed template
  - `parse_and_render(template_str, context) -> Result<String>` -- convenience method
- Create `src/template/context.rs` -- YAML-to-Liquid value conversion
  - `yaml_to_liquid(serde_yaml::Value) -> liquid::model::Value` -- recursive conversion for all YAML types (string, integer, float, bool, null, sequence, mapping)
  - `yaml_map_to_object(serde_yaml::Mapping) -> liquid::Object` -- convert a YAML mapping to liquid object
  - Helper to build a context with `page` and `site` namespaces from YAML data
- Create `src/template/error.rs` -- `TemplateError` enum using `thiserror`
  - `ParseError(String)` -- template parse failures
  - `RenderError(String)` -- template render failures
  - `ConversionError(String)` -- YAML-to-Liquid conversion failures
- Register module in `src/lib.rs` as `pub mod template;`
- Comprehensive tests (20+)

### Out of scope
- Custom filter implementations: `where_exp`, `jsonify`, `date_to_string`, `date_to_xmlschema`, `markdownify`, `relative_url` (Issue 07)
- Layout chain rendering, includes loaded from `_includes/` directory, Jekyll include-syntax preprocessing (Issue 08)

## Acceptance Criteria

- [ ] `liquid = "0.26"` is in `Cargo.toml` `[dependencies]`
- [ ] `liquid-core = "0.26"` is in `Cargo.toml` `[dependencies]`
- [ ] `liquid-lib` with `jekyll` feature is in `Cargo.toml` `[dependencies]`
- [ ] `src/template/mod.rs` exists and re-exports the public API (`TemplateEngine`, `TemplateError`, conversion functions)
- [ ] `src/template/engine.rs` exists with `TemplateEngine` struct
- [ ] `src/template/context.rs` exists with YAML-to-Liquid conversion functions
- [ ] `src/template/error.rs` exists with `TemplateError` enum using `thiserror`
- [ ] `TemplateEngine::new()` creates a parser with stdlib + Jekyll filters (slugify, push)
- [ ] `TemplateEngine::parse()` returns `Result` with a meaningful error on invalid templates
- [ ] `TemplateEngine::render()` renders a parsed template with the given context
- [ ] `TemplateEngine::parse_and_render()` works as a convenience shortcut
- [ ] `yaml_to_liquid()` correctly converts all YAML types: String, Integer, Float, Bool, Null, Sequence, Mapping (including nested structures)
- [ ] `yaml_map_to_object()` converts a YAML mapping to `liquid::Object`
- [ ] Undefined variables in templates produce empty output (not errors) -- matching Jekyll behavior
- [ ] `lib.rs` declares `pub mod template;`
- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with 20+ new tests covering all scenarios below

## Test Scenarios

### Unit: Template parsing
- Parse a valid template string containing `{{ variable }}`, verify no error
- Parse an invalid template (e.g., `{% if %}` with no condition), verify `TemplateError::ParseError` returned
- Parse and render a simple template, verify correct output

### Unit: Variable output with dot notation
- Render `{{ name }}` with context `{ name: "Alice" }` -- verify `"Alice"`
- Render `{{ page.title }}` with nested context `{ page: { title: "Hello" } }` -- verify `"Hello"`
- Render `{{ page.links.youtube }}` with 3-level nesting -- verify correct value
- Render `{{ missing_var }}` -- verify empty string output (not error)

### Unit: For loops
- `{% for item in items %}{{ item }} {% endfor %}` with `["a", "b", "c"]` -- verify `"a b c "`
- For loop with `forloop.index` -- verify outputs `"1"`, `"2"`, `"3"`
- For loop with `forloop.first` and `forloop.last` -- verify correct boolean behavior
- `{% for item in items limit:2 %}{{ item }}{% endfor %}` -- verify only 2 items rendered
- Nested for loops -- verify correct scoping
- For loop over empty array -- verify no output, no error

### Unit: Conditionals
- `{% if x %}yes{% endif %}` with x present and x absent
- `{% if x == 1 %}one{% elsif x == 2 %}two{% else %}other{% endif %}` with x=1, x=2, x=3
- `{% unless x %}no{% endunless %}` with truthy and falsy values
- `{% if items contains "b" %}yes{% endif %}` with `["a", "b"]` and `["a", "c"]`
- `{% if a and b %}both{% endif %}` with various truth combinations
- `{% if a or b %}either{% endif %}` with various truth combinations
- `{% if x != "bad" %}good{% endif %}`
- `{% if x <= 5 %}lte{% endif %}`
- `{% if page.links.youtube and page.links.youtube != 'TODO' %}show{% endif %}` -- realistic condition from podcast.html

### Unit: Assign and capture
- `{% assign x = "hello" %}{{ x }}` -- verify `"hello"`
- `{% assign url = base | append: "/path" %}{{ url }}` -- verify filter works inside assign
- `{% capture msg %}hello {{ name }}{% endcapture %}{{ msg }}` -- verify capture concatenates

### Unit: Break
- For loop with `{% if item == "stop" %}{% break %}{% endif %}` -- verify loop stops early

### Unit: Whitespace control
- `  {%- assign x = "hi" -%}  {{ x }}` -- verify leading/trailing whitespace stripped around the assign tag

### Unit: Built-in filters (verify crate's stdlib works for our patterns)
- `{{ items | where: "name", "b" | first }}` with array of objects -- verify correct filtering
- `{{ items | sort | first }}` -- verify sorting
- `{{ text | strip_html }}` -- verify HTML removed
- `{{ x | default: "fallback" }}` with x set and x unset -- verify default behavior
- `{{ x | plus: 1 }}` -- verify arithmetic
- `{{ items | map: "name" | join: ", " }}` -- verify chaining
- `{{ items | reverse }}` -- verify array reversal
- `{{ text | truncate: 10 }}` -- verify truncation
- `{{ title | slugify }}` -- verify Jekyll slugify filter works (from liquid-lib jekyll feature)

### Unit: YAML-to-Liquid value conversion
- Convert `serde_yaml::Value::String("hello")` -- verify liquid string
- Convert `serde_yaml::Value::Number` (integer, e.g. 42) -- verify liquid integer
- Convert `serde_yaml::Value::Number` (float, e.g. 3.14) -- verify liquid float
- Convert `serde_yaml::Value::Bool(true)` -- verify liquid boolean
- Convert `serde_yaml::Value::Null` -- verify liquid Nil
- Convert a YAML sequence `[1, 2, 3]` -- verify liquid array
- Convert a YAML mapping `{ title: "Hello", count: 5 }` -- verify liquid object
- Convert a nested structure (mapping containing arrays of mappings) -- verify correct recursive conversion
- Round-trip test: parse YAML front matter string, convert to liquid context, render template accessing the values

### Integration: Realistic template rendering
- Create a simplified version of the author.html pattern: for loop over posts, `unless forloop.last` for comma separator, `where` filter, `first`, dot notation -- render with realistic context and verify correct HTML output
- Create a snippet mimicking podcast.html's capture+append pattern for building action URLs -- verify string concatenation works

### Edge cases
- Render with nil value in `{% if nil %}` -- verify falsy
- Empty array in for loop -- no output, no error
- Template parse error -- verify returns `TemplateError`, not panic
- Very deeply nested dot access (4+ levels) -- verify works
- Integer 0 in condition -- verify truthy (Liquid considers 0 truthy, unlike many languages)

## Notes for the engineer

1. Use `liquid::ParserBuilder::with_stdlib()` then register Jekyll filters via `.filter(liquid_lib::jekyll::Slugify)`, `.filter(liquid_lib::jekyll::Push)`, etc.
2. The `liquid::object!({})` macro is convenient for building test contexts.
3. For YAML-to-Liquid conversion: `serde_yaml::Value` variants map to `liquid_core::model::Value` (aliased as `liquid::model::Value`). Write a recursive function.
4. The `liquid` crate errors on undefined variables by default. Jekyll silently outputs empty strings. You MUST handle this. Options: (a) catch render errors related to undefined variables and re-render or return empty, (b) pre-populate all variables, (c) check if the crate has a lenient mode. This is a critical requirement -- many Jekyll templates access variables that may not exist on every page.
5. The `liquid` crate's `include` tag requires quoted filenames (`"file.html"`). Do NOT try to solve the Jekyll unquoted-include syntax here -- that is Issue 08's responsibility.
6. The `TemplateEngine` should be designed for extensibility -- Issue 07 will need to register additional custom filters on the parser builder.
7. Consider making the parser builder accessible or providing `TemplateEngine::builder()` so Issue 07 can add custom filters before building.
