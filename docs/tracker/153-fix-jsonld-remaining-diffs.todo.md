# Issue 153: Fix remaining JSON-LD diffs (~270 diffs, 245 files)

## Problem

Remaining JSON-LD diffs: timezone offsets (build timestamps differ), text content differences, null-vs-empty.

## Goal

Reduce JSON-LD DOM diffs. Build timestamp diffs are expected (different build times) but other content diffs should be fixed.

## Acceptance criteria

- Non-timestamp JSON-LD diffs reduced to 0
- Build timestamp diffs documented as expected
