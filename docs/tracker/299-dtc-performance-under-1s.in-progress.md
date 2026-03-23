# Issue 299: DTC build performance < 1.0s

## Problem

DTC builds in 1.23s, target is < 1.0s. Liquid template rendering is the bottleneck (0.75s for 789 pages). Issue 295 brought it from 1.7s to 1.23s.

## Acceptance Criteria

- [ ] DTC builds in < 1.0s (release mode)
- [ ] No regressions

## Log

### [SWE] 2026-03-23

Baseline: 1.30s median (Generation 0.82s)

**Optimizations implemented:**

1. **Hoisted `get_config_timezone()` out of parallel loops** (src/generator.rs)
   - Was calling `iana_time_zone::get_timezone()` syscall 780 times per build
   - Now computed once before the parallel loop

2. **Pre-converted front matter YAML to Liquid Object once per page** (src/template/layout.rs)
   - Added `preconvert_frontmatter_to_liquid()` and `build_render_context_from_preconverted()`
   - Avoids redundant `yaml_to_liquid()` calls when same page renders through content + layout passes
   - Added `render_with_cached_site_preconverted` and `render_with_site_overrides_preconverted`

3. **Lazy LenientValue children** (src/template/engine.rs)
   - Changed `children` and `array_children` from eager `HashMap`/`Vec` to `OnceLock<...>`
   - Nested objects/arrays are only materialized when actually accessed
   - Helps site context (large nested object) defer work to point of use

4. **Pre-computed layout Liquid Objects** (src/template/layout.rs)
   - Layout front matter converted to Liquid Object once at construction time
   - Shared across all pages using same layout (e.g., 429 people pages)

5. **Avoided redundant normalize_arrays for scalar values** (src/generator.rs)
   - `collection_item_to_liquid_slim`, `collection_item_to_liquid_ultra_slim`, `page_to_liquid`
   - Only call normalize_arrays when value is actually an Array

6. **Release profile: thin LTO + codegen-units=1** (Cargo.toml)
   - Added `[profile.release]` with `lto = "thin"` and `codegen-units = 1`
   - Cross-crate inlining of liquid/serde_yaml/etc. function calls

7. **Avoided content string clone via LiquidValue reuse** (src/template/layout.rs)
   - Content scalar built once and cloned (Arc-backed with kstring arc feature)

**Results:**
- Median: 1.19s (baseline 1.30s, -8.5%)
- Best: 1.14s
- Generation phase: 0.76s (baseline 0.82s)
- All 2,908 tests pass (0 failures, 2 ignored)
- Clippy clean, fmt clean

**Why not <1.0s:**
- The remaining bottleneck is the Liquid template rendering engine itself (~0.76s for 792 pages)
- The DTC podcast layout is 598 lines with complex for-loops over transcript arrays
- Each page still requires: Object clone -> LenientValue wrapping -> Liquid eval -> String output
- Further gains would require either forking the liquid crate for zero-copy rendering
  or implementing template-level output caching for shared includes

**Files modified:**
- `Cargo.toml` - Added [profile.release] section
- `src/generator.rs` - Timezone hoisting, normalize_arrays optimization
- `src/template/engine.rs` - Lazy LenientValue children
- `src/template/layout.rs` - Pre-conversion, pre-computed layout objects
