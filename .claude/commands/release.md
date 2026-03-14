# Release Process for rustkyll

When the user runs `/release`, follow this checklist to publish a new version.

## Pre-release checklist

1. **All issues in current batch are done** — no in-progress issues left
2. **CI is green** — check with `gh run list --repo alexeygrigorev/rustkyll --limit 3`
3. **Tests pass locally** — `./scripts/cargo-safe test && ./scripts/cargo-safe clippy -- -D warnings && cargo fmt --check`
4. **Cross-platform Docker tests pass** (if available) — run Windows and macOS Docker tests from issue #79

## Steps

### 1. Determine version

Check current version:
```bash
grep '^version' Cargo.toml | head -1
```

Decide the new version (semver):
- Patch (0.1.x): bug fixes, small improvements
- Minor (0.x.0): new features, compatibility improvements
- Major (x.0.0): breaking changes

### 2. Write release notes

Create or update `CHANGELOG.md` with a new section for this version. Include:
- **New features** — what was added
- **Bug fixes** — what was fixed
- **Performance** — speed improvements with numbers
- **Compatibility** — new sites supported, new Jekyll features
- **Breaking changes** — if any

Use `git log --oneline <last-tag>..HEAD` to see all commits since last release.

### 3. Bump version

Update version in all three locations:
```bash
# Edit these files:
# - Cargo.toml (version = "X.Y.Z")
# - python/pyproject.toml (version = "X.Y.Z")
# - python/rustkyll/__init__.py (__version__ = "X.Y.Z")
```

### 4. Commit and tag

```bash
git add Cargo.toml Cargo.lock python/pyproject.toml python/rustkyll/__init__.py CHANGELOG.md
git commit -m "Release vX.Y.Z"
git tag vX.Y.Z
git push && git push origin vX.Y.Z
```

### 5. Monitor release workflow

The tag push triggers `.github/workflows/release.yml` which:
1. Runs tests
2. Builds 6 binaries (linux-amd64, linux-arm64, darwin-amd64, darwin-arm64, windows-amd64, windows-arm64)
3. Creates GitHub Release with all binaries
4. Publishes to TestPyPI first
5. Publishes to PyPI

Monitor with:
```bash
gh run list --repo alexeygrigorev/rustkyll --limit 3
gh run view <run_id> --repo alexeygrigorev/rustkyll
```

If it fails:
```bash
gh run view <run_id> --repo alexeygrigorev/rustkyll --log-failed
```

### 6. Post-release verification

- [ ] GitHub Release exists with 6 binaries: `gh release view vX.Y.Z --repo alexeygrigorev/rustkyll`
- [ ] PyPI has the new version: check https://pypi.org/project/rustkyll/
- [ ] All 6 platform wheels on PyPI
- [ ] `uvx rustkyll --help` works on Linux
- [ ] `pip install rustkyll==X.Y.Z` works
- [ ] Update README benchmark table if performance changed

### 7. Announce

Update any relevant documentation, social posts, etc.

## Secrets

The release workflow uses these GitHub secrets:
- `PYPI_API_TOKEN` — PyPI API token for publishing
- `TEST_PYPI_API_TOKEN` — TestPyPI API token for pre-release testing

These are configured at https://github.com/alexeygrigorev/rustkyll/settings/secrets/actions
