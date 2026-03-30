# Issue 444: Liquid template rendering gaps (includes, filters, dates)

## Problem

Some sites show raw Liquid syntax in output instead of rendered content. The original report cited:
- `{{ page.date | date: ... }}` appearing as literal text
- `{% include %}` blocks not resolving
- Future-dated posts not filtered correctly

### Affected Sites (at time of filing)

- hydeout (was 0/13, now 17/30 after issues #352-354) -- category pages had raw Liquid
- jekyll-vitepress-theme (0/17) -- unresolved includes

### Current Status

The hydeout issues have been largely addressed by completed issues #352 (Liquid `or` syntax), #353 (`find` filter), and #354 (category URL case and pagination path). Hydeout is now at 17/30 pages matching.

The jekyll-vitepress-theme issues have been investigated in issue #443 (groomed), which identified 4 distinct root causes: missing Ruby-hook-generated `<style>` tags, unresolved `auto` version string, content structural diffs, and syntax highlighting token differences.

## Root Cause

This is an umbrella issue that has been superseded by more specific follow-up issues. The remaining rendering gaps fall into these categories:

1. **Hydeout remaining diffs (13/30 pages):** These are likely related to pagination rendering, sidebar includes, or remaining Liquid edge cases not covered by #352-354. Need investigation.
2. **Vitepress rendering (0/17 pages):** Fully tracked in #443.
3. **Generic Liquid rendering gaps:** Any remaining raw-Liquid-in-output patterns across all test sites.

## Dependencies

- Issue #443 (jekyll-vitepress-theme rendering) -- covers vitepress specifically
- Issues #352, #353, #354 (hydeout fixes) -- already done

## Key Files

- `src/template/engine.rs` -- Liquid template rendering engine
- `src/template/layout.rs` -- layout/include resolution
- `src/template/filters/mod.rs` -- filter registry
- `websites/hydeout/` -- hydeout test site
- `websites/jekyll-vitepress-theme/` -- vitepress test site

## Scope

1. Investigate remaining hydeout diffs (13/30 unmatched pages) to identify which are caused by Liquid rendering gaps vs. other root causes
2. Check all other test sites for any remaining raw-Liquid-in-output patterns
3. Fix any generic Liquid rendering bugs found (not theme-specific workarounds)
4. For theme-specific issues (Ruby hooks, gem-version injection), document as known limitations or create targeted follow-ups

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] No test site outputs raw Liquid syntax (`{{`, `{%`) in rendered HTML where Jekyll would have rendered it (spot-check at least: hydeout, chirpy, minimal-mistakes, al-folio, DTC)
- [ ] Hydeout match count does not regress below 17/30
- [ ] Any newly discovered Liquid rendering gaps are either fixed or tracked in follow-up issues
- [ ] DTC DOM baseline stays at 788/790 or above
- [ ] No regression on any other test site DOM counts
- [ ] `cargo test` passes with all new and existing tests
- [ ] `cargo fmt --check` and `cargo clippy` pass cleanly
- [ ] Tests include non-ASCII content scenarios

## Test Scenarios

### Investigation: hydeout remaining diffs
- Build hydeout with rustkyll and run DOM comparison
- Identify the 13 unmatched pages and categorize their diff types
- Check if any contain raw Liquid syntax in output
- For each raw-Liquid occurrence, trace back to the template/include that failed to render

### Investigation: cross-site raw Liquid check
- For each test site (DTC, chirpy, minimal-mistakes, al-folio, hydeout, yat), grep the rustkyll HTML output for `{{` and `{%` patterns
- Filter out legitimate uses (e.g. JavaScript template literals, code blocks)
- Document any remaining raw Liquid occurrences

### Unit: Liquid include resolution
- Template with `{% include header.html %}` where `_includes/header.html` exists -- verify rendered content appears
- Template with `{% include missing.html %}` -- verify graceful error handling (not raw syntax in output)
- Template with nested includes (include A includes B) -- verify full resolution

### Unit: date filter rendering
- `{{ page.date | date: "%Y-%m-%d" }}` with a valid date -- verify formatted output
- `{{ page.date | date: "%B %d, %Y" }}` -- verify full month name rendering
- Date filter on nil/missing date -- verify no raw Liquid output

### Integration: site builds
- Build hydeout and verify match count >= 17/30
- Build DTC and verify DOM count >= 788/790
- Build at least one other test site and verify no regression

### Regression: DTC DOM
- Build DTC site and run DOM comparison
- Verify match count is at least 788/790
