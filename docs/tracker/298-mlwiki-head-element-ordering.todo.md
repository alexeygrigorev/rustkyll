# Issue 298: mlwiki.org head element ordering (0/639)

## Problem

mlwiki.org matches 0/639 (was 236/639 before, regressed). Main diff category is head element ordering — extra link/script elements, attribute ordering in `<head>`. mlwiki.org is a standard Jekyll site. Goal: match Jekyll output exactly.

## Acceptance Criteria

- [ ] mlwiki.org DOM match improves significantly
- [ ] No regressions on other sites
- [ ] cargo test passes
