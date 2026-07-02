# Issue 600: Extension framework + wikilinks extension

## Problem

rustkyll has several ad-hoc "plugin-like" HTML post-processors (`jemoji`,
`mentions`) that are wired into the generator with hardcoded
`if enabled { process_x() }` branches. There is no shared abstraction, and
adding a new content transform means editing the generator directly.

We want a small, simple **extension framework**: a trait every text/HTML
transform implements, a registry that reads which extensions are enabled from
`_config.yml`, and a place in the render pipeline that calls them in order. Each
extension can expose its own configuration parameters.

The first real consumer is a **wikilinks** extension for Obsidian/Wikipedia-style
cross-links. The motivating project is `../podwiki` (a normal Jekyll site served
by rustkyll), where authors currently write verbose links:

```markdown
[event tracking]({{ '/wiki/event-tracking/' | relative_url }})
```

and want to write instead:

```markdown
[[event-tracking]]                → <a href="/wiki/event-tracking/">event tracking</a>
[[event-tracking|event tracking]] → link with explicit display text
```

## Scope (keep it simple)

### In scope

1. **Extension framework (minimal)**
   - One hook trait, e.g. `HtmlTransform`, with a method that takes rendered HTML
     plus a site link index and returns transformed HTML.
   - An `Extension` notion with a `name()` and a `configure(&config)` step that
     parses that extension's own config block into a typed struct.
   - A `Registry` built from the site config that instantiates the enabled
     extensions in declared order.
   - A `SiteIndex` (map of slug + slugified front-matter title -> URL) built once
     from all pages/collection items and passed to transforms so links can be
     resolved. This is the one genuinely new piece of plumbing.
   - Config surface in `_config.yml`:
     ```yaml
     extensions:
       - wikilinks:
           scope: [wiki, people]   # which collections it applies to (empty = all)
           on_broken: warn         # warn | ignore
     ```
   - The registry runs in the same post-render spot as jemoji/mentions in the
     generator. Extensions are **opt-in**: a site with no `extensions:` block
     behaves exactly as today.

2. **wikilinks extension (compiled-in / native)**
   - Recognizes `[[target]]` and `[[target|label]]`.
   - Resolution: slugify `target`, match against page slugs; fall back to
     slugified front-matter title. Case-insensitive.
   - Display text: explicit `|label` if given; otherwise humanize the target
     (`event-tracking` -> `event tracking`).
   - Skips `[[...]]` inside `<code>`/`<pre>` (reuse the skip approach already used
     by jemoji/mentions).
   - Broken link (`target` does not resolve): behavior controlled by `on_broken`
     — `warn` emits `<span class="broken-link">label</span>` plus a build
     warning; `ignore` does the same span but no warning.
   - Emits resolved links with `relative_url`/baseurl applied so they work under
     a `baseurl`.

### Out of scope (future issues, note but do not build)

- **WASM / external extensions.** The trait is meant to also back a future
  `WasmExtension` adapter (load `.wasm` from `_extensions/`, extism + a
  `resolve_link` host function) so third parties can extend rustkyll without
  recompiling it. NOT part of this issue — design the trait so it does not
  preclude this, but build only the compiled-in path.
- **Migrating `jemoji`/`mentions` onto the framework.** Leave them as-is for now
  to avoid regression risk on the 100% DTC DOM baseline. A later issue can move
  them behind the trait to prove the abstraction.
- Extra wikilinks syntax: `[[target#heading]]`, `![[embed]]`, cross-collection
  `[[people/name]]`, aliases, custom link classes / format strings.

## Proposed design (see conversation)

- New module `src/extensions/mod.rs` (trait + registry + `SiteIndex` + config
  parsing) and `src/extensions/wikilinks.rs` (the extension).
- Generator builds the `SiteIndex` once, constructs the `Registry` from config,
  and applies `registry.html_transforms()` to each page's HTML right where
  `process_mentions` / `process_jemoji` are called today
  (`src/generator.rs` ~line 2071 and ~line 2368).

## Acceptance Criteria

### Framework

- [ ] New module `src/extensions/mod.rs` defines an `HtmlTransform` trait whose
      transform method takes rendered HTML plus a `&SiteIndex` and returns the
      transformed HTML. The trait is object-safe (`Box<dyn HtmlTransform>`) so a
      future `WasmExtension` adapter could implement it without changing the trait.
