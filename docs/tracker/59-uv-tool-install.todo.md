# Issue 59: Make rustkyll installable via uv (uvx / uv tool install)

## Problem

Users should be able to install rustkyll with a single command without needing the Rust toolchain. uv supports installing standalone tools via `uvx` or `uv tool install`.

## Goal

Make rustkyll available so users can run:

```bash
uvx rustkyll build
# or
uv tool install rustkyll
rustkyll serve
```

## Approach

1. Research how uv supports non-Python tools (it can wrap standalone binaries via Python package with entry points pointing to bundled binaries, or via cargo-like install)
2. Create a Python package wrapper that bundles platform-specific rustkyll binaries
3. Publish to PyPI so uv/pip can install it
4. The package should detect the platform and install the correct binary
5. Entry point should be `rustkyll` command

Alternatively, if uv supports cargo install directly, document that path.

## Dependencies

- Issue 58 (cross-platform binaries) -- need compiled binaries for all platforms first

## Acceptance criteria

- `uvx rustkyll --help` works and shows the CLI help
- `uv tool install rustkyll` installs the tool globally
- Works on Linux (amd64, arm64), macOS (Intel, Apple Silicon), and Windows
- README updated with uv installation instructions
- Published to PyPI (or equivalent registry uv supports)
