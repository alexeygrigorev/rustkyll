# Issue 02: Configuration Parsing

## Description

Parse the Jekyll `_config.yml` file into a Rust struct. Extract site metadata (url, name, title, twitter), collections config (name, output, permalink pattern), default layouts per collection, permalink patterns, and exclude list.

## Dependencies

- Issue 01 (project setup) -- done

## Scope

- `src/config.rs` module
- `SiteConfig` struct with all relevant fields
- Parse the actual `datatalksclub.github.io/_config.yml`
- Handle missing optional fields gracefully
- Unit tests for parsing

## Reference: Actual _config.yml Structure

The real config file (`datatalksclub.github.io/_config.yml`) contains these top-level keys:

- `theme`: `jekyll-theme-cayman` (string, not needed for our generator)
- `url`: `https://datatalks.club` (string)
- `name`: `DataTalks.Club` (string)
- `title`: `DataTalks.Club` (string)
- `twitter`: `@DataTalksClub` (string)
- `permalink`: `/blog/:title.html` (string -- default permalink for posts)
- `exclude`: list of 12 strings (directories and files to skip)
- `collections`: map of 6 collections (books, people, conferences, podcast, courses, tools), each with `output: bool` and `permalink: string`
- `defaults`: list of 3 scope/values entries mapping collection types to layouts (people->author, books->book, podcast->podcast)

## Implementation Details

### SiteConfig struct

Must include at minimum:

- `url: String`
- `name: String`
- `title: String`
- `twitter: Option<String>`
- `permalink: String` (global default for posts)
- `exclude: Vec<String>`
- `collections: HashMap<String, CollectionConfig>`
- `defaults: Vec<DefaultConfig>`

### CollectionConfig struct

- `output: bool`
- `permalink: String`

### DefaultConfig struct

- `scope: DefaultScope` (with `path: String` and `type_name: String`)
- `values: DefaultValues` (with `layout: String`)

### Loading

- Provide a function like `SiteConfig::from_file(path: &Path) -> Result<SiteConfig, ConfigError>`
- Use `serde` + `serde_yaml` for deserialization
- Define a custom error type (or use `thiserror`) for config-related errors

### Convenience methods

- `SiteConfig::default_layout_for(&self, collection_type: &str) -> Option<&str>` -- look up the default layout for a collection type from the `defaults` list
- `SiteConfig::collection(&self, name: &str) -> Option<&CollectionConfig>` -- look up a collection by name

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes with no warnings
- [ ] `cargo fmt -- --check` shows no formatting issues
- [ ] A `src/config.rs` module exists and is registered in `src/lib.rs`
- [ ] `SiteConfig::from_file` successfully parses `datatalksclub.github.io/_config.yml` without errors
- [ ] Parsed `url` equals `"https://datatalks.club"`
- [ ] Parsed `name` equals `"DataTalks.Club"`
- [ ] Parsed `title` equals `"DataTalks.Club"`
- [ ] Parsed `twitter` equals `Some("@DataTalksClub")`
- [ ] Parsed `permalink` equals `"/blog/:title.html"`
- [ ] Parsed `exclude` contains exactly 12 items, including `"Gemfile"`, `"node_modules/"`, and `"scripts/"`
- [ ] Parsed `collections` contains 6 entries: books, people, conferences, podcast, courses, tools
- [ ] Each collection has `output: true` and `permalink: "/:collection/:title.html"`
- [ ] Parsed `defaults` contains 3 entries mapping people->author, books->book, podcast->podcast
- [ ] `default_layout_for("people")` returns `Some("author")`
- [ ] `default_layout_for("books")` returns `Some("book")`
- [ ] `default_layout_for("podcast")` returns `Some("podcast")`
- [ ] `default_layout_for("courses")` returns `None` (no default layout defined for courses)
- [ ] `collection("books")` returns the books collection config
- [ ] `collection("nonexistent")` returns `None`
- [ ] Parsing a YAML file with missing optional fields (e.g., no `twitter`) does not error
- [ ] Parsing an invalid YAML file returns a meaningful error (not a panic)
- [ ] Parsing a YAML file with missing required fields (e.g., no `url`) returns an error
- [ ] `cargo test` passes with all tests green

## Test Scenarios

### Unit: Parse the real config file
- Load `datatalksclub.github.io/_config.yml`, verify `url`, `name`, `title`, `twitter`, `permalink` are correctly extracted
- Verify the exclude list has 12 entries and contains known items like `"Gemfile"` and `"node_modules/"`

### Unit: Collections parsing
- Load the real config, verify 6 collections are parsed
- Verify each collection name matches expected (books, people, conferences, podcast, courses, tools)
- Verify all collections have `output: true`
- Verify all collections have permalink `"/:collection/:title.html"`

### Unit: Defaults parsing
- Load the real config, verify 3 default entries are parsed
- Verify people maps to layout "author"
- Verify books maps to layout "book"
- Verify podcast maps to layout "podcast"

### Unit: Convenience methods
- `default_layout_for("people")` returns `Some("author")`
- `default_layout_for("books")` returns `Some("book")`
- `default_layout_for("podcast")` returns `Some("podcast")`
- `default_layout_for("courses")` returns `None`
- `default_layout_for("nonexistent")` returns `None`
- `collection("books")` returns `Some(...)` with correct output and permalink
- `collection("nonexistent")` returns `None`

### Unit: Missing optional fields
- Parse a YAML string with only `url`, `name`, `title`, `permalink` (no `twitter`, no `exclude`, no `collections`, no `defaults`)
- Verify `twitter` is `None`, `exclude` is empty vec, `collections` is empty map, `defaults` is empty vec

### Unit: Error handling
- Parse an empty string -- should return an error
- Parse invalid YAML (e.g., `": : :"`) -- should return an error, not panic
- Parse valid YAML missing required field `url` -- should return an error with a message indicating what is missing

### Unit: Round-trip sanity
- Construct a `SiteConfig` programmatically, verify convenience methods work correctly on it