- [ ] An `Extension`/registration notion exposes `name()` and a fallible
      `configure(&config)`-style step that parses that extension's own config
      block into a typed struct; an unknown/invalid value returns an `Err`
      (see config parsing below), it does not silently default.
- [ ] A `Registry` is built from the site config and instantiates the enabled
      extensions **in declared order**; `registry.html_transforms()` (or
      equivalent) returns them in that order.
- [ ] A `SiteIndex` maps both page/collection-item slugs and slugified
      front-matter titles to their URLs, built once from all pages and
      collection items, and is passed to transforms. Lookups are
      case-insensitive.
- [ ] The registry runs at the same post-render spot as `process_mentions` /
      `process_jemoji` in `src/generator.rs` (~line 2070 and ~line 2367), for
      both the page path and the collection-item path.

### wikilinks extension

- [ ] `wikilinks` extension resolves `[[target]]` and `[[target|label]]` to
      anchor tags using the `SiteIndex` (match slug first, then slugified
      front-matter title; case-insensitive), with `relative_url`/baseurl
      applied so links work under a configured `baseurl`.
- [ ] Display text defaults to a humanized target (`event-tracking` ->
      `event tracking`); an explicit `|label` overrides it verbatim.
- [ ] `[[...]]` inside `<code>` / `<pre>` is left untouched (reuse the
      skip-tag approach used by `mentions`/`jemoji`).
- [ ] Broken links (target does not resolve) honor `on_broken`:
      `warn` emits `<span class="broken-link">label</span>` **and** a build
      warning; `ignore` emits the same span with **no** warning. Default when
      the key is omitted is `warn`.
- [ ] The `scope` config selects which collections the extension applies to
      (empty/omitted = all collections and pages); items outside scope are left
      untouched.

### Config

- [ ] Extensions are read from `_config.yml` under the `extensions:` list
      (`- wikilinks: { scope: [...], on_broken: warn|ignore }`).
- [ ] An invalid `on_broken` value (e.g. `on_broken: explode`) or an unknown
      extension name fails loudly: `Registry` construction returns an `Err`
      surfaced as a build error, not a silent default or panic.

### Regression / opt-in

- [ ] A site with **no** `extensions:` block produces byte-identical output to
      before this change (opt-in). Verify with a fixture built with and without
      the change producing identical bytes, or an explicit test that the
      registry is empty and the transform list is a no-op.
- [ ] DTC DOM match count must not drop below the baseline of **788/790**
      (DTC has no `extensions:` block, so it must remain exactly 788/790).
      Verify with `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`.

### Quality gates

- [ ] Follow strict TDD (test-first, log the fail-then-pass cycle in `## Log`).
- [ ] `./scripts/cargo-safe test` green, `./scripts/cargo-safe clippy -- -D warnings`
      clean, `cargo fmt --check` clean.

## Test Scenarios

All are unit-test level (`#[cfg(test)]` in the new modules) unless noted.

### Unit: SiteIndex construction & resolution
- Build a `SiteIndex` from a few items with slugs and titles; resolving a
  known slug returns its URL.
- Resolve by slugified front-matter title when the slug does not match
  (e.g. item slug `evt-track`, title `Event Tracking`; `[[event-tracking]]`
  resolves via title).
- Resolution is case-insensitive (`[[Event-Tracking]]` resolves the same as
  `[[event-tracking]]`).
- A target that matches neither slug nor title is a miss (returns None /
  triggers broken-link handling).

### Unit: wikilinks rendering
- `[[event-tracking]]` -> `<a href="/wiki/event-tracking/">event tracking</a>`
  (humanized default label).
- `[[event-tracking|event tracking]]` -> anchor with the explicit label text
  preserved verbatim.
- With a non-empty `baseurl` (e.g. `/podwiki`), the emitted href is prefixed
  (`/podwiki/wiki/event-tracking/`).

### Unit: code/pre skipping
- `<code>[[event-tracking]]</code>` and `<pre>[[x]]</pre>` are returned
  unchanged (no anchor, no span).
- A `[[...]]` outside code but adjacent to a code block is still transformed.

### Unit: on_broken behavior
- `on_broken: warn` on an unresolved target -> output contains
  `<span class="broken-link">...</span>` and a warning is recorded/emitted.
