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
