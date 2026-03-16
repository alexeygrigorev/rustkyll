# Issue 140: Fix book listing end date off-by-one

## Problem

On `books.html`, the book-of-the-week date ranges show end dates that are one day later in Jekyll than in rustkyll. This affects all 78 book entries.

Example:
- Jekyll: `(from 06 Oct 2025 to 11 Oct 2025)`
- Rustkyll: `(from 06 Oct 2025 to 10 Oct 2025)`

This is likely a timezone issue where the end date calculation rounds differently.

Related to issue #116 (books listing timezone regression) which was marked done but this specific symptom persists.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Book date ranges on books.html match Jekyll output exactly
- No regressions in other date calculations