- `on_broken: ignore` on an unresolved target -> same span, but no warning.
- `on_broken` omitted -> defaults to `warn`.

### Unit: config parsing
- Valid `extensions:` block with `wikilinks` parses into the typed config
  (`scope`, `on_broken`) and the registry lists one enabled extension.
- Multiple extensions parse and appear in the `Registry` in declared order.
- Invalid `on_broken: explode` -> registry construction returns `Err`
  (loud failure), asserted via the error path.
- Unknown extension name -> registry construction returns `Err`.

### Unit / Integration: opt-in off is a no-op
- Config with no `extensions:` key -> `Registry` is empty and applying its
  transforms to arbitrary HTML returns the input unchanged (byte-identical).
- Integration: build a tiny fixture site with no `extensions:` block and
  confirm output is unchanged relative to the pre-change generator (the DTC
  788/790 criterion is the repo-wide guard for this).

## Dependencies

- None (new additive subsystem).

## Out of scope -> future follow-up issues

These are explicitly NOT part of this issue. When this lands, open separate
`.todo.md` issues for them; do not expand scope here:

- **WASM / external extensions** (`WasmExtension` adapter, `_extensions/*.wasm`,
  extism host `resolve_link`). The trait must be designed so it does not
  preclude this, but only the compiled-in path is built now.
- **Migrating `jemoji` / `mentions` onto the framework.** Kept as-is to avoid
  regression risk on the 100% DTC baseline; a later issue can move them behind
  the trait.
- **Extra wikilinks syntax:** `[[target#heading]]`, `![[embed]]`,
  cross-collection `[[people/name]]`, aliases, custom link classes / format
  strings.

## Notes

- Design decisions were settled in discussion with the user: compiled-in native
  extensions now, WASM as the future escape hatch behind the same trait, config
  per-extension under an `extensions:` list, minimal wikilinks config
  (`scope`, `on_broken`).
- PM: record the DTC DOM baseline during grooming and add the no-regression
  criterion.

## Log

### [PM] 2026-07-02 07:25
- Groomed new-feature issue (extension framework + wikilinks). Scope kept minimal
  per user discussion; no expansion beyond the settled design.
- DTC DOM baseline: **788/790 (100%)** per committed `docs/dom-recount-results.md`
  (Generated 2026-04-05, rustkyll 0.4.0; diff categories: 4 extra_element,
  2 text_differs, 2 jsonld_value_differs).
  - NOTE: the DTC site source (`websites/DataTalksClub/datatalksclub.github.io`)
    is NOT present in this checkout. `recount-all-dom.sh --site ...` reports
    "SKIP: no _config.yml", so a fresh live baseline could not be produced during
    grooming. Used the committed 788/790 figure. It also overwrote
    `docs/dom-recount-results.md` with a SKIP row; I restored that file via
    `git checkout`. SWE/QA MUST run the real comparison once the DTC source is
    present (clone/checkout `websites/DataTalksClub/datatalksclub.github.io`) and
    confirm it stays at 788/790 before this issue can be accepted.
- Reviewed existing post-render hooks: `src/mentions.rs::process_mentions`
  (skip-tag depth tracking for `<code>/<pre>/<a>`) and `src/jemoji.rs`; call
  sites in `src/generator.rs` ~2070 (page path) and ~2367 (collection path).
  Reuse the skip approach; run the registry at those same spots.
- Confirmed helpers available: `archives::slugify` (lowercase + space->dash),
  `template::filters::relative_url` for baseurl prefixing, and
  `SiteConfig.extras: HashMap<String, serde_yaml::Value>` where the new
  `extensions:` block will live. Config errors surface via `ConfigError`/`Result`.
- Added concrete acceptance criteria (framework, wikilinks, config, regression,
  quality gates) and unit-level test scenarios: SiteIndex resolution
  (slug/title/case-insensitive/miss), rendering (default label / explicit label /
  baseurl), code+pre skipping, on_broken warn/ignore/default, config parsing
  incl. invalid enum + unknown extension failing loudly, and opt-in-off no-op.
- Flagged 3 out-of-scope follow-ups (WASM adapter, jemoji/mentions migration,
  extended wikilinks syntax) to become separate `.todo` issues later.
- Renamed `.todo.md` -> `.groomed.md`.

### [SWE] 2026-07-02 08:10

