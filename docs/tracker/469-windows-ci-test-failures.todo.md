# Issue 469: Windows CI integration test failures

## Problem

The scheduled Integration Tests workflow fails on windows-latest with:
1. `test_load_pages_includes_readme_in_subdirectory` — path separator issue
2. `test_load_pages_readme_without_front_matter` — same
3. Multiple kramdown_parser tests — likely CRLF line ending issues

## Scope

Fix Windows path handling in collection.rs page loading and CRLF
handling in kramdown parser test fixtures.

## Priority

Medium — per-push CI (Linux) passes. This is the scheduled cross-platform job.
