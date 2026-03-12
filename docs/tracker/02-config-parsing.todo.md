# Issue 02: Configuration Parsing

## Description

Parse the Jekyll `_config.yml` file into a Rust struct. Extract site metadata (url, name, title, twitter), collections config (name, output, permalink pattern), default layouts per collection, permalink patterns, and exclude list.

## Dependencies

- Issue 01 (project setup)

## Scope

- `src/config.rs` module
- `SiteConfig` struct with all relevant fields
- Parse the actual `datatalksclub.github.io/_config.yml`
- Handle missing optional fields gracefully
- Unit tests for parsing
