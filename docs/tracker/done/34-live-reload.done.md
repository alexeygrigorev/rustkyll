# Issue 34: Live Reload

## Problem

During development, users want to see changes immediately without manually rebuilding and refreshing the browser.

## Requirements

- Watch source files for changes (using `notify` crate or similar)
- Automatically rebuild changed pages when source files are modified
- Inject a live-reload script into served pages that triggers browser refresh on rebuild
- Works with `rustkyll serve` command (add `--livereload` flag, on by default)
- Support `--no-livereload` to disable

## Scope

- `Cargo.toml` -- add `notify` crate for file watching and a WebSocket crate (e.g., `tungstenite`) for live reload communication
- `src/main.rs` -- add `--livereload` / `--no-livereload` flags to the `Serve` command variant; wire up the file watcher and live reload server
- `src/server.rs` (or wherever serving logic lives after issue #33) -- add live reload WebSocket endpoint and script injection into HTML responses
- New `src/watcher.rs` module (or similar) for file watching logic: which directories to watch, debouncing, triggering rebuilds

## Dependencies

- Issue #33 (serve command) must be done first. The live reload feature extends the serve command.

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes with all existing tests plus new tests
- [ ] `Serve` command has a `--livereload` flag that defaults to `true`
- [ ] `--no-livereload` disables live reload
- [ ] When live reload is enabled, a WebSocket server (or similar mechanism) starts alongside the HTTP server
- [ ] HTML responses from the server have a live-reload `<script>` tag injected before `</body>` (only when live reload is enabled)
- [ ] The injected script connects to the WebSocket server and triggers `location.reload()` on receiving a reload message
- [ ] Non-HTML responses (CSS, JS, images) are NOT modified by the script injection
- [ ] When `--no-livereload` is used, no script is injected and no WebSocket server starts
- [ ] File watcher monitors the source directory for changes to: `*.md`, `*.html`, `*.yml`, `*.yaml`, `_layouts/*`, `_includes/*`, `_data/*`, `assets/*`, and collection directories
- [ ] File changes trigger an automatic rebuild (calling `build_site`)
- [ ] After a successful rebuild, a reload message is sent to all connected WebSocket clients
- [ ] If the rebuild fails, an error is printed to the terminal but the server continues running (does not crash)
- [ ] File change events are debounced (e.g., 300ms) to avoid multiple rapid rebuilds when saving a file triggers multiple FS events
- [ ] The watcher ignores changes in the destination directory (to avoid infinite rebuild loops)
- [ ] The watcher ignores common non-content files: `.git/`, `node_modules/`, `*.swp`, `*.swo`, `*~`, `.DS_Store`
- [ ] Ctrl+C cleanly stops the watcher, WebSocket server, and HTTP server
- [ ] The implementation must be generic -- no site-specific hardcoding

## Test Scenarios

### Unit: CLI parsing for livereload flags

- Parse `rustkyll serve` -- verify livereload is enabled by default
- Parse `rustkyll serve --no-livereload` -- verify livereload is disabled
- Parse `rustkyll serve --livereload` -- verify livereload is explicitly enabled

### Unit: Script injection

- Given an HTML string with `</body>`, inject the live-reload script before `</body>` and verify the script tag is present
- Given an HTML string without `</body>`, return it unmodified (or append the script)
- Given a CSS string, verify no injection occurs
- Verify the injected script contains the WebSocket connection URL
- Verify the injected script calls `location.reload()` on message receipt

### Unit: File watcher configuration

- Verify the watcher watches the source directory
- Verify the watcher ignores the destination directory
- Verify the watcher ignores `.git/` directories
- Verify the watcher responds to `.md` file changes
- Verify the watcher responds to `_layouts/` file changes
- Verify the watcher responds to `_data/` file changes

### Unit: Debouncing

- Simulate multiple rapid file change events within the debounce window, verify only one rebuild is triggered
- Simulate file changes spaced beyond the debounce window, verify each triggers a separate rebuild

### Integration: Full live reload cycle

- Start the server with live reload enabled
- Verify the WebSocket endpoint is reachable
- Modify a source file, verify rebuild is triggered
- Verify a reload message is sent over the WebSocket after successful rebuild
- Verify the served HTML contains the live-reload script

### Integration: Rebuild error handling

- Start the server, introduce a broken source file (e.g., invalid front matter)
- Verify the server prints an error but continues running
- Fix the source file, verify the next rebuild succeeds and sends a reload message
