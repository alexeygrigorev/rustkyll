# Issue 04: Data File Loading

## Description

Load YAML data files from `_data/` directory and make them available as `site.data.*`. Support nested directories (e.g., `_data/faqs/` becomes `site.data.faqs.*`). Files: events.yaml, events_extra.yaml, header.yaml, navigation.yaml, sponsors.yaml, faqs/*.yml.

## Dependencies

- Issue 01 (project setup) -- done

## Scope

- `src/data.rs` module
- Recursively load `_data/` directory
- Parse YAML files into a tree structure (nested HashMap or serde_yaml::Value)
- Subdirectories become nested keys
- Unit tests loading actual data files from the Jekyll site

## Data File Inventory

The `_data/` directory in the reference Jekyll site contains:

### Top-level files

| File | Format | Structure |
|------|--------|-----------|
| `events.yaml` | YAML | Array of mappings. Each item has: `time` (datetime), `title` (string), `speakers` (array of strings), `type` (string: podcast/workshop/webinar), `link` (url), optionally `youtube` (url), optionally `draft` (bool). ~3065 lines, hundreds of entries. |
| `events_extra.yaml` | YAML | Array of mappings. Same schema as events but smaller (extra events not in main list). Items have: `title`, `speakers`, `youtube`. |
| `header.yaml` | YAML | Mapping with key `announcement` containing: `text` (string), `link_text` (string), `link` (url). |
| `navigation.yaml` | YAML | Mapping with keys `top` and `bottom`, each an array of mappings with: `text` (string), `link` (url), `new_window` (bool). |
| `sponsors.yaml` | YAML | Array of mappings. Each item has: `name` (string), `link` (url), `image` (path), `from` (datetime), `to` (datetime). |

### Subdirectory: `faqs/`

Contains 9 `.yml` files, each an array of FAQ mappings with `question` (string) and `answer` (multiline string). Files:
- `ai-dev-tools-zoomcamp.yml`
- `data-engineering-zoomcamp.yml`
- `data-science-slack-communities.yml`
- `free-datatalksclub-courses-zoomcamps.yml`
- `free-ml-courses.yml`
- `llm-zoomcamp.yml`
- `machine-learning-zoomcamp.yml`
- `mlops-zoomcamp.yml`
- `open-source-free-ai-agent-evaluation-tools.yml`

These are accessed in templates as `site.data.faqs.data-engineering-zoomcamp`, etc.

## How Data Files Are Used in Templates

Templates access these via `site.data.<key>`:
- `site.data.header.announcement` -- header bar announcement
- `site.data.navigation.top` -- top navigation links
- `site.data.events` -- event listings on index, events page, author pages
- `site.data.sponsors` -- sponsor logos on index page
- `site.data.faqs.<slug>` -- FAQ accordions in blog posts (e.g., `site.data.faqs.data-engineering-zoomcamp`)

Note: `events_extra` does not appear in current templates but must still be loadable as `site.data.events_extra`.

## Implementation Notes

- Use `serde_yaml::Value` as the value type -- this preserves all YAML types (strings, numbers, booleans, datetimes, arrays, mappings) without needing typed structs for every file.
- The data tree should be a `HashMap<String, serde_yaml::Value>` where the key is derived from the filename (without extension). For subdirectories, the directory name maps to a nested `HashMap` (or `serde_yaml::Mapping`).
- Both `.yaml` and `.yml` extensions must be supported.
- File stem becomes the key (e.g., `events.yaml` -> key `events`, `faqs/data-engineering-zoomcamp.yml` -> nested key `faqs.data-engineering-zoomcamp`).
- Provide a public function like `load_data(data_dir: &Path) -> Result<HashMap<String, serde_yaml::Value>>` that the site builder will call.
- Errors on invalid YAML should include the filename in the error message for debuggability.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] A `src/data.rs` module exists and is registered in `lib.rs`
- [ ] `load_data()` function accepts a `Path` to the `_data/` directory and returns a `Result` containing a data tree
- [ ] Top-level `.yaml` and `.yml` files are loaded; the file stem (without extension) is used as the key
- [ ] Subdirectories create nested keys (e.g., `_data/faqs/foo.yml` is accessible as `data["faqs"]["foo"]`)
- [ ] Loading the actual `datatalksclub.github.io/_data/` directory succeeds without errors
- [ ] The loaded `events` key is a YAML sequence (array) with hundreds of entries
- [ ] The loaded `header` key is a YAML mapping containing an `announcement` key
- [ ] The loaded `navigation` key is a YAML mapping with `top` and `bottom` keys
- [ ] The loaded `sponsors` key is a YAML sequence
- [ ] The loaded `faqs` key is a mapping with 9 sub-keys (one per FAQ file)
- [ ] Each FAQ sub-key (e.g., `faqs["data-engineering-zoomcamp"]`) is a YAML sequence of question/answer mappings
- [ ] Invalid YAML produces an error that includes the filename
- [ ] Empty `_data/` directory returns an empty (but valid) data tree
- [ ] Non-existent `_data/` directory returns a clear error
- [ ] `cargo test` passes with all tests below

## Test Scenarios

### Unit: YAML parsing basics
- Load a single YAML file containing a sequence, verify it parses to a `Value::Sequence`
- Load a single YAML file containing a mapping, verify it parses to a `Value::Mapping`
- Attempt to load an invalid YAML file, verify the error includes the filename

### Unit: Directory traversal and key derivation
- Given a file `foo.yaml`, verify the key is `"foo"`
- Given a file `bar.yml`, verify the key is `"bar"` (both extensions work)
- Given a subdirectory `sub/baz.yml`, verify the result has `data["sub"]["baz"]`

### Unit: Empty and missing directories
- Load from an empty directory, verify result is an empty HashMap
- Load from a non-existent path, verify a descriptive error is returned

### Integration: Load actual DataTalks.Club data
- Load `datatalksclub.github.io/_data/` and verify:
  - `data["events"]` is a sequence with length > 100
  - `data["events_extra"]` is a sequence
  - `data["header"]` is a mapping, and `data["header"]["announcement"]` exists
  - `data["navigation"]` is a mapping with keys `"top"` and `"bottom"`
  - `data["navigation"]["top"]` is a sequence with length > 0
  - `data["sponsors"]` is a sequence with length > 0
  - `data["faqs"]` is a mapping with exactly 9 keys
  - `data["faqs"]["data-engineering-zoomcamp"]` is a sequence of mappings, each with `"question"` and `"answer"` keys
  - The first event in `data["events"]` has keys `"time"`, `"title"`, `"speakers"`, `"type"`, `"link"`
  - A sponsor entry has keys `"name"`, `"link"`, `"image"`, `"from"`, `"to"`
