# Issue 249: Fix JavaScript syntax highlighting Rouge compatibility

## Problem

Descoped from issue 229. All 10 GitHub Pages theme sites share the same JavaScript code block in `index.md`:

```js
// Javascript code with syntax highlighting.
var fun = function lang(l) {
  dateformat.i18n = require('./lang/' + l)
  return true;
}
```

Rustkyll's syntect-based highlighter produces different token classes and span boundaries compared to Rouge (Jekyll's highlighter), causing ~59 DOM differences per theme site page. The same pattern appears in 8 theme sites (architect, cayman, dinky, hacker, leap-day, merlot, midnight, slate, primer, time-machine) with identical diffs.

### Root Cause Analysis

**Root cause 1: Function name class mapping (nf vs nx)**

In `src/syntax.rs` line 60, the rule `("source.js entity.name.function", "nx")` maps ALL JS function names to `nx` (name.other). This is wrong: Rouge uses `nf` (name.function) for function declarations like `lang` in `function lang(l)` and function calls like `require(...)`. The `nx` override was intended for plain identifier references, but `entity.name.function` is only emitted by syntect for actual function names and function calls -- not for general identifiers.

Fix: Remove the `source.js entity.name.function` override (line 60) so the generic `entity.name.function -> nf` mapping (line 77) applies. The other JS overrides (`variable.parameter`, `variable.other`, `variable.function`, `meta.property.object` -> `nx`) remain correct for plain identifiers.

Evidence from DOM diffs (all theme sites show identical pattern):
- `attribute_differs - expected: "class='nf'", actual: "class='nx'"` (appears twice per page: `lang` and `require`)

**Root cause 2: String delimiter splitting (dl + s1 vs single s1)**

Rouge splits single-quoted JavaScript strings into three tokens:
```html
<span class="dl">'</span><span class="s1">./lang/</span><span class="dl">'</span>
```

Syntect emits `'./lang/'` as a single `string.quoted.single` scope, which maps to `s1`, producing:
```html
<span class="s1">'./lang/'</span>
```

This requires post-processing in `accumulate_and_emit` or a new pass: when emitting an `s1` (or `s2`) token whose text starts and ends with a quote character (`'` or `"`), split it into `dl` + `s1`/`s2` + `dl`.

This splitting should apply to JavaScript (and potentially other languages where Rouge uses `dl`). It should NOT apply to languages where Rouge does not split strings (e.g., JSON strings should remain as single `s2` spans, Python strings as single `s` spans).

Evidence from DOM diffs:
- `attribute_differs - expected: "class='dl'", actual: "class='s1'"` (opening quote)
- `text_differs - expected: "'", actual: "'./lang/'"` (Rustkyll has whole string in one span)
- `attribute_differs - expected: "class='dl'", actual: "class='nx'"` (closing quote cascades)

**Root cause 3: Cascading span boundary shifts**

Because Rustkyll emits the full quoted string as one span, every subsequent token in the line has shifted boundaries. The `+` operator, the `l` identifier, the `)` punctuation -- all appear with wrong classes because the DOM comparison is positional. Fixing root causes 1 and 2 will automatically fix these cascading differences.

Evidence: Lines 10-15 in every theme site's DOM diff show `text_differs` and `attribute_differs` for tokens after the string literal. These are not independent bugs.

## Scope

1. Remove the `source.js entity.name.function -> nx` override so JS function names get `nf`
2. Implement string delimiter splitting for JS single-quoted and double-quoted strings: emit `dl` + `s1`/`s2` content + `dl` instead of a single `s1`/`s2` span when the token text is a complete quoted string
3. Ensure the splitting does NOT apply to JSON, Python, or other languages that do not use `dl` in Rouge output
4. Verify cascading diffs resolve automatically

## Implementation Notes

### Fix 1: entity.name.function mapping

