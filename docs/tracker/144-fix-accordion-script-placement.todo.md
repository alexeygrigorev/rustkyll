# Issue 144: Fix accordion script tag placement/attributes

## Problem

Some pages include `<script src='/assets/accordion.js'>` for FAQ accordion functionality. In rustkyll, the script tag is either missing, in the wrong position, or has different attributes than Jekyll's output. 9 instances across 4 files.

The issue manifests as:
- `missing_attribute: src='/assets/accordion.js'` (script exists but without src)
- `extra_attribute: type='application/ld+json'` (script has wrong type)

This suggests rustkyll is confusing the accordion script with a JSON-LD script.

Discovered in issue #119 DOM diff audit.

## Acceptance criteria

- Accordion script tags match Jekyll placement and attributes
- No regressions
