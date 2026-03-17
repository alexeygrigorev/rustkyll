# Issue 185: Fix JSON-LD FAQ/transcript whitespace in markdownify output

## Problem

JSON-LD FAQ `acceptedAnswer.text` and podcast `transcript` fields have trailing whitespace and newline differences compared to Jekyll. The markdownify filter produces slightly different whitespace when its output ends up inside JSON-LD.

Sample diffs:
- Trailing space: `'<p>...fees.</p>'` vs `'<p>...fees.</p> '`
- Trailing newline in description: `'...DataTalks.Club'` vs `'...DataTalks.Club\n'`
- Markdown links not rendered: `'[Accents Welcome](https://...)'` vs `'Accents Welcome'`

## Goal

Match Jekyll's whitespace handling in markdownify output when used inside JSON-LD.

## Affected Sites

- DataTalksClub/datatalksclub.github.io: ~100 FAQ/podcast pages

## Approach (TDD)

1. Write tests for markdownify output whitespace trimming
2. Verify tests fail
3. Fix whitespace handling
4. Verify tests pass

## Acceptance Criteria

- [ ] No trailing spaces or extra newlines in markdownify output used in JSON-LD
- [ ] Markdown links in description fields rendered consistently with Jekyll
- [ ] DTC FAQ and podcast pages improve
