# Issue 32: Cross-Site Build Testing

## Problem

Rustkyll must be a generic Jekyll replacement. We need to find all Jekyll sites from the user's GitHub and DataTalks.Club GitHub, and verify they all build.

## Requirements

- Find all Jekyll websites from github.com/alexeygrigorev and github.com/DataTalksClub
- Clone each one (shallow, into `websites/` directory which is gitignored)
- Attempt `rustkyll build` on each site
- Document which sites build successfully and which fail (and why)
- Create follow-up issues for any new blockers discovered
- Goal: ALL Jekyll sites from both GitHub accounts must build without errors

## Dependencies

- Issue #23 (flexible config) should be done first, as it's the #1 blocker for external sites
