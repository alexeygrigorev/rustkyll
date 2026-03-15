# Issue 97: Improve structural comparison to catch all HTML differences

## Problem

The structural comparison script (scripts/compare-output.sh) only extracts headings, links, and images for comparison. It misses:
- `<p>&nbsp;</p>` spacer elements
- CSS classes on elements
- Inline styles
- Data attributes
- Form elements
- Script/noscript blocks
- Arbitrary HTML elements that aren't headings/links/images

This means the "structural match" claim is incomplete — many HTML differences go undetected.

## Goal

The structural comparison should detect ALL meaningful HTML differences, not just headings and links. Two HTML files are structurally equivalent when their DOM trees produce the same rendered output (ignoring whitespace-only differences and attribute ordering).

## Approach

Replace the grep-based extraction with a proper DOM tree comparison:
1. Parse both HTML files into DOM trees (use Python with BeautifulSoup or lxml)
2. Normalize: sort attributes alphabetically, collapse insignificant whitespace (whitespace between tags, not inside text nodes)
3. Compare the full normalized DOM trees — not just headings and links
4. Report every difference with XPath or element context
5. The DOM trees must be IDENTICAL after normalization — same elements, same attributes, same text content, same nesting

## What "same DOM tree" means

Two HTML files have the same DOM tree when:
- Same elements in same order at same nesting depth
- Same attributes on each element (values match, order doesn't matter)
- Same text content in each text node (whitespace-normalized)
- Same number of child elements per parent
- `<p>&nbsp;</p>` in one but not the other = DIFFERENT
- `class="foo"` in one but not the other = DIFFERENT
- `target="_blank"` in one but not the other = DIFFERENT

What to ignore:
- Attribute ordering (`class="a" id="b"` == `id="b" class="a"`)
- Whitespace between tags (`<div> <span>` == `<div><span>`)
- Trailing whitespace in text nodes

## Acceptance criteria

- Comparison parses full DOM tree, not just extracted elements
- Detects `<p>&nbsp;</p>` differences
- Detects CSS class differences on ANY element
- Detects missing/extra elements at any depth
- Detects attribute differences (target, rel, class, id, data-*, etc.)
- Detects text content differences
- Ignores attribute ordering
- Ignores insignificant whitespace between tags
- Per-file report: list of differences with element context
- Summary: files matching, files with diffs, total diff count
- Script exits nonzero when any file has DOM differences
- For DTC site: run on ALL 787 HTML files, report exact match count
- Results documented with per-file status