Remove line 60 from `build_scope_map()`:
```rust
// REMOVE: ("source.js entity.name.function", "nx"),
```
The generic rule `("entity.name.function", "nf")` at line 77 will then apply to JS.

### Fix 2: String delimiter splitting

The cleanest approach is a post-processing step in `highlight_code()` (similar to the existing `merge_python_dotted_modules` and `postprocess_xml_tag_tokens` passes). For JS (and Ruby, which also uses `dl` in Rouge), when a `<span class="s1">'...'</span>` or `<span class="s2">"..."</span>` token is encountered whose text starts and ends with a matching quote character, split it into:
- `<span class="dl">QUOTE</span><span class="s1">CONTENT</span><span class="dl">QUOTE</span>`

The language detection can use the `syntax.name` field or the `source.js` / `source.ruby` scope to decide when to apply this splitting.

### Existing tests to update

`tests/syntax_highlighting.rs`:
- `test_js_identifiers_are_nx` (line 36): Currently asserts `class="nx"` exists. Still valid since `fun`, `dateformat`, `i18n` etc. remain `nx`. But `lang` and `require` will now be `nf`, so the test passes as-is (it only checks that `nx` class exists somewhere).
- `test_js_theme_code_exact` (line 50): Asserts `require` is `nx`. This must be updated to assert `require` is `nf`.

## Dependencies

- Issue 229 (site.github fixes) -- DONE

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes cleanly
- [ ] `cargo fmt -- --check` passes
- [ ] `highlight_code("javascript", "var fun = function lang(l) {...}")` produces `<span class="nf">lang</span>` (not `nx`)
- [ ] `highlight_code("javascript", "...require('./lang/' + l)...")` produces `<span class="nf">require</span>` (not `nx`)
- [ ] JS identifiers that are NOT function declarations/calls (e.g., `fun`, `dateformat`, `i18n`, `l`) still produce `nx`
- [ ] Single-quoted JS strings produce `<span class="dl">'</span><span class="s1">CONTENT</span><span class="dl">'</span>` (three spans, not one)
- [ ] Double-quoted JS strings produce `<span class="dl">"</span><span class="s2">CONTENT</span><span class="dl">"</span>`
- [ ] JSON strings are NOT split into dl tokens (remain as single `s2` spans)
- [ ] Python strings are NOT split into dl tokens (remain as single `s` spans)
- [ ] Ruby single-quoted strings ARE split into dl + s1 + dl (Rouge uses dl for Ruby too)
- [ ] Existing tests in `tests/syntax_highlighting.rs` pass (after updating `require` assertion from `nx` to `nf`)
- [ ] All new tests pass via `./scripts/cargo-safe test`

## Test Scenarios

### Unit: JS function name class (nf vs nx)

1. **test_js_function_declaration_name_is_nf**: Parse `function lang(l) { return true; }` -- verify `lang` gets `<span class="nf">lang</span>`
2. **test_js_function_call_is_nf**: Parse `require('./lang/' + l)` -- verify `require` gets `<span class="nf">require</span>`
3. **test_js_plain_identifiers_remain_nx**: Parse the full theme code block -- verify `fun`, `dateformat`, `i18n`, `l` all get `nx` class, NOT `nf`

### Unit: String delimiter splitting

4. **test_js_single_quoted_string_split_dl**: Parse JS code containing `'./lang/'` -- verify output contains `<span class="dl">'</span><span class="s1">./lang/</span><span class="dl">'</span>`
5. **test_js_double_quoted_string_split_dl**: Parse JS code containing `"hello"` -- verify output contains `<span class="dl">"</span><span class="s2">hello</span><span class="dl">"</span>`
6. **test_json_strings_no_dl_split**: Parse JSON `{"key": "value"}` -- verify NO `dl` class appears; strings remain as single `s2` spans
7. **test_python_strings_no_dl_split**: Parse Python `x = 'hello'` -- verify NO `dl` class appears
8. **test_ruby_string_split_dl**: Parse Ruby `puts 'hello'` -- verify `dl` splitting occurs (Rouge uses dl for Ruby strings too)

