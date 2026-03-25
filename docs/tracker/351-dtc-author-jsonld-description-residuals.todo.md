# Issue 351: DTC author JSON-LD description residuals from #340

## Problem

After the syntax-highlighting fixes in `#340`, both target pages are down to a single remaining DOM diff:

- `blog/open-source-free-ai-agent-evaluation-tools.html`
- `blog/naming-variables-in-machine-learning.html`

In both cases, the residue is in JSON-LD:

- `body > script > jsonld.@graph[0].author[0].description`

This is outside the syntax/tokenization layer fixed in `#340`.

## Scope

1. Investigate why `author[0].description` in JSON-LD still differs from Jekyll on the two pages above.
2. Match Jekyll’s output exactly for the author description value.
3. Verify that the fix does not regress the repo-wide DTC DOM count.

## Current Diff Context

- `blog/open-source-free-ai-agent-evaluation-tools.html`: only remaining diff is `jsonld.@graph[0].author[0].description`
- `blog/naming-variables-in-machine-learning.html`: only remaining diff is `jsonld.@graph[0].author[0].description`

## Baseline

- Recorded from the shared tree after `#340` syntax fixes: `771/790`

## Acceptance Criteria

- [ ] Both target pages have `0` remaining diffs against cached Jekyll output
- [ ] `author[0].description` JSON-LD value matches Jekyll exactly on both pages
- [ ] Repo-wide DTC DOM count does not drop below `771/790`

## Dependencies

- Follow-up from `#340`
