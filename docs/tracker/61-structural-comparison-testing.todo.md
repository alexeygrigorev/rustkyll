# Issue 61: Structural comparison testing for DTC site

## Problem

Issues #49 and #57 required structural comparison of rustkyll vs Jekyll output but this was descoped. We need to verify that rustkyll produces structurally equivalent HTML to Jekyll for the DTC site and kids-horror-stories-ru.

## Goal

Build and run a structural comparison test suite that:
1. Builds the site with both Jekyll and rustkyll
2. Compares file trees (same HTML files generated)
3. For each HTML file, extracts structural elements (title, h1-h6, links, images) and diffs them
4. Reports any structural differences
5. Exits nonzero if differences exceed thresholds

A comparison script already exists at scripts/compare-output.sh (from issue #49) but it was never verified end-to-end.

## Sites to compare

- DataTalksClub/datatalksclub.github.io
- kids-horror-stories-ru

## Dependencies

None

## Acceptance criteria

- Structural comparison script runs end-to-end on both sites
- File tree comparison: same HTML files generated (within 5% tolerance)
- For at least 10 sample DTC pages: title, headings, links, and images match Jekyll output
- No raw Liquid tags in any generated HTML
- No empty HTML files
- RSS/Atom feeds are valid XML with expected number of entries
- Sitemap lists the same URLs as Jekyll's sitemap (within 5% tolerance)
- Script exits nonzero on structural differences above threshold
- Results documented
