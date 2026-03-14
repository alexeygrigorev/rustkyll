# Issue 56: Add more large Jekyll websites for benchmarking

## Problem

The benchmark currently only has meaningful speed comparisons for 2 large sites (DTC 787 pages, kids-horror-stories-ru 1345 pages). Most other sites have fewer than 20 pages, which makes the benchmark unrepresentative. We need more large Jekyll sites (100+ pages) to validate rustkyll's performance characteristics and identify remaining bottlenecks.

## Goal

Find and add 5-10 large open-source Jekyll sites to the benchmark. Focus on sites where Jekyll takes 5+ seconds to build -- these are the sites where a Rust replacement would provide the most value.

## Candidate sources

- Government sites (18F, UK GDS, NHS)
- Large documentation sites (e.g., Bootstrap docs, GitHub docs archives)
- Conference/event sites with many pages
- Organization blogs with hundreds of posts
- Project documentation sites
- Well-known Jekyll blogs or community sites

## Approach

1. Research large Jekyll sites on GitHub (look for repos with many `_posts/`, pages, collections)
2. Clone each (shallow: `git clone --depth 1`) into `websites/`
3. Run both Jekyll and rustkyll builds using `scripts/benchmark.sh --site SITE_PATH`
4. Keep sites that build with at least one tool and have 100+ pages
5. Update `docs/benchmark/results.md` by rerunning the full benchmark
6. Update the `discover_sites()` function in `scripts/benchmark.sh` if new sites are under a new org-level directory structure
7. Note: no changes to `src/` are expected

## Dependencies

None.

## Acceptance Criteria

- [ ] At least 5 new sites added to `websites/` directory, each producing 100+ HTML pages with at least one tool (Jekyll or rustkyll)
- [ ] Sites are from at least 3 different categories (e.g., docs, blog, government, portfolio -- not all the same type)
- [ ] Each new site is a shallow clone (`git clone --depth 1`) to minimize disk usage
- [ ] `scripts/benchmark.sh` discovers and benchmarks all new sites without errors (exit code 0)
- [ ] `docs/benchmark/results.md` is regenerated and includes all new sites with their page counts, build times, and speedup ratios
- [ ] At least 3 of the new sites build successfully with Jekyll (not FAIL), confirming they are valid Jekyll sites
- [ ] At least 2 of the new sites take 5+ seconds to build with Jekyll, providing meaningful large-site benchmarks
- [ ] The `websites/` directory remains gitignored (verify with `git status` -- no new sites show as untracked)
- [ ] No changes to any files under `src/`
- [ ] A brief note is added to `docs/benchmark/results.md` (in the summary section or as a new section) listing which new sites were added and their categories

## Test Scenarios

This issue is purely about adding benchmark data and does not involve code changes to rustkyll itself. "Testing" means verifying the benchmark infrastructure works correctly with the new sites.

### Validation: Site discovery

- Run `scripts/benchmark.sh --site NEW_SITE` for each newly added site individually; verify it completes without script errors
- Run `scripts/benchmark.sh` (full run) and verify all new sites appear in the results table

### Validation: Page counts

- For each new site that builds successfully, verify the page count in results.md is 100+ (the whole point is large sites)
- Cross-check at least 2 sites manually: run the build, then `find websites/SITE/_site -name '*.html' | wc -l` and compare against results.md

### Validation: Build times are plausible

- For sites that Jekyll builds successfully, verify reported times are in a plausible range (not 0.000s, not suspiciously identical across all runs)
- For sites where Jekyll takes 5+ seconds, confirm this is due to genuine page count and not a `bundle install` or network delay being included in the timing

### Validation: Results file integrity

- Open `docs/benchmark/results.md` and verify the markdown table renders correctly (no misaligned columns, no missing pipes)
- Verify the "Sites where both tools succeeded" section (if present) includes any new sites where both tools passed
- Verify FAIL/TIMEOUT entries have appropriate page counts (N/A if both fail, numeric if one succeeded)

### Validation: Gitignore

- Run `git status` after adding all sites; verify no files under `websites/` appear as untracked
