# Issue 99: Publish scripts for generating synthetic benchmark sites

## Problem

The benchmark includes two synthetic sites (large-blog-3000 and large-docs-site) that were generated locally but the generation scripts are not published. Users can't reproduce the benchmark without these scripts.

The sites themselves don't need to be committed (they're too large), but the scripts to generate them should be in the repo.

## Goal

Publish the scripts used to generate the synthetic benchmark sites so anyone can reproduce them.

## Deliverables

1. `scripts/generate-large-blog.sh` (or .py) — generates a Jekyll site with 3000 blog posts, categories, tags, and an index page
2. `scripts/generate-large-docs.sh` (or .py) — generates a Jekyll site with 800 documentation pages across 10 sections

Each script should:
- Create the site in `websites/<name>/`
- Initialize a git repo (for benchmark script compatibility)
- Include a `_config.yml` with standard Jekyll settings
- Include at least one layout
- Be idempotent (can be run again without issues)

## Dependencies

None

## Acceptance criteria

- Both generation scripts exist and are executable
- Running each script produces a valid Jekyll site in websites/
- The generated sites build with both Jekyll and rustkyll
- Page counts match the benchmark (3001 for large-blog, 801 for large-docs)
- Scripts are documented (usage instructions in comments or README)
- Scripts are committed to the repo (not the generated sites)

## Log

### [SWE] 2026-03-15

- Found that `scripts/generate-synthetic-sites.sh` already existed (from issue #110) and generates both sites
- The script was missing: git repo initialization, had description typos (500/300 instead of 3000/800)
- Changes made to `scripts/generate-synthetic-sites.sh`:
  - Added `--only blog|docs` flag to allow generating a single site
  - Added git repo initialization (git init + commit) after each site generation
  - Fixed description typos in _config.yml (500->3000, 300->800)
- Created `scripts/generate-large-blog.sh` -- thin wrapper calling generate-synthetic-sites.sh --only blog
- Created `scripts/generate-large-docs.sh` -- thin wrapper calling generate-synthetic-sites.sh --only docs
- Both wrapper scripts are executable and documented with usage comments
- Verified idempotent behavior: running on existing sites skips generation
- Verified from-scratch generation: both scripts produce correct page counts in temp directories
- Verified git repo initialization works correctly
- Ran rustkyll page count tests (ignored tests run explicitly):
  - test_large_blog_3000_page_count: PASS (3001 HTML files)
  - test_large_docs_site_page_count: PASS (801 HTML files)
- All tests pass: 1321 passed, 0 failed (across all test suites)
- Clippy clean
- Pre-existing fmt issues in main.rs and livereload.rs (not related to this issue)

Files created:
- scripts/generate-large-blog.sh (new)
- scripts/generate-large-docs.sh (new)

Files modified:
- scripts/generate-synthetic-sites.sh (added --only flag, git init, fixed descriptions)
- docs/tracker/99-publish-synthetic-site-generators.in-progress.md (this file)
