# Issue 51: Fix benchmark page count accuracy

## Problem

Several sites in the benchmark report suspicious page counts:

- bitcoin-org: "?" (unknown -- both tools FAIL, so no `_site/` is produced)
- edition-template: "?" (unknown -- both tools FAIL, so no `_site/` is produced)
- data-science-interviews: 0 pages (builds successfully but produces no HTML)
- academicpages: 1 page (seems too low for a full academic portfolio theme)
- minimal-mistakes: 1 page (seems too low for a feature-rich theme)
- beautiful-jekyll: 3 pages (might be too low)

The page count should reflect the actual number of HTML files generated, and sites that produce 0 or suspiciously low pages should be investigated to determine whether the count is correct or whether rustkyll is failing to render pages.

## Goal

1. Investigate each suspicious page count and determine whether it is accurate or indicates a rustkyll bug.
2. Fix any rustkyll bugs that cause pages to not be rendered.
3. Where page counts are genuinely low (e.g., a theme demo with only 1 sample page), document the reason.
4. Resolve all "?" entries in the benchmark -- either fix the build or mark as "both tools fail" with a reason.
5. Update the benchmark results to reflect accurate, verified page counts.

## Scope

This issue covers **investigation and documentation** of page counts, plus **fixing any rustkyll bugs** that cause pages to be missing from output. It does NOT require making bitcoin-org or edition-template build successfully (those are separate compatibility issues).

## Dependencies

- Issue 49 (large-site performance) -- done

## Acceptance Criteria

- [ ] For each of the 6 suspicious sites listed above, there is a written explanation of why the page count is what it is (either "correct because X" or "bug: Y, fixed by Z")
- [ ] "?" entries (bitcoin-org, edition-template) are replaced with either a page count or "N/A (both fail)" in the benchmark results
- [ ] The `count_pages` function in `scripts/benchmark.sh` handles the case where both tools fail -- it should report "N/A" or "0" instead of "?"
- [ ] For data-science-interviews (0 pages): investigation determines whether pages are missing because no layouts are specified (expected) or because of a rustkyll bug; the reason is documented
- [ ] For academicpages (1 page): compare rustkyll output page count against Jekyll output page count; if they differ, file/fix the discrepancy
- [ ] For minimal-mistakes (1 page): compare rustkyll output page count against Jekyll output page count; if they differ, file/fix the discrepancy
- [ ] For beautiful-jekyll (3 pages): compare rustkyll output page count against Jekyll output page count; if they differ, file/fix the discrepancy
- [ ] The benchmark script is re-run and updated results are committed to `docs/benchmark/results.md`
- [ ] `./scripts/cargo-safe build` compiles without errors
- [ ] `./scripts/cargo-safe test` passes

## Test Scenarios

### Manual: Page count verification

- For each suspicious site, build with rustkyll and count HTML files in `_site/` manually (using `find _site -name '*.html' | wc -l`)
- For sites where Jekyll also builds (academicpages, minimal-mistakes, beautiful-jekyll), build with Jekyll and compare the HTML file count
- Verify the benchmark script's `count_pages` function returns the same number as the manual count

### Manual: "?" resolution

- Attempt to build bitcoin-org with rustkyll; confirm it fails and document the error
- Attempt to build edition-template with rustkyll; confirm it fails and document the error
- Verify the benchmark script now outputs "N/A" or "0" instead of "?" for these sites

### Manual: Benchmark script logic

- Run `scripts/benchmark.sh --site academicpages --runs 1` and verify the page count in the output matches the actual `find` count
- Run the full benchmark and verify no "?" entries remain in the output

### Unit: Any rustkyll fixes (if bugs are found)

- If a bug is found causing missing pages (e.g., pages without explicit layout not being rendered), add a Rust test that builds a minimal site exercising that scenario and verifies the correct number of HTML files are generated
- If no rustkyll bugs are found (all low counts are accurate), no new Rust tests are needed -- document this finding

## Investigation Guide

For each suspicious site, the engineer should:

1. Build with rustkyll: `cd websites/SITE && /path/to/rustkyll build`
2. Count output: `find _site -name '*.html' | wc -l`
3. List output files: `find _site -name '*.html'` (to see what was actually generated)
4. If Jekyll also builds the site, repeat steps 1-3 with Jekyll and compare
5. If counts differ, inspect what Jekyll generates that rustkyll does not
6. Document findings in a summary section added to the benchmark results

## Notes

- The current `count_pages` function (line 116-123 of `scripts/benchmark.sh`) looks correct -- it counts `*.html` files in `_site/`. The "?" values come from the logic on line 230 where `pages` is set to "?" when rustkyll fails, and line 241-245 tries to fall back to Jekyll's count but only if Jekyll succeeds.
- The root cause of "?" is that both tools fail for those sites, so neither produces a `_site/` directory to count.
- data-science-interviews was previously documented as "Pages not rendered (no layout specified)" in `docs/cross-site-results.md`, so 0 may be correct.
