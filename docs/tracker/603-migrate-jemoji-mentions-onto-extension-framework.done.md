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

### [SWE] 2026-07-02

Zero-behavior-change refactor: migrated `jemoji`/`mentions` onto the #600
`HtmlTransform`/`Registry` framework, keeping the existing `plugins:`/`gems:`
activation trigger (no `extensions:` block required).

**Fix 1: `Mentions` / `Jemoji` HtmlTransform impls (delegating, not reimplementing)**
- Created `src/extensions/mentions.rs` (`Mentions`, name `"mentions"`) and
  `src/extensions/jemoji.rs` (`Jemoji`, name `"jemoji"`). Both `transform()`
  bodies delegate verbatim to `crate::mentions::process_mentions` /
  `crate::jemoji::process_jemoji` — skip-tag/code/pre/email/team-mention/emoji
  logic reused unchanged. `Mentions` captures `base_url` from
  `mentions_base_url(cfg)` at construction, NOT `TransformContext::baseurl`.
  Both ignore `SiteIndex` and `ctx.collection`.

**Fix 2: `Registry::from_config` auto-activation (TDD)**
- Wrote tests FIRST in `src/extensions/mod.rs`:
  `test_registry_auto_activates_jemoji_from_plugins`,
  `test_registry_auto_activates_mentions_from_gems`,
  `test_registry_empty_when_neither_plugin_nor_extensions`,
  `test_registry_order_is_mentions_jemoji_wikilinks`,
  `test_registry_order_independent_of_plugin_list_order`,
  `test_mentions_transform_honors_custom_base_url_not_ctx_baseurl`,
  `test_registry_byte_identical_to_inline_pipeline`.
- Ran tests: FAILS as expected — 6 failed. E.g. ordering test got `["wikilinks"]`,
  expected `["mentions", "jemoji", "wikilinks"]`; gems test got `[]`, expected
  `["mentions"]`; base_url + round-trip failed (registry empty, no transforms).
- Implemented auto-activation in `from_config` (`src/extensions/mod.rs`): prepend
  `Mentions` then `Jemoji` (gated on `has_mentions_plugin`/`has_jemoji_plugin`)
  BEFORE parsing the optional `extensions:` block. Order is fixed
  `[mentions, jemoji, <extensions...>]`, independent of plugin-list order.
  Removed the old early-return-on-missing-`extensions:` so plugin activation runs
  regardless of the block.
- Ran tests: PASSES — all 20 `extensions::tests` pass (14 pre-existing + 6 new,
  plus the empty-neither test = 7 new total).

**Fix 3: remove inline generator branches + unused locals**
- Removed `mentions_base_url`/`jemoji_enabled` locals and the two
  `if enabled { process_x() }` blocks at both call sites in `src/generator.rs`
  (collection path formerly ~1912/2111, pages path formerly ~2303/2413). Each is
  now replaced by the single existing `apply_extensions(...)` call, with a
  comment documenting the preserved `mentions -> jemoji -> <extensions...>` order.
- Wrote generator end-to-end test FIRST:
  `test_jemoji_mentions_run_via_registry_in_page_generation` — builds a
  `plugins: [jemoji, jekyll-mentions]` config (NO `extensions:` block), asserts
  the registry is non-empty with names `["mentions","jemoji"]`, runs
  `generate_pages_cached_with_config_and_progress` with an `ExtensionRuntime`,
  and asserts emoji `<img class="emoji">` + `<a ... class="user-mention">@alice`
  are emitted while `<code>:heart: @bob</code>` stays untouched. PASSES.

**Regression-trap verification (jemoji/mentions now run ONLY via the `ext` path):**
- `src/main.rs` builds `extension_runtime = Some` iff the registry is non-empty
  (line ~870), and passes `extension_runtime.as_ref()` / `ext_ref` to BOTH real
  generator call sites (`generate_collection_pages_cached_with_progress` ~941,
  `generate_pages_cached_with_config_and_progress` ~1054). Because
  `from_config` now auto-adds mentions/jemoji, a `plugins:`/`gems:` site's
  registry is non-empty → runtime is `Some` → transforms run. Verified.
- Confirmed no production build path passes `ext: None` with a plugin configured.
  The `ext: None` convenience wrappers (`generate_pages`, `generate_pages_cached`,
  `generate_collection_pages*`, `generate_pages_cached_with_config`) are only
  used by unit tests, none of which asserted jemoji/mentions output (grep across
  repo found zero `user-mention`/`class="emoji"`/`process_*` references outside
  `src/jemoji.rs`, `src/mentions.rs`, `src/generator.rs`). No test needed updating.

**Byte-identical proof (DTC source absent — see grooming caveat):**
- Built release binary from HEAD (69808f5, via `git worktree`) and from this
  branch. Built a fixture site with `plugins: [jemoji, jekyll-mentions]`, a
  `baseurl`, a `people` collection, and NO `extensions:` block. Content exercises
  emoji, `@mention`, team-mention `@jekyll/core`, email `user@example.com`,
  inline `<code>`, and a fenced code block, across a page, a post, and a
  collection item.
