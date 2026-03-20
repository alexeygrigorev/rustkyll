# Issue 277: DTC strong element double-nesting

## Problem

On `blog/data-engineers-arent-plumbers.html`, there is `<strong><strong>` double-nesting that issue #275 (mixed delimiter emphasis) did not fix. This appears to be a same-delimiter nesting case rather than a mixed `_`/`*` delimiter case.

## Origin

Identified during issue #275 investigation. The mixed-delimiter fix handles `_*text*_` patterns but not same-delimiter double-nesting like `**__text__**`.

## Acceptance Criteria

- [ ] Investigate the specific markdown pattern causing `<strong><strong>` nesting
- [ ] Fix if feasible without regressing other emphasis handling
- [ ] DOM comparison improves for the affected page
