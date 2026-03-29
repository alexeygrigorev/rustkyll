# Issue 353: Implement Liquid `find` filter

## Problem

Rustkyll does not support the `find` Liquid filter. Hydeout's `_includes/back-link.html` uses it:

```liquid
{% assign back_page = site.pages | find: "name", page.back_page %}
```

The `find` filter was added in Liquid 5.0 (used by Jekyll 4.3+). It takes an array, a property name, and a value, and returns the **first** matching item -- unlike `where` which returns all matches as an array.

Because the filter is unrecognized, template parsing fails for any page that includes `back-link.html`. This affects every page using the `page` layout: about, tags, category pages, and markup/edge-case pages.

Related to issue #241 (Hydeout theme support).

## Scope

Implement the `find` filter following the same pattern as the existing `where` filter in `src/template/filters/where_filter.rs`:

1. Create `src/template/filters/find_filter.rs` with the `Find` filter struct
2. Register it in `src/template/filters/mod.rs` (module declaration + pub use)
3. Register it in `src/template/engine.rs` (`.filter(filters::Find)`)
4. The filter must:
   - Accept two arguments: property name (string) and target value (string)
   - Iterate the input array and return the first item where `item[property] == value`
   - Return `Nil` if no match is found (not an empty array -- this is what distinguishes `find` from `where`)
   - Return `Nil` if the input is not an array
   - Use string comparison via `to_kstr()` to match Jekyll/`where` behavior

## Dependencies

- Issue #241 (Hydeout theme support) -- done

## DOM Baselines

- DTC: **790/790** (must not regress)
- hydeout: **0/13** (may improve if this unblocks page rendering)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo fmt` produces no changes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `find` filter is registered and available in templates
- [ ] `site.pages | find: "name", "about.md"` returns the first page object with `name == "about.md"`
- [ ] `site.pages | find: "name", "nonexistent"` returns `Nil` (not an empty array)
- [ ] Non-array input returns `Nil`
- [ ] Items missing the specified property are skipped without error
- [ ] Hydeout site builds without template parse errors on pages using `back-link.html`
- [ ] DTC DOM match count does not drop below **790/790**
- [ ] hydeout DOM match count does not drop below **0/13**

## Test Scenarios

### Unit: find filter behavior

- **Basic match**: Array of objects, find by property -- returns first matching object
- **No match**: Array of objects, find by property with nonexistent value -- returns Nil
- **Multiple matches**: Array with 2+ items matching -- returns only the first one
- **Empty array**: Empty array input -- returns Nil
- **Non-array input**: Scalar string input -- returns Nil
- **Missing property**: Some items lack the searched property -- those are skipped, matching item still found
- **Unicode values**: Find by a property with non-ASCII value (e.g., "uber" vs "name" containing Unicode characters)

### Integration: hydeout template rendering

- Build hydeout site, verify no template parse errors related to `find` filter
- Verify pages using `page` layout (about.md, tags.html, category pages) render without errors
- DOM comparison shows no regression from 0/13 baseline

## Log

### [SWE] 2026-03-29
- Created `src/template/filters/find_filter.rs` with `Find` filter struct
  - Follows same pattern as `where_filter.rs` (FilterParameters, FilterReflection, etc.)
  - Returns first matching item (not array) or `Nil` on no match / non-array input
  - String comparison via `to_kstr()` matching Jekyll behavior
- Registered in `src/template/filters/mod.rs` (mod declaration + pub use)
- Registered in `src/template/engine.rs` (`.filter(filters::Find)`)
- 7 unit tests written and passing:
  - test_find_filter_basic_match
  - test_find_filter_no_match_returns_nil
  - test_find_filter_multiple_matches_returns_first
  - test_find_filter_empty_array
  - test_find_filter_non_array_input
  - test_find_filter_missing_property_skipped
  - test_find_filter_unicode_values
- Full test suite: all pass, 0 failures
- clippy: clean (no warnings)
- fmt: clean (no changes)
- DTC DOM: 790/790 (no regression)
- hydeout DOM: 0/13 (no regression from baseline)
- Files created: `src/template/filters/find_filter.rs`
- Files modified: `src/template/filters/mod.rs`, `src/template/engine.rs`
