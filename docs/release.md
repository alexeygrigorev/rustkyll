# Release Process

Releases are handled like any other issue — create a release issue in `docs/tracker/`, groom it, implement it, QA it, PM accepts it.

## When to release

- After a batch of significant bug fixes or features
- When the user requests it
- After fixing critical bugs (e.g. panics, data loss)

## How to release

### 1. Create a release issue

Create `docs/tracker/NN-release-vX.Y.Z.todo.md` with:

```markdown
# Issue NN: Release vX.Y.Z

## Changes in this release

(List the issues completed since last release)

- Issue #AA: description
- Issue #BB: description
- ...

## Version

- Patch (0.1.x): bug fixes, small improvements
- Minor (0.x.0): new features, compatibility improvements
- Major (x.0.0): breaking changes

## Acceptance criteria

- [ ] CHANGELOG.md updated with release notes
- [ ] Version bumped in Cargo.toml, python/pyproject.toml, python/rustkyll/__init__.py
- [ ] All tests pass locally
- [ ] CI is green
- [ ] Cross-platform Docker tests pass (Windows + macOS via dockur)
- [ ] Tag vX.Y.Z pushed
- [ ] Release workflow completes: 6 binaries built, GitHub Release created
- [ ] TestPyPI publish succeeds
- [ ] PyPI publish succeeds with 6 platform wheels
- [ ] `uvx rustkyll --help` works on Linux
- [ ] README benchmark table updated if performance changed
- [ ] Release notes written (not just auto-generated)
```

### 2. PM grooms

PM reviews the issue list, writes release notes, verifies the version bump is correct.

### 3. SWE implements

- Update CHANGELOG.md with release notes
- Bump version in all 3 files
- Commit, tag, push

### 4. QA verifies

- CI green
- GitHub Release has 6 binaries
- TestPyPI and PyPI have 6 wheels each
- `uvx rustkyll --help` works
- Cross-platform Docker tests pass (if available)

### 5. PM accepts and publishes release notes

- Verify all wheels are on PyPI (done means DONE)
- Write proper release notes with: highlights, new features, bug fixes, installation, platform table
- Publish via: `gh release edit vX.Y.Z --repo alexeygrigorev/rustkyll --notes "$(cat release-notes.md)"`
- The `--notes` flag replaces the auto-generated notes with proper human-readable content
- Do NOT leave the release with just "Full Changelog" — write real notes

## Release workflow

The tag push triggers `.github/workflows/release.yml`:

1. Tests run
2. 6 binaries built (linux-amd64, linux-arm64, darwin-amd64, darwin-arm64, windows-amd64, windows-arm64)
3. GitHub Release created with all binaries
4. Wheels published to TestPyPI
5. Wheels published to PyPI

## Secrets

- `PYPI_API_TOKEN` — for PyPI publishing
- `TEST_PYPI_API_TOKEN` — for TestPyPI publishing

Configured at https://github.com/alexeygrigorev/rustkyll/settings/secrets/actions

## Monitoring

```bash
gh run list --repo alexeygrigorev/rustkyll --limit 3
gh run view <run_id> --repo alexeygrigorev/rustkyll
gh run view <run_id> --repo alexeygrigorev/rustkyll --log-failed
```

## Post-release verification

```bash
gh release view vX.Y.Z --repo alexeygrigorev/rustkyll
uvx rustkyll --help
pip install rustkyll==X.Y.Z
```
