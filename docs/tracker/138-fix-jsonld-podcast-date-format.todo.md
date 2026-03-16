# Issue 138: Fix JSON-LD podcast date format (dateModified, startDate, endDate)

## Problem

In JSON-LD for podcast episodes, `dateModified`, `startDate`, and `endDate` are rendered as full datetime strings (e.g., `"2026-03-16 08:23:32 +0100"`) by Jekyll, but rustkyll renders them differently (e.g., `"2026-03-16 08:30:38 +0000"` or just `"2025-11-07"`). This affects ~386 date fields across ~200 podcast files.

Issues:
1. `dateModified` includes the build timestamp (with timezone), but rustkyll uses UTC (+0000) instead of the local timezone. This also differs because of different build times.
2. `startDate`/`endDate` are rendered as date-only strings by rustkyll (`"2025-11-07"`) but as full datetime by Jekyll (`"2025-11-07 00:00:00 +0100"`).

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Podcast `startDate`/`endDate` format matches Jekyll (full datetime with timezone)
- `dateModified` uses the same timezone format as Jekyll
- No regressions
