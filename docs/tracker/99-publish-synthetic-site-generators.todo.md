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
