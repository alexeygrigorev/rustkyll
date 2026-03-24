# Issue 334: muan-blog -- remaining 28/2218 pages with diffs

## Problem

After issue 330 fixed 18 pages (2172 -> 2190/2218), 28 pages still have diffs. These are from categories NOT addressed in issue 330 (Categories A/B were inapplicable; Categories C/D/E/F were fixed where applicable).

The remaining 28 pages have diffs from diverse, pre-existing categories:

### Breakdown

1. **pages/blogroll** (85 diffs) -- `sample:` filter produces random order; unfixable without seeded randomness
2. **pages/hacking-with-swift** (30 diffs) -- syntax highlighting class differences
3. **posts/border-box-in-github** (34 diffs) -- complex HTML rendering differences in inline HTML
4. **notes/2023-01-25-mm** (14 diffs) -- meta description truncation with unmatched quotes
5. **pages/issues** (12 diffs) -- code classes, autolink in HTML context, curly quotes
6. **photos.html** (11 diffs) -- smart punctuation, hardbreaks rendering
7. **Various posts** -- iframe rendering, timezone handling, permalink differences, URL encoding issues

## Scope

Investigate and fix as many of the 28 remaining pages as feasible. Priority:
1. Pages with small diff counts (likely single-fix pages)
2. Systematic issues that affect patterns reusable across sites (e.g., smart punctuation, hardbreaks)

Skip `pages/blogroll` (random order is inherently non-deterministic).

## Dependencies

- Issue 330 must be `.done.md` first

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `./scripts/cargo-safe test` passes
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] muan-blog DOM match improves beyond 2190/2218
- [ ] Each remaining diff category is investigated and either fixed or documented as known limitation
- [ ] No regressions on DTC (751+/790) or any other site
- [ ] Tests include non-ASCII/Unicode content where applicable

## Test Scenarios

### Integration: muan-blog full site DOM comparison
- Build muan-blog with rustkyll
- Run DOM comparison against Jekyll cached output
- Verify improvement beyond 2190/2218

## Notes

- Spun off from issue 330 which achieved 2190/2218 (target was 2200+)
- The 2200+ target was based on Categories A/B being fixable (~40 pages), but those categories turned out inapplicable (site uses own template, not seo tag)
