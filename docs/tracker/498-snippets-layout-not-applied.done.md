# Issue 498: snippets site — layout not applied to 17 pages

## Problem

snippets (8/25) has 17 pages missing `<head>/<body>` wrapper despite
having local _layouts/ with default.html, snippet.html, category.html.
No render errors logged. Liquid rendering fails silently.

## Scope

Investigate why layouts are not applied. Check if Liquid template
parsing fails for these specific pages (like #441 found for other sites).

## Baseline

DTC 790/790. snippets 8/25. Must not regress.

## Log

### [SWE] 2026-03-29

**Root cause:** The snippets `_config.yml` uses glob patterns in defaults path scope:
```yaml
defaults:
  - scope:
      path: "snippets/*/*.md"
      type: "pages"
    values:
      layout: "snippet"
  - scope:
      path: "snippets/*/README.md"
      type: "pages"
    values:
      layout: "category"
```

The `*` in `snippets/*/*.md` is a glob wildcard, but `defaults_for_page()` and
`defaults_for()` in `src/config.rs` used `starts_with()` for path matching --
treating the scope path as a literal prefix. Since no item path starts with the
literal string `snippets/*/*.md`, no defaults matched, and all 17 snippet pages
rendered without a layout.

**Fix:** Added `scope_path_matches()` function that detects glob metacharacters
(`*`, `?`, `[`) in scope paths and uses fnmatch-style matching (with
FNM_PATHNAME semantics where `*` does not cross `/`). Non-glob paths still use
the existing prefix matching. Both `defaults_for()` and `defaults_for_page()`
now call this function instead of `starts_with()`.

**TDD:**
- Wrote 5 tests: glob single star, README override, depth mismatch, collection defaults, star-slash boundary
- Ran tests: 4 FAIL, 1 pass (as expected -- depth mismatch correctly found no match)
- Implemented fnmatch_pathname() and scope_path_matches()
- Ran tests: all 5 PASS

**Verification:**
- Full test suite: 3,430+ tests pass, 0 failures
- Clippy clean, fmt clean
- DTC DOM: 790/790 (no regression)
- Snippets DOM: 8/25 (baseline maintained -- layouts now applied but other DOM differences remain)
- Confirmed snippet pages (toyaikit-loop, llm-api-cost-tracker, async_map_tqdm, etc.) now output `<!DOCTYPE html>` wrapper

**Files modified:** src/config.rs (added fnmatch_pathname, scope_path_matches; updated defaults_for and defaults_for_page; added 5 tests)
