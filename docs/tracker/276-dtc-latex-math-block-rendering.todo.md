# Issue 276: DTC LaTeX math block rendering

## Problem

On 2 DTC blog posts, `$$...$$` block math delimiters are being wrapped in `<p>` tags instead of passed through as raw text nodes for MathJax. This causes cascading DOM shifts (198 diffs on ner-reformers.html, 47 on regularization-in-regression.html = 245 total diffs).

Jekyll renders `$$...$$` as text nodes (not wrapped in `<p>`) so MathJax can pick them up client-side.

## Acceptance Criteria

- [ ] `$$...$$` blocks pass through without `<p>` wrapping
- [ ] DOM comparison improves dramatically for the 2 affected pages
- [ ] Tests verify math block pass-through