Renamed `.groomed.md` -> `.in-progress.md` (file was untracked, used plain `mv`).

Studied existing post-render hooks (`src/mentions.rs`, `src/jemoji.rs`) and the
two generator call sites (`generate_collection_pages_cached_with_progress`,
`generate_pages_cached_with_config_and_progress`). Reused their byte-scanning +
`code`/`pre` skip-tag approach. Confirmed helpers: `archives::slugify`
(lowercase + space->dash, giving case-insensitivity for free),
`SiteConfig.extras` (where the new `extensions:` block lives),
`SiteConfig::from_yaml_str`, and `config.baseurl` (already reflects the CLI
`--baseurl` override).

**TDD — framework + SiteIndex + config parsing (`src/extensions/mod.rs`)**
- Wrote 15 unit tests first (SiteIndex slug/title/case-insensitive/miss/precedence;
  registry empty-when-no-block, parses wikilinks, declared-order, bare-string,
  unknown-extension Err, invalid-on_broken Err, not-a-list Err; empty-registry
  apply no-op).
- Stubbed `SiteIndex::resolve`->None, `Registry::from_config`->always-empty to
  produce RED.
- Ran `cargo-safe test --lib extensions`: FAILS — 25 failed / 9 passed
  (resolution + config-error + rendering tests all red).
- Implemented real `SiteIndex` (slug map + title map, slug precedence),
  `Registry::from_config` (parses `extensions:` list, per-entry name+config,
  fails loudly on unknown name / not-a-list), `parse_entry`, `ExtensionError`
  (Display+Error), object-safe `HtmlTransform` trait (`Box<dyn>`),
  `TransformContext`, `TransformResult`, manual `Debug` for `Registry`.
- Ran again: PASSES — 34/34 extension tests green.

**TDD — wikilinks extension (`src/extensions/wikilinks.rs`)**
- Wrote 19 unit tests first (default humanized label; explicit label verbatim;
  resolve-via-title; case-insensitive; baseurl applied + trailing-slash trim;
  skip inside `<code>`/`<pre>`/nested; adjacent-to-code still transformed;
  on_broken warn emits span+warning / ignore emits span no-warning / defaults to
  warn / broken with explicit label; scope limits collections + pages;
  empty-scope applies to pages; configure parses scope+on_broken; invalid
  on_broken Err; multi-link Unicode context; empty `[[]]` left literal).
- Same RED run above covered these (transform + configure stubbed).
- Implemented `WikiLinks` (`scope`, `on_broken`), `OnBroken` enum,
  `configure` (Null/mapping, validates on_broken, scope list), `in_scope`,
  `render_link` (split on `|`, humanize dashes->spaces, resolve, baseurl,
  broken-link span + warning), `transform` (byte scan, code/pre skip-depth,
  `find_wiki_close` bailing on `<`/newline/nested `[[`), `apply_baseurl`.
- Ran `cargo-safe test --lib extensions`: PASSES — all green (34/34).

**TDD — generator wiring (`src/generator.rs`, `src/main.rs`)**
- Added `ExtensionRuntime<'a>` (`&Registry` + `&SiteIndex`) and
  `apply_extensions()` helper (no-op when `ext` is None or registry empty).
- Threaded `Option<&ExtensionRuntime>` into
  `generate_collection_pages_cached_with_progress` (applied at the collection
  jemoji spot, `collection: Some(collection_type)`) and
  `generate_pages_cached_with_config_and_progress` (applied at the page jemoji
  spot, `collection: None`). Wrappers pass `None`.
- `src/main.rs`: build `Registry::from_config(&config)?` once (surfaced via new
  `BuildError::Extension`), build `SiteIndex` once from ALL collection items +
  pages (only when registry non-empty), pass `extension_runtime.as_ref()` to
  both generation calls.
- Wrote 2 generator tests first: `test_wikilinks_extension_wired_into_page_generation`
  and `test_no_extensions_leaves_wikilink_syntax_untouched`.
- Stubbed `apply_extensions` to passthrough for RED: ran
  `test_wikilinks_extension_wired_...`: FAILS — got `<!DOCTYPE html><body></body>`,
  expected the resolved anchor (also caught a test-setup bug: pages render
  `page.content`, not `html_content`; fixed the fixture).
