# Issue 245: Fix DTC/docs head element ordering and CSS activation DOM match

## Problem

Issue 233 fixed the core just-the-docs theme support (navigation, grammar, map filter, group_by_exp). However, two acceptance criteria from issue 233 remain unmet:

1. **Head element ordering**: In rustkyll output, `<link rel="icon">` (favicon) appears before `<title>`, but Jekyll produces `<title>` before `<link rel="icon">`. This affects all 57 pages.

2. **CSS activation fallback**: The DOM comparison tool reports that non-homepage pages still show the fallback CSS rule (`.site-nav ul li a { background-image: none; }`) instead of the expected `:nth-child` selectors. Note: direct inspection of the current build output shows `:nth-child` selectors ARE present, so this may be a comparison-tool issue or a whitespace/formatting difference that causes the DOM tool to misclassify.

3. **Missing meta element**: The DOM comparison reports a missing `<meta>` element in the `<head>` section (57 pages).

Together these prevent any DOM matches for DTC/docs (0/57).

## Descoped from

Issue 233 acceptance criteria:
- "head elements appear in the same order as fresh Jekyll output"
- "At least 40/57 pages achieve DOM match (>70%)"

## Acceptance Criteria

- [ ] `<title>` appears before `<link rel="icon">` in the `<head>` section, matching Jekyll output order
- [ ] Identify and fix the missing `<meta>` element that the DOM comparison tool reports
- [ ] Re-run DOM comparison: at least 40/57 pages achieve DOM match
- [ ] `cargo test` passes with all existing and new tests

## Dependencies

- Issue 233 (done)
