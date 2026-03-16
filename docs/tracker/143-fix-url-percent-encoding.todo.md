# Issue 143: Fix URL percent-encoding for spaces in image/thumbnail URLs

## Problem

Jekyll percent-encodes spaces in URLs as `%20`, but rustkyll leaves them as literal spaces. This affects `thumbnailUrl` and `image` fields in JSON-LD for podcast pages with spaces in filenames.

Example:
- Jekyll: `hybrid%20search.jpg`
- Rustkyll: `hybrid search.jpg`

4 instances in 1 file.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- URLs with spaces are percent-encoded as `%20`
- No regressions in URL generation
