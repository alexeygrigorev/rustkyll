# Issue 57: Further performance optimization with quality verification

## Priority

HIGH -- DTC site currently builds in 6s, target is under 2s (10x faster than Jekyll's 19.4s).

## Problem

Issue #49 eliminated the O(n^2) deep-cloning bottleneck, bringing DTC from 300s to 6s. But 6s is only 3.2x faster than Jekyll, not the 10x target. The remaining bottleneck is the liquid crate's template interpreter.

## Goal

Get DTC site build time under 2s while preserving output correctness. This likely requires writing a custom Liquid template renderer (Option B from issue #49).

## Approaches

### Option A: Optimize within the liquid crate

- Profile the liquid crate's hot paths (template parsing, value cloning on stack access)
- Patch or fork the liquid crate to reduce allocations
- Pre-compile templates to avoid re-parsing

### Option B: Write a custom Liquid renderer

Replace the liquid crate with a purpose-built Liquid-compatible renderer:
- Parse templates once, compile to an efficient AST or bytecode
- Zero-copy variable resolution using references into the site context
- Avoid cloning values on stack push/pop
- Only implement the Liquid subset that Jekyll actually uses

### Option C: Parallel template rendering

- Use rayon to render pages in parallel across CPU cores
- Requires the renderer to be thread-safe (no shared mutable state)

## Output quality verification (MANDATORY)

### Structural comparison

Build a comparison script that:
- Builds the site with both Jekyll and rustkyll
- Compares file trees (same HTML files generated)
- For each HTML file, extracts structural elements (title, h1-h6, links, images) and diffs them
- Exits nonzero on any structural difference

### Playwright visual comparison

Sites MUST be served over HTTP so CSS, images, fonts, and JS all load:
1. Serve Jekyll _site/ on one port, rustkyll _site/ on another
2. Verify no 404 errors in browser console
3. Use Playwright to visit key pages (homepage, a post, a collection page, an archive)
4. Take full-page screenshots from both servers
5. Compare screenshots with a pixel diff threshold (<5%)
6. Flag any pages where visual diff exceeds threshold

Both structural and visual comparison must pass for DTC site and kids-horror-stories-ru.

## Dependencies

- Issue 49 (done)

## Acceptance criteria

- DTC site builds in under 2 seconds (10x faster than Jekyll)
- kids-horror-stories-ru builds in under 1 second
- Structural comparison script passes for both sites
- Playwright visual comparison passes for both sites (served over HTTP with full assets)
- All existing tests still pass
- No correctness regressions
- Benchmark script confirms the speedup numbers
