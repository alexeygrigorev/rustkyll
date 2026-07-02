# Issue 603: migrate `jemoji` / `mentions` onto the extension framework

Follow-up to #600 (extension framework + wikilinks). Explicitly deferred as
out-of-scope in #600 to avoid regression risk on the DTC DOM baseline; tracked
here.

## Problem

rustkyll's `jemoji` and `mentions` HTML post-processors are currently wired into
the generator with hardcoded `if enabled { process_x() }` branches
(`src/jemoji.rs`, `src/mentions.rs`, call sites in `src/generator.rs`). #600
introduced a proper `HtmlTransform` trait + `Registry`, but intentionally left
`jemoji`/`mentions` as-is to keep the 100% DTC DOM baseline safe.

Moving them behind the trait proves the abstraction is sufficient for the
existing transforms and removes the ad-hoc branches.

## Chosen activation approach (DECIDED — do not guess)

**Keep the existing `plugins:` / `gems:` activation trigger. Do NOT require any
site to add an `extensions:` block.** `jemoji` and `mentions` become
`HtmlTransform` impls that the `Registry` **auto-activates** from the same
config gospel it uses today (`has_jemoji_plugin` / `has_mentions_plugin`).

Rationale: DTC (and any other real site) activates these via `plugins:`/`gems:`
with no `extensions:` block. Forcing them into the `extensions:` block would be
a silent site-config change and would break every site that relies on the
current auto-activation. The recommended-in-the-brief "latter" option is
therefore mandatory here.

Concretely:

