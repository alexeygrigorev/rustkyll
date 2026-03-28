# Issue 428: Update DTC Rouge/Jekyll gem versions for consistency

## Problem

DTC uses an older Rouge version that produces `no` for YAML booleans,
while syntect (and newer Rouge) produces `kc`. Updating DTC's Gemfile
to the latest Rouge would eliminate this acceptable-diff filter.

## Scope

1. Check DTC's current Gemfile.lock for Rouge version
2. Update to latest Rouge
3. Rebuild Jekyll cache and verify output changes
4. Create PR for DTC repo if needed
5. Verify the kc/no difference disappears

## Note

This is a DTC source repo change, not a rustkyll code change.
