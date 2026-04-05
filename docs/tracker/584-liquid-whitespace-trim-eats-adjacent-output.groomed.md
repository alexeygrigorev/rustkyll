# Issue #584: Liquid whitespace trimming eats adjacent tag output

## Problem

When a Liquid output tag using whitespace-strip syntax (`{{- ... -}}`) is adjacent to a
non-stripping tag like `{{ ' ' }}`, the dash-trimmed tag's trimming consumes the output of
the neighboring tag. The `{{ ' ' }}` pattern is commonly used to insert a literal space
between two trimmed outputs.

**Example from chirpy read-time.html:**
```liquid
{{- read_time -}}
{{ ' ' }}
{{- site.data.locales[include.lang].post.read_time.unit -}}
```

**Expected:** `1 min` (with a space between number and unit)
**Actual:** `1min` (space is consumed by the adjacent `{{- -}}` trimming)

The `{{-` prefix on the `unit` tag trims whitespace to its left, which should only strip
literal template whitespace (spaces/newlines between tags in the template source), NOT the
rendered output of preceding tags. The space character output by `{{ ' ' }}` is being
incorrectly treated as trimmable whitespace.

## Affected Sites

- **chirpy**: Reading time shows `1min` instead of `1 min`, `3min` instead of `3 min`, etc.
  (3 pages affected: customize-the-favicon, getting-started, text-and-typography)
- Potentially any site using the `{{ ' ' }}` pattern between dash-trimmed tags

## Root Cause

The Liquid template engine's whitespace trimming logic strips whitespace from the rendered
output buffer when it encounters a `{{-` or `{%-` prefix, rather than only stripping the
literal whitespace between template tags in the source.

In Jekyll's Liquid implementation, whitespace stripping only removes whitespace that was
literally present in the template source between tags -- it never touches the output
produced by other tags.

## Acceptance Criteria

- [ ] `{{- val -}}{{ ' ' }}{{- unit -}}` produces `1 min` (not `1min`)
- [ ] `{%- assign x = "a" -%}{{ ' ' }}{%- assign y = "b" -%}` preserves the space in output
- [ ] Regular whitespace trimming still works: `{{- "hello" -}}` trims surrounding template whitespace
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM match count must not drop below 789/790

## Test Scenarios

### Unit: Whitespace trim does not eat adjacent tag output
- `{{- 1 -}}{{ ' ' }}{{- "min" -}}` renders `1 min`
- `{{- "a" -}}{{ "X" }}{{- "b" -}}` renders `aXb`
- `{%- assign x = 1 -%}{{ ' ' }}{%- assign y = 2 -%}` renders a single space
- `{{ 'hello' }}{{- ' world' -}}` renders `hello world` (left-trim on ' world' only strips template ws)

### Unit: Normal whitespace trimming still works
- `  {{- "hello" -}}  ` renders `hello` (template whitespace stripped)
- `{{ "a" }} {{ "b" }}` renders `a b` (no trimming, spaces preserved)

### Integration: Chirpy reading time
- Build chirpy site, verify reading time output contains a space between number and unit
- Verify chirpy DOM match count improves (currently 13/17)

## Dependencies

None.

## DOM Baseline

- DTC: 789/790 matched
- chirpy: 13/17 matched, 46 total diffs

## Log

### [PM] 2026-04-02 10:00
- Created from analysis of chirpy DOM diffs
- Root cause: whitespace trim consuming rendered output of adjacent tags
- Reproduced with minimal test case
