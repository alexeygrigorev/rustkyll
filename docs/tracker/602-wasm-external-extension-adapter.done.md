# Issue 602: WASM / external extension adapter

Follow-up to #600 (extension framework + wikilinks). Explicitly deferred as
out-of-scope in #600; tracked here.

## Problem

#600 built a compiled-in extension framework: an object-safe `HtmlTransform`
trait, a `Registry` built from the `extensions:` config block, and a `SiteIndex`
for link resolution. The trait was deliberately designed to be `dyn`-compatible
so a future **WASM-backed** extension could implement it without changing the
trait or the registry.

This issue builds that escape hatch: third parties should be able to extend
rustkyll with content transforms **without access to its source and without
recompiling it**. A `.wasm` module written in any language that targets
WebAssembly is dropped into the site, declared in `_config.yml`, and driven
through the exact same `HtmlTransform` trait and `Registry` as the compiled-in
extensions. The render pipeline (`Registry::apply`) must call native and wasm
extensions identically.

## Runtime choice (confirmed)

**Runtime: [`extism`](https://extism.org/) Rust host SDK — `extism = "1"`
(latest 1.30.0 on crates.io, confirmed reachable during grooming via
`cargo search extism`).** Sample plugins are authored with `extism-pdk = "1"`
(1.4.1). extism gives us: a stable bytes-in/bytes-out plugin ABI, first-class
**host functions** (needed for `resolve_link`), per-plugin **config**, and
capability controls (`allowed_paths` / `allowed_hosts`) for sandboxing.

Concerns and decisions:

- **Build size / compile time.** extism embeds a wasm engine (wasmtime) — a
  large dependency that adds noticeable compile time and ~10-15MB to the binary.
  Mitigation (required, see criteria): gate the entire wasm adapter behind a
  cargo feature named `wasm`, **enabled by default** so released/prebuilt
  binaries support the escape hatch, while `--no-default-features` (or a
  `wasm`-off profile) lets CI / minimal builds skip the wasmtime dependency
  entirely. When the `wasm` feature is off, a `- wasm:` config entry must fail
  loudly with a clear "built without wasm support" error (not a silent skip).
- **Offline cargo.** extism is on crates.io and fetched fine here; if a future
  offline/vendored build breaks, the documented fallback is to depend on
  `wasmtime` directly and implement a thin ABI. **Default is extism** per the
  settled design; wasmtime is only a documented fallback, not implemented in
  this issue.

## Scope

### In scope

- A `WasmExtension` adapter that implements `HtmlTransform` by calling into a
  loaded `.wasm` module. Same trait, same `Box<dyn HtmlTransform>` in the
  `Registry` as compiled-in extensions.
- **Instantiate once, reuse.** The `.wasm` module is loaded and instantiated
  once at registry-build time and reused for every document. It is NOT
  reloaded/reinstantiated per document.
- Load `.wasm` modules by **path** declared in `_config.yml`
  (`- wasm: _extensions/foo.wasm`), resolved relative to the site source root.
- An optional per-entry `config:` block, serialized to JSON and passed to the
  plugin at instantiation.
- A `resolve_link(target) -> url | null` **host function** exposed to the
  plugin so it can query the `SiteIndex` (slug/title -> URL) without the whole
  index crossing the wasm boundary. This is the key perf design point: only the
  queried string and its resolved URL cross the boundary.
- A `Registry` that loads **both** builtins (by name) and wasm modules (by path)
  into one ordered list, honoring declared order.
- **Sandboxing: default deny.** Third-party wasm is untrusted. By default the
  plugin gets NO filesystem and NO network access (extism `allowed_paths` and
  `allowed_hosts` left empty). Granting access is out of scope for this issue.
- Loud failure on a missing/invalid `.wasm` file, an ABI mismatch (missing
  `transform` export), or (feature-off) a wasm entry with no wasm support —
  mirror #600's `ExtensionError` semantics (`Err`, no silent default, no panic).
- A **small committed sample plugin** (prebuilt `.wasm` fixture + its source)
  and tests that load the real `.wasm` and exercise transform + resolve_link.

### Out of scope

- Granting filesystem/network capabilities to plugins (default-deny only here;
  revisit in a follow-up if needed).
- Distributing/packaging a marketplace of extensions.
- Hot-reload of `.wasm` on change during `serve`/watch.
- Direct-wasmtime fallback implementation (documented only).

## Plugin ABI contract

This is the stable contract between rustkyll (host) and a `.wasm` plugin. It
mirrors the native `HtmlTransform::transform` signature and `TransformResult`.

### Plugin exports (called by the host)

- `transform` — an extism export function (bytes in, bytes out).
  - **Input:** JSON object
    ```json
    { "html": "<p>…</p>", "collection": "wiki", "baseurl": "" }
    ```
    `collection` is a string or `null` (standalone pages). `baseurl` is the site
    baseurl (may be empty).
  - **Output:** JSON object
    ```json
    { "html": "<p>…</p>", "warnings": ["…", "…"] }
    ```
    `warnings` may be omitted or empty. A non-JSON or schema-invalid output is an
    ABI mismatch and must surface as a build error (not a silent pass-through).

### Host functions (imported by the plugin)

- `resolve_link` — takes a single string `target`, returns a string URL.
  - **Semantics:** returns the URL for a resolved slug/title (same result the
    compiled-in wikilinks transform would get from `SiteIndex::resolve`), or an
    **empty string** to signal "unresolved / `None`". (Empty string is the miss
    sentinel; document this in the contract.)
  - Only `target` and the returned URL cross the boundary — the full index does
    not.

### Config passing

- The optional `config:` YAML mapping under a `- wasm:` entry is serialized to a
  JSON string and provided to the plugin as extism plugin config under a single
  well-known key (e.g. `config`). The plugin reads it via the PDK
  (`extism_pdk::config::get`). A plugin with no `config:` block sees an
  absent/empty value.

### Implementation guidance (non-normative but important)

- `HtmlTransform: Send + Sync`, but an extism `Plugin` is `Send` and not `Sync`
  and `call` takes `&mut self`. Wrap the single instantiated `Plugin` in a
  `Mutex` so `WasmExtension` is `Sync`; wasm calls then serialize through the
  lock (acceptable — note the perf tradeoff vs. the parallel native path).
- `resolve_link` needs the `SiteIndex` at call time, but `Registry::from_config`
  runs BEFORE the index exists (the index is built later in `main.rs` from
  collections + pages). The `SiteIndex` is immutable for the whole build. Bind
  the index to the host function via a shared slot (e.g. `Arc<ArcSwapOption>` /
  `Arc<Mutex<Option<…>>>` / extism `UserData`) that `WasmExtension::transform`
  populates from the per-call `&SiteIndex` before invoking the plugin. Do not
  deep-clone the whole index per document.
- `Registry::from_config` needs the site source root to resolve relative wasm
  paths (builtins don't). Extend the signature (or add a sibling constructor);
  the compiled-in path and existing tests must remain correct. This API change
  is expected — update the `main.rs` call site accordingly.

## Sample plugin (deliverable — makes the issue actually DONE)

- A tiny sample plugin authored in Rust with `extism-pdk`, compiled to wasm, and
  the resulting artifact **committed** to `tests/fixtures/wasm/` (e.g.
  `sample.wasm`) so CI needs no wasm toolchain. Commit the plugin **source**
  (small crate) alongside it plus a short README with the build command, so the
  artifact is reproducible.
- Suggested behavior (small, but exercises the whole ABI): scan input HTML for a
  token `{{link:SLUG}}` (token configurable via `config.marker`, default
  `link`), call `resolve_link(SLUG)`, and replace with
  `<a href="URL">SLUG</a>` on a hit or `<span class="broken">SLUG</span>` plus a
  warning on a miss. This exercises: transform mutation, resolve_link hit AND
  miss, warnings propagation, and config passthrough.

## Acceptance Criteria

- [ ] `extism` added as a dependency behind a cargo feature `wasm`, enabled by
      default. `./scripts/cargo-safe build --no-default-features` compiles
      without pulling extism/wasmtime.
- [ ] A `WasmExtension` type implements `HtmlTransform` (`name`, `transform`) and
      is stored in the `Registry` as `Box<dyn HtmlTransform>` — the pipeline
      calls it identically to compiled-in extensions.
- [ ] `Registry::from_config` (with source-root awareness) loads a `- wasm:
      _extensions/foo.wasm` entry, resolving the path relative to the site
      source root, and preserves declared order alongside builtins (e.g.
      `wikilinks` then a wasm entry appear in order in the registry).
- [ ] The `.wasm` module is instantiated exactly once at registry-build time and
      reused across documents (assert/observe a single instantiation, not
      per-document).
- [ ] The committed sample `.wasm` is loaded and executed by rustkyll: an
      end-to-end fixture build applies the wasm transform to page HTML and the
      output reflects the transform (real wasm loaded + run, per "Done Means
      DONE" — not just adapter code compiling).
- [ ] `resolve_link` host function returns the correct URL for a known
      slug/title (matching `SiteIndex::resolve`) and the empty-string miss
      sentinel for an unknown target; a round-trip test proves both.
- [ ] Per-entry `config:` block is serialized to JSON and reaches the plugin; a
      test changes plugin behavior via config and asserts the effect.
- [ ] Warnings returned by the plugin are surfaced in `TransformResult.warnings`
      and flow through `Registry::apply` like native warnings.
- [ ] Missing `.wasm` file, invalid/corrupt `.wasm` bytes, and a module missing
      the `transform` export each fail loudly as an `ExtensionError` build error
      (Err, not panic, not silent skip). Separate test per case.
- [ ] With the `wasm` feature disabled, a `- wasm:` entry fails loudly with a
      clear "built without wasm support" error.
- [ ] Sandboxing default-deny: the plugin is instantiated with no
      `allowed_paths` and no `allowed_hosts`. A test asserts a plugin attempting
      filesystem or network access is denied / errors (not silently succeeding).
- [ ] Opt-in preserved: a site with no `- wasm:` entries and no `_extensions/`
      directory spins up **no** wasm runtime (no extism `Plugin` instantiated)
      and behaves byte-identically to today.
- [ ] DTC DOM match count must not drop below the #600 committed baseline of
      **788/790** (DTC has no `extensions:` block, so it must remain exactly
      788/790). Verify with
      `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`.
- [ ] `./scripts/cargo-safe test` green (default features and
      `--no-default-features`), `./scripts/cargo-safe clippy -- -D warnings`
      clean, `cargo fmt --check` clean.
- [ ] TDD followed: tests written first, fail-then-pass logged in `## Log`.

## Test Scenarios

### Unit / integration: registry + adapter

- Config with `- wasm: <fixture path>` loads a `WasmExtension` into the registry;
  assert it is present and in declared order (e.g. after a `wikilinks` entry).
- Config with builtins only (no wasm) behaves exactly as #600 — existing tests
  unchanged.
- No `extensions:` block and no `_extensions/` dir -> empty registry, no wasm
  runtime instantiated (observe via a marker / instantiation counter).

### Integration: real wasm execution (loads committed sample.wasm)

- Load `tests/fixtures/wasm/sample.wasm`, run `transform` on HTML containing
  `{{link:known-slug}}` and `{{link:missing-slug}}`; assert:
  - known slug -> `<a href="…">` with the URL from the `SiteIndex`
  - missing slug -> `<span class="broken">…</span>` + a warning in the result
- `resolve_link` round-trip: hit returns the exact `SiteIndex::resolve` URL;
  miss returns empty string and the plugin renders the broken form.
- Config passthrough: pass `config: { marker: ref }`, assert the plugin now
  transforms `{{ref:…}}` tokens instead of `{{link:…}}`.

### Integration: loud failures

- `- wasm: _extensions/does-not-exist.wasm` -> `ExtensionError` build error.
- A corrupt/invalid `.wasm` (e.g. a text file renamed) -> `ExtensionError`.
- A wasm module missing the `transform` export -> `ExtensionError` (ABI
  mismatch), not a panic.
- (feature off) `- wasm:` entry with `wasm` feature disabled -> clear
  "built without wasm support" error.

### Integration: sandbox

- A plugin (or the sample plugin under a config flag) attempting to read a file
  or open a socket is denied/errors because no `allowed_paths`/`allowed_hosts`
  were granted.

### End-to-end fixture build

- Build a minimal fixture site whose `_config.yml` declares the sample wasm
  extension and whose page contains a `{{link:…}}` token; build to a temp dir
  and assert the generated HTML contains the transformed anchor.

## Dependencies

- #600 (extension framework + wikilinks) — `.done.md` (satisfied:
  `docs/tracker/600-extension-framework-and-wikilinks.done.md`).

## Notes

- Deferred from #600 per user discussion (compiled-in native extensions now,
  WASM as the future escape hatch behind the same trait). The #600
  `HtmlTransform` trait is already object-safe to support this.
- DTC DOM baseline for this issue: **788/790 (100%)** per committed
  `docs/dom-recount-results.md` / the #600 done file. A live recount could not be
  produced during grooming because the DTC site is not checked out under
  `websites/DataTalksClub/datatalksclub.github.io/` with a matching
  `_site_jekyll_cached/` locally (the `/tmp` copy builds 0 HTML, the
  `~/git` copy has no Jekyll cache) — same situation as #600 grooming. Since DTC
  has no `extensions:` block and this feature is opt-in, DTC output must remain
  byte-identical at exactly 788/790; SWE/QA must confirm with a live recount.

## Log

### [PM] 2026-07-02 — Groomed
- Read the issue, `docs/PROCESS.md`, and the #600 extension framework
  (`src/extensions/mod.rs`, `src/extensions/wikilinks.rs`) plus the pipeline
  wiring in `src/main.rs:832-864` and `src/generator.rs`.
- Confirmed runtime: **extism** (host SDK `extism = "1"`, plugins via
  `extism-pdk = "1"`). Verified reachable on crates.io (`cargo search extism`
  -> 1.30.0 / pdk 1.4.1). Not in local cargo cache, but network fetch works.
  Documented wasmtime-direct as fallback only.
- Recorded the plugin ABI contract (exports `transform` JSON in/out; host fn
  `resolve_link(target)->url|"" `; config as JSON under a well-known key) and
  key implementation constraints (Send+Sync via `Mutex<Plugin>`; late-bound
  `SiteIndex` slot for the host fn; source-root-aware `Registry::from_config`;
  `wasm` cargo feature default-on with `--no-default-features` escape).
- Added concrete, testable acceptance criteria and test scenarios covering:
  real wasm load+execute, resolve_link hit/miss round-trip, config passthrough,
  warnings propagation, loud failures (missing/invalid/ABI-mismatch/feature-off),
  default-deny sandbox, opt-in no-runtime, and the 788/790 no-regression guard.
- Required a **committed prebuilt sample `.wasm` + source** so the issue is only
  DONE when a real wasm is loaded and run by rustkyll (per "Done Means DONE"),
  with no wasm toolchain needed in CI.
- DOM baseline: attempted a live DTC recount; the site is not checked out under
  `websites/` with a usable Jekyll cache (mirrors #600). Using the committed
  788/790 figure; SWE/QA must confirm live (opt-in feature => must stay exactly
  788/790).
- Renamed `.todo.md` -> `.groomed.md` via `git mv`.

### [SWE] 2026-07-02

**Environment checks (done first, per instruction):**
- `extism` fetchable: YES. `cargo add extism@1` resolved to **1.30.0** and
  `cargo fetch` pulled the whole tree (wasmtime/wiggle/wasi-common etc.) with
  exit 0. `extism-pdk` = **1.4.1**, also fetched. crates.io is reachable here.
- wasm toolchain buildable: YES. `rustup target add wasm32-unknown-unknown` and
  `wasm32-wasip1` both installed (exit 0). No `wat2wasm`/`wasm-tools`, but not
  needed — the sample plugin is a real Rust/extism-pdk crate. NOT blocked.

**Sample plugin (committed deliverable):**
- Source crate: `tests/fixtures/wasm/sample-plugin/` (Cargo.toml, src/lib.rs,
  README.md with the exact build command, .gitignore for /target). Standalone
  (own empty `[workspace]`) so it is NOT pulled into the rustkyll workspace.
- Built with `cargo build --release --target wasm32-wasip1` -> committed
  `tests/fixtures/wasm/sample.wasm` (195 KB). Reproducible per README.
- Behavior: replaces `{{MARKER:SLUG}}` (MARKER from `config.marker`, default
  `link`) via the `resolve_link` host fn -> anchor on hit, `<span class="broken">`
  + warning on miss; `config.probe=fs` attempts an fs read to demonstrate the
  sandbox (records `sandbox-fs-denied` / `-ALLOWED`). Exercises the full ABI.

**Fix 1: resolve_link host-fn round-trip (hit + miss)**
- Wrote test: test_sample_wasm_transforms_and_resolves_hit + _resolve_link_miss_*
  (src/extensions/wasm.rs).
- Demonstrated FAIL-FIRST genuinely: temporarily made `resolve_link` always
  return `""`. Ran test: FAILS — got `<span class="broken">event-tracking</span>`,
  expected `<a href="/wiki/event-tracking/">event-tracking</a>`. Restored fix.
- Ran test: PASSES (hit -> anchor with exact SiteIndex URL; miss -> broken span
  + warning; empty-string is the miss sentinel).

**Fix 2: per-entry config passthrough**
- Wrote test: test_registry_wasm_config_block_reaches_plugin (mod.rs) +
  test_sample_wasm_config_changes_marker (wasm.rs).
- Demonstrated FAIL-FIRST: temporarily dropped `manifest.with_config_key(...)`.
  Ran test: FAILS — got `{{ref:foo}}` (unchanged), expected
  `<a href="/wiki/foo/">foo</a>`. Restored fix.
- Ran test: PASSES (`config: { marker: ref }` switches the recognized token).

**Remaining behaviors (tests authored for this greenfield adapter; each asserts
loudly, none skip):**
- WasmExtension implements HtmlTransform, stored as `Box<dyn HtmlTransform>` and
  called identically by `Registry::apply` (src/extensions/wasm.rs).
- Single instantiation reused across docs: test_instantiated_once_reused_across_documents
  (1 load, 5 transform calls -> `call_count()==5`, one `Mutex<Plugin>`).
- Source-root-aware loader: added `Registry::from_config_with_source(config, root)`;
  `from_config` kept as a thin wrapper (existing tests unchanged). main.rs now
  calls `from_config_with_source(&config, source)`.
- Declared order: test_registry_loads_wasm_after_wikilinks_in_order
  (`wikilinks` then `wasm:sample.wasm`).
- Warnings flow through apply: test_registry_apply_surfaces_wasm_warnings.
- Loud failures (separate test each): missing file, corrupt bytes, missing
  `transform` export (all -> `ExtensionError::WasmLoad`); feature-off `- wasm:`
  -> `ExtensionError::WasmUnsupported` (test compiled only under
  `--no-default-features`, verified passing).
- Sandbox default-deny: test_sandbox_denies_filesystem_access (no allowed_paths/
  allowed_hosts; plugin fs read is denied -> `sandbox-fs-denied`, never ALLOWED).
- End-to-end real-wasm build: integration_tests/tests/integration_wasm_extension.rs
  builds a temp site declaring `- wasm: _extensions/sample.wasm`, page uses
  `{% raw %}{{link:...}}{% endraw %}` (raw needed so Liquid leaves the `{{ }}`
  literal for the post-render transform). Asserts the resolved anchor + broken
  span appear in generated `index.html`. PASSES against the release binary
  (proves rustkyll actually LOADS + EXECUTES the .wasm end-to-end).

**Design notes:**
- `WasmExtension` = `Mutex<Plugin>` (Plugin is Send not Sync, call is &mut) +
  a late-bound `UserData<usize>` slot holding the address of the current
  `&SiteIndex`. The pointer is written only while holding the plugin mutex and
  cleared before releasing it, so concurrent transform calls (serialized by the
  lock) never see another call's index and no deep-clone of the index happens.
  `resolve_link` reads that slot; only the target string + resolved URL cross
  the boundary.
- Runtime-time ABI errors (non-JSON/invalid plugin output, call failure) are
  surfaced as loud warnings (pipeline prints warnings to stderr) with the HTML
  left unchanged — the `HtmlTransform` trait is intentionally not made fallible.
  Load-time errors (missing/corrupt/missing-export) are hard `Err`s.

**Feature gating:**
- `extism` is an optional dep behind cargo feature `wasm`, DEFAULT-ON
  (`default = ["wasm"]`, `wasm = ["dep:extism"]`). `wasm.rs` and the loader arm
  are `#[cfg(feature = "wasm")]`. Verified BOTH configs:
  - default features: `cargo-safe test` -> **4586 passed, 0 failed**.
  - `--no-default-features`: compiles WITHOUT extism/wasmtime and tests pass,
    including test_registry_wasm_entry_errors_without_feature (feature-off `- wasm:`
    -> `WasmUnsupported`).

**Build / lint results:**
- `cargo-safe test` (default): 4586 passed, 0 failed.
- `cargo-safe test --no-default-features`: passes (extism not compiled).
- `cargo-safe clippy -- -D warnings` (project-standard, lib+bins): CLEAN (exit 0).
  Note: `clippy --all-targets` surfaces ~62 PRE-EXISTING lints in unrelated test
  code (e.g. src/pagination.rs `useless vec!`) that are not part of the project's
  clippy gate and not touched by this issue. Two pre-existing unused-import
  warnings in src/plugin_generators.rs tests exist under BOTH default and
  `--no-default-features` (confirmed) — pre-existing, not from this change.
- `cargo fmt --check`: CLEAN.

**Files added:**
- src/extensions/wasm.rs (adapter + host fn + 9 unit/integration tests)
- integration_tests/tests/integration_wasm_extension.rs (e2e real-wasm build)
- tests/fixtures/wasm/sample.wasm (committed prebuilt plugin)
- tests/fixtures/wasm/sample-plugin/{Cargo.toml,src/lib.rs,README.md,.gitignore}

**Files modified:**
- Cargo.toml (extism optional dep + `wasm` feature, default-on)
- Cargo.lock (dependency tree)
- src/extensions/mod.rs (wasm module decl, ExtensionError::{WasmUnsupported,
  WasmLoad}, `from_config_with_source`, `parse_wasm_entry`, wasm entry arm,
  5 new registry tests)
- src/main.rs (call `from_config_with_source(&config, source)`)

**DTC DOM / perf — DEFERRED (per orchestrator + documented in Notes):**
- DTC is NOT checked out under `websites/DataTalksClub/datatalksclub.github.io/`
  and there is no `_site_jekyll_cached/` locally (same situation as #600/grooming),
  so a live 788/790 recount and the perf timing could not be produced here.
  Opt-in is structural: `from_config`/`from_config_with_source` load a wasm
  plugin ONLY on a `- wasm:` entry, and DTC has no `extensions:` block, so no
  extism `Plugin` is instantiated and output stays byte-identical. QA must run
  `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
  once the site + Jekyll cache are available and confirm exactly 788/790.

**Known limitations:**
- wasm calls serialize through the per-extension `Mutex<Plugin>` (documented
  perf tradeoff vs. the parallel native path).
- Granting filesystem/network capabilities to plugins is out of scope (default-deny only).

### [QA] 2026-07-02
Independently verified (did NOT trust SWE-reported numbers). Ran all commands
myself with `./scripts/cargo-safe`.

**Build / test / lint (verified counts):**
- Release binary built (default features) — OK, 1m37s.
- `cargo-safe test` (default features): **4586 passed, 0 failed, 2 ignored**
  (matches SWE claim). The 2 ignored are pre-existing kramdown_parser tests
  (`kramdown_block_06_codeblock_rouge_multiple`, `kramdown_block_14_table_errors`),
  unrelated to this issue.
- All 13 wasm unit/registry tests ran and passed (transforms_and_resolves_hit,
  resolve_link_miss, baseurl_applied, config_changes_marker,
  instantiated_once_reused, sandbox_denies_filesystem_access,
  missing_wasm_file, missing_transform_export, corrupt_wasm,
  loads_wasm_after_wikilinks_in_order, wasm_missing_file, config_block_reaches_plugin,
  apply_surfaces_wasm_warnings). None skipped.
- Note: root `cargo test` runs only the `.` package (root pkg == workspace root),
  so the e2e crate is NOT covered by the default suite. Ran it explicitly:
  `cargo-safe test -p integration-tests --test integration_wasm_extension`
  -> **1 passed**. Test asserts (panics) on missing binary/wasm — does NOT
  silently skip. Informational for CI: e2e requires `-p integration-tests`.
- `cargo-safe test --no-default-features`: **4574 passed, 0 failed**. Confirmed
  extism/wasmtime are NOT compiled (0 "Compiling extism/wasmtime" lines), and
  `test_registry_wasm_entry_errors_without_feature` ran and passed (feature-off
  `- wasm:` entry -> loud `WasmUnsupported`, not a silent skip). The only 2
  warnings are pre-existing unused imports in src/plugin_generators.rs tests
  (not in this diff), present under both feature configs.
- `cargo-safe clippy -- -D warnings` (default): exit 0. `--no-default-features`:
  exit 0. Only a renamed-lint note from vendored liquid-lib; new code clean.
- `cargo fmt --check`: clean.

**Independent end-to-end with the RELEASE binary (step 7):**
- Built a fixture site with `extensions: [- wasm: _extensions/sample.wasm]`,
  an `about.md` page, and an `index.md` with raw `{{link:about}}` and
  `{{link:ghost}}`. Output: `Link: <a href="/about.html">about</a> and broken
  <span class="broken">ghost</span>` — the real .wasm was LOADED + EXECUTED,
  `resolve_link` resolved `about` to the real page URL (hit) and returned the
  empty-string miss sentinel for `ghost` (broken span). Confirmed independently
  of the committed test.
- Control fixture with NO `extensions:` block: raw `{{link:about}}` token left
  byte-literal, no transform, no wasm runtime — opt-in preserved.

**Sample plugin deliverable:**
- `tests/fixtures/wasm/sample.wasm` present (195357 bytes, magic `\0asm`, real
  module). `git add -n` would stage source + README + .wasm; `target/` correctly
  gitignored. Currently untracked (expected — commit happens after PM accept).

**Per-criterion:**
- extism behind default-on `wasm` feature; `--no-default-features` skips
  extism/wasmtime: PASS (verified 0 compiles).
- WasmExtension impls HtmlTransform, stored as `Box<dyn HtmlTransform>`, called
  identically by `Registry::apply`: PASS.
- `from_config_with_source` loads `- wasm:` relative to source root, preserves
  declared order after wikilinks: PASS.
- Instantiated once, reused across docs (`Mutex<Plugin>`, call_count==5 over 5
  docs): PASS.
- Real sample .wasm loaded + executed end-to-end (e2e test + my manual build):
  PASS.
- resolve_link hit -> URL, miss -> empty string, backed by SiteIndex via
  late-bound slot populated before plugin.call: PASS.
- Per-entry `config:` -> JSON -> plugin, changes behavior (marker=ref): PASS.
- Plugin warnings surface in TransformResult.warnings via apply: PASS.
- Loud load failures — missing file / corrupt bytes / missing `transform`
  export, each a separate `WasmLoad` Err (not panic, not silent): PASS.
- Feature-off `- wasm:` -> loud `WasmUnsupported`: PASS.
- Sandbox default-deny (no allowed_paths/hosts); fs-read probe denied and test
  asserts `sandbox-fs-denied` present AND `sandbox-fs-ALLOWED` absent: PASS.
- Opt-in: no `- wasm:` entry => no Plugin instantiated, byte-identical output:
  PASS (verified via control build).
- Invalid runtime plugin output -> loud warning, HTML unchanged (not corrupted):
  code path correct (no dedicated test since the sample plugin always emits valid
  JSON; not a listed separate-test criterion) — PASS with note.
- TDD: `## Log` shows genuine RED->GREEN for the two key fixes (resolve_link:
  forced `""` -> observed broken-span failure; config: dropped `with_config_key`
  -> observed unchanged `{{ref:foo}}`), with expected-vs-actual logged: PASS.

**DTC DOM baseline (788/790):** NOT run — DTC source is absent under
`websites/DataTalksClub/datatalksclub.github.io/` (only `mediumish/` present)
and there is no Jekyll cache, same situation as #600/grooming. Per orchestrator
guidance, not failing solely for absent source: the feature is opt-in and I
verified structurally (control build) that a site with no `extensions:` block
runs no wasm runtime and stays byte-identical, and DTC has no `extensions:`
block. A live recount with
`bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
must still be run before final done if/when the site + cache are available.
Perf check likewise deferred (no DTC); my fixture builds in ~0.002s Generation,
and no plugin is instantiated for DTC.

- VERDICT: **PASS** (with the standing DTC live-recount confirmation owed once
  the site is checked out; all code, tests, lint, fmt, feature-off build, loud
  failures, sandbox, opt-in, and real-wasm e2e execution verified independently).

### [PM] 2026-07-02 — Acceptance review
Reviewed diff (5 tracked files + 3 untracked): Cargo.toml (+9), Cargo.lock
(dep tree), src/extensions/mod.rs (+182), src/main.rs (call-site swap),
src/extensions/wasm.rs (new adapter, 416 lines, 9 tests),
integration_tests/tests/integration_wasm_extension.rs (e2e), and
tests/fixtures/wasm/ (sample.wasm 195357 B real `\0asm` module + sample-plugin
source/README). Read all source; did not rely solely on the QA report.

**Independent verification I ran myself:**
- e2e crate: `cargo-safe test -p integration-tests --test integration_wasm_extension`
  -> 1 passed. Test panics (not skips) on a missing binary/wasm.
- wasm unit/registry tests: 13 passed, 0 failed, 0 ignored (default features).
- Manual RELEASE-binary build (own fixture, not the committed test): a `- wasm:`
  site rendered `Good: <a href="/about.html">about</a> Bad:
  <span class="broken">ghost</span>` — the real .wasm was LOADED + EXECUTED,
  `resolve_link` returned the real page URL on a hit and the empty-string miss
  sentinel on a miss. This is the decisive "Done Means DONE" check: a real wasm
  runs, not just adapter code compiling.
- Opt-in control (no `extensions:` block): `{{link:about}}` left byte-literal,
  no transform, no runtime.
- Feature gate: `cargo-safe check --no-default-features` -> exit 0, clean (no
  extism/wasmtime errors); Cargo.toml confirms `extism` optional + `wasm`
  default-on feature. Feature-off `- wasm:` -> `WasmUnsupported` (QA-verified).

**Per-criterion verdicts:**
1. extism behind default-on `wasm`; `--no-default-features` skips it: PASS.
2. WasmExtension impls HtmlTransform, stored as `Box<dyn HtmlTransform>`: PASS.
3. `from_config_with_source` loads `- wasm:` rel. to source root, order after
   wikilinks preserved: PASS.
4. Instantiated once, reused (`Mutex<Plugin>`, call_count==5/5): PASS.
5. Real sample.wasm loaded + executed e2e: PASS (verified by me directly).
6. resolve_link hit->URL / miss->empty sentinel round-trip: PASS.
7. Per-entry `config:` -> JSON -> plugin, changes behavior (marker=ref): PASS.
8. Plugin warnings surface via Registry::apply: PASS.
9. Loud load failures — missing / corrupt / missing-`transform`-export, each a
   separate `WasmLoad` Err (not panic, not skip): PASS.
10. Feature-off `- wasm:` -> loud `WasmUnsupported`: PASS.
11. Sandbox default-deny (no allowed_paths/hosts, fs probe denied): PASS.
12. Opt-in: no `- wasm:` => no Plugin, byte-identical: PASS (control build).
13. DTC DOM 788/790 no-regression: DEFERRED — DTC source absent under
    `websites/` (only `mediumish/` present), no Jekyll cache; live recount not
    possible here (same as #600/grooming). ACCEPTED on the structural opt-in
    guarantee: DTC has no `extensions:` block => no wasm runtime instantiated =>
    output byte-identical. **USER ACTION owed:** run
    `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
    once the site + cache are checked out and confirm exactly 788/790.
14. Tests green both configs (4586 default / 4574 no-default per QA), clippy
    `-D warnings` clean both configs, `cargo fmt --check` clean: PASS.
15. TDD fail-then-pass logged (resolve_link forced `""`; config `with_config_key`
    dropped) with expected-vs-actual: PASS.

**QA note adjudication (not dropped):**
1. DTC live recount deferred — adjudicated at criterion 13 above; accepted on
   opt-in structural guarantee, USER ACTION note carried.
2. e2e test lives in `integration-tests` crate (needs `-p integration-tests`).
   CI ALREADY covers this: `.github/workflows/integration.yml` builds the
   release binary (lines 119-120: `cargo build --release`) and runs
   `cargo test -p integration-tests` (line 123, "Run ALL integration tests (no
   skips)"). No follow-up issue needed.

- Output verification: real .wasm loaded + executed via release binary; correct
  anchor href from SiteIndex on hit, broken span + warning on miss; opt-in
  byte-identical control confirmed.
- Results verified: real wasm execution present (manual + committed e2e).
- Acceptance criteria: 14 met; 1 (DTC live recount) deferred with structural
  justification + USER ACTION note (no silent drop).
- Follow-up issues created: none (CI already runs `-p integration-tests`).
- VERDICT: **ACCEPT** — engineer may commit. (Reviewer does NOT rename to
  `.done.md` or commit, per task instructions.)
