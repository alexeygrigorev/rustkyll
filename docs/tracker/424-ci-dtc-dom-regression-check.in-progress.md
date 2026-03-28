# Issue 424: CI -- DTC DOM regression check

## Problem

We reached 790/790 (100%) DTC DOM match. We need CI to prevent regressions.

## Scope

Add a new CI job in `.github/workflows/ci.yml` that builds the DTC site with both Jekyll and rustkyll, runs `scripts/dom_compare.py`, and fails on any regression from 790/790.

This is a **new job** (`dom-check`) within the existing `ci.yml` workflow, running in parallel with the existing `check` job. It must not slow down the fast build/test feedback loop.

## Requirements

- Uses GitHub Actions
- Clones DTC repo fresh (no cached Jekyll output in our repo)
- Installs Ruby, Bundler, Jekyll gems for DTC site (using DTC's Gemfile/Gemfile.lock)
- Installs Python (via uv) with beautifulsoup4 (for dom_compare.py)
- Builds rustkyll in release mode
- Builds DTC site with both Jekyll and rustkyll
- Runs DOM comparison
- Fails on any regression from 790/790

## Design Decisions

### Same workflow file, separate job
The `dom-check` job goes in `.github/workflows/ci.yml` alongside the existing `check` job. This avoids a separate workflow file while keeping the jobs independent and parallel. The `ci.yml` triggers are already correct (push to main, PRs, on src/tests/Cargo/scripts changes).

### Ruby version
Use `ruby/setup-ruby@v1` with `ruby-version: '3.3'` -- matches `integration.yml` and is compatible with DTC's Gemfile.lock.

### Caching
- Cargo: use `Swatinem/rust-cache@v2` (same as existing jobs)
- Ruby gems: use `actions/cache@v4` keyed on `Gemfile.lock` from the DTC repo (same pattern as integration.yml)

### Timeout
Set `timeout-minutes: 30` on the job. Jekyll DTC build typically takes 3-5 minutes, rustkyll build <1 minute, but gem installation can be slow on cold cache.

### Assertion logic
Parse the dom_compare.py summary line to extract the matched file count and total diffs. The summary line format is:
```
Summary: NNN files matched, NNN files with differences, NNN total differences
```
Assert: matched >= 790, files with differences == 0, total diffs == 0.

The dom_compare.py script already exits non-zero when there are diffs, but the CI job must also parse and verify the exact matched count (to catch the case where files disappear and there are fewer comparisons).

## Acceptance Criteria

- [ ] `.github/workflows/ci.yml` contains a new `dom-check` job
- [ ] The existing `check` job (Build & Test) is unchanged
- [ ] `dom-check` clones `DataTalksClub/datatalksclub.github.io` (shallow, `--depth 1`)
- [ ] `dom-check` installs Ruby 3.3, runs `bundle install` for the DTC site using its Gemfile.lock
- [ ] `dom-check` installs uv (for running dom_compare.py with inline script dependencies)
- [ ] `dom-check` builds rustkyll in release mode with Cargo caching (`Swatinem/rust-cache@v2`)
- [ ] `dom-check` builds DTC with Jekyll (`bundle exec jekyll build`) and rustkyll (`target/release/rustkyll build`)
- [ ] `dom-check` runs `uv run scripts/dom_compare.py --jekyll-dir ... --rustkyll-dir ...`
- [ ] The job **fails** if dom_compare.py exits non-zero (any file has diffs)
- [ ] The job **fails** if the matched file count is less than 790 (guards against missing files reducing the comparison set)
- [ ] The job has `timeout-minutes: 30`
- [ ] Ruby gems are cached using `actions/cache@v4` keyed on the DTC Gemfile.lock hash
- [ ] Cargo artifacts are cached using `Swatinem/rust-cache@v2`
- [ ] The `dom-check` job runs in parallel with `check` (no `needs:` dependency between them)
- [ ] `cargo test` still passes (no Rust code changes expected, but verify)
- [ ] The workflow YAML is valid (use `actionlint` or at minimum ensure `act` / manual review shows no syntax errors)

## Test Scenarios

### Validation: Workflow syntax
- Verify the YAML parses correctly (no syntax errors)
- Verify the `dom-check` job has the expected steps in order: checkout, setup-ruby, cache gems, install uv, clone DTC, bundle install, build rustkyll, build DTC with Jekyll, build DTC with rustkyll, run dom_compare.py, assert counts

### Validation: Assertion logic
- The step that parses dom_compare.py output must fail the job if matched < 790
- The step must fail the job if dom_compare.py exits non-zero
- Verify the grep/parse of the summary line is correct by testing against the known output format

### Validation: Independence
- The `check` job definition must be identical before and after (diff the job block)
- The `dom-check` job must not have `needs: check` or similar

### Manual verification
- Push a branch with the new workflow and confirm both jobs appear in the Actions tab
- Confirm `dom-check` is green on the current codebase (790/790, 0 diffs)

NOTE: The "CI passes on current main" criterion from the original issue means the engineer should verify the workflow works by pushing a test branch or by locally validating the YAML and build steps. Final proof is the green CI run after merge, which the orchestrator verifies before moving to `.done.md`.

## Dependencies

None -- this is a CI-only change with no code dependencies on other issues.

## Baseline

DTC DOM: 790/790, 0 total diffs
