# Issue 546: Liquid bytecode compilation for sub-500ms builds

## Problem

After issues #427/#467/#468, the remaining performance bottleneck across all sites is the liquid crate's AST-walking template render loop at 2.4ms per page for layout templates. This is an inherent cost of the current execution model that cannot be reduced by fast-path optimizations.

Current build times vs targets:
- DTC: 0.57s (target 0.50s) -- 70ms gap
- jekyll-docs: 0.82s (target 0.31s) -- 510ms gap (markdown conversion of history.md dominates)
- large-blog-3000: 0.97s (target 0.44s) -- 530ms gap (pure volume * per-page render cost)

## Root Causes

1. **Liquid AST walking**: The liquid crate renders templates by walking a Vec<Box<dyn Renderable>> and calling render_to() on each element. This has virtual dispatch overhead per AST node.
2. **Markdown preprocessing string copies**: jekyll-docs history.md (4659 lines) goes through 30+ preprocessing passes, each copying the full string. A Cow<str> pipeline would eliminate copies when no transformation is needed.
3. **Per-page render cost**: At 2.4ms per layout render, 3001 pages = 7.1s thread-total. Even with 8-thread parallelism, wall time cannot drop below ~0.9s.

## Candidate Approaches

### A: Liquid bytecode/compiled execution
Replace AST walking with a bytecode interpreter or compiled execution model. This would reduce per-element dispatch overhead.

### B: Cow<str> markdown preprocessing pipeline
Change markdown_to_html_with_options to use Cow<str> throughout, avoiding string copies for passes that do not modify the content. Would primarily help jekyll-docs.

### C: Template specialization
For simple layouts (no conditionals, just variable interpolation), generate specialized render functions that skip the generic AST walk.

## Acceptance Criteria

- [ ] DTC full-site build under 500ms (release, median of 3)
- [ ] jekyll-docs build time measurably improved from 0.82s
- [ ] large-blog-3000 build time measurably improved from 0.97s
- [ ] DTC DOM >= 596/790 (no regression)
- [ ] All existing tests pass

## Dependencies

- Issues #427, #467, #468 (done -- established baselines and identified bottleneck)

## Notes

- This is a significant architectural change to the liquid crate or markdown pipeline
- The liquid crate is vendored, so we have full control over its internals
- Profile-guided optimization (PGO) could also help but is a build-system change, not a code change
