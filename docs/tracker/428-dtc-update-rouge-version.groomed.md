# Issue 428: Update DTC Rouge/Jekyll gem versions for consistency

## Problem

The DTC site uses Rouge 3.30.0 (via `github-pages` gem v232), which produces the CSS class `no` for YAML boolean values (`true`, `false`, `yes`, `no`). Syntect (used by rustkyll) and newer Rouge versions produce `kc` (keyword constant) for the same tokens. This causes DOM diffs in syntax-highlighted YAML code blocks that are currently filtered as "acceptable diffs" in the comparison tooling.

Updating the DTC site's Rouge version would eliminate this class of acceptable-diff filtering, making DOM comparison results cleaner and more trustworthy.

## Root Cause

Rouge 3.30.0 classifies YAML booleans (`true`, `false`, `yes`, `no`) as `Name.Other` (CSS class `no`). Newer Rouge versions (4.x+) and syntect both classify them as `Keyword.Constant` (CSS class `kc`). The DTC site is pinned to Rouge 3.30.0 via the `github-pages` gem constraint.

## Scope

This is a **DTC source repository change**, not a rustkyll code change. The work involves:

1. Check DTC's current `Gemfile` and `Gemfile.lock` for Rouge and github-pages versions
2. Determine if Rouge can be updated independently or if it requires updating the github-pages gem
3. If possible, update Rouge to 4.x+ in the DTC repo's Gemfile
4. Rebuild the Jekyll cache for DTC and verify:
   - The `no` -> `kc` class change occurs as expected
   - No other unexpected changes in the Jekyll output
5. Update the DOM comparison acceptable-diff filters to remove the `no`/`kc` workaround
6. Verify DTC DOM count remains at 788/790 or above with the new cache

## Dependencies

- None (this is independent of rustkyll code changes)

## Key Files

- `datatalksclub.github.io/Gemfile` -- gem dependencies
- `datatalksclub.github.io/Gemfile.lock` -- locked versions (Rouge 3.30.0 currently)
- `scripts/dom_compare.py` or equivalent -- acceptable diff filters (if `no`/`kc` filter exists)
- `datatalksclub.github.io/_site_jekyll_cached/` -- cached Jekyll output to rebuild

## Risk

The `github-pages` gem (v232) pins Rouge to exactly 3.30.0. Updating Rouge independently may conflict with the github-pages gem constraint. Options:
- (a) Update the github-pages gem to a newer version (if available) that uses Rouge 4.x
- (b) Remove the github-pages gem and use standalone Jekyll + Rouge 4.x (the DTC site is already built locally for caching, not deployed via GitHub Pages gem)
- (c) If neither works, this issue may not be actionable and should be closed as won't-fix

## Acceptance Criteria

- [ ] DTC's current Rouge version documented (expected: 3.30.0)
- [ ] Investigation completed: can Rouge be updated to 4.x+ within the DTC Gemfile constraints?
- [ ] If update is possible:
  - [ ] Gemfile updated with new Rouge version
  - [ ] Jekyll cache rebuilt (`_site_jekyll_cached/`)
  - [ ] DOM comparison shows `no` -> `kc` class changes in YAML code blocks (and no other unexpected changes)
  - [ ] Acceptable-diff filter for `no`/`kc` removed or updated
  - [ ] DTC DOM count stays at 788/790 or above
- [ ] If update is NOT possible:
  - [ ] Document why (github-pages gem constraint, version conflicts, etc.)
  - [ ] Decision: close as won't-fix OR create follow-up for alternative approach
- [ ] No rustkyll code changes required (or if any, they are minimal and well-tested)
- [ ] `cargo test` still passes (to verify no rustkyll regression from cache changes)

## Test Scenarios

### Investigation: version constraints
- Read `datatalksclub.github.io/Gemfile` and `Gemfile.lock` to confirm Rouge 3.30.0
- Run `bundle outdated rouge` in the DTC repo to see available updates
- Check if `github-pages` gem allows Rouge 4.x

### Verification: Jekyll cache rebuild (if update proceeds)
- Run `bundle update rouge` (or equivalent) in the DTC repo
- Run `bundle exec jekyll build` to generate fresh output
- Diff the new Jekyll output against the old cached output
- Verify the only changes are `no` -> `kc` class substitutions in `<span>` tags within code blocks
- Look for any unexpected changes in non-code-block content

### Regression: DTC DOM after cache update
- Copy new Jekyll output to `_site_jekyll_cached/`
- Run DOM comparison between rustkyll output and new Jekyll cache
- Verify match count is at least 788/790
- Verify total diff count does not increase (should decrease due to eliminated `no`/`kc` diffs)

### Edge case: non-YAML code blocks
- Verify that code blocks in other languages (Python, Bash, SQL, etc.) are not affected by the Rouge update
- Spot-check at least 3 different language code blocks in the DTC output
