# Issue 350: DTC guidelines page zero-width-space text normalization mismatch

## Problem

After issue `343` fixed the partial-loose list paragraph wrapping, one residual
DOM diff remains on
`blog/guidelines-to-get-data-engineer-job-against-odds.html`.

The mismatch is an unrelated text-node tail difference:
- Jekyll expected text ends with `straightforward`
- rustkyll output ends with `straightforward\u200b`

## Scope

1. Reproduce the single remaining ZWSP diff on
   `blog/guidelines-to-get-data-engineer-job-against-odds.html`.
2. Determine where `\u200b` is introduced (source content handling vs
   markdown/HTML post-processing).
3. Fix rustkyll to match Jekyll behavior for this page without regressing other
   DTC pages.
4. Add focused regression coverage for the chosen normalization behavior.
5. Reference `#343` in implementation notes and verification logs.

## Priority

MEDIUM - required to fully close the residual target-page diff after `#343`.
