# Issue 61: Structural comparison testing for DTC site

## Problem

Issues #49 and #57 required structural comparison of rustkyll vs Jekyll output but this was descoped. We need to verify that rustkyll produces structurally equivalent HTML to Jekyll for the DTC site and kids-horror-stories-ru.

## Goal

Run the existing structural comparison script (`scripts/compare-output.sh`) end-to-end on both sites, fix any issues found in the script or in rustkyll output, and produce a documented comparison report.

## Scope

This issue covers:
- Building both sites with Jekyll and rustkyll
- Running `scripts/compare-output.sh` end-to-end for both sites
- Fixing bugs in the comparison script if it fails to run
- Fixing rustkyll output issues if structural differences exceed thresholds
- Documenting the comparison results
- Validating no raw Liquid tags and no empty HTML files in rustkyll output

This issue does NOT cover (handled by other issues):
- RSS/Atom feed validation and sitemap comparison (issue #63)
- Playwright visual screenshot comparison (issue #62)

## Sites to compare

- `websites/DataTalksClub/datatalksclub.github.io`
- `websites/alexeygrigorev/kids-horror-stories-ru`

## Prerequisites

- Jekyll is installed at `/home/alexey/.rvm/gems/ruby-3.3.7/bin/jekyll`
- rustkyll must be built in release mode (`./scripts/cargo-safe build --release`)
- Both site source directories exist under `websites/`

## Dependencies

None.

## Acceptance Criteria

### Script execution (MUST pass)

- [ ] `scripts/compare-output.sh --site DataTalksClub/datatalksclub.github.io` runs to completion without crashing (exit code 0 or 1, not a bash error)
- [ ] `scripts/compare-output.sh --site alexeygrigorev/kids-horror-stories-ru` runs to completion without crashing
- [ ] The script successfully builds the site with both Jekyll and rustkyll (not just rustkyll-only mode with the "Jekyll output not found" skip path)
- [ ] The script produces clear, readable output showing file counts, missing files, and structural diffs

### File tree comparison (MUST pass)

- [ ] DTC site: rustkyll generates HTML files within 5% of Jekyll's file count (the script's built-in threshold)
- [ ] kids-horror-stories-ru: rustkyll generates HTML files within 5% of Jekyll's file count
- [ ] DTC site: missing files (present in Jekyll but absent in rustkyll) are within 5% threshold
- [ ] kids-horror-stories-ru: missing files are within 5% threshold

### Structural element comparison (MUST pass)

- [ ] For DTC site: the script compares at least 10 common HTML files (the script samples up to 50)
- [ ] For DTC site: fewer than half of sampled files have structural differences (titles, headings, links, images)
- [ ] For kids-horror-stories-ru: the script compares at least 10 common HTML files
- [ ] For kids-horror-stories-ru: fewer than half of sampled files have structural differences

### Output quality checks (MUST pass)

- [ ] DTC site rustkyll output: no HTML file contains raw Liquid tags (`{{`, `{%`)
- [ ] DTC site rustkyll output: no empty HTML files (every `.html` file is at least 100 bytes)
- [ ] kids-horror-stories-ru rustkyll output: no raw Liquid tags
- [ ] kids-horror-stories-ru rustkyll output: no empty HTML files

### Script exits correctly (MUST pass)

- [ ] `scripts/compare-output.sh --site DataTalksClub/datatalksclub.github.io` exits with code 0 (all checks pass)
- [ ] `scripts/compare-output.sh --site alexeygrigorev/kids-horror-stories-ru` exits with code 0

### Documentation (MUST pass)

- [ ] Results are saved to a file (e.g., `docs/comparison/structural-results.md`) containing:
  - Date of comparison
  - File counts for both Jekyll and rustkyll for each site
  - Number of missing files in each direction
  - Number of structural differences found
  - List of any files with structural differences (at least the first 10)
  - Pass/fail status for each site

### Script improvements (if needed)

- [ ] If the comparison script has bugs preventing end-to-end execution (e.g., the `validate_output` function is called before it is defined -- it is currently defined at line 115 but called at line 84), fix them
- [ ] If the script's `--site` mode does not actually invoke Jekyll to build the site (it currently only checks if pre-built output exists), either add Jekyll build support to the script or document the manual steps and use `--jekyll-dir` / `--rustkyll-dir` mode instead

## Test Scenarios

### Manual: End-to-end script execution on DTC site
1. Build rustkyll in release mode: `./scripts/cargo-safe build --release`
2. Build DTC site with Jekyll: `cd websites/DataTalksClub/datatalksclub.github.io && bundle exec jekyll build --destination /tmp/compare-jekyll-DataTalksClub-datatalksclub.github.io`
3. Run: `./scripts/compare-output.sh --site DataTalksClub/datatalksclub.github.io`
4. Verify script completes, reports file counts, and exits 0
5. Inspect output for any structural differences

### Manual: End-to-end script execution on kids-horror-stories-ru
1. Build kids-horror-stories-ru with Jekyll: `cd websites/alexeygrigorev/kids-horror-stories-ru && bundle exec jekyll build --destination /tmp/compare-jekyll-alexeygrigorev-kids-horror-stories-ru`
2. Run: `./scripts/compare-output.sh --site alexeygrigorev/kids-horror-stories-ru`
3. Verify script completes, reports file counts, and exits 0

### Manual: Raw Liquid tag check
1. After rustkyll builds DTC site, run: `grep -rlP '\{%|\{\{' /tmp/compare-rustkyll-DataTalksClub-datatalksclub.github.io/*.html` (recursively)
2. Verify no files are found
3. Repeat for kids-horror-stories-ru

### Manual: Empty file check
1. After rustkyll builds DTC site, run: `find /tmp/compare-rustkyll-DataTalksClub-datatalksclub.github.io -name "*.html" -size -100c`
2. Verify no files are found
3. Repeat for kids-horror-stories-ru

### Manual: Structural diff spot-check
1. Pick 3 files from the comparison output that are "common" (exist in both)
2. Open the Jekyll and rustkyll versions side by side
3. Verify that titles, headings, and major links are the same
4. If differences exist, verify they are cosmetic (whitespace, attribute ordering) not content differences

### Script bug verification
1. Read `scripts/compare-output.sh` and verify `validate_output` is defined before it is first called
2. Verify the `--site` mode either builds with Jekyll or clearly directs the user to do so
3. Fix any issues found

## Notes

- The comparison script currently has a bug: `validate_output` is called at line 84 but defined at line 115. This must be fixed for the script to work in rustkyll-only mode.
- The script's `--site` mode does not build with Jekyll -- it only builds with rustkyll and checks if pre-built Jekyll output exists. The engineer must either add Jekyll build support or use the `--jekyll-dir` / `--rustkyll-dir` flags with manually pre-built outputs.
- These are manual integration tests, not `cargo test` tests. The comparison script is a bash script, not a Rust test.
- Mark any Rust-level tests as `#[ignore]` if they require large site builds, per project convention.
- RSS/Atom and sitemap validation are out of scope for this issue -- they are tracked in issue #63.
