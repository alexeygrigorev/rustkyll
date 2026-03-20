# Issue 277: DTC strong element double-nesting

## Status: SUPERSEDED by kramdown parser reimplementation

This issue addressed a pulldown-cmark emphasis parsing bug where `"**text**" or "**text**"` patterns produced `<strong><strong>` double-nesting. A postprocessing workaround (`fix_mismatched_emphasis_nesting()`) was implemented in `src/kramdown.rs`.

Once the kramdown parser (issues 278-283) is fully integrated into the rendering pipeline (issue 283), this postprocessing hack will be unnecessary -- the native kramdown parser handles emphasis correctly per kramdown's own rules.

## Current state

- The postprocessing fix in `src/kramdown.rs` works for the known patterns
- The fix will be removed as part of issue 283 (Phase 4 integration) when kramdown postprocessing hacks are cleaned up
- No further work needed on this issue

## Disposition

Closing as superseded. The fix is already in place and will be cleaned up during kramdown integration (issue 283).
