# Issue 404: Bash `local` builtin should use class `nb`

## Problem

In bash code blocks, the `local` keyword renders as plain text.
Jekyll/Rouge wraps it as `<span class="nb">local</span>`.

## Scope

Single postprocessing rule in `src/syntax.rs`: in the bash postprocessing
section (around line 437), add a string replacement that wraps bare `local`
in `<span class="nb">local</span>`. Follow the same pattern as
`postprocess_bash_install` which does an identical replacement for `install`.

The replacement must be targeted to avoid false positives -- `local` can
appear as a substring in other words (e.g. `postgres_volume_local`). Replace
only whole-word occurrences that appear as bare text (not already inside a span).

## Example

Input: `docker volume create --name postgres_volume_local -d local`

- Jekyll: `... -d <span class="nb">local</span>`
- Rustkyll before: `... -d local`
- Rustkyll after: `... -d <span class="nb">local</span>`

## Acceptance Criteria

- [ ] In bash/sh/shell code blocks, standalone bare `local` is wrapped as `<span class="nb">local</span>`
- [ ] The word `local` inside other tokens (e.g. `postgres_volume_local`, or already inside a span) is NOT affected
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (no regressions)
- [ ] DTC DOM baseline: 788/790 -- must not drop below this

## Test Scenarios

### Unit: bash `local` builtin wrapping
- Highlight `local var="hello"` as bash, verify output contains `<span class="nb">local</span>`
- Highlight `docker volume create -d local` as bash, verify trailing `local` is wrapped
- Highlight a line containing `postgres_volume_local`, verify `local` inside the identifier is NOT wrapped
- Highlight a line where `local` is already inside a span, verify no double-wrapping

## Dependencies

None.
