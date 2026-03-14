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

Follow the pattern used by projects like `ruff`, `uv`, and `oxlint`: create a Python package that bundles platform-specific pre-built binaries and exposes them as console script entry points.

### Package structure

```
python/
  pyproject.toml
  rustkyll/
    __init__.py       # version and metadata
    __main__.py       # allows `python -m rustkyll`
    _main.py          # entry point: finds and executes the bundled binary
```

### How it works

1. The `pyproject.toml` defines `rustkyll` as a Python package with a `[project.scripts]` entry point: `rustkyll = "rustkyll._main:main"`
2. The `_main.py` entry point locates the platform-appropriate binary bundled inside the package's data directory (or a companion platform-specific wheel) and delegates execution via `os.execvp` (Unix) or `subprocess` (Windows)
3. Platform-specific binaries are included via one of two strategies:
   - **Option A (simpler):** A single sdist/wheel per platform, with the binary embedded. Build 5 platform-tagged wheels (e.g., `rustkyll-0.1.0-py3-none-manylinux_2_17_x86_64.whl`)
   - **Option B (ruff/uv pattern):** A "meta" package `rustkyll` that depends on a platform-specific companion package (e.g., `rustkyll-x86-64-unknown-linux-gnu`) which contains the actual binary. The companion packages are auto-selected via platform markers in `pyproject.toml`.
4. A GitHub Actions workflow (or addition to the existing `release.yml`) builds the platform-tagged wheels and publishes them to PyPI on tag push

### Recommended: Option A with platform-tagged wheels

Option A is simpler and well-supported by uv/pip. The key is using the correct wheel tags so pip/uv installs the right binary for the platform.

### CI/CD integration

Extend or add a new workflow that:
1. Downloads the release binaries from the existing `release.yml` artifacts
2. Packages each binary into a platform-tagged wheel
3. Publishes all wheels plus an sdist to PyPI (using `twine` or `uv publish`)

This requires a PyPI API token stored as a GitHub Actions secret.

## Dependencies

- Issue 58 (cross-platform binaries) -- DONE. Release workflow produces binaries for all 5 targets.

## Acceptance Criteria

### Python package structure
- [ ] Directory `python/` exists at the repo root containing the Python packaging files
- [ ] `python/pyproject.toml` exists with correct metadata: name=`rustkyll`, version matching `Cargo.toml`, license, description, and project URLs
- [ ] `python/rustkyll/__init__.py` exists with `__version__` matching the package version
- [ ] `python/rustkyll/__main__.py` exists, allowing `python -m rustkyll` to work
- [ ] `python/rustkyll/_main.py` exists with a `main()` function that locates and executes the bundled binary

### Entry point behavior
- [ ] The entry point script finds the correct binary for the current platform (linux-amd64, linux-arm64, darwin-amd64, darwin-arm64, windows-amd64)
- [ ] The entry point passes all command-line arguments through to the binary unchanged (e.g., `rustkyll build --source /tmp` must pass `build --source /tmp` to the Rust binary)
- [ ] The entry point preserves the binary's exit code (if the binary exits with code 1, the Python wrapper also exits with code 1)
- [ ] On Unix, the entry point uses `os.execvp` to replace the Python process with the binary (no intermediate process)
- [ ] On Windows, the entry point uses `subprocess.run` and forwards the return code
- [ ] If the binary is not found for the current platform, the entry point prints a clear error message listing the supported platforms and exits with code 1

### Wheel building
- [ ] A script or workflow step exists that packages each platform binary into a correctly tagged wheel:
  - `rustkyll-{version}-py3-none-manylinux_2_17_x86_64.manylinux2014_x86_64.whl`
  - `rustkyll-{version}-py3-none-manylinux_2_17_aarch64.manylinux2014_aarch64.whl`
  - `rustkyll-{version}-py3-none-macosx_10_12_x86_64.whl`
  - `rustkyll-{version}-py3-none-macosx_11_0_arm64.whl`
  - `rustkyll-{version}-py3-none-win_amd64.whl`
