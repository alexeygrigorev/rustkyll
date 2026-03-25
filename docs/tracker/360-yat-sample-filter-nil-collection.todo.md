# Issue 360: Fix sample filter on nil collection (Yat theme)

## Problem

The Yat theme's `post.html` layout uses `site[page.collection] | sample:4` for the "Related Articles" section. When `page.collection` is nil (e.g., the about page which uses the post layout but is not a collection page), `site[nil]` evaluates to nil, and the `sample` filter fails with "Expected scalar, found nil".

This causes `about.html` and `404.html` to render without layout wrapping (fallback output).

## Root Cause

The `sample` filter (or possibly the bracket access `site[nil]`) does not handle nil input gracefully. Jekyll returns an empty array for `site[nil] | sample:4`, allowing the template to continue.

## Discovered In

Issue #243 (Yat theme benchmark)

## Acceptance Criteria

- [ ] `site[nil]` returns nil without error
- [ ] `nil | sample:4` returns an empty array (or nil) without error
- [ ] The Yat theme's `about.html` and `404.html` pages render with full layout wrapping
- [ ] No DTC DOM regression (baseline: 771/790)
