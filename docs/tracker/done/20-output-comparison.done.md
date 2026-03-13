# Issue 20: 100% Output Compatibility with Jekyll

## Description

The output of rustkyll must produce the **same HTML** as Jekyll for the datatalksclub.github.io site. Build both versions, diff every generated file, and fix all discrepancies until the output matches exactly.

## Dependencies

- Issue 19 (CLI and full build)

## Scope

### Reference Build

- Build the datatalksclub.github.io site with Jekyll, save output to `_site_jekyll/`
- Build with rustkyll, save output to `_site_rustkyll/`

### Comparison

- Diff **every** generated HTML file, not just a sample
- For each difference, categorize as:
  - **Content difference** (missing/wrong text, broken links, wrong URLs) -- MUST fix
  - **Structural difference** (wrong tags, missing elements, wrong nesting) -- MUST fix
  - **Metadata difference** (missing JSON-LD, wrong Open Graph tags) -- MUST fix
  - **Whitespace difference** (extra newlines, indentation) -- fix if feasible, document if not
  - **Attribute ordering** (different order of HTML attributes) -- acceptable, normalize before comparing

### Automated Regression Test

- Create a comparison script (`scripts/compare-output.sh`) that:
  1. Clones datatalksclub.github.io (or uses existing checkout)
  2. Builds with Jekyll, saves output to `_site_jekyll/`
  3. Builds with rustkyll, saves output to `_site_rustkyll/`
  4. Normalizes both outputs (sort attributes, normalize whitespace)
  5. Diffs and reports any differences

### CI/CD Integration

- Add a **separate CI job** (in the workflow from issue 05b) that runs after unit tests pass:
  1. Install Ruby + Jekyll
  2. Clone datatalksclub.github.io
  3. Build with Jekyll
  4. Build with rustkyll (`cargo run -- build`)
  5. Run comparison script
  6. Fail the job if any non-whitespace differences found
- This job should be clearly labeled (e.g., `compatibility-check`) and can have a longer timeout since it builds the full site twice

### Fix Iteration

- For each category of differences found, fix the rustkyll output
- Re-run comparison after each fix batch
- Continue until diff is clean (or only acceptable whitespace differences remain)

## Acceptance Criteria

- [ ] Every HTML page produced by rustkyll matches Jekyll's output (after normalization)
- [ ] RSS feed matches
- [ ] Sitemap matches
- [ ] Static files are identical (byte-for-byte copy)
- [ ] Automated comparison test exists and passes
- [ ] Any remaining intentional differences are documented with justification

## Notes

- The goal is drop-in replacement: switching from Jekyll to rustkyll should produce no visible change on the website
- Start with the simplest pages (people, tools) and work up to complex ones (podcast episodes)
- JSON-LD must match exactly -- search engines are sensitive to schema differences