### Integration: Full theme code block

9. **test_js_theme_code_block_full_match**: Parse the exact theme site JS code block. Verify the complete span sequence matches Rouge output:
   - `var` -> `kd`
   - `fun` -> `nx`
   - `function` -> `kd`
   - `lang` -> `nf`
   - `(` -> `p`
   - `l` -> `nx`
   - `)` -> `p`
   - `{` -> `p`
   - String `'./lang/'` -> `dl` + `s1` + `dl` (three spans)
   - `+` -> `o`
   - `require` -> `nf`
   - `return` -> `k`
   - `true` -> `kc`

### Unit: Unicode content (required per project conventions)

10. **test_js_unicode_string_dl_split**: Parse JS code with non-ASCII string content like `'caf\u00E9'` -- verify dl splitting works correctly with Unicode content between the delimiters

## Output Verification

After implementation, the engineer should build a theme site and verify DOM diffs decrease. Specifically:
- The 3 `nf`/`nx` attribute diffs per theme site should be eliminated
- The `dl`/`s1` attribute diffs and cascading text_differs should be eliminated
- Total diff count per theme site should drop by approximately 7-10 differences (the JS-related subset of the ~59 total)

Note: the remaining ~49-52 diffs per theme site page are from other languages (Ruby code block) and non-syntax issues -- those are out of scope for this issue.

## Log

- 2026-03-19: Created, descoped from issue 229.
- 2026-03-20: [PM] Groomed. Root cause analysis confirmed: (1) incorrect entity.name.function->nx override, (2) missing string delimiter splitting for JS/Ruby. Added concrete acceptance criteria and test scenarios.

### [SWE] 2026-03-20
- **TDD cycle 1: Fix 1 - nf vs nx for JS function names**
  - Wrote 3 tests: test_js_function_declaration_name_is_nf, test_js_function_call_is_nf, test_js_plain_identifiers_remain_nx
  - Ran tests: FAIL as expected -- `lang` got `nx` instead of `nf`, `require` got `nx` instead of `nf`
  - Implemented fix: removed `("source.js entity.name.function", "nx")` override so generic `entity.name.function -> nf` applies; changed `("source.js variable.function", "nx")` to `("source.js variable.function", "nf")` so function calls also get `nf`
  - Ran tests: PASS -- `lang` and `require` now get `nf`; `fun`, `dateformat`, `i18n`, `l` still get `nx`
  - Updated existing test_js_theme_code_exact: changed `require` assertion from `nx` to `nf`

- **TDD cycle 2: Fix 2 - String delimiter splitting for JS/Ruby**
  - Wrote 6 tests: test_js_single_quoted_string_split_dl, test_js_double_quoted_string_split_dl, test_json_strings_no_dl_split, test_python_strings_no_dl_split, test_ruby_string_split_dl, test_js_unicode_string_dl_split
  - Wrote 1 integration test: test_js_theme_code_block_full_match (full token sequence verification)
  - Ran tests: FAIL as expected -- JS/Ruby strings were single s1/s2 spans, no dl splitting
  - Implemented fix: added `is_dl_split_language()` (JS/Ruby) and `postprocess_string_delimiter_split()` that splits `<span class="s1">'...'</span>` into `<span class="dl">'</span><span class="s1">...</span><span class="dl">'</span>` (same for s2/double-quotes)
  - Ran tests: PASS -- all 19 syntax_highlighting tests pass

- Build: 2172+ tests pass, 0 fail
- Clippy: pre-existing failure in liquid-core dependency (not caused by this change, confirmed same on main)
- Fmt: clean after `cargo fmt`
- Files modified: src/syntax.rs, tests/syntax_highlighting.rs
- Files renamed: docs/tracker/249-fix-js-syntax-highlighting-rouge-compat.groomed.md -> .in-progress.md
