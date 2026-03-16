# Issue 167: Use uv run python in all scripts

## Problem

Scripts use `python` or `python3` directly. Should use `uv run python` for consistency with the project's uv-based tooling.

## Acceptance criteria

- All Python script invocations use `uv run python`
- Affects: scripts/dom_compare.py, scripts/categorize_diffs.py, scripts/validate-dtc-xml.py, scripts/build-wheels.py, and any shell scripts that call python
- CI workflows updated too

## Log

### [SWE] 2026-03-16
- Replaced all `python3` invocations in shell scripts with `uv run python`:
  - `scripts/visual-compare.sh` (2 occurrences)
  - `scripts/batch-visual-compare.sh` (2 occurrences)
  - `scripts/visual-compare-site.sh` (2 occurrences)
  - `scripts/compare-output.sh` (1 occurrence)
- Updated CI workflow `.github/workflows/release.yml`:
  - Added `astral-sh/setup-uv@v4` step to both publish-testpypi and publish-pypi jobs
  - Changed `python scripts/build-wheels.py` to `uv run python scripts/build-wheels.py` (2 occurrences)
  - Changed `pip install twine` to `uv pip install twine --system` (2 occurrences)
  - Changed `twine upload` to `uv run twine upload` (2 occurrences)
- Updated usage docstrings in Python scripts:
  - `scripts/build-wheels.py`
  - `scripts/validate-dtc-xml.py`
  - `scripts/dom_compare.py`
- Updated prerequisite comment in `scripts/visual-compare.sh`
- Note: Python shebang lines (`#!/usr/bin/env python3`) left unchanged -- those are for direct script execution, not invocations from other scripts
- Build: 16 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: scripts/visual-compare.sh, scripts/batch-visual-compare.sh, scripts/visual-compare-site.sh, scripts/compare-output.sh, .github/workflows/release.yml, scripts/build-wheels.py, scripts/validate-dtc-xml.py, scripts/dom_compare.py
