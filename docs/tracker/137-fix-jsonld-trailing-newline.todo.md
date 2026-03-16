# Issue 137: Fix JSON-LD trailing newline in person description

## Problem

In JSON-LD structured data, the `description` field for person/author entries has a double trailing newline (`\n\n`) in rustkyll output, while Jekyll outputs a single trailing newline (`\n`). This affects ~211 files.

Also, podcast transcript fields have minor whitespace differences in ~59 files.

Discovered in issue #119 DOM diff audit.

## Example

Jekyll: `"description": "Valeriia Kuka is a Content Manager...\n"`
Rustkyll: `"description": "Valeriia Kuka is a Content Manager...\n\n"`

## Acceptance criteria

- Person description in JSON-LD has single trailing newline matching Jekyll
- Podcast transcript text matches Jekyll output
- No regressions in JSON-LD output
