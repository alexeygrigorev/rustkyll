# Issue 43: Handle Duplicate YAML Keys in Config

## Problem

Complex site testing (Issue 35) revealed that bitcoin.org has duplicate keys in its `_config.yml` (e.g., duplicate redirect entries). The `serde_yaml` parser rejects these with "duplicate entry with key" errors, while Ruby's YAML parser silently takes the last value.

## Affected Sites

- bitcoin.org -- duplicate redirect keys in `_config.yml`

## Requirements

- Handle duplicate YAML keys gracefully (either last-wins like Ruby, or warn and continue)
- Do not fail the entire build on duplicate config keys

## Dependencies

None.