- Add two new `HtmlTransform` impls (recommended location: `src/extensions/`,
  e.g. `src/extensions/jemoji.rs` and `src/extensions/mentions.rs`, or keep the
  impls in `src/jemoji.rs`/`src/mentions.rs` — SWE's choice). Each impl **must
  delegate to the existing `process_jemoji` / `process_mentions` functions** so
  the byte-for-byte replacement logic is reused unchanged. Do not reimplement
  the skip-tag / code-pre / email / team-mention logic.
- `Registry::from_config` gains auto-activation: when `has_mentions_plugin(cfg)`
  is true it prepends a `Mentions` transform; when `has_jemoji_plugin(cfg)` is
  true it prepends a `Jemoji` transform — **in that order, BEFORE any
  `extensions:`-block transforms** (so registry order is
  `[Mentions, Jemoji, <extensions: entries e.g. wikilinks>]`).
- The `Mentions` transform captures its own `base_url` from
  `mentions_base_url(cfg)` at construction time. It must NOT use
  `TransformContext::baseurl` (that is the site `baseurl` used by wikilinks —
  a different value).
- `Jemoji` and `Mentions` ignore `SiteIndex` and `TransformContext.collection`
  (they apply to all documents, as today).
- Remove the two inline `if enabled { process_x() }` blocks in
  `src/generator.rs` (~lines 2111-2121 in `generate_collection_pages_*` and
  ~lines 2413-2423 in `generate_pages_*`). After removal, the single
  `apply_extensions(...)` call at each site is the only place these transforms
  run. Also remove the now-unused `mentions_base_url` / `jemoji_enabled` locals
  computed at ~line 1912 and ~line 2303.

### Ordering (MUST be preserved exactly)

Today both call sites run: **mentions → jemoji → wikilinks(registry)**. The
auto-prepend order above (`[Mentions, Jemoji, wikilinks...]`) reproduces this
exactly through `Registry::apply`, which threads HTML through transforms in
declared order. Document this ordering in a code comment.

### Wiring consequence (regression trap — address explicitly)

Currently `mentions`/`jemoji` run inline **independently of the `ext`
parameter**. After this change they run ONLY via the registry / `ext` path, so:

- `src/main.rs` (the only real build entry) already passes
  `extension_runtime.as_ref()` to both generator functions, and builds
  `extension_runtime = Some` whenever the registry is non-empty (line ~857).
  Because `Registry::from_config` now auto-adds jemoji/mentions, DTC's registry
  becomes non-empty and the runtime becomes `Some` → transforms run. Verify this
  path.
- The convenience wrappers that pass `ext: None` (e.g. `generate_pages`,
  `generate_collection_pages`, `generate_pages_cached_with_config`) will **no
  longer** run jemoji/mentions. This is acceptable ONLY because they are not
  used by any real build path. **Verify no production build path passes
  `ext: None` while a jemoji/mentions plugin is configured.** Update any test
  that relied on the old inline behavior to build a `Registry` and pass an
  `ExtensionRuntime` instead.
- The `SiteIndex` in `main.rs` is built only when the registry is non-empty; it
  will now also be built for jemoji/mentions-only sites. This is a harmless
  no-op cost (jemoji/mentions ignore the index) and does NOT change output.
  Optionally (not required) gate index construction on whether an
  index-consuming transform is present, to preserve the "no cost on common
  path" property.

## Scope

- Reimplement `jemoji` and `mentions` as `HtmlTransform` extensions using the
  #600 framework, delegating to the existing `process_jemoji` /
  `process_mentions` functions (reuse their skip-tag / code-pre / email /
  team-mention logic verbatim).
- Auto-activate them from `plugins:`/`gems:` via `Registry::from_config`,
  prepended before `extensions:`-block transforms in the fixed order
  `Mentions → Jemoji`.
- Remove the hardcoded generator branches once the transforms run via the
  registry.
- **Out of scope:** changing emoji tables, mention parsing rules, activation
  keys, or any site's `_config.yml`. No new user-facing config.

## Acceptance Criteria

- [ ] `jemoji` and `mentions` run via the `Registry` / `HtmlTransform` path;
      the old hardcoded `if enabled` branches in `src/generator.rs` (both call
      sites) are removed, along with the now-unused `mentions_base_url` /
      `jemoji_enabled` locals.
- [ ] Activation is unchanged: a site listing `jemoji` / `jekyll-mentions` under
      `plugins:` or `gems:` gets those transforms with **no `extensions:` block
      required**. A test asserts `Registry::from_config` on a config with only
      `plugins: [jemoji, jekyll-mentions]` (and no `extensions:` block) yields a
      non-empty registry containing both transforms.
- [ ] Registry order is exactly `[mentions, jemoji, <extensions: entries>]`. A
      test builds a config with `plugins: [jemoji, jekyll-mentions]` **and** an
      `extensions: [wikilinks]` block and asserts `html_transforms()` names are
      `["mentions", "jemoji", "wikilinks"]` in that order.
- [ ] The `Mentions` transform honors a custom `jekyll-mentions.base_url`
      (e.g. `https://gitlab.com`) via config, NOT `TransformContext.baseurl`.
      A test asserts a `@user` link uses the configured base_url while
      `TransformContext.baseurl` is set to a different value.
- [ ] **Byte-identical output** for DTC and all existing sites — the core
      constraint. No emoji/mention behavior change. A round-trip test proves
      `Registry::apply` (with mentions+jemoji auto-activated) on a fixture
      containing an emoji, a mention, a code block, an email, and a team mention
      produces output **identical** to running the old
      `process_mentions` then `process_jemoji` pipeline on the same input.
- [ ] All existing `jemoji`/`mentions` unit tests still pass unchanged (the
      `process_*` functions are reused, not rewritten).
- [ ] DTC DOM match count stays at **exactly 788/790** (the #600 baseline,
      committed in `docs/dom-recount-results.md`). Verify with
      `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`.
      **DTC DOM must not drop below 788/790** — unconditional.
- [ ] Run `bash scripts/recount-all-dom.sh` across ALL sites present in the
      checkout and confirm no regression on any site.
- [ ] `./scripts/cargo-safe test` green, `./scripts/cargo-safe clippy -- -D warnings`
      clean, `cargo fmt --check` clean. No dead-code warnings for removed
      helpers (delete or `#[cfg(test)]`-scope anything now unused).
- [ ] TDD: test-first, log fail-then-pass in the issue Log.

### DTC source-absent caveat (USER ACTION required)

DTC source is **not present** in this checkout (`websites/` contains only
`mediumish`, which does not use jemoji/mentions). The committed
`docs/dom-recount-results.md` records DTC at **788/790**, but that number
cannot be re-derived live here. Therefore:

- QA must rely on: (a) the existing jemoji/mentions unit tests, (b) the new
  registry-ordering + auto-activation tests, and (c) the round-trip
  byte-identical test above, which together prove the migrated path produces
  output identical to the old inline path.
- **USER ACTION:** before final sign-off, re-run
  `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
  on a checkout that has the DTC source and confirm it stays at **≥ 788/790**.
  The issue may be ACCEPTED on the strength of the identical-output proof, but
  the PM log must record this pending live verification as a USER ACTION.

## Test Scenarios

### Unit: auto-activation from `plugins:`/`gems:`
- Config with `plugins: [jemoji]` (no `extensions:`) → registry non-empty,
  contains a `jemoji` transform.
- Config with `gems: [jekyll-mentions]` → registry contains a `mentions`
  transform.
- Config with neither plugin and no `extensions:` block → empty registry
  (byte-identical no-op path preserved).

### Unit: ordering
- Config with `plugins: [jemoji, jekyll-mentions]` + `extensions: [wikilinks]`
  → `html_transforms()` names == `["mentions", "jemoji", "wikilinks"]`.
- Config with only `plugins: [jekyll-mentions, jemoji]` → names ==
  `["mentions", "jemoji"]` (order independent of plugin-list order).

### Unit: mentions base_url
- Custom `jekyll-mentions: { base_url: https://gitlab.com }` → `@user` resolves
  to `https://gitlab.com/user` even when `TransformContext.baseurl` is `/blog`.

### Integration: byte-identical round-trip
- Fixture HTML containing `:heart:`, `@alice`, `<code>:heart: @bob</code>`,
  `user@example.com`, and `@jekyll/core`. Assert
  `Registry::apply(fixture)` (mentions+jemoji auto-activated) ==
  `process_jemoji(process_mentions(fixture, base_url))`.

### Integration: generator end-to-end
- Build a minimal site (via `generate_collection_pages_*` and
  `generate_pages_*`) with `plugins: [jemoji, jekyll-mentions]` and an
  `ExtensionRuntime` built from that config; assert the written HTML contains
  the converted emoji `<img class="emoji">` and `<a ... class="user-mention">`,
  and that a `<code>` block is left untouched — matching current output.

### Regression: existing suites
- All existing tests in `src/jemoji.rs` and `src/mentions.rs` pass unchanged.
- Existing `src/extensions/mod.rs` registry tests (empty registry no-op,
  wikilinks parsing, unknown-extension error) still pass.

## Dependencies

- #600 (extension framework + wikilinks) — **DONE** (`600-...done.md`).

## Notes

- Deferred from #600 per grooming ("Kept as-is to avoid regression risk on the
  100% DTC baseline; a later issue can move them behind the trait").
- Call sites for reference (as of grooming): inline branches at
  `src/generator.rs` ~2111-2121 (collections) and ~2413-2423 (pages); the
  single shared entry point is `apply_extensions` at ~line 44; runtime built in
  `src/main.rs` ~line 835-864.
- `apply_jemoji_if_enabled` / `apply_mentions_if_enabled` are used only by their
  own module tests — safe to keep, repurpose, or delete as the SWE sees fit.

## Log

### [PM] 2026-07-02 — Grooming
- Investigated current wiring: inline `if enabled { process_x() }` branches at
  `src/generator.rs` ~2111-2121 (collections) and ~2413-2423 (pages), each
  followed by the shared `apply_extensions(...)` call (~line 44). Confirmed
  today's run order is **mentions → jemoji → wikilinks(registry)** at both sites.
- Confirmed `Registry::from_config` (`src/extensions/mod.rs`) currently reads
  only the `extensions:` block; `src/main.rs` (~835-864) builds the runtime as
  `Some` iff the registry is non-empty and passes it to both generator
  functions.
- **Decision:** keep the `plugins:`/`gems:` activation trigger (no site config
  change); implement jemoji/mentions as `HtmlTransform` impls that
  `Registry::from_config` auto-prepends (Mentions, then Jemoji) before
  `extensions:`-block transforms. This is the brief's recommended "latter"
  option and the only one that preserves DTC's current activation (DTC has no
  `extensions:` block — verified via #600 done doc and this checkout).
- Flagged the regression trap: after removing the inline branches, jemoji/
  mentions run ONLY via the `ext` path; added acceptance criteria to verify the
  real build (`main.rs`) always passes a `Some` runtime and that no production
  path passes `ext: None` with the plugin configured.
- DOM baseline: DTC source is **absent** from this checkout (`websites/` has
  only `mediumish`, which uses neither plugin). Committed
  `docs/dom-recount-results.md` records DTC **788/790**; set as the mandatory,
  non-regressible baseline. Because live DTC recount is impossible here, added
  a byte-identical round-trip acceptance criterion and flagged a USER ACTION to
  re-run `recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io` on a
  DTC-present checkout before final sign-off.
- Added concrete acceptance criteria + unit/integration/regression test
  scenarios (auto-activation, ordering, custom base_url, byte-identical
  round-trip, generator end-to-end). Renamed `.todo.md` → `.groomed.md`.