- `diff -r` of the two `_site` trees (excluding `.rustkyll-manifest.json`, whose
  only delta is non-deterministic HashMap key ordering / build metadata, not site
  output): **IDENTICAL — all HTML byte-identical** between HEAD and new binary.
- Rendered output verified correct: emoji→`<img class="emoji">`, `@alice`/`@bob`
  →`user-mention`, `@jekyll/core` NOT linked, `user@example.com` NOT converted,
  inline+fenced code untouched.

**DOM baseline:**
- DTC source is absent from this checkout (`websites/` has only `mediumish`,
  which uses neither plugin and has no cached Jekyll site). `recount-all-dom.sh`
  reports "Found 0 sites" here — no live DTC/mediumish comparison is possible.
  (The script overwrote `docs/dom-recount-results.md` with the 0-site run; I
  restored the committed baseline via `git checkout`.)
- **USER ACTION (deferred, per groomed spec):** on a DTC-present checkout, run
  `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
  and confirm it stays **≥ 788/790**. The byte-identical HEAD-vs-new proof above
  is the standing evidence that output is unchanged.

**Summary:**
- Files modified: `src/extensions/mod.rs` (module decls + auto-activation +
  7 tests), `src/generator.rs` (removed 2 inline branches + 2 unused locals,
  added 1 end-to-end test).
- Files created: `src/extensions/mentions.rs`, `src/extensions/jemoji.rs`.
- Tests added: 8 (7 registry unit tests + 1 generator end-to-end).
- Build results: `cargo-safe test` — 4130 lib tests + all integration suites
  pass, 0 fail. `cargo-safe clippy -- -D warnings` clean. `cargo fmt --check`
  clean. (`clippy --all-targets` surfaces pre-existing `useless_vec`-style lints
  in unrelated test code — none in files touched here.)
- Known limitations: live DTC/mediumish DOM recount not runnable in this
  checkout (sources absent); covered by the byte-identical proof + deferred USER
  ACTION above.

### [QA] 2026-07-02
- Tests (`./scripts/cargo-safe test`): all green, 0 failed. `--lib`: 4130 passed,
  0 failed, 2 ignored (both pre-existing, not touched by this issue; diff adds no
  `#[ignore]`).
- Clippy (`./scripts/cargo-safe clippy -- -D warnings`): clean, exit 0. The only
  2 warnings are from the external `liquid-lib` dependency (renamed-lint notices),
  not rustkyll.
- Fmt (`cargo fmt --check`): clean, exit 0.
- TDD verification: Fix 2 (auto-activation, the substantive logic change) shows a
  genuine RED->GREEN cycle in the log — 6 tests written first, ran and FAILED with
  concrete expected-vs-actual (ordering got `["wikilinks"]`, gems got `[]`, base_url
  + round-trip empty), then implemented, then PASSED. Adequate TDD evidence for the
  behavior-adding change. (The Fix 3 end-to-end test is a post-removal verification
  test and does not log a distinct RED, which is acceptable for a pure-removal
  refactor whose real guarantee is the byte-identical output proof below.)

Acceptance criteria:
- Registry/HtmlTransform path + inline branches removed: PASS. `git diff
  src/generator.rs` confirms both `if enabled { process_x() }` blocks and both
  `mentions_base_url`/`jemoji_enabled` locals are deleted at both call sites;
  replaced by the single `apply_extensions(...)` with an order comment.
- Auto-activation with no `extensions:` block: PASS. `from_config` prepends
  Mentions/Jemoji gated on `has_mentions_plugin`/`has_jemoji_plugin` before parsing
  the optional `extensions:` block (early-return removed). Unit tests
  `test_registry_auto_activates_jemoji_from_plugins` /
  `_mentions_from_gems` / `_empty_when_neither...` cover it.
- Order exactly `[mentions, jemoji, wikilinks]` regardless of plugin-list order:
  PASS. `test_registry_order_is_mentions_jemoji_wikilinks` and
  `test_registry_order_independent_of_plugin_list_order` verify; code fixes order
  before parsing extensions.
- Mentions honors custom `jekyll-mentions.base_url`, NOT `ctx.baseurl`: PASS.
  `Mentions::from_config` captures `mentions_base_url(cfg)` at construction; unit
  test asserts `https://gitlab.com/user` while `ctx.baseurl="/blog"`. Live fixture
  with default config resolved `@alice` to `https://github.com/alice`.
- Byte-identical output (round-trip + independent binary diff): PASS.
  `test_registry_byte_identical_to_inline_pipeline` asserts registry ==
  `process_jemoji(process_mentions(...))`. INDEPENDENTLY REPRODUCED: built release
  binaries from this working tree AND from pre-work commit 69808f5 (git worktree),
  built a fixture site with `plugins: [jemoji, jekyll-mentions]`, NO `extensions:`
  block, content with emoji/@mention/@org-team/email/inline-code/fenced-code across
  a page + a `people` collection item. `diff -r` of the two `_site` trees (excl.
  `.rustkyll-manifest.json`) = IDENTICAL, exit 0 (feed.xml identical too). Rendered
  output correct: emoji -> `<img class="emoji">`, `@alice`/`@eve`/`@frank` ->
  `user-mention`, `@jekyll/core`/`@octo/team` NOT linked, `user@example.com` plain,
  inline `<code>:heart: @carol</code>` and fenced code untouched.
