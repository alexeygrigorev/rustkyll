# Issue 252: Document all pulldown-cmark to kramdown postprocessing steps

## Problem

We apply numerous postprocessing transformations to pulldown-cmark's markdown output to match Jekyll's kramdown behavior. These are scattered across `src/kramdown.rs` and `src/frontmatter.rs` with no central documentation of what each does, why it exists, and which sites it affects.

## Goal

Create a comprehensive reference document listing every preprocessing and postprocessing step applied to bridge the gap between pulldown-cmark and kramdown. For each transformation, document:

1. **What it does** (input -> output)
2. **Why** (what kramdown behavior it matches)
3. **Where** (function name, file, line)
4. **When it runs** (pre-markdown, post-markdown, post-layout)
5. **Which sites it affects** (if known)
6. **Risk level** (does it ever over-apply or cause regressions?)

## Acceptance Criteria

- [ ] Document created at `docs/pulldown-kramdown-postprocessing.md`
- [ ] All preprocessing steps in `src/frontmatter.rs` listed (e.g., consecutive quote protection, math content protection, markdown="1" processing)
- [ ] All postprocessing steps in `src/kramdown.rs` listed (e.g., smart quote direction fix, void element normalization, code class wrapping, IAL parsing, pipe table handling, block spacing)
- [ ] All normalization steps in layout/output pipeline listed (e.g., normalize_br_only, normalize_html_output)
- [ ] Each entry includes the 6 fields above
- [ ] Document is accurate against current code

## Dependencies

- None (documentation only)
