# Issue 155: Fix timestamp formatting (209 diffs)

## Problem

Podcast timestamps like `36.0` should render as `0:36`. YAML values like `0:36` are parsed as sexagesimal (36 seconds = 36.0) by some parsers. Need to format these back as `M:SS` or `H:MM:SS`.

## Acceptance criteria

- Podcast timestamps render as `M:SS` format matching Jekyll
- 209 DOM diffs eliminated
- TDD: failing test, fix, test passes
