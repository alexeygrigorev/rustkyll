# Issue 33: Serve Command

## Problem

There's no way to preview the built site without an external tool. Users must run `cargo run -- build` then manually start `python3 -m http.server` or similar.

## Requirements

- Add `rustkyll serve` CLI subcommand
- Serve the built site on a local HTTP server (default port 4000, like Jekyll)
- Support `--port` flag to customize
- Build the site first, then serve it
- Support `--source` and `--destination` flags (same as `build`)
- Print the local URL to stdout when serving
