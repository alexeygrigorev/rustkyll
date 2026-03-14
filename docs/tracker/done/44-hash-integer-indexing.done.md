# Issue 44: Support Integer Indexing on Hash/Map Values in Liquid

## Problem

Jekyll/Liquid allows integer indexing on hashes (maps/objects). For example, `locale[0]` returns the first key-value pair of a hash loaded from a data file. The `liquid` crate only supports string-key access on objects (`ObjectView::get(&str)`). When a template uses `hash[0]`, the integer `0` is converted to the string `"0"` for lookup, which finds nothing.

This breaks sites like opensource.guide which use patterns like:
```liquid
{% assign lang = locale[0] %}
```
where `locale` is a YAML mapping like `{ en: { ... }, es: { ... } }`.

## Scope

This issue requires intercepting index access on objects when the index is an integer, and returning the Nth key-value pair instead. The fix belongs in the `LenientValue` ObjectView implementation (or a similar wrapper), since that is where all object access is routed in rustkyll.

**Jekyll behavior to match:** When you index a hash with an integer in Jekyll:
- `hash[0]` returns a two-element array `[key, value]` (the first entry)
- `hash[1]` returns `[key, value]` for the second entry
- Negative indexing is NOT supported on hashes in Jekyll (returns nil)
- Out-of-bounds returns nil

## Implementation Approach

The `ObjectView::get(&str)` method receives the index as a string. When the index string can be parsed as an integer (e.g., `"0"`, `"1"`), and no string key with that name exists in the object, fall back to positional access:

1. In `LenientValue`'s `ObjectView::get()` implementation, after the normal string-key lookup fails (or returns Nil):
   - Try parsing the index as `i64`
   - If successful, get the Nth key-value pair from the object's iteration order
   - Return it as a two-element array `[key_string, value]`
2. The tricky part is lifetime management: `ObjectView::get` returns `Option<&dyn ValueView>`, so the returned array must be pre-computed and stored alongside the object.

**Pre-computation approach (recommended):**
- When constructing a `LenientValue` from an `Object`, also pre-compute a `Vec<LenientValue>` of `[key, value]` pairs (stored as two-element arrays)
- Store these as `positional_children: Vec<LenientValue>` on the struct
- In `get()`, when the string key is not found and the index parses as an integer, return from `positional_children`

**Alternative:** Modify `yaml_to_liquid` or `normalize_arrays` to convert hashes into a structure that supports both key and positional access. This is less clean since it changes the data model globally.

## Files to Modify

- `src/template/engine.rs` -- modify `LenientValue` struct and its `ObjectView` implementation
  - Add `positional_children: Vec<LenientValue>` field
  - Update `from_value()` to pre-compute positional entries as `[key, value]` arrays
  - Update `ObjectView::get()` to fall back to positional lookup when the key parses as an integer

## Dependencies

None. This is independent of other open issues.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `hash[0]` in a Liquid template returns a two-element array `[key, value]` for the first entry of a hash
- [ ] `hash[1]` returns the second entry, etc.
- [ ] Out-of-bounds integer index on a hash returns nil (empty string in output)
- [ ] String keys that happen to look like integers (e.g., a hash with key `"0"`) are found by normal string lookup first (string lookup takes priority over positional)
- [ ] Existing string-key access on objects is not affected
- [ ] The DTC site (`datatalksclub.github.io/`) still builds successfully
- [ ] No panics or unwraps in the indexing path

## Test Scenarios

### Unit: Integer indexing on objects

- Create a Liquid object with keys `{a: 1, b: 2, c: 3}`, render `{{ obj[0] }}` -- verify output is the first key-value pair (rendered as array)
- Create a Liquid object, render `{{ obj[0][0] }}` -- verify it returns the key string
- Create a Liquid object, render `{{ obj[0][1] }}` -- verify it returns the value
- Render `{{ obj[5] }}` on a 3-entry object -- verify output is empty (nil)
- Render `{{ obj[-1] }}` on an object -- verify output is empty (nil, matching Jekyll)

### Unit: String key priority over integer fallback

- Create an object with a key literally named `"0"` (e.g., `{ "0": "zero" }`), render `{{ obj[0] }}` -- verify it returns `"zero"` (the string-keyed value), not a positional pair

### Unit: Normal string-key access unaffected

- Create an object `{name: "Alice"}`, render `{{ obj.name }}` -- verify it returns `"Alice"`
- Create an object `{name: "Alice"}`, render `{{ obj["name"] }}` -- verify it returns `"Alice"`

### Integration: Template rendering with hash indexing

- Set up a template context with a data-file-style hash (e.g., locales: `{en: {...}, es: {...}}`), render a template that does `{% assign first = locales[0] %}{{ first[0] }}` -- verify it outputs the first key (e.g., `en`)
- Verify `{% for i in (0..locales.size) %}{{ locales[i][0] }}{% endfor %}` iterates over hash entries by position

### Integration: Backward compatibility

- All existing template engine tests still pass unchanged
- Build the DTC site end-to-end and verify no regressions
