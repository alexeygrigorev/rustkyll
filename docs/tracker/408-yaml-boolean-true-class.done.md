# Issue 408: YAML boolean `true`/`false` should use class `no` not `kc`

## Problem

In YAML code blocks, boolean values `true` and `false` use class `kc`
(keyword constant). Jekyll/Rouge uses class `no` (Name.Other).

- Jekyll: `<span class="no">true</span>`
- Rustkyll: `<span class="kc">true</span>`

## Scope

Single postprocessing rule in `src/syntax.rs`: in the YAML postprocessing
section (around line 428), add string replacements to remap
`<span class="kc">true</span>` to `<span class="no">true</span>` and
`<span class="kc">false</span>` to `<span class="no">false</span>`.

This follows the exact same pattern as the existing `on` remap from `kc` to
`na` on that same block, except the target class is `no` instead of `na`,
and the replacement is unconditional (not requiring a trailing `:`).

## Example

Input YAML: `published: true`

- Jekyll: `<span class="na">published</span><span class="pi">:</span> <span class="no">true</span>`
- Rustkyll before: `... <span class="kc">true</span>`
- Rustkyll after: `... <span class="no">true</span>`

## Acceptance Criteria

- [ ] In YAML code blocks, `<span class="kc">true</span>` is replaced with `<span class="no">true</span>`
- [ ] In YAML code blocks, `<span class="kc">false</span>` is replaced with `<span class="no">false</span>`
- [ ] Non-YAML code blocks are not affected (e.g. Python `True`/`False` keeps its existing class)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (no regressions)
- [ ] DTC DOM baseline: 788/790 -- must not drop below this

## Test Scenarios

### Unit: YAML boolean class remapping
- Highlight `enabled: true` as YAML, verify output contains `<span class="no">true</span>`
- Highlight `published: false` as YAML, verify output contains `<span class="no">false</span>`
- Highlight `enabled: true` as Python, verify `true` does NOT get class `no`

## Dependencies

None.
