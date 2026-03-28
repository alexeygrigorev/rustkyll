# Issue 427: Push DTC build time under 500ms

## Problem

DTC full-site build currently takes ~1.1s (release mode). The target is under 500ms.

## Baseline (measured 2026-03-28)

Total: 1.11s. Phase breakdown:

| Phase | Time | % of total |
|-------|------|------------|
| Generation | 0.710s | 64% |
| Collections | 0.159s | 14% |
| Static files | 0.022s | 2% |
| Context | 0.026s | 2% |
| Pages | 0.015s | 1% |
| Data | 0.005s | <1% |
| Other | ~0.17s | 15% |

The **Generation phase** (template rendering of 789 pages) is the dominant bottleneck at 64% of build time. The Collections phase (loading 777 items from disk, parsing front matter, converting markdown) is the second bottleneck at 14%.

## Architecture Analysis

### Generation phase (0.710s) -- what it does

For each of the 789 pages, the generation phase:

1. Clones front matter and applies config defaults
2. Builds a `page` Liquid Object from front matter (per page)
3. Checks if content has Liquid tags (`{{` or `{%`)
4. If yes: parses the content through the Liquid engine (multiple preprocessing passes: include paths, capture tags, Jekyll tags, nil-contains, nil-eq-false, nested braces, for-loop filters, parenthesized assigns, render_mapping rewrite)
5. For markdown files: runs dedent, heading marking, blank-line collapse, then markdown-to-HTML conversion (which includes syntax highlighting for code blocks)
6. Renders through the layout chain (content -> layout -> parent layout), each requiring a Liquid parse+render pass
7. Runs `normalize_html_output_owned` on the final HTML
8. Writes to disk

Key observations:
- **Liquid template parsing is per-page**: content templates are parsed fresh for each page (layouts are pre-compiled, but page content is not). With 777 collection items, this means 777 separate parse passes.
- **Preprocessing pipeline in `parse()`**: 8 preprocessing functions run on every template string before parsing (include paths, capture tags, Jekyll tags, nil-contains, nil-eq-false, nested braces, for-loop filters, parenthesized assigns, render_mapping rewrite). Each does string scanning/allocation.
- **Syntax highlighting postprocessing**: The `highlight_code()` function in `syntax.rs` runs up to 16 regex-based postprocessing passes per code block for bash, ~8 for Python, ~4 for SQL, etc. DTC has ~90 code blocks across 55 posts (36 Python, 22 bash, 16 YAML, 7 SQL). Each postprocessing pass allocates a new String.
- **Per-page front matter cloning**: `item.front_matter.clone()` runs 777 times in the parallel loop.
- **`page_obj` construction**: `build_page_object()` converts YAML front matter to Liquid Values per-page.

### Collections phase (0.159s) -- what it does

- Reads 777 files from disk (parallel via rayon)
- Parses front matter (YAML) for each
- Converts markdown to HTML for each (including syntax highlighting)
- Builds `CollectionItem` structs

Key observation: markdown-to-HTML conversion (including syntax highlighting) happens TWICE -- once during collection loading (to populate `html_content`) and once during generation (when markdown pages go through the Liquid->markdown->HTML pipeline). The collection-loading conversion result is only used for `page.content` in non-markdown rendering paths.

## Scope

This is an investigation and optimization issue. The SWE must:

1. Add fine-grained timing instrumentation to identify the exact bottleneck(s) within the Generation phase
2. Implement optimizations to bring the total build time under 500ms
3. Ensure no regressions in output correctness

## Profiling Approach

The SWE should add optional timing instrumentation (behind a `--profile` flag or `RUSTKYLL_PROFILE=1` env var) that reports:

- Per-collection generation time (posts, people, podcast, etc.)
- Within generation: time spent on Liquid parsing vs rendering vs markdown conversion vs disk I/O
- Number and total time of syntax highlighting calls

This instrumentation should be temporary (can be removed after optimization) or gated behind a flag.

## Candidate Optimizations (investigate in priority order)

### P0: Skip redundant markdown-to-HTML in collection loading

Collection items that will be rendered through the markdown pipeline (render_markdown_page_with_cached_site) don't need `html_content` pre-computed during loading. The generation phase re-runs markdown-to-HTML after Liquid processing anyway. Skipping the initial conversion for markdown collection items could save significant time in the Collections phase.

However, verify first: some templates access `page.content` which uses the pre-computed `html_content`. Check if DTC templates use `{{ page.content }}` or `{{ content }}` (the latter is layout content injection and comes from the generation pipeline, not the pre-computed field).

### P1: Cache or skip Liquid parsing for plain-content pages

Most of the 427 people and 196 podcast items likely have NO Liquid tags in their content. The code already checks for `{{` and `{%` markers, but the `parse()` function still runs 8 preprocessing passes on every template string (including layouts). Investigate whether the preprocessing in `parse()` can be short-circuited when the string has no Liquid tags.

### P2: Reduce syntax highlighting string allocations

Each postprocessing pass in `highlight_code()` allocates a new String. For bash code blocks, that's 16 sequential allocations. Options:
- Combine multiple simple string replacements into a single pass
- Use in-place mutation where possible (e.g., `Cow<str>`)
- Profile to confirm this is actually significant before optimizing

### P3: Reduce per-page front matter cloning

`item.front_matter.clone()` copies the full YAML mapping 777 times. Consider:
- Building the page Object directly from the CollectionItem without cloning
- Using `Cow` or `Arc` for front matter sharing

### P4: Template precompilation for collection item content

If many collection items have identical or near-identical Liquid content patterns, a template cache keyed by content hash could avoid re-parsing. However, this may not help if each item has unique content.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo fmt` and `cargo clippy -- -D warnings` pass cleanly
- [ ] DTC full-site build time (release mode) is under 500ms (measured as the "Time:" line in build output)
- [ ] DTC DOM baseline does not regress (must remain at current level or improve)
- [ ] All existing tests pass (`./scripts/cargo-safe test`)
- [ ] At least 3 new unit or integration tests covering the optimization changes
- [ ] Generated HTML output is byte-identical to pre-optimization output (or any differences are documented and justified)
- [ ] The profiling instrumentation used to find bottlenecks is documented in the issue log

## Test Scenarios

### Unit: Optimization correctness

- Verify that skipping markdown pre-conversion (if implemented) still produces correct `page.content` values for templates that access it
- Verify that any Liquid parsing short-circuit still handles edge cases (content with `{` but not `{{`)
- Verify that combined syntax highlighting postprocessing produces identical output to the sequential version

### Integration: Build output correctness

- Build DTC site before and after optimization, diff the output directories -- they must be identical (or differences must be explicitly documented)
- Build DTC site and verify timing is under 500ms (release mode)

### Regression: DOM baseline

- Run DOM comparison against Jekyll output, verify match count does not drop below baseline

## Dependencies

None. This is a standalone performance optimization issue.

## Notes

- The DTC site has 777 collection items: 427 people, 196 podcast, 99 books, 55 posts
- Only 55 posts have markdown content with code blocks; people/podcast/books are mostly plain HTML or simple markdown
- The `where_exp` filter was previously optimized (pre-parsed expressions, pre-resolved runtime tokens) and is not expected to be a bottleneck
- Rayon parallel iteration is already used for both collection loading and page generation
- Layout templates are pre-compiled; only page content is parsed per-render
