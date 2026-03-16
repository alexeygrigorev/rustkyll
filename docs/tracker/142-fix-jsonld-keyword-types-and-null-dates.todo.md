# Issue 142: Fix JSON-LD keyword type coercion and null date handling

## Problem

Two minor JSON-LD issues:

1. **String-vs-number in keywords**: When a keyword is a pure number (e.g., `2024`), Jekyll keeps it as a string (`"2024"`) in the JSON-LD keywords array, but rustkyll outputs it as a number (`2024`). 5 instances across 2 files.

2. **Null vs empty string for dates**: When a page has no date, Jekyll outputs `"datePublished": null` but rustkyll outputs `"datePublished": ""`. 2 instances in 1 file (slack/guidelines.html).

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Keywords in JSON-LD are always strings, even if they look like numbers
- Missing dates render as `null` not `""`
- No regressions
