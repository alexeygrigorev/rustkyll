# Issue 141: Fix heading ID generation (double-dash and ampersand handling)

## Problem

Kramdown generates heading IDs differently from rustkyll in two ways:

1. **Double-dash preservation**: When a heading contains special characters that map to dashes (e.g., `DevOps / Site Reliability Engineer`), kramdown preserves double dashes (`devops--site-reliability-engineer`) while rustkyll collapses them to single dash (`devops-site-reliability-engineer`). 12 instances across 3 files.

2. **Ampersand in IDs**: Kramdown keeps `--` for `&` in headings (e.g., `free--free-to-audit-courses`) while rustkyll converts `&` to `amp` in the ID (e.g., `free-amp-free-to-audit-courses`). 7 instances across 2 files.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Heading IDs match kramdown output for special characters
- Double dashes preserved where kramdown preserves them
- Ampersand handling matches kramdown behavior
- No regressions