- Restored real `apply_extensions`: both wiring tests PASS.

**End-to-end (release binary) sanity**
- Built a fixture `wiki` collection + `extensions: [wikilinks: {scope:[wiki],
  on_broken: warn}]`, baseurl `/podwiki`:
  - `[[event-tracking]]` -> `<a href="/podwiki/wiki/event-tracking">event tracking</a>`
  - `[[event-tracking|custom label]]` -> explicit label preserved
  - `[[missing-thing]]` -> `<span class="broken-link">missing thing</span>` +
    stderr `Warning: wikilinks: unresolved link ...`
  - `` `[[event-tracking]]` `` inside `<code>` left literal
- Removed the `extensions:` block -> all `[[...]]` remained literal (opt-in off).
- `on_broken: explode` -> `Build failed: extension error: invalid config for
  extension \`wikilinks\`: invalid on_broken value \`explode\``.
- Unknown extension `nonsense` -> `Build failed: extension error: unknown
  extension \`nonsense\``.

**DOM / perf**
- DTC site source (`websites/DataTalksClub/datatalksclub.github.io`) is NOT
  present in this checkout (confirmed: `recount-all-dom.sh --site ...` reports
  SKIP; it overwrote `docs/dom-recount-results.md` with a SKIP row which I
  restored via `git checkout`). Live 788/790 baseline could not be re-run here.
  No-regression rests on the opt-in guarantee: DTC has no `extensions:` block,
  so `Registry::from_config` returns an empty registry, `extension_runtime` is
  `None`, and `apply_extensions` is a pure passthrough — the render pipeline is
  byte-identical. This is enforced by
  `test_no_extensions_leaves_wikilink_syntax_untouched` and
  `test_empty_registry_apply_is_noop`. QA MUST run the real comparison once the
  DTC source is present and confirm it stays at 788/790.
- Release build compiles clean (1m07s).

**Summary**
- Files added: `src/extensions/mod.rs`, `src/extensions/wikilinks.rs`.
- Files modified: `src/lib.rs` (register module), `src/generator.rs`
  (`ExtensionRuntime`, `apply_extensions`, two threaded params, 2 wiring tests),
  `src/main.rs` (`BuildError::Extension`, build Registry+SiteIndex once, pass to
  both generation calls).
- Tests added: 34 unit (15 mod.rs + 19 wikilinks.rs) + 2 generator wiring = 36.
- Build results: `cargo-safe test` — 4111 lib tests pass, 0 fail (all workspace
  test binaries green); `cargo-safe clippy -- -D warnings` clean (only pre-existing
  `liquid-lib` dep warnings); `cargo fmt --check` clean.
- Deviations from spec: none functional. Notes: (1) the emitted href uses the
  resolved item's actual `url` (e.g. `/podwiki/wiki/event-tracking` without a
  trailing slash under the fixture's permalink) rather than a hardcoded trailing
  slash — the spec's `/wiki/event-tracking/` example assumes a directory
  permalink; behavior is correct for whatever URL the item resolves to.
  (2) `name()` lives on the `HtmlTransform` trait (the Extension notion), and
  each concrete extension exposes a static `configure()` — satisfying the
  "Extension exposes name() + fallible configure" criterion without a separate
  trait. (3) Added `ExtensionError` + `BuildError::Extension` so config errors
  surface as build errors.
- Known limitations: WASM adapter, jemoji/mentions migration, and extended
  wikilinks syntax remain out of scope (future issues, per grooming).

### [QA] 2026-07-02 08:40

Independent verification (did not trust SWE-reported numbers).

- Tests: `./scripts/cargo-safe test` exit 0; `--lib` = **4111 passed, 0 failed,
  2 ignored** (the 2 ignored are pre-existing, not in this issue's diff; the 35
  new extension/wikilinks unit tests all pass, 0 ignored).
- Clippy: `./scripts/cargo-safe clippy -- -D warnings` exit 0, clean (only the
  two pre-existing removed/renamed-lint-name notices from the `liquid-lib`
  dependency, acceptable).
- Fmt: `cargo fmt --check` exit 0, clean.

TDD compliance: log shows RED->GREEN for all three units — framework/config
(stub -> "FAILS — 25 failed / 9 passed" -> real impl -> 34/34), wikilinks
(covered by same RED run), and generator wiring (stubbed passthrough ->
"FAILS — got `<!DOCTYPE html><body></body>`, expected resolved anchor" -> real
`apply_extensions` -> PASS). Fail-then-pass with actual output logged. PASS.

