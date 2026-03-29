# Issue 441: Missing HTML head/body wrapper for theme sites

## Problem

Several sites output raw content without `<html>`, `<head>`, and `<body>` wrappers. Jekyll applies layouts that add the full HTML document structure, but rustkyll's Liquid rendering fails on these themes' templates and falls back to writing raw content.

## Affected Sites

- text-theme (0/6, 25 diffs) -- 404, archive, index, posts missing wrapper
- hydeout (0/13, 207 diffs) -- all pages missing wrapper
- minimal-mistakes (0/1, 2 diffs) -- missing wrapper, only 1 common file

## Root Cause (Investigation Results)

Layouts ARE being found and loaded from `_layouts/`. The problem is that Liquid template rendering fails during layout application, causing the `render_page` call to return an error. The generator's error handler (generator.rs ~line 1942) catches the error and falls back to writing `item.html_content` (raw rendered markdown without any layout wrapping).

Two categories of Liquid rendering failures observed:

1. **"liquid: failed to evaluate value"** -- Expression evaluation failures in Liquid templates. Seen in text-theme (home, articles, page layouts) and hydeout (post layout chain). Likely caused by unsupported Liquid constructs or missing context variables.

2. **"Unknown partial-template"** -- Include resolution failures. Seen in minimal-mistakes (nearly all pages) and text-theme (archive). The `{% include %}` tag cannot find the referenced partial, even though the files exist in `_includes/`.

### Per-Site Specifics

**text-theme:** Layout chain is `home -> articles -> page -> base -> none`. The `articles.html` layout uses advanced Liquid features like `{% case %}` with multiple `{% else %}` branches (which is invalid Liquid -- should be `{% when %}`). The `base.html` layout uses `{% include snippets/get-lang.html %}` (subdirectory includes). Failures occur in the layout chain rendering, not in layout lookup.

**hydeout:** Layout chain is `post -> default`. The `default.html` layout contains `{% include head.html %}`, `{% include sidebar.html %}`, etc. The `related_posts.html` include uses `site.related_posts` which may not be populated. Rendering fails at layout application time (line 17:32 errors reference positions within the template).

**minimal-mistakes:** Layout chain is `home -> default`. The `default.html` layout uses `{% include %}` for many partials. "Unknown partial-template" errors suggest the include resolver may not be finding partials that reference other nested includes or use include parameters in complex ways.

## Scope

Fix the Liquid rendering failures that prevent layout application on these three themes. The fix should address the fallback-to-raw-content problem by either:
1. Fixing the specific Liquid rendering errors in the template engine
2. Making the template engine more resilient to unsupported features (partial rendering instead of full failure)

This issue focuses on getting the HTML document structure (`<html>`, `<head>`, `<body>`) to appear in output. Content correctness within the wrapper is a separate concern (see issue 444 for Liquid rendering gaps).

## Dependencies

- None (this is a foundational fix for these themes)

## Related Issues

- Issue 238 (support text-theme) -- in-progress
- Issue 241 (support hydeout) -- done
- Issue 444 (Liquid template rendering gaps) -- overlapping root cause but different symptom
- Issue 245 (text-theme multiline include partials) -- related include resolution

## Baselines

