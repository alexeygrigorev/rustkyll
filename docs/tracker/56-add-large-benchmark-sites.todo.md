# Issue 56: Add more large Jekyll websites for benchmarking

## Problem

The benchmark currently only has meaningful speed comparisons for 2 large sites (DTC 787 pages, kids-horror-stories-ru 1345 pages). We need more large Jekyll sites (100+ pages) to validate rustkyll's performance advantage and identify remaining bottlenecks.

## Goal

Find and add 5-10 large open-source Jekyll sites to the benchmark. Focus on sites where Jekyll takes 5+ seconds to build — these are the sites where rustkyll's speed advantage matters.

## Candidate sources

- Government sites (18F, UK GDS, NHS)
- Large documentation sites
- Conference/event sites with many pages
- Organization blogs with hundreds of posts
- Project documentation sites
- Well-known Jekyll blogs

## Approach

1. Research large Jekyll sites on GitHub (look for repos with many posts, pages, collections)
2. Clone each (shallow) into websites/
3. Run both Jekyll and rustkyll builds
4. Add sites that build with at least one tool to the benchmark
5. Update docs/benchmark/results.md and README.md benchmark table

## Dependencies

None

## Acceptance criteria

- At least 5 new large sites added (100+ pages each)
- Benchmark results updated with new sites
- README benchmark table updated with any sites where both tools succeed
- Sites chosen from diverse categories (not all blogs, not all docs)
- websites/ directory remains gitignored
- No code changes to src/
