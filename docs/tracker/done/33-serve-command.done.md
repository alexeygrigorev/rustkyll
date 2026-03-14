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

## Scope

- `Cargo.toml` -- add a lightweight HTTP server dependency (e.g., `tiny_http` or `warp` or `axum`; prefer something minimal like `tiny_http` since we just need to serve static files)
- `src/main.rs` -- add `Serve` variant to `Commands` enum with `--source`, `--destination`, `--port` flags; implement the serve handler that calls `build_site` then starts the HTTP server
- Optionally a new `src/server.rs` module if the serving logic is non-trivial enough to warrant separation

## Dependencies

- No issue dependencies. The `build_site` function already exists in `main.rs` and can be called directly before starting the server.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `Commands` enum has a `Serve` variant with `source`, `destination`, and `port` fields
- [ ] `rustkyll serve` builds the site (calls `build_site`) and then starts an HTTP server
- [ ] Default port is 4000
- [ ] `--port 8080` flag overrides the port
- [ ] `--source` and `--destination` flags work the same as for the `build` command
- [ ] Server prints `Serving at http://127.0.0.1:<port>` (or similar) to stdout before blocking
- [ ] Server correctly serves static files from the destination directory (HTML, CSS, JS, images)
- [ ] Server returns 404 for files that do not exist
- [ ] Server sets appropriate `Content-Type` headers based on file extension (at minimum: `.html`, `.css`, `.js`, `.png`, `.jpg`, `.xml`, `.json`, `.svg`)
- [ ] Requesting a directory path (e.g., `/blog/`) serves `index.html` from that directory if it exists
- [ ] Requesting a path without extension (e.g., `/about`) tries `about/index.html` and `about.html` as fallbacks (Jekyll-style clean URLs)
- [ ] The server can be stopped with Ctrl+C (standard signal handling)
- [ ] The implementation must be generic -- no site-specific hardcoding

## Test Scenarios

### Unit: CLI parsing for serve command

- Parse `rustkyll serve` -- verify `Serve` variant with default port 4000, source `.`, destination `_site`
- Parse `rustkyll serve --port 8080` -- verify port is 8080
- Parse `rustkyll serve --source /tmp/site --destination /tmp/out` -- verify paths are correct
- Parse `rustkyll serve --port 3000 --source /src --destination /dst` -- verify all three flags

### Unit: Static file serving logic

- Given a destination dir with `index.html`, request for `/` returns the file with `Content-Type: text/html`
- Given a destination dir with `style.css`, request for `/style.css` returns the file with `Content-Type: text/css`
- Request for `/nonexistent.html` returns 404
- Given a destination dir with `blog/index.html`, request for `/blog/` returns the index file
- Given a destination dir with `about.html`, request for `/about` returns `about.html` (clean URL fallback)
- Given a destination dir with `about/index.html`, request for `/about` returns `about/index.html`
- Request for paths with `..` (directory traversal) returns 404 or is sanitized

### Integration: Build-then-serve flow

- Verify that `build_site` is called before the server starts (the destination directory must exist and contain generated files before serving)
- Verify that if `build_site` fails, the serve command exits with an error and does not start the server

### Content-Type mapping

- `.html` maps to `text/html`
- `.css` maps to `text/css`
- `.js` maps to `application/javascript`
- `.json` maps to `application/json`
- `.xml` maps to `application/xml`
- `.png` maps to `image/png`
- `.jpg`/`.jpeg` maps to `image/jpeg`
- `.svg` maps to `image/svg+xml`
- Unknown extension maps to `application/octet-stream`