- [ ] Each wheel contains the Rust binary in the correct location within the package (e.g., `rustkyll/bin/rustkyll` or `rustkyll.data/scripts/rustkyll`)
- [ ] The binary inside the wheel has executable permissions set (Unix wheels)

### GitHub Actions workflow
- [ ] A workflow file exists (either new `pypi-publish.yml` or added job in `release.yml`) that:
  - Triggers on the same `v*` tag push as the release workflow
  - Downloads the built binaries from the release workflow artifacts
  - Builds platform-tagged wheels for all 5 targets
  - Publishes wheels to PyPI using a stored API token secret
- [ ] The workflow waits for the release workflow's build jobs to complete before packaging wheels (uses `needs:` or `workflow_run:` trigger)
- [ ] The workflow does NOT publish if tests fail

### End-to-end verification
- [ ] `pip install rustkyll` (from TestPyPI or local wheel) installs successfully
- [ ] After installation, `rustkyll --version` prints the correct version
- [ ] After installation, `rustkyll --help` shows the CLI help text
- [ ] `uvx rustkyll --help` works (installs ephemerally and runs)
- [ ] `uv tool install rustkyll` installs the tool globally and `rustkyll build --help` works

### Documentation
- [ ] README.md is updated with a "Installation" section showing `uvx rustkyll` and `uv tool install rustkyll` as the primary install methods
- [ ] README.md also mentions `pip install rustkyll` as an alternative
- [ ] README.md mentions building from source with `cargo install` for users who prefer that

## Test Scenarios

Since this issue is primarily about packaging and CI/CD (not Rust library code), testing focuses on the Python package correctness and the end-to-end install experience.

### Unit: Python entry point
- Import `rustkyll._main` and verify `main` function exists and is callable
- Verify `rustkyll.__version__` matches the version in `pyproject.toml`
- Verify `python -m rustkyll` invokes the entry point (test with a mock binary or by checking the error message when no binary is present)
- Test platform detection logic: mock `sys.platform` and `platform.machine()` to verify correct binary name is selected for each of the 5 supported targets
- Test that unsupported platforms produce a clear error message

### Unit: Wheel building script
- Run the wheel-building script with a dummy binary and verify the output `.whl` file:
  - Has the correct filename/tag
  - Contains the binary at the expected path inside the wheel
  - Contains the `METADATA` file with correct package name and version
- Verify the wheel can be installed with `pip install <path-to-whl>` in a fresh venv

### Integration: Local wheel install
- Build a wheel for the current platform (using the locally compiled `cargo build --release` binary)
- Install it in a fresh virtual environment: `pip install ./dist/rustkyll-*.whl`
- Run `rustkyll --version` and verify output matches Cargo.toml version
- Run `rustkyll --help` and verify it shows the CLI help
- Run `rustkyll build --source <test-site> --destination <tmp-dir>` and verify it produces HTML output

### Integration: uvx ephemeral install
- After publishing to TestPyPI: `uvx --index-url https://test.pypi.org/simple/ rustkyll --help`
- Verify the output matches the expected CLI help text

### Edge cases
- Install on a platform where no binary is available (e.g., linux-arm32): verify clear error message
- Verify that the Python wrapper adds no measurable overhead (the `os.execvp` call replaces the process)
- Verify that signals (SIGINT/Ctrl+C) are properly forwarded to the binary
- Verify that stdin/stdout/stderr are properly connected (e.g., `rustkyll build` output appears on the terminal)

## Notes

- The PyPI package name "rustkyll" is confirmed available.
- Reference implementations to study:
  - `ruff`: https://github.com/astral-sh/ruff/tree/main/python (uses companion platform packages)
  - `oxlint`: similar pattern
  - `zig-build`: simpler single-package approach
- For TestPyPI testing, publish with `--repository testpypi` first before going to production PyPI.
- The `pyproject.toml` should specify `requires-python = ">=3.8"` since the wrapper code is minimal and should work with older Python versions.
- Consider whether to support `cargo install rustkyll` as well (by publishing the crate to crates.io) -- that is a separate issue.
- The version in `pyproject.toml` must stay in sync with `Cargo.toml`. Consider a script or CI check that verifies this.
