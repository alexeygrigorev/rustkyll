# rustkyll sample WASM extension plugin

Source for the committed `../sample.wasm` fixture used by the issue #602 WASM
external-extension adapter tests.

It implements the rustkyll plugin ABI:

- Export `transform`: JSON `{ "html", "collection", "baseurl" }` in ->
  JSON `{ "html", "warnings" }` out.
- Imports host function `resolve_link(target) -> url` (empty string = miss).
- Reads per-entry config from the well-known config key `config` (a JSON
  string), fields: `marker` (default `link`) and `probe`.

Behavior: replaces `{{MARKER:SLUG}}` tokens with `<a href="{baseurl}{url}">SLUG</a>`
on a resolve hit, or `<span class="broken">SLUG</span>` plus a warning on a miss.
When `config.probe == "fs"` it tries to read `/etc/hostname` and records
`sandbox-fs-denied` / `sandbox-fs-ALLOWED` as a warning to demonstrate the
host's default-deny filesystem sandbox.

## Build

Requires the `wasm32-wasip1` target (`rustup target add wasm32-wasip1`).

```bash
cargo build --release --target wasm32-wasip1
cp target/wasm32-wasip1/release/rustkyll_sample_plugin.wasm ../sample.wasm
```

The resulting `sample.wasm` is committed so CI needs no wasm toolchain.
