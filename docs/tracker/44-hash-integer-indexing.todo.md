# Issue 44: Support Integer Indexing on Hash/Map Values in Liquid

## Problem

Complex site testing (Issue 35) revealed that GitHub's opensource.guide uses `locale[0]` to access the first key-value pair of a hash (data file loaded as a map). Jekyll/Liquid allows integer indexing on hashes (accessing by position), but rustkyll's Liquid engine only supports string key access on maps.

## Affected Sites

- opensource.guide -- `{% assign lang = locale[0] %}` where `locale` is a hash with keys like `en`, `es`, etc.

## Requirements

- Support integer indexing on Liquid objects/hashes (access by position)
- `hash[0]` should return the first key-value pair (or first value, matching Jekyll behavior)

## Dependencies

None.
