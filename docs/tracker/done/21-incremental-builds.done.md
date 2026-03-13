# Issue 21: Incremental Build Support (Low Priority)

## Description

Support incremental builds that only regenerate pages whose source files (or dependencies) have changed since the last build. Track file modification times and dependency relationships to skip unchanged pages.

## Dependencies

- Issue 19 (CLI and full build -- need the full build working first)

## Scope

- Track file modification times from previous build (store in a build manifest/cache file)
- Detect changed source files (content, layouts, includes, data files)
- Dependency tracking: if a layout or include changes, rebuild all pages using it
- If a data file changes (e.g., events.yaml), rebuild pages that reference it
- If config changes, do a full rebuild
- `--incremental` flag on the CLI (default: full build)
- Clean build with `--clean` flag to remove the cache and rebuild everything
- Unit tests for change detection logic

## Notes

- This is a low priority feature -- implement after the full site builds correctly
- Collection cross-references (e.g., author pages listing their posts) mean that changing a post may require rebuilding the author page too
- Start simple: track source file mtime, rebuild if newer than output. More sophisticated dependency tracking can come later.
