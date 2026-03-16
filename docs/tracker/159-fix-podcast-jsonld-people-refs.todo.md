# Issue 159: Fix podcast JSON-LD unresolved people references (201 diffs)

## Problem

Podcast JSON-LD has unresolved `site.people.X.picture` references that should resolve to actual image URLs. 201 diffs across 193 podcast files.

## Acceptance criteria

- site.people references in podcast JSON-LD resolve correctly
- 201 DOM diffs eliminated
- TDD: failing test, fix, test passes