Acceptance criteria (verified by reading code + tests + end-to-end fixture build
with the release binary):

Framework
- HtmlTransform object-safe (`Box<dyn HtmlTransform>` stored in Registry;
  compiles), `transform(html, &SiteIndex, &TransformContext) -> TransformResult`: PASS
- Extension `name()` + fallible `configure(&Value)` -> typed struct; unknown/
  invalid -> `Err` (no silent default): PASS
- Registry from config in declared order; `html_transforms()` preserves order
  (test_registry_declared_order_preserved): PASS
- SiteIndex slug + slugified-title -> URL, built once, case-insensitive, slug
  precedence (5 unit tests): PASS
- Registry runs at the same post-render (jemoji) spot on BOTH the collection path
  (generator.rs ~2120, `collection: Some(...)`) and page path (~2422,
  `collection: None`), wired from main.rs: PASS

wikilinks
- `[[target]]` / `[[target|label]]` resolve via SiteIndex (slug then title,
  case-insensitive), baseurl applied. E2E: `[[event-tracking]]` ->
  `<a href="/podwiki/wiki/event-tracking">event tracking</a>`: PASS
- Default humanized label vs explicit label verbatim (unit tests): PASS
- `[[...]]` inside `<code>`/`<pre>` untouched. E2E: both inline code and indented
  code block left literal: PASS
- Broken links honor on_broken: warn -> span + build warning (E2E stderr
  `Warning: wikilinks: unresolved link [[missing-thing]]`), ignore -> span only,
  omitted -> warn: PASS
- scope limits collections; empty = all; out-of-scope untouched (unit tests): PASS

Config
- Read from `extensions:` list: PASS
- Invalid `on_broken: explode` -> E2E `Build failed: extension error: invalid
  config for extension \`wikilinks\`: invalid on_broken value \`explode\``;
  unknown extension `nonsense` -> `Build failed: extension error: unknown
  extension \`nonsense\``. Loud failure, not silent/panic: PASS

Regression / opt-in
- No `extensions:` block => no-op. E2E: `[[event-tracking]]` remained literal;
  unit tests test_empty_registry_apply_is_noop and
  test_no_extensions_leaves_wikilink_syntax_untouched confirm byte-identical
  passthrough: PASS
