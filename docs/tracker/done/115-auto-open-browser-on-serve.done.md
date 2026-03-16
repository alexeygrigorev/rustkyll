# Issue 115: Automatically open browser on serve

## Problem

When running `rustkyll serve`, the user has to manually open http://127.0.0.1:4000 in their browser. Jekyll does this automatically.

## Goal

After the site is built and the server starts, automatically open the default browser to the serve URL. Add `--no-browser` flag to disable this.

## Acceptance criteria

- `rustkyll serve` opens the default browser to http://127.0.0.1:{port}
- `--no-browser` flag disables auto-open
- Works on Linux (xdg-open), macOS (open), Windows (start)
- Browser opens AFTER the server is ready (not before)
- No error if browser can't be opened (e.g. headless server) — just skip silently
- All existing tests still pass

## Log

### [SWE] 2026-03-16
- Implemented `open_browser(url)` in `src/server.rs` using platform-specific commands (xdg-open on Linux, open on macOS, cmd /C start on Windows)
- Added `start_server_with_options()` that accepts `auto_open_browser` flag; browser opens after server binds but before the request loop
- Original `start_server()` preserved as a wrapper with `auto_open_browser=false` for backward compatibility
- Added `--no-browser` CLI flag to Serve command in `src/main.rs`, defaults to false (browser opens by default)
- Browser process is detached via a spawned thread; errors are silently ignored
- Tests added: 5 new tests (2 in server.rs for open_browser, 3 in main.rs for CLI flag parsing)
- Build: 1429 tests pass, 0 fail, clippy clean, fmt clean
- Files modified: `src/server.rs`, `src/main.rs`
