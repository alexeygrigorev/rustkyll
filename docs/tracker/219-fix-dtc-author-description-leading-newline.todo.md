# Issue 219: Fix DTC author description leading newline

## Problem

Issue 217 introduced a regression: collection item raw content now has a leading `\n`. Expected: `"Alexey Grigorev is the founder..."`, Got: `"\nAlexey Grigorev is the founder..."`. Affects ~200+ DTC blog pages.

The fix in `src/generator.rs` `collection_item_to_liquid_slim()` switched from `html_content` to `content` (raw markdown), but the raw content starts with a newline. Need to trim leading whitespace from the content field.

## Scope

1. Identify where `collection_item_to_liquid_slim()` sets the content field
2. Trim leading whitespace from the raw markdown content before assigning it
3. Verify no other collection item fields are affected

## Acceptance Criteria

- [ ] Collection item content field has no leading `\n` or whitespace
- [ ] DTC author description JSON-LD matches Jekyll output exactly
- [ ] No regressions in other collection item content usage
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Tests include non-ASCII author description content

## Log

- 2026-03-18: Created. Regression from issue 217.
