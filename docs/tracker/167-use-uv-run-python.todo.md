# Issue 167: Use uv run python in all scripts

## Problem

Scripts use `python` or `python3` directly. Should use `uv run python` for consistency with the project's uv-based tooling.

## Acceptance criteria

- All Python script invocations use `uv run python`
- Affects: scripts/dom_compare.py, scripts/categorize_diffs.py, scripts/validate-dtc-xml.py, scripts/build-wheels.py, and any shell scripts that call python
- CI workflows updated too