- DTC DOM baseline: **790/790** (must not regress)
- DTC docs DOM baseline: **57/57** (must not regress)
- text-theme DOM baseline: **0/6** (must improve)
- hydeout DOM baseline: **0/13** (must improve)
- minimal-mistakes DOM baseline: **0/1** (must improve)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo fmt` reports no changes needed
- [ ] `cargo test` passes (all existing tests, no regressions)
- [ ] DTC DOM match count remains at or above **790/790**
- [ ] DTC docs DOM match count remains at or above **57/57**
- [ ] **text-theme**: At least 3/6 common HTML files contain a proper `<!DOCTYPE html>` or `<html` opening tag and `<head>` and `<body>` elements (up from 0/6)
- [ ] **hydeout**: At least 7/13 common HTML files contain a proper `<html` opening tag and `<head>` and `<body>` elements (up from 0/13)
- [ ] **minimal-mistakes**: The 1 common HTML file (index.html) contains a proper `<html` opening tag and `<head>` and `<body>` elements (up from 0/1)
- [ ] The rustkyll build for each affected site produces zero or fewer "failed to render ... writing fallback" warnings than before (at least 50% reduction per site)
- [ ] No new fallback-to-raw-content regressions on any other site (lanyon, yat, DTC, etc.)

## Test Scenarios

### Unit: Liquid expression evaluation
- Template with `{% case %}` / `{% when %}` and a `{% else %}` fallback renders without error
- Template with `site.related_posts` accessing an empty/missing array does not crash
- Template with nested `{% include %}` from subdirectories (e.g., `snippets/get-lang.html`) resolves correctly

### Unit: Include resolution
- An `{% include %}` referencing a file in a subdirectory of `_includes/` resolves correctly
- An `{% include %}` with parameters (e.g., `{% include post-meta.html post=page %}`) renders without error
- A missing include produces a warning but does not abort the entire layout chain

### Integration: Layout chain rendering
- Build the text-theme site; verify the index.html output starts with `<!DOCTYPE html>` or `<html` and contains `<head>` and `<body>` tags
- Build the hydeout site; verify at least half of post HTML files contain `<html`, `<head>`, and `<body>` tags
- Build the minimal-mistakes site; verify index.html contains `<html`, `<head>`, and `<body>` tags

### Integration: Regression safety
- Build the DTC site; verify DOM match count is at or above 790/790
- Build the DTC docs site; verify DOM match count is at or above 57/57
- Build the lanyon site; verify no pages that previously had wrappers now lack them

## Output Verification

After building each affected site, the engineer and tester must:
1. Run `head -5` on at least 3 output HTML files per site and confirm `<!DOCTYPE html>` or `<html` appears
2. Run `grep -c '<body' _site/**/*.html` to count how many files have body tags
3. Compare the count of "failed to render" warnings before and after the fix
4. Run the DOM comparison tool and record updated match counts

## Log

### [SWE] 2026-03-29

**Root causes identified:**
1. `{% case %}` with duplicate `{% else %}` blocks failed to parse (text-theme articles.html)
2. Include parameter evaluation errored on undefined variables instead of returning Nil (text-theme snippets/assign.html)
3. Include paths with leading `/` didn't resolve (minimal-mistakes /comments-providers/scripts.html)
4. Multiline include tags embedded newline in path (text-theme snippets/prepend-path.html)
5. `{{ a or b }}` output syntax not supported (hydeout disqus.html)
6. Stray `}` before `%}` closing tag caused parse error (text-theme article-info.html)

**TDD cycle:**
- Wrote 7 failing tests first (4 in engine.rs, 3 in case_block.rs)
- Verified all tests failed as expected
- Implemented fixes one by one, verified each test passes after fix

**Fixes applied:**
1. `vendor/liquid-lib/src/stdlib/blocks/case_block.rs`: Modified `else` handler to manually parse remaining tokens, gracefully handling duplicate `{% else %}` blocks
2. `src/template/include_tag.rs:305-314`: Changed `try_evaluate` to `evaluate` with Nil fallback for include parameters
3. `src/template/include_tag.rs:326`: Strip leading `/` from include name before partial lookup
4. `src/template/include_tag.rs:507`: Added `\n` and `\r` to path delimiter characters for multiline includes
5. `src/template/engine.rs`: Added `preprocess_output_or()` to rewrite `{{ a or b }}` to `{{ a | default: b }}`
6. `src/template/engine.rs`: Added `preprocess_stray_brace_in_tags()` to strip stray `}` before `%}` closing

**Results:**
- text-theme: 0 render failures (was 10), 9/11 HTML files have wrappers (was 0)
- hydeout: 4 render failures (was 28, 86% reduction), 31/35 HTML files have wrappers (was 0)
- minimal-mistakes: 0 render failures (was many), 21/21 HTML files have wrappers (was 0)
- DTC DOM: 790/790 (no regression)
- DTC docs DOM: 57/57 (no regression)
- All tests: 3079 passed, 0 failed, clippy clean, fmt clean

**Remaining hydeout failures (4):** Pages using `| find:` filter (about, markup, edge-case, tags). This is a missing filter, separate from this issue's scope.

**Files modified:**
- `vendor/liquid-lib/src/stdlib/blocks/case_block.rs`
- `src/template/include_tag.rs`
- `src/template/engine.rs`
- `docs/tracker/441-missing-html-head-body-wrapper.in-progress.md`
