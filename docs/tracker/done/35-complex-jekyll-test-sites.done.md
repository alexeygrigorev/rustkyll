# Issue 35: Find and Test Complex Jekyll Websites

## Problem

We currently only test against 4 relatively simple Jekyll sites (minima, beautiful-jekyll, minimal-mistakes, choosealicense.com) plus the user's own repos (covered in #32). We need complex, real-world Jekyll sites comparable to datatalks.club to stress-test rustkyll across a wider variety of Jekyll features.

## Requirements

- Research and find 5-10 complex open-source Jekyll websites (large page counts, multiple collections, data files, custom plugins, heavy template logic)
- These must NOT be from alexeygrigorev or DataTalksClub organizations (those were covered in #32)
- Good candidates: government sites (18F), project documentation sites, conference sites, organization homepages, large blogs
- Clone each (shallow) into `websites/` directory
- Attempt `rustkyll build` on each
- Document results: which build, which fail, what the blockers are
- Create follow-up issues for any new feature gaps discovered
- Sites should exercise features like: pagination, categories/tags, multiple collections, data-driven pages, custom includes, Sass/SCSS, plugins

## Candidate Sources

- GitHub Pages showcase / popular Jekyll sites lists
- Large open-source project docs built with Jekyll (e.g., Jekyll's own docs site)
- Government/nonprofit sites known to use Jekyll (e.g., 18F, NHS, UK GDS)
- Conference/meetup sites with schedules, speakers, talks collections
- Well-known blogs built with Jekyll

## Dependencies

- Issue #32 (cross-site testing) -- DONE. This issue extends that work to external sites.
- Issue #19 (CLI + full build) -- DONE. Needed to run `rustkyll build`.

## Acceptance Criteria

- [ ] At least 5 complex Jekyll sites identified from well-known open-source projects (not from alexeygrigorev or DataTalksClub)
- [ ] Each site is shallow-cloned into `websites/<site-name>/`
- [ ] `rustkyll build` is attempted on each site
- [ ] A results document exists at `docs/complex-site-results.md` with:
  - [ ] A table listing each site: name, GitHub URL, approximate page count, notable features used (collections, plugins, data files, etc.)
  - [ ] Build status for each: success, partial success (some pages rendered), or failure
  - [ ] Error details for failures, categorized by phase (config parsing, collection loading, template rendering, etc.)
  - [ ] Summary statistics (X of Y sites build, total pages rendered across all sites)
- [ ] Follow-up `.todo.md` issues created in `docs/tracker/` for each distinct failure mode not already tracked
- [ ] Sites chosen collectively exercise at least 4 of the following features: pagination, multiple collections, data-driven pages (using `_data/`), custom plugins, Sass/SCSS processing, category/tag pages, nested includes, complex Liquid logic (capture, assign, multi-level for loops)
- [ ] `cargo test` still passes (no regressions)
- [ ] `cargo clippy -- -D warnings` still passes
- [ ] The `websites/` directory is NOT committed (verify it is in `.gitignore`)
- [ ] No site-specific hardcoding is introduced into rustkyll source code

## Test Scenarios

### Manual: Site discovery and selection

- Search GitHub for popular Jekyll sites by stars, forks, or known lists
- For each candidate, verify it is actually a Jekyll site (has `_config.yml` at root)
- Verify it is complex enough to be a useful stress test (not a trivial blog with 3 posts)
- Document why each site was selected (what features it exercises)

### Manual: Build each site

- For each site, run `cargo run --release -- build --source websites/<site>/ --destination websites/<site>/_site`
- Record the output: success message with page counts, or error message with stack trace
- For sites that fail, categorize the failure:
  - Config parsing error (unknown keys, unsupported config features)
  - Collection loading error (unexpected file formats, missing layouts)
  - Template/layout error (missing layouts, unknown Liquid filters/tags, plugin-dependent features)
  - Other errors (file I/O, encoding, etc.)

### Manual: Feature coverage matrix

- Create a matrix of Jekyll features vs. sites tested
- Verify that the selected sites collectively cover pagination, collections, data files, includes, and at least 2 other advanced features
- This matrix should be included in the results document

### Regression: Existing sites still build

- After any code changes, verify the datatalksclub.github.io site and the 4 existing test sites still build
- Output counts should match previous builds

## Notes

- Use `git clone --depth 1` for all clones to minimize disk usage
- The `websites/` directory is already in `.gitignore`
- Some sites will use Jekyll plugins that rustkyll does not support -- this is expected and should be documented, not fixed in this issue
- Focus on sites that are publicly available and have permissive licenses
- This is primarily a research and documentation issue; code changes should only be made if trivial fixes are needed to unblock builds
