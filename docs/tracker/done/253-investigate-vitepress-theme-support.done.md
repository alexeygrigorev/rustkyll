# Issue 253: Investigate theme support approach and document workflow

## Problem

rustkyll cannot install Ruby gems, so gem-based Jekyll themes do not work out of the box. Users need a documented, validated approach for using Jekyll themes with rustkyll. Without this, theme adoption is blocked and issues #235-#244 lack a common methodology.

## Goal

1. **Document the theme usage workflow** -- create `docs/theme-support.md` explaining how to use Jekyll themes with rustkyll (since we cannot install gems)
2. **Validate with jekyll-vitepress-theme** (https://github.com/crmne/jekyll-vitepress-theme) as the primary worked example
3. **Validate with at least 1 additional theme** from the backlog (#235-#244) to confirm the approach generalizes
4. **Identify common blockers** across themes and produce a prioritized list of engine fixes needed

## Theme Support Approach (to document)

Since rustkyll cannot install gems, the approach is:
- Clone the theme repo (or download the gem contents)
- Copy `_layouts/`, `_includes/`, `_sass/`, `assets/` into the site directory (or use the theme repo as the base directory)
- Remove or comment out `theme:` and `remote_theme:` from `_config.yml`
- Remove gem plugin references that rustkyll does not support
- Build with rustkyll on top of the theme files

The documentation must cover:
- Step-by-step: how to set up a site with a gem-based theme for rustkyll
- What files to copy from the theme gem/repo and where they go
- How `_config.yml` `theme:` and `remote_theme:` settings map to the file-based approach
- Common pitfalls (missing includes, plugin dependencies, theme-specific Liquid tags)
- Worked examples with real build output showing what succeeded and what failed

## Acceptance Criteria

- [ ] **AC1:** jekyll-vitepress-theme cloned into `websites/jekyll-vitepress-theme/`
- [ ] **AC2:** rustkyll build attempted against jekyll-vitepress-theme; all build errors and successes captured with exact error messages
- [ ] **AC3:** At least 1 additional theme from #235-#244 cloned into `websites/` and built with rustkyll; errors and successes captured
- [ ] **AC4:** `docs/theme-support.md` created containing all of the following sections:
  - "How Themes Work in Jekyll" (brief background on gem-based vs file-based themes)
  - "Using a Theme with rustkyll" (step-by-step instructions: clone, copy files, edit _config.yml, build)
  - "Worked Example: jekyll-vitepress-theme" (exact commands run, exact output/errors, what was fixed)
  - "Worked Example: [second theme]" (same detail level as the vitepress example)
  - "Common Theme Blockers" (table of unsupported features found across tested themes)
  - "Blocker Priority" (each blocker with effort estimate: easy/medium/hard and brief rationale)
  - "Troubleshooting" (common error messages and how to resolve them)
- [ ] **AC5:** The "Common Theme Blockers" table lists every unsupported Liquid tag, filter, or plugin encountered across all tested themes, with the theme(s) that triggered it
- [ ] **AC6:** Each blocker has an effort estimate (easy = a few hours, medium = 1-2 days, hard = multiple days or architectural change) with a one-line rationale
- [ ] **AC7:** The guide's step-by-step instructions are concrete enough that someone could follow them without reading rustkyll source code (no "figure it out" steps -- every command is explicit)

## Test Scenarios

This is primarily an investigation/documentation issue, so the "tests" are verification checks rather than `cargo test` unit tests.

### Verification: Theme cloning and build attempts
- Clone jekyll-vitepress-theme into `websites/jekyll-vitepress-theme/` and run `rustkyll build` against it; capture stdout/stderr
- Clone at least one more theme (e.g., al-folio or chirpy) into `websites/` and run `rustkyll build`; capture stdout/stderr
- For each theme, record: (a) did it build without errors? (b) what errors occurred? (c) were any errors fixable by editing _config.yml or removing plugin references?

### Verification: Documentation completeness
- `docs/theme-support.md` exists and contains all 7 required sections from AC4
- The worked examples include real build output (not placeholder text)
- The blocker table has at least 1 entry per tested theme (themes always use some unsupported feature)
- Every blocker has an effort estimate

### Verification: Documentation accuracy
- The step-by-step instructions, when followed literally against one of the cloned themes, produce the same results described in the worked examples
- Error messages quoted in the doc match what rustkyll actually produces

## Out of Scope

- Fixing any of the identified blockers (that is what issues #235-#244 and follow-ups are for)
- Automated theme compatibility tests in CI
- SASS/SCSS compilation (rustkyll does not compile SASS; themes that require it will need pre-compiled CSS)

## Dependencies

- None (investigation + documentation issue)

## Log

### [SWE] 2026-03-20
- Cloned jekyll-vitepress-theme into `websites/jekyll-vitepress-theme/`
- Built with rustkyll: 18 pages generated, 67 static files copied, 0.42s
  - Main issue: `:name` permalink placeholder not resolved (all 16 collection pages output to literal `/:name/`)
  - No Liquid errors -- all layouts, includes, and filters worked
- Cloned jekyll-theme-chirpy into `websites/jekyll-theme-chirpy/`
- Built with rustkyll: 13 pages generated, 14 static files copied, 0.56s
  - 10 of 13 pages failed to render (wrote fallback HTML)
  - Blockers: `number_of_words: 'auto'` filter arg, dynamic include paths, `{% highlight %}` tag, SCSS compilation, missing `jekyll-archives` plugin
- Created `docs/theme-support.md` with all 7 required sections:
  - "How Themes Work in Jekyll" -- background on gem vs file-based themes
  - "Using a Theme with rustkyll" -- 6-step instructions with explicit commands
  - "Worked Example: jekyll-vitepress-theme" -- real commands, full build output, analysis
  - "Worked Example: jekyll-theme-chirpy" -- real commands, full build output with all warnings, analysis
  - "Common Theme Blockers" -- 8-entry table covering both themes
  - "Blocker Priority" -- effort estimates (3 easy, 3 medium, 2 hard) with rationale
  - "Troubleshooting" -- 6 common errors with exact messages and workarounds
- All error messages in the doc are copy-pasted from actual rustkyll output
- Existing tests: 1975 passed, 0 failed (no code changes, so no new tests needed)
- Clippy: pre-existing warnings in vendored liquid-core only; rustkyll source clean
- Fmt: clean
- Files created: `docs/theme-support.md`
- Files renamed: issue 253 groomed -> in-progress
