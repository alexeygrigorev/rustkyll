# Issue 360: Yat theme 404.html empty output (numeric layout name)

## Problem

The Yat theme's `404.html` renders as 0 bytes. The original issue blamed the
`sample` filter on nil collections, but investigation shows:

- **sample filter on nil**: ALREADY FIXED. The `sample` filter in
  `src/template/filters/sample.rs` handles nil input gracefully (returns empty
  array or nil). The `about.html` page renders with full layout wrapping
  (1077 lines).
- **404.html empty output**: STILL BROKEN. The `404.html` page has front matter
  `layout: 404`. YAML parses this as an integer, not a string. The layout
  resolution code in `src/generator.rs` calls `v.as_str()` on the layout value,
  which returns `None` for integers, so no layout is applied and the page body
  is empty (the source file has no content beyond the front matter).

## Root Cause

Two code paths resolve layout names using `as_str()` which fails for
non-string YAML values:

1. **`src/generator.rs` line ~2197** (standalone pages):
   ```rust
   let layout_name: Option<String> = layout_value
       .and_then(|v| v.as_str())  // Returns None for integer 404
       .filter(|s| !s.is_empty())
       .map(|s| s.to_string());
   ```

2. **`src/generator.rs` line ~1247** (`resolve_layout` for collection items):
   ```rust
   if let Some(layout_str) = layout_val.as_str() {  // Returns None for integer
   ```

Both should fall back to converting non-string values to their string
representation (e.g., integer `404` -> string `"404"`).

## Fix

In both code paths, when `as_str()` returns `None` and the value is not null,
convert the YAML value to a string. For example:

```rust
fn yaml_value_to_layout_string(val: &serde_yaml::Value) -> Option<String> {
    if let Some(s) = val.as_str() {
        if s.is_empty() { None } else { Some(s.to_string()) }
    } else if val.is_null() {
        None  // layout: null means no layout
    } else {
        // Integer, float, bool -- convert to string (e.g., 404 -> "404")
        Some(format!("{}", serde_yaml::to_string(val).unwrap_or_default().trim()))
    }
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests plus new tests below
- [ ] Yat theme `404.html` renders with full layout wrapping (non-zero bytes, contains `<!DOCTYPE html>`)
- [ ] Yat theme `about.html` continues to render with full layout wrapping (no regression)
- [ ] Pages with `layout: 404` (integer) resolve to layout file `_layouts/404.html`
- [ ] Pages with `layout: "default"` (string) continue to work as before
- [ ] Pages with `layout: null` continue to get no layout (no regression)
- [ ] Pages with `layout: true` or `layout: 3.14` also resolve correctly (edge cases)
- [ ] DTC DOM baseline: 787/787 pages matched -- must not regress

## Test Scenarios

### Unit: yaml_value_to_layout_string (or equivalent helper)
- Input: `serde_yaml::Value::Number(404)` -> Output: `Some("404")`
- Input: `serde_yaml::Value::String("default")` -> Output: `Some("default")`
- Input: `serde_yaml::Value::String("")` -> Output: `None`
- Input: `serde_yaml::Value::Null` -> Output: `None`
- Input: `serde_yaml::Value::Bool(true)` -> Output: `Some("true")`
- Input: `serde_yaml::Value::Number(3.14)` -> Output: `Some("3.14")` (or similar)

### Unit: resolve_layout with numeric layout
- Create a `CollectionItem` with `front_matter: {"layout": 404}` (integer)
- Call `resolve_layout()` and assert it returns `Some("404")`

### Integration: Yat theme build
- Build Yat theme and verify `404.html` output is non-empty
- Verify `404.html` contains `<!DOCTYPE html>` (layout-wrapped)
- Verify `about.html` output is non-empty and layout-wrapped
- Verify no new rendering errors in build output

### Regression: DTC site
- Build DTC site and run DOM comparison
- Must stay at 787/787 matched pages

## Dependencies

- None (standalone fix)
