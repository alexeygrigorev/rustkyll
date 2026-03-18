# Issue 218: Fix FAQ acceptedAnswer.text whitespace diffs

## Problem

DTC pages with FAQ JSON-LD structured data show character-level differences in `acceptedAnswer.text` fields between Jekyll and Rustkyll output. Approximately 10 pages are affected.

The comparison output shows the texts are identical up to the display limit, suggesting trailing whitespace or trailing content differences. The root cause is likely in the `markdownify` custom Liquid filter's output postprocessing (trailing whitespace, newlines, or paragraph tag handling).

## Origin

Descoped from issue 217 (Fix DTC JSON-LD author description diffs), where the SWE investigated and determined this is a separate code path from the `collection_item_to_liquid_slim` fix. The FAQ `acceptedAnswer.text` values go through the `markdownify` filter, not through collection item content fields.

## Scope

1. Identify the specific pages with FAQ acceptedAnswer.text diffs (build DTC site, compare JSON-LD FAQ sections)
2. Root-cause the differences (likely trailing whitespace/newline from markdownify output)
3. Fix the markdownify filter or its postprocessing to match Jekyll output exactly
4. Verify fixes do not regress other markdownify usage

## Dependencies

- Issue 217 (Fix DTC JSON-LD author description diffs) - done

## Acceptance Criteria

- [ ] Build DTC site and identify all pages with FAQ `acceptedAnswer.text` diffs vs Jekyll reference
- [ ] Root cause documented in the issue log
- [ ] FAQ `acceptedAnswer.text` matches Jekyll output exactly for all affected pages
- [ ] No regressions in other `markdownify` filter usage (check non-FAQ pages that use markdownify)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with all new and existing tests
- [ ] Tests include at least one FAQ answer with non-ASCII content

## Test Scenarios

### Unit: markdownify output whitespace

- Parse a markdown string through the markdownify filter, compare trailing whitespace with Jekyll reference
- Test markdownify with multi-paragraph content, verify trailing newline handling

### Integration: FAQ JSON-LD output

- Build a test site with FAQ structured data, verify acceptedAnswer.text matches expected output
- Build DTC site and compare FAQ JSON-LD sections against Jekyll reference

## Log

- 2026-03-18: Created as follow-up from issue 217 (descoped FAQ acceptedAnswer.text diffs).
