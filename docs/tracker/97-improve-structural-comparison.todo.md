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

Instead of extracting specific elements, use a proper HTML diff:
1. Parse both HTML files into DOM trees
2. Normalize (sort attributes, collapse whitespace)
3. Diff the normalized trees
4. Report differences with context

Consider using a Python script with BeautifulSoup or lxml instead of grep-based extraction.

## Acceptance criteria

- Comparison detects `<p>&nbsp;</p>` differences
- Comparison detects CSS class differences on any element
- Comparison detects missing/extra elements (not just headings/links)
- Comparison detects attribute differences (target, rel, class, id, etc.)
- False positive rate is acceptable (whitespace-only differences filtered out)
- Script exits nonzero when meaningful differences found
- Backward compatible (same CLI interface as current script)
- Results documented
