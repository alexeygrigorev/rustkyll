# Issue 55: serve command should default to current directory

## Problem

`rustkyll serve` requires `--source /path/to/site` to work. Jekyll's `serve` command defaults to the current directory -- you just `cd` into your site and run `jekyll serve`.

## Current State

The CLI already defines `default_value = "."` for the `--source` flag on both `build` and `serve` commands. However, this behavior needs to be verified end-to-end: that using `"."` (or omitting `--source` entirely) correctly resolves `_config.yml`, `_layouts`, `_includes`, `_data`, collections, and the destination directory relative to the current working directory.

Additionally, the destination `_site` is relative to CWD, not relative to `--source`. This matches Jekyll behavior but needs to be explicitly tested.

## Goal

Match Jekyll's behavior: `rustkyll serve` (with no `--source` flag) should serve the site in the current working directory. Same for `rustkyll build`.

## Expected behavior

```bash
cd /path/to/my-site
rustkyll serve --port 4000
# Should work, building and serving the site in the current directory

cd /path/to/my-site
rustkyll build
# Should build _site/ in the current directory
```

The `--source` flag should still work as an override, just like Jekyll.

## Dependencies

None

## Scope

This is primarily a testing and verification issue. The CLI defaults are already in place. The work involves:

1. Verifying that all path resolution works correctly when `--source` is `"."` (the default)
2. Ensuring `destination` defaults to `_site` relative to CWD (not relative to `--source`)
3. Adding tests that exercise the full build pipeline with default arguments
4. Fixing any path resolution bugs discovered during testing

## Acceptance Criteria

- [ ] `rustkyll build` with no `--source` flag reads `_config.yml` from the current working directory
- [ ] `rustkyll build` with no `--destination` flag writes output to `_site/` in the current working directory
- [ ] `rustkyll serve` with no `--source` flag reads `_config.yml` from the current working directory
- [ ] `--source /some/path` still works as an override for both `build` and `serve`
- [ ] `--destination /some/path` still works as an override for both `build` and `serve`
- [ ] When `--source` is omitted, layouts, includes, data, and collections are all found relative to CWD
- [ ] When `--source` is a relative path like `../other-site`, it resolves correctly
- [ ] The `build_site` function works correctly when passed `Path::new(".")` as source
- [ ] All existing tests still pass
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes

## Test Scenarios

### Unit: CLI argument parsing (already exists, extend if needed)
- Parse `rustkyll build` with no args, verify `source` is `"."` and `destination` is `"_site"`
- Parse `rustkyll serve` with no args, verify `source` is `"."` and `destination` is `"_site"`
- Parse `rustkyll build --source /tmp/site`, verify source is `/tmp/site`
- Parse `rustkyll build --source ../relative`, verify source is `../relative`

### Integration: Build with default source directory
- Create a temporary directory with a minimal site (`_config.yml`, one page with front matter)
- Call `build_site(Path::new("."), ...)` from within that directory (or equivalently, pass the temp dir as source using `"."` semantics)
- Verify output is generated in the destination directory
- Verify `_config.yml` was read correctly (check a config value in the output)

### Integration: Build with explicit source vs default
- Create a temp site directory
- Build with `--source <temp_dir>` and verify output
- Build with source as `"."` from within the temp dir and verify identical output

### Integration: Destination is relative to CWD
- Create a temp site, build with default destination
- Verify `_site/` appears as a subdirectory (relative to CWD, not nested inside source if source differs)

### Edge case: Source directory does not contain _config.yml
- Call `build_site` with a directory that has no `_config.yml`
- Verify graceful error handling (not a panic)
