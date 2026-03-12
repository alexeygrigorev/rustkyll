# Issue 04: Data File Loading

## Description

Load YAML data files from `_data/` directory and make them available as `site.data.*`. Support nested directories (e.g., `_data/faqs/` becomes `site.data.faqs.*`). Files: events.yaml, events_extra.yaml, header.yaml, navigation.yaml, sponsors.yaml, faqs/*.yml.

## Dependencies

- Issue 01 (project setup)

## Scope

- `src/data.rs` module
- Recursively load `_data/` directory
- Parse YAML files into a tree structure (nested HashMap or serde_yaml::Value)
- Subdirectories become nested keys
- Unit tests loading actual data files from the Jekyll site
