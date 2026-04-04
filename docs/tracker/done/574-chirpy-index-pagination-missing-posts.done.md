# Issue 574: Chirpy index pagination shows only 2 of 4 posts

## Problem

Chirpy's index.html should show all 4 posts (Customize the Favicon, Getting Started, Write a New Post, Text and Typography) but rustkyll only renders 2 unique posts (Getting Started, Text and Typography), each duplicated. Jekyll correctly shows all 4.

This was identified during issue #567 acceptance review as a pre-existing pagination bug unrelated to timezone handling.

## Expected Behavior

Chirpy index.html should list (newest first):
1. Customize the Favicon (Aug 10 UTC)
2. Getting Started (Aug 9)
3. Write a New Post (Aug 8)
4. Text and Typography (Aug 8)

## Current Behavior

Only Getting Started and Text and Typography appear, each duplicated.

## Scope

- Investigate why Chirpy's jekyll-paginate-v2 pagination drops 2 of 4 posts
- Fix so all posts appear in correct order on the index

## Dependencies

None (issue #567 already fixed the timezone sorting).

## DTC DOM Baseline

790/790, 0 total diffs

## Log

### [SWE] 2026-04-02

**Root cause analysis:**
The Chirpy home layout uses `where_exp: 'item', 'item.pin != true and item.hidden != true'`
to filter posts. The `where_exp` filter's expression parser (`parse_expression`) did NOT
support `and`/`or` logical operators. It parsed the expression as a single comparison:
`item.pin != "true and item.hidden != true"` where the RHS became a garbage path resolving
to Nil. This caused:
- Pinned posts (pin=true): `true != Nil` -> true -> incorrectly KEPT in `all_normal`
- Non-pinned posts (pin=nil): `Nil != Nil` -> false -> incorrectly EXCLUDED from `all_normal`
The template then iterated `all_normal[0..3]` but only found 2 items, showing duplicates.

**Fix 1: Add and/or logical operator support to where_exp**
- Wrote test: test_where_exp_and_operator (where_exp.rs)
- Ran test: FAILS -- assertion failed: Expected 1 result (Normal Post only), got 2
- Wrote test: test_where_exp_or_operator (where_exp.rs)
- Ran test: FAILS -- assertion failed: Expected MCU and DCEU movies, got 0
- Wrote test: test_where_exp_and_with_unicode_values (where_exp.rs)
- Ran test: FAILS -- assertion failed: Expected only the Russian article
- Implemented fix in src/template/filters/where_exp.rs:
  - Added `LogicalOp` enum (And, Or)
  - Added `Compound` variant to `ParsedExpr`
  - Added `split_logical_operators()` to split on ` and ` / ` or ` outside quotes
  - Modified `parse_expression()` to check for logical operators first
  - Added evaluation of `Compound` in `evaluate_parsed_expr()`
- Ran tests: ALL 3 PASS

**Verification:**
- Chirpy index.html now shows all 4 posts: Getting Started, Text and Typography, Customize the Favicon, Writing a New Post (matching Jekyll)
- DTC DOM: 790/790, 0 total diffs (no regression)
- DTC build time: 0.795s (under 1.0s)

**Summary:**
- Files modified: src/template/filters/where_exp.rs
- Tests added: 3 (test_where_exp_and_operator, test_where_exp_or_operator, test_where_exp_and_with_unicode_values)
- Build results: 3918+ tests pass, 0 fail, clippy clean, fmt clean
- Known limitations: none

### [PM] 2026-04-02 18:30
- Reviewed diff: 5 files changed (where_exp.rs +238, engine.rs fmt-only, issue rename, dom-recount)
- Output verification: Built Chirpy site, inspected /tmp/chirpy_574/index.html -- all 4 posts present (Getting Started, Text and Typography, Customize the Favicon, Writing a New Post)
- Results verified: DTC DOM 790/790, 0 total diffs (no regression)
- Tests: 3 new tests (and_operator, or_operator, and_with_unicode_values) -- all meaningful, cover the exact Chirpy pattern plus or and unicode
- Acceptance criteria: all met
- Follow-up issues created: none
- VERDICT: ACCEPT
