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
rustkyll with content transforms **without recompiling** it.

## Scope

### In scope

- A `WasmExtension` adapter that implements `HtmlTransform` by calling into a
  WASM module.
- Load `.wasm` modules from `_extensions/` in the site source.
- Use [extism](https://extism.org/) (or an equivalent WASM host) as the runtime.
- Provide a `resolve_link` **host function** so a WASM extension can query the
  `SiteIndex` (slug/title -> URL) the same way the compiled-in wikilinks
  extension does.
- Wire WASM extensions into the same `Registry` / declared-order model, so
  `_config.yml` `extensions:` entries can reference a WASM extension by name.
- Loud failure on a missing/invalid `.wasm` file or a host-function ABI mismatch
  (mirror #600's `ExtensionError` semantics — no silent default).

### Out of scope

- Sandboxing/capability policy beyond what the WASM host provides by default
  (revisit if needed).
- Distributing/packaging a marketplace of extensions.

## Acceptance Criteria

- [ ] A sample `.wasm` extension in `_extensions/` is loaded and appears in the
      `Registry` in declared order alongside compiled-in extensions.
- [ ] The WASM extension can call the `resolve_link` host function and get the
      same URL a compiled-in transform would for a known slug/title.
- [ ] An end-to-end fixture build applies a WASM transform to page HTML and the
      output reflects the transform.
- [ ] Missing/invalid `.wasm` or ABI mismatch fails loudly as a build error
      (not a silent skip or panic).
- [ ] Opt-in preserved: a site with no WASM extensions and no `_extensions/`
      dir behaves byte-identically to today.
- [ ] DTC DOM match count must not drop below the #600 baseline of **788/790**.
- [ ] `./scripts/cargo-safe test` green, clippy clean, `cargo fmt --check` clean.
- [ ] TDD: test-first, log fail-then-pass.

## Test Scenarios

- Load a fixture `.wasm` transform; assert it is registered and runs.
- `resolve_link` host fn returns the correct URL for a known target and `None`
  for a miss.
- Invalid/missing `.wasm` -> build error.
- No `_extensions/` dir -> empty WASM contribution, no-op.

## Dependencies

- #600 (extension framework + wikilinks) must be `.done.md` first.

## Notes

- Deferred from #600 per user discussion (compiled-in native extensions now,
  WASM as the future escape hatch behind the same trait). The #600
  `HtmlTransform` trait is already object-safe to support this.
