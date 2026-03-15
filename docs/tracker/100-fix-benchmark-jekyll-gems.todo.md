# Issue 100: Fix Jekyll gem installation for benchmark sites

## Problem

Several benchmark sites show Jekyll as "FAIL" not because of actual incompatibility but because their Ruby gems aren't installed. These sites work perfectly with both tools — we just can't benchmark them.

Affected sites:
- alexeygrigorev/aihero — works fine, see https://alexeygrigorev.com/aihero/
- alexeygrigorev/data-science-interviews — works fine, see https://alexeygrigorev.com/data-science-interviews/
- Other sites with Gemfile that need `bundle install`

## Goal

Run `bundle install` for all benchmark sites that have a Gemfile, then re-run the benchmark. Sites that work with Jekyll should show real timings, not "FAIL".

## Approach

1. For each site in websites/ with a Gemfile, run `bundle install`
2. Re-run the benchmark
3. Update results with real Jekyll timings
4. Move sites from "rustkyll only" to "both tools succeed" where applicable

## Acceptance criteria

- aihero builds with Jekyll (with real timing)
- data-science-interviews builds with Jekyll (with real timing)
- All other Gemfile sites attempted
- Benchmark results updated
- Dual-success site count increases
