# Issue 233: Fix DTC/docs (just-the-docs theme) — 0/57 pages

## Problem

DataTalks.Club/docs uses `theme: just-the-docs` with custom layouts. Currently 0/57 pages match. The site has 0 liquid leaks (fixed by issue 215), so pages render but with systematic differences.

### Diff patterns (all 57 pages):

1. **Navigation `<style>` block** — Jekyll generates complex CSS for active nav item highlighting based on `_data/nav.yml` with `:nth-child()` selectors. Rustkyll outputs simplified fallback CSS. This is the biggest single diff.

2. **`<head>` element ordering** — `<link>` and `<title>` appear in wrong order (child[9] vs child[10] swapped). This cascades into many `tag_name_differs`.

3. **Missing `generator` meta tag** — Jekyll outputs `<meta name="generator" content="Jekyll v4.4.1">`, rustkyll doesn't.

4. **Extra SEO meta/link tags** — Rustkyll injects OpenGraph and other SEO tags that Jekyll either doesn't generate or positions differently.

## Site config

```yaml
theme: just-the-docs
title: DataTalks.Club Documentation
url: "https://datatalks.club"
repository: DataTalksClub/docs
permalink: pretty
```

Has custom `_layouts/` (about, default, home, minimal, page, post, table_wrappers, vendor, vocabulary_term).

## Goal

Get DTC/docs pages matching by fixing head element ordering, generator meta, and navigation CSS generation.

## Acceptance Criteria

- [ ] `<head>` elements appear in the same order as Jekyll output
- [ ] `<meta name="generator">` tag included with version string
- [ ] Navigation CSS matches Jekyll's data-driven generation (or close enough)
- [ ] At least 30/57 pages match (>50%)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with new tests
