# Issue 43: Handle Duplicate YAML Keys in Config

## Problem

`serde_yaml` 0.9 rejects YAML documents containing duplicate keys with a "duplicate entry with key" error. Ruby's YAML parser (Psych) silently accepts duplicate keys using last-value-wins semantics. Real-world Jekyll sites (e.g., bitcoin.org) have duplicate keys in `_config.yml`, causing rustkyll to fail where Jekyll succeeds.

This blocks building any site that has duplicate keys in its config, data files, or front matter.

## Scope

The duplicate key problem can appear in three places:

1. `_config.yml` parsing (`src/config.rs` -- `SiteConfig::from_yaml_str`)
2. Data file loading (`src/data.rs` -- `load_yaml_file`)
3. Front matter parsing (`src/frontmatter.rs` -- wherever YAML front matter is parsed)

All three use `serde_yaml::from_str` which rejects duplicates. All three must be fixed.

## Implementation Approach

**Option A (Recommended): Pre-process with `serde_yaml::Value` and deduplicate.**

Parse the raw YAML string into a `serde_yaml::Value` first (which still rejects duplicates in 0.9), so this alone is not enough.

**Option B (Recommended): Switch to `unsafe-serde-yaml` or a fork that allows duplicate keys.**

The `unsafe-serde-yaml` crate is a maintained fork of `serde_yaml` that allows duplicate keys with last-value-wins semantics. This is a drop-in replacement.

**Option C: Manual YAML pre-processing.**

Before passing to `serde_yaml`, scan the raw YAML text and remove duplicate keys. This is fragile for nested YAML.

**Option D: Use `yaml-rust2` directly for initial parsing.**

Parse with `yaml-rust2` (which allows duplicate keys), then convert to `serde_yaml::Value` or directly to the target structs.

**Recommended path:** Try Option B first (`unsafe-serde-yaml`). If that crate is unsuitable, use Option D. The engineer should evaluate and choose.

Whichever approach is chosen:
- Duplicate keys MUST use last-value-wins semantics (matching Ruby/Jekyll behavior)
- A warning log or `eprintln!` should be emitted when duplicate keys are detected (optional but preferred)
- The fix must apply everywhere YAML is parsed, not just `_config.yml`

## Files to Modify

- `Cargo.toml` -- potentially add/swap YAML dependency
- `src/config.rs` -- update `from_yaml_str` to handle duplicate keys
- `src/data.rs` -- update `load_yaml_file` to handle duplicate keys
- `src/frontmatter.rs` -- update front matter parsing to handle duplicate keys
- Any shared YAML parsing utility if one is extracted

## Dependencies

None. This is independent of other open issues.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] A `_config.yml` with duplicate top-level keys parses successfully (last value wins)
- [ ] A `_config.yml` with duplicate nested keys parses successfully (last value wins)
- [ ] A data file (`.yaml`/`.yml`) with duplicate keys parses successfully (last value wins)
- [ ] Front matter with duplicate keys parses successfully (last value wins)
- [ ] The DTC site (`datatalksclub.github.io/`) still builds successfully
- [ ] No panics or unwraps in the YAML parsing paths -- all errors are handled with `Result`

## Test Scenarios

### Unit: Duplicate keys in config YAML

- Parse config YAML with duplicate top-level key (e.g., `url` appears twice) -- verify second value is used
- Parse config YAML with duplicate key in a nested mapping (e.g., two `style` keys under `sass:`) -- verify last value wins
- Parse config YAML with duplicate key where values have different types (e.g., `foo: 1` then `foo: "bar"`) -- verify last value wins and type is correct
- Parse config YAML with three occurrences of the same key -- verify the third (last) value wins

### Unit: Duplicate keys in data files

- Load a data file with duplicate keys -- verify last value wins
- Load a data file with duplicate keys in a nested mapping -- verify last value wins

### Unit: Duplicate keys in front matter

- Parse front matter with duplicate keys (e.g., `title` appears twice) -- verify last value wins

### Integration: Backward compatibility

- All existing config tests still pass unchanged
- All existing data loading tests still pass unchanged
- All existing front matter tests still pass unchanged
- Build the DTC site end-to-end and verify no regressions

### Integration: Realistic duplicate key scenario

- Create a config file mimicking bitcoin.org's pattern (duplicate redirect-type entries) -- verify it parses without error and the last entry is preserved
