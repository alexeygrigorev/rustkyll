# Issue 32: Cross-Site Build Testing

## Problem

Rustkyll must be a generic Jekyll replacement, not hardcoded for a single site. We need to find all Jekyll sites from the user's GitHub organizations, clone them, and verify they all build (or at least parse configs and load data without crashing).

## Requirements

- Find all Jekyll websites from github.com/alexeygrigorev and github.com/DataTalksClub
- Clone each one (shallow, into `websites/` directory which is gitignored)
- Attempt `rustkyll build` on each site
- Document which sites build successfully and which fail (and why)
- Create follow-up issues for any new blockers discovered
- Goal: ALL Jekyll sites from both GitHub accounts must build without errors

## Scope

### In scope

1. **Discovery**: Use the GitHub API (`gh` CLI) to list all repositories in `alexeygrigorev` and `DataTalksClub` organizations. For each repo, check if it is a Jekyll site by looking for `_config.yml` at the repo root.

2. **Cloning**: Shallow-clone (`git clone --depth 1`) each Jekyll site into `websites/<org>/<repo>/`. The `websites/` directory is already in `.gitignore`.

3. **Building**: Run `cargo run --release -- build --source websites/<org>/<repo>/ --destination websites/<org>/<repo>/_site` on each site.

4. **Results documentation**: Create a results file (e.g., `docs/cross-site-results.md`) documenting:
   - Which sites were found
   - Which built successfully (pages generated, static files copied, time taken)
   - Which failed and at what phase (config parsing, collection loading, template rendering, etc.)
   - Specific error messages for failures

5. **Follow-up issues**: For each distinct failure mode, create a new `.todo.md` issue in `docs/tracker/`.

### Out of scope

- Fixing the failures found (that is for follow-up issues)
- Testing non-Jekyll static sites
- Testing sites from other GitHub organizations

## Dependencies

- Issue #23 (flexible config parsing) -- DONE. This was the primary blocker: external sites have different config keys.

## Acceptance Criteria

- [ ] All Jekyll sites from `github.com/alexeygrigorev` are identified (list them in the results doc)
- [ ] All Jekyll sites from `github.com/DataTalksClub` are identified (list them in the results doc)
- [ ] Each identified site is shallow-cloned into `websites/`
- [ ] `rustkyll build` is attempted on each site
- [ ] A results document exists at `docs/cross-site-results.md` with:
  - [ ] A table of all sites tested
  - [ ] Build status for each (success/failure)
  - [ ] Error details for failures
  - [ ] Summary statistics (X of Y sites build successfully)
- [ ] Follow-up `.todo.md` issues are created in `docs/tracker/` for each distinct failure mode discovered
- [ ] `cargo test` still passes (no regressions from this work)
- [ ] `cargo clippy -- -D warnings` still passes
- [ ] The `websites/` directory is NOT committed (verify it is in `.gitignore`)

## Test Scenarios

### Manual: Site discovery

- Run `gh repo list alexeygrigorev --limit 200 --json name,url` and `gh repo list DataTalksClub --limit 200 --json name,url`
- For each repo, check `gh api repos/<org>/<repo>/contents/_config.yml` to see if it exists
- Expected: at least `datatalksclub.github.io` and `alexeygrigorev.github.io` are found

### Manual: Build each site

- For each Jekyll site found, run the build command
- Record the output (success message with counts, or error message)
- For sites that fail, categorize the failure:
  - Config parsing error (unknown keys, invalid YAML)
  - Collection loading error (unexpected file formats)
  - Template/layout error (missing layouts, unknown Liquid filters)
  - Other errors

### Integration: Config parsing compatibility

- Every discovered Jekyll site's `_config.yml` must parse without error using `SiteConfig::from_file`
- This is the minimum bar: even if page generation fails, config parsing must work (issue #23 should have ensured this)

### Regression: Existing site still builds

- The `datatalksclub.github.io` site must still build successfully after any code changes made during this issue
- Output should match the previous build (same page count, same static file count)

## Notes

- Use `gh` CLI for GitHub API access (it handles authentication)
- Jekyll sites typically have `_config.yml` at the root. Some repos may have Jekyll sites in subdirectories -- focus on root-level Jekyll sites only.
- Common GitHub Pages Jekyll sites follow the pattern `<org>.github.io` or have a `gh-pages` branch. Check the default branch first.
- Sites may use Jekyll plugins that rustkyll does not support -- document these as known limitations.
- The `websites/` directory is already in `.gitignore` so cloned sites will not be committed.