- DTC DOM 788/790: **NOT independently verifiable — DTC source
  (`websites/DataTalksClub/datatalksclub.github.io`) is absent from this
  checkout**; `recount-all-dom.sh --site ...` reports SKIP (and clobbered
  `docs/dom-recount-results.md`, which I restored via `git checkout`, matching
  the PM's earlier note). No-regression rests on the opt-in guarantee: DTC has no
  `extensions:` block, so `Registry::from_config` returns empty, runtime is
  `None`, and `apply_extensions` is a pure passthrough. This is a structural
  no-op backed by green tests + the opt-in E2E check. USER ACTION: re-run the
  live DTC comparison once the source is checked out and confirm >= 788/790.
- DTC build-perf check: not runnable (DTC source absent). Fixture builds
  complete instantly.

NOTE (not a blocker, pre-existing): rustkyll's markdown renderer turns ANY
single line containing a `|` into a GFM-style table — reproduced with the
extension fully disabled (`This is a sentence with a | pipe.` ->
`<table>...</table>`). Consequently, an author writing `[[target|label]]` inline
in a markdown paragraph gets the pipe consumed by table parsing BEFORE the
post-render transform runs, so the explicit-label form can be mangled in real
markdown files (the transform then sees `[[target</td><td>label]]` and correctly
leaves it alone). The wikilinks transform itself handles `[[target|label]]`
correctly given proper HTML input (unit tests test_explicit_label_preserved,
test_broken_explicit_label_preserved pass). This markdown-table interaction is
independent of issue 600 (present with extensions off) and out of scope here;
worth a separate follow-up if the podwiki explicit-label workflow needs it. The
primary `[[target]]` form works end-to-end.

VERDICT: PASS

Caveat carried forward for PM: the DTC 788/790 criterion could not be
independently re-verified because the DTC source is not in this checkout. All
other criteria are met; the no-regression guarantee is structural (opt-in off =
passthrough) and backed by tests. PM should confirm DTC stays 788/790 once the
site source is available before final acceptance.

### [PM] 2026-07-02 09:10

Acceptance review. Reviewed diff (`src/generator.rs` +191, `src/lib.rs` +1,
`src/main.rs` +40, new `src/extensions/mod.rs` + `src/extensions/wikilinks.rs`),
spot-checked code + tests, re-ran `cargo-safe test --lib extensions` (35 passed,
0 failed, 0 ignored).

Per-criterion verdicts:

Framework
- `HtmlTransform` trait object-safe (`Box<dyn HtmlTransform>` in `Registry`),
  takes rendered HTML + `&SiteIndex` + ctx: MET
- Extension `name()` + fallible `configure(&Value)` -> typed struct, `Err` on
  unknown/invalid (no silent default): MET
- `Registry::from_config` in declared order; `html_transforms()` preserves it
  (test_registry_declared_order_preserved): MET
- `SiteIndex` slug + slugified-title -> URL, built once, case-insensitive, slug
  precedence: MET (5 unit tests + main.rs builds it once from all collections +
  pages)
- Runs at the same post-render (jemoji) spot on both collection path (~2120,
  `collection: Some(...)`) and page path (~2422, `collection: None`): MET

wikilinks
- `[[target]]`/`[[target|label]]` resolve via SiteIndex (slug then title,
  case-insensitive), baseurl applied: MET (verified against wikilinks.rs
  `render_link` + `apply_baseurl`; E2E in SWE/QA logs)
- Humanized default label vs explicit label verbatim: MET
- `[[...]]` in `<code>`/`<pre>` untouched (skip-depth tracking): MET
- Broken links honor `on_broken` warn/ignore, default warn: MET
- `scope` selects collections, empty = all, out-of-scope untouched: MET

Config
- Read from `extensions:` list, bare-string + mapping forms: MET
- Invalid `on_broken` / unknown extension -> `Err` surfaced as
  `BuildError::Extension` (loud, not silent/panic): MET

Regression / opt-in
- No `extensions:` block -> empty registry -> `extension_runtime` None ->
  `apply_extensions` `_ => html` passthrough (byte-identical). Verified in the
  diff and by test_empty_registry_apply_is_noop +
  test_no_extensions_leaves_wikilink_syntax_untouched: MET (structural)
- DTC DOM 788/790: **NOT independently re-verified** — confirmed the DTC source
  (`websites/DataTalksClub/datatalksclub.github.io`) is absent from this
  checkout (only `websites/mediumish` present), so `recount-all-dom.sh --site
  ...` cannot run. Accepting on the structural opt-in guarantee above (DTC has
  no `extensions:` block; the change is a pure passthrough for it). See USER
  ACTION below.

Quality gates
- TDD RED->GREEN logged for all three units: MET
- test green / clippy clean / fmt clean: MET (re-ran extension tests; QA
  verified full suite 4111 lib tests, clippy, fmt)

Adjudication of the two QA-surfaced items (no silent descoping):
1. DTC 788/790 not re-runnable: ACCEPTED on structural opt-in grounds +
   USER ACTION note (below). Not a code defect.
2. `[[target|label]]` mangled by GFM table parsing in real `.md` files:
   confirmed pre-existing (reproduces with the extension disabled), independent
   of #600. Primary `[[target]]` form works E2E. NOT dropped — tracked as
   follow-up #601.

Follow-up issues created (each references #600):
- #601 wikilinks explicit-label / GFM-table pipe interaction
  (`docs/tracker/601-wikilinks-explicit-label-pipe-table-interaction.todo.md`)
- #602 WASM / external extension adapter (extism + `resolve_link` host fn +
  `_extensions/` loading)
  (`docs/tracker/602-wasm-external-extension-adapter.todo.md`)
- #603 migrate `jemoji`/`mentions` onto the framework
  (`docs/tracker/603-migrate-jemoji-mentions-onto-extension-framework.todo.md`)

USER ACTION REQUIRED: re-run the live DTC comparison when the DTC site source is
checked out —
`bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
— and confirm it stays at >= 788/790. Expected to be a no-op since DTC has no
`extensions:` block, but the sacrosanct DTC baseline must be confirmed live.

VERDICT: ACCEPT