- Existing jemoji/mentions unit tests pass unchanged: PASS (process_* reused, not
  rewritten; full suite green).
- Regression trap (no production `ext: None` with plugin configured): PASS. Traced
  `src/main.rs`: `extension_registry = Registry::from_config` (l320), `site_index`
  and `extension_runtime = Some(...)` iff registry non-empty (l853/l870); both real
  call sites pass `ext_ref`/`extension_runtime.as_ref()` (l941, l1062). Since
  `from_config` now auto-adds mentions/jemoji, a plugin site's registry is non-empty
  -> runtime `Some` -> transforms run. The `ext: None` convenience wrappers are only
  called by tests and internal wrapper chains (grep confirms), never by main.rs.
- DTC 788/790: NOT LIVE-VERIFIABLE HERE — DTC source absent (`websites/` has only
  `mediumish`, which uses neither plugin). Per groomed caveat, relied on the
  independent byte-identical binary diff + green suite. USER ACTION remains: re-run
  `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io` on
  a DTC-present checkout and confirm >= 788/790 before final sign-off.
- VERDICT: PASS.

### [PM] 2026-07-02 — Acceptance Review
- Reviewed diff: 4 files (src/extensions/mod.rs modified, src/generator.rs
  modified, src/extensions/mentions.rs + src/extensions/jemoji.rs new/untracked).
  `git diff --stat` + full diff of each file inspected directly.
- Independently re-ran (not relying solely on QA):
  - `cargo-safe test --lib extensions::` -> 49 passed, 0 failed.
  - `cargo-safe test --lib test_jemoji_mentions_run_via_registry` -> 1 passed.
  - `cargo fmt --check` -> clean (exit 0).
  - `cargo-safe clippy -- -D warnings` -> clean; the only 2 warnings are from the
    external `liquid-lib` dependency (renamed-lint notices), not rustkyll.
- Code review: both new impls are thin adapters delegating verbatim to
  `process_mentions` / `process_jemoji` (no logic reimplemented). `Mentions`
  captures `base_url` from `mentions_base_url(cfg)` at construction, NOT
  `ctx.baseurl` — confirmed. `from_config` prepends `[mentions, jemoji]` before
  the `extensions:` block; the old early-return-on-missing-`extensions:` is gone
  so plugin activation runs regardless of block presence. Generator diff removes
  both inline `if enabled { process_x() }` branches and both
  `mentions_base_url`/`jemoji_enabled` locals at both call sites, replaced by the
  single existing `apply_extensions(...)` with an order comment. Matches spec.

Per-criterion verdicts:
- Runs via Registry/HtmlTransform, inline branches + unused locals removed: MET
  (generator diff confirms deletion at both call sites).
- Auto-activation with no `extensions:` block (non-empty registry, both
  transforms): MET (test_registry_auto_activates_jemoji_from_plugins /
  _mentions_from_gems / _empty_when_neither...).
- Order exactly `[mentions, jemoji, <extensions>]`, plugin-list-order independent:
  MET (test_registry_order_is_mentions_jemoji_wikilinks /
  _order_independent_of_plugin_list_order).
- Mentions honors custom base_url not ctx.baseurl: MET
  (test_mentions_transform_honors_custom_base_url_not_ctx_baseurl).
- Byte-identical output: MET — round-trip unit test
  (test_registry_byte_identical_to_inline_pipeline) plus QA's independent
  HEAD(69808f5)-vs-branch release-binary `_site` diff = byte-identical.
- Existing jemoji/mentions unit tests unchanged: MET (process_* reused; suite
  green).
- test/clippy/fmt clean: MET (re-verified above).
- TDD fail-then-pass logged: MET (Fix 2 shows genuine RED->GREEN with concrete
  expected-vs-actual).
- Regression trap (no production `ext: None` with plugin configured): MET (main.rs
  builds runtime Some when registry non-empty; ext:None only in tests).

- DTC 788/790 live recount: DEFERRED, not dropped. DTC source is absent from this
  checkout (`websites/` has only `mediumish`, which uses neither plugin -> registry
  stays empty for it, no regression path). Committed baseline is 788/790. Per the
  groomed "DTC source-absent caveat" this criterion is explicitly acceptable on the
  byte-identical binary-diff proof. USER ACTION carried forward: on a DTC-present
  checkout run `bash scripts/recount-all-dom.sh --site
  DataTalksClub/datatalksclub.github.io` and confirm >= 788/790 before merge.
- Output verification: this is a zero-behavior-change refactor; the decisive output
  evidence is QA's byte-identical HEAD-vs-branch `_site` diff, independently
  reproduced. Accepted on that basis.
- Follow-up issues created: none (no criteria descoped; only the DTC live recount
  is deferred as a USER ACTION per the pre-agreed caveat).
- VERDICT: ACCEPT.
