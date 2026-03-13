# Issue 23: Flexible Config Parsing

## Problem

Rustkyll requires `url`, `name`, and `title` as mandatory string fields. Real Jekyll sites often omit them. The `twitter` field is hardcoded as `Option<String>`, but some sites use it as a map. Unknown config keys cause parse errors.

This blocks ALL external Jekyll sites from building.

## Requirements

- Make `url`, `name`, `title` optional with sensible defaults (empty string for `url`, empty string for `name` and `title`)
- Change `twitter` from `Option<String>` to `Option<serde_yaml::Value>` to accept both strings and maps
- Allow unknown config keys using `#[serde(flatten)]` with a catch-all `HashMap<String, serde_yaml::Value>`
- Expose extra config values in the template context as `site.<key>` (e.g., a config key `locale: "en"` must be accessible as `{{ site.locale }}` in templates)
- All existing tests must continue to pass (the DTC site config still has these fields, so they should parse fine with defaults)

## Scope

This issue is limited to `src/config.rs` and `src/generator.rs` (the `build_site_context` function). No changes to filters, layouts, or CLI.

## Dependencies

- Issue #22 (done) -- provides research context

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] A `_config.yml` with no `url`, `name`, or `title` fields parses successfully, with empty-string defaults
- [ ] A `_config.yml` with only comments (no actual key-value pairs) parses successfully
- [ ] An empty `_config.yml` file parses successfully
- [ ] A `_config.yml` with `twitter: "@handle"` (string) parses successfully
- [ ] A `_config.yml` with `twitter: { username: "handle" }` (map) parses successfully
- [ ] A `_config.yml` with arbitrary unknown keys (e.g., `locale`, `sass`, `kramdown`, `paginate`, `timezone`, `markdown`, `highlighter`) parses successfully without errors
- [ ] Unknown config keys are stored in the catch-all `HashMap` and are accessible via a public method or field
- [ ] `build_site_context` populates `site.url`, `site.name`, `site.title` from config (using defaults if not provided)
- [ ] `build_site_context` populates `site.twitter` correctly for both string and map values (string renders as string, map renders as nested object)
- [ ] `build_site_context` populates `site.<key>` for every unknown/extra config key from the catch-all map
- [ ] The DTC site (`datatalksclub.github.io/`) still builds successfully with existing config
- [ ] Programmatic construction of `SiteConfig` (used in tests) still works -- provide sensible `Default` or adjust struct construction sites

## Test Scenarios

### Unit: Optional fields with defaults

- Parse YAML with no `url` field -- verify `config.url` is `""`
- Parse YAML with no `name` field -- verify `config.name` is `""`
- Parse YAML with no `title` field -- verify `config.title` is `""`
- Parse YAML with none of `url`, `name`, `title` -- verify all default to `""`
- Parse YAML with only `url` set -- verify `name` and `title` default, `url` has provided value
- Parse empty string YAML -- verify it parses (empty/default config, not an error)
- Parse YAML that is entirely comments -- verify it parses to defaults

### Unit: Twitter as flexible type

- Parse `twitter: "@DataTalksClub"` -- verify stored as `Value::String`
- Parse `twitter: { username: "handle" }` -- verify stored as `Value::Mapping`
- Parse config with no `twitter` key -- verify `twitter` is `None`
- Parse `twitter: null` -- verify `twitter` is `None` or `Some(Value::Null)` (either is acceptable)

### Unit: Unknown config keys (catch-all)

- Parse YAML with `locale: "en"` -- verify it is captured in the extras map
- Parse YAML with `sass: { style: compressed }` -- verify the nested map is captured
- Parse YAML with `kramdown: { input: GFM }` -- verify captured
- Parse YAML with 10+ unknown keys -- verify all are captured, none cause errors
- Verify known fields (`url`, `name`, `permalink`, `collections`, etc.) are NOT duplicated in the extras map

### Unit: Existing fields still work

- Parse the DTC `_config.yml` -- verify `url`, `name`, `title`, `twitter`, `permalink`, `collections`, `defaults`, `exclude`, `repository` all parse correctly (same values as before)

### Integration: Site context population

- Build a site context from a config with `url: ""` (default) -- verify `site.url` renders as `""`
- Build a site context from a config with extras `{locale: "en", author: "Alice"}` -- verify `{{ site.locale }}` renders `"en"` and `{{ site.author }}` renders `"Alice"`
- Build a site context from a config with `twitter: { username: "handle" }` -- verify `{{ site.twitter.username }}` renders `"handle"`
- Build a site context from a config with a nested extra like `sass: { style: compressed }` -- verify `{{ site.sass.style }}` renders `"compressed"`

### Regression: DTC site

- Run the full build against `datatalksclub.github.io/` and verify it still produces the same number of pages (777) and no new errors

## Notes

- The `SiteConfig` struct currently has fields directly used in `build_site_context`. After this change, the generator must iterate over the extras map and insert each key-value pair into the site Liquid object.
- The `twitter` field is currently inserted as a scalar string in `build_site_context`. After this change, it must use `yaml_to_liquid()` to convert the `serde_yaml::Value` to a Liquid value so maps work correctly.
- Be careful with `#[serde(flatten)]`: it captures ALL keys not matched by named fields. Verify that `collections`, `defaults`, `exclude`, and other explicitly-typed fields do not leak into the extras map.

## References

- Issue #22 compatibility research, gaps #1 and #2
- `src/config.rs` -- `SiteConfig` struct and parsing
- `src/generator.rs` -- `build_site_context` function
- `src/template/context.rs` -- `yaml_to_liquid` conversion utility
