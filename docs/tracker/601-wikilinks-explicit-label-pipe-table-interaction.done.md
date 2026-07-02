# Issue 601: wikilinks `[[target|label]]` mangled by GFM table parsing

Follow-up to #600 (extension framework + wikilinks). Do NOT reopen #600 for this;
it is a pre-existing markdown-renderer limitation surfaced during #600 QA.

## Problem

rustkyll's markdown renderer turns **any** single line containing an unescaped
`|` into a GFM-style table *before* the post-render HTML transforms (including
wikilinks) run. As a result, an author writing the explicit-label wikilink form
inline in a markdown paragraph:

```markdown
See [[event-tracking|event tracking]] for details.
```

gets the `|` consumed by table parsing first, so the wikilinks transform later
sees mangled HTML (e.g. `[[event-tracking</td><td>event tracking]]`) and
correctly leaves it alone — the link is never resolved.

QA confirmed this reproduces with the wikilinks extension **fully disabled**
(`This is a sentence with a | pipe.` -> `<table>...</table>`), so it is
independent of #600. The primary `[[target]]` form (no pipe) works end-to-end;
only the explicit-label form is affected in real `.md` files. The wikilinks
transform itself handles `[[target|label]]` correctly given proper HTML input
(unit tests `test_explicit_label_preserved`, `test_broken_explicit_label_preserved`
pass).

This blocks the podwiki explicit-label authoring workflow, which is a motivating
use case for #600.

## Root Cause (investigated)

Pipeline order for a markdown page/collection item:

1. Liquid render -> markdown text.
2. `kramdown_parser` converts markdown -> HTML. **Table detection happens here.**
3. `mentions` / `jemoji` post-processing (if enabled).
4. `apply_extensions(...)` runs the wikilinks `HtmlTransform` on the finished
   HTML (`src/generator.rs:2123` for collections, `:2426` for pages).

The wikilinks transform (`src/extensions/wikilinks.rs`) is **post-render** and
operates on HTML. It correctly:
- splits `[[target|label]]` on `|` (`render_link`, line 118),
- bails out of a `[[...]]` span on `<`, newline, or nested `[[`
  (`find_wiki_close`, line 233 — so a wikilink never spans tags/lines),
- skips `[[...]]` inside `<code>` / `<pre>` (`skip_depth`).

The table trigger is `is_table_line` in `src/kramdown_parser/parser.rs:1559`.
It returns `true` for **any** line with an unescaped `|` that is not inside a
backtick code span, `<code>` tag, or math (`$...$`) region. It has **no**
awareness of `[[...]]` spans, so the lone inline pipe inside `[[a|b]]` is treated
as a cell separator and the line becomes a table (`try_parse_table`,
`split_table_cells`, and the paragraph-break check at parser.rs:1238-1241 all
route through the same `is_table_line` / `try_parse_separator_line` gate).

Because table detection is in the shared parser (which runs for **all** sites)
and the wikilinks fix must stay opt-in, the fix must be gated on the wikilinks
extension being enabled and must not alter output for sites without it.

## Recommended Approach

**Primary: pre-render pipe protection + unconditional restore, gated on
wikilinks being enabled.** This is the most surgical option: it touches neither
the heavily-tested shared kramdown parser nor the already-tested wikilinks
transform.

1. When (and only when) the wikilinks extension is enabled for the build, run a
   protection pass over the **markdown text** immediately before the
   `kramdown_parser` conversion. For each `[[...]]` span that stays on a single
   line and contains no `<` (i.e. the same spans `find_wiki_close` would accept),
   replace the interior `|` with a private-use sentinel character (e.g. `U+E000`,
   which cannot appear in normal DTC/podwiki content and is passed through as
   literal text by both the parser and Liquid).
2. The kramdown parser no longer sees a pipe in those spans, so `is_table_line`
   does not fire and the line renders as a normal paragraph. The
   `[[…sentinel…]]` text passes through exactly like the already-working bare
   `[[target]]` form.
3. Immediately before `apply_extensions` (i.e. after markdown, before the
   wikilinks transform runs), restore the sentinel back to `|` across the whole
   HTML. This is unconditional and total, so **no sentinel can ever leak** into
   output, and `[[a|b]]` that happened to sit inside a fenced code block is
   restored to a literal `|` inside `<code>`/`<pre>` (which the transform then
   correctly leaves untouched).
4. The existing wikilinks `HtmlTransform` resolves `[[event-tracking|event tracking]]`
   with **no changes** — its explicit-label handling is already unit-tested.

Both protect and restore are no-ops unless the wikilinks extension is enabled, so
sites without the extension (including DTC) get byte-identical output. Put the
protect/restore helpers in `src/extensions/wikilinks.rs` (or the extensions
module) and wire them into the markdown render path guarded by the same
enabled-check used for `apply_extensions`.

**Acceptable alternative (only if the pre-render plumbing proves too invasive):**
add a `protect_wikilink_pipes: bool` field to `kramdown_parser::Options`
(default `false`), set it only when wikilinks is enabled, and make
`is_table_line` / `split_table_cells` / the paragraph-break check treat pipes
inside `[[...]]` spans as non-separators when the flag is set. This keeps
code-context handling in the parser where it already lives. It is acceptable
**only** if the flag defaults off and DTC output stays byte-identical.

Whichever is chosen it MUST NOT: regress the bare `[[target]]` form, break
genuine pipe-tables, alter output for sites without the extension, or change the
DTC DOM baseline.

## Scope

Make the explicit-label `[[target|label]]` form work end-to-end in real markdown
files, opt-in via the wikilinks extension. Do **not** attempt the broader
"a lone `|` in a paragraph should not become a table" kramdown-vs-GFM behavior
change for non-wikilinks sites — that is out of scope here (would risk the DTC
baseline) and can be tracked separately if desired.

## Acceptance Criteria

- [ ] End-to-end (release binary on a fixture site with `extensions: [wikilinks]`):
      a markdown paragraph `See [[event-tracking|event tracking]] here.` renders
      to a resolved `<a href="...event-tracking...">event tracking</a>` — the
      output contains **no** `<table>`, `<td>`, or `<tr>` fragment for that line.
- [ ] End-to-end: the bare `[[event-tracking]]` form still resolves to an anchor
      (no regression).
- [ ] End-to-end: a genuine markdown pipe-table (header row + `| --- | --- |`
      separator row) on the same page still renders as a `<table>`.
- [ ] `[[a|b]]` written inside a fenced code block / `<code>` / `<pre>` is left
      literal (the sentinel/flag never leaks; no anchor is emitted there).
- [ ] With the wikilinks extension **disabled**, output is byte-identical to the
      pre-change build for a page containing `[[a|b]]`, a genuine pipe-table, and
      a lone-`|` paragraph (protect/restore is a no-op; the shared parser is
      unchanged or gated off). Verify by building the fixture with and without
      the extension and diffing the non-wikilinks page output.
- [ ] The wikilinks unit tests (`test_explicit_label_preserved`,
      `test_broken_explicit_label_preserved`, `test_default_label_humanized`,
      code/pre skip tests) all still pass unchanged.
- [ ] DTC DOM match count must not drop below the #600 baseline of **788/790**.
      DTC has no `extensions:` block, so this fix is inert for DTC and it must
      remain exactly 788/790. Verify with
      `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
      once the DTC source is present (see baseline note below).
- [ ] `./scripts/cargo-safe test` green, `./scripts/cargo-safe clippy -- -D warnings`
      clean, `cargo fmt --check` clean.
- [ ] TDD: write the failing end-to-end/integration test first, log the
      fail-then-pass cycle in `## Log`.

## Test Scenarios

### Unit: pipe protection / restore (or parser flag)
- `protect` on `See [[event-tracking|event tracking]] here.` replaces only the
  interior `|` with the sentinel and leaves everything else identical.
- `protect` leaves bare `[[event-tracking]]` untouched (no `|`).
- `protect` does not touch a `|` that is **outside** any `[[...]]` span
  (e.g. a real table row `| a | b |` is unchanged).
- `protect` does not span lines/tags: `[[a\n|b]]` and `[[a<b|c]]` are left as-is
  (mirrors `find_wiki_close`).
- `restore` maps every sentinel back to `|` and is a perfect inverse of the
  no-`<`, single-line protect (round-trip identity).
- Sentinel is a private-use codepoint that round-trips through `protect`/`restore`
  even in unicode-adjacent text (reuse the `test_multiple_links_and_unicode_context`
  style input).

### Integration: end-to-end via generator on a fixture
- Fixture wiki page (extension enabled) with:
  - `See [[event-tracking|event tracking]] here.` -> asserts output contains
    `<a href="…event-tracking…">event tracking</a>` and does **not** contain
    `<table>` / `<td>` on that line.
  - A genuine pipe-table with a `| --- |` separator -> asserts a `<table>` IS
    produced.
  - A lone-`|` prose line (`cost is $5 | $10`) — assert and document the intended
    behavior (unchanged from today; still a table for non-wikilink pipes, since
    fixing that is out of scope).
  - Bare `[[event-tracking]]` -> asserts a resolved anchor.
  - `[[a|b]]` inside a fenced code block -> asserts it stays literal (no anchor,
    no sentinel char in output).
- Extension-off build of the same fixture: the non-wikilinks page output is
  byte-identical to a build from committed code (no `extensions:` block).

## Dependencies

- #600 (extension framework + wikilinks) — `.done.md` (committed). Satisfied.

## DTC DOM Baseline

- #600 baseline: **788/790 (100%)** per `docs/dom-recount-results.md`.
- The DTC source (`websites/DataTalksClub/datatalksclub.github.io`) is **absent**
  in this checkout, so the live recount cannot be run during grooming.
- **Opt-in guarantee:** DTC has no `extensions:` block, so the wikilinks
  extension is disabled for DTC and the protect/restore (or parser flag) is a
  no-op. DTC output must therefore be byte-identical and the count must stay
  exactly 788/790. The engineer/tester MUST run
  `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
  once the DTC source is checked out and confirm it stays at 788/790 before
  acceptance. Any drop below 788/790 is an automatic REJECT.

## Notes

- Surfaced by #600 QA (2026-07-02). See #600 `## Log` [QA] entry for the exact
  reproduction.

## Log

### [PM] 2026-07-02 — Grooming
- Investigated root cause: table detection lives in the shared
  `kramdown_parser::is_table_line` (`src/kramdown_parser/parser.rs:1559`), which
  fires on any unescaped inline `|` and runs **before** the post-render wikilinks
  `HtmlTransform` (`src/extensions/wikilinks.rs`, applied at
  `src/generator.rs:2123`/`:2426`). The transform already handles
  `[[target|label]]` and code/pre skipping correctly; the parser mangles the pipe
  first. Confirmed the transform bails on `<`/newline inside a span
  (`find_wiki_close`), which the protection pass should mirror.
- Recommended approach: pre-render pipe protection with a private-use sentinel,
  gated on wikilinks being enabled, plus an unconditional post-markdown /
  pre-transform restore so no sentinel can leak and code-block pipes stay
  literal. Parser flag documented as an acceptable, opt-in alternative.
- Chosen for least regression risk: no changes to the shared parser or the
  tested wikilinks transform; both hooks are no-ops when the extension is off, so
  DTC (no `extensions:` block) is byte-identical.
- Added concrete end-to-end + unit acceptance criteria and test scenarios
  (explicit-label resolves, genuine table preserved, bare form unaffected,
  code-block literal, extension-off byte-identical).
- DTC source absent in this checkout: recorded the 788/790 baseline and the
  opt-in inertness guarantee; live recount deferred to implementation/QA.
- Renamed `601-...todo.md` -> `601-...groomed.md` via `git mv`.

### [SWE] 2026-07-02

Renamed `.groomed.md` -> `.in-progress.md` via `git mv`.

Implemented the recommended pre-render pipe-protection + restore approach,
gated on the `wikilinks` extension being enabled. Both hooks are no-ops when the
extension is off.

**Fix 1: `protect_pipes` / `restore_pipes` helpers + `SENTINEL` (src/extensions/wikilinks.rs)**
- Wrote unit tests first: `test_protect_replaces_only_interior_pipe`,
  `test_protect_leaves_bare_wikilink_untouched`,
  `test_protect_does_not_touch_pipe_outside_span`,
  `test_protect_does_not_span_newline`, `test_protect_does_not_span_lt`,
  `test_protect_multiple_spans_and_unicode`, `test_restore_is_inverse_of_protect`,
  `test_restore_maps_every_sentinel_back`, `test_protect_empty_wikilink`.
- Ran tests: FAILS (compile error) — `protect_pipes`, `restore_pipes`, `SENTINEL`
  not found in scope.
- Implemented `SENTINEL = '\u{E000}'`, `protect_pipes` (reuses the existing
  `find_wiki_close` so protected spans exactly match what the transform resolves:
  single-line, no `<`), and `restore_pipes` (total, unconditional inverse).
- Ran tests: PASSES (29 wikilinks unit tests green, incl. the pre-existing
  `test_explicit_label_preserved`, `test_broken_explicit_label_preserved`,
  `test_default_label_humanized`, code/pre skip tests — all unchanged).

**Fix 2: single choke-point protect/restore in markdown conversion (src/frontmatter.rs)**
- First attempt threaded a `protect_wikilink_pipes` flag through `LayoutEngine`
  and its 5 `markdown_to_html_with_options` call sites. The release-binary
  fixture check revealed this MISSED the common collection-item path: markdown
  collection items WITHOUT Liquid tags have their `html_content` pre-computed
  during collection loading (src/collection.rs:1246 / :1679) via a direct
  `markdown_to_html_with_options` call that never touches the layout engine, so
  `[[a|b]]` was still mangled into a `<table>` end-to-end. Reverted the layout.rs
  change (`git checkout src/template/layout.rs`).
- Moved the protection to the single shared choke point: added a process-global
  `PROTECT_WIKILINK_PIPES` `AtomicBool` (mirroring the existing
  `MARKDOWNIFY_*` flags) with `set_/get_protect_wikilink_pipes`. Renamed the body
  of `markdown_to_html_with_options` to `markdown_to_html_with_options_impl` and
  made `markdown_to_html_with_options` a thin wrapper: when the flag is set it
  `protect_pipes(input)` -> impl -> `restore_pipes(output)`, all contained within
  the one call so no sentinel can leak and code-block pipes stay literal. When
  off it calls the impl directly => byte-identical.

**Fix 3: wire the flag before ANY markdown is converted (src/main.rs)**
- Second bug found via the release binary: setting the flag near the layout-engine
  setup (line ~683) was TOO LATE — collections load at line ~354 and pre-compute
  `html_content` before that. Moved the registry build + `has_wikilinks` check +
  `set_protect_wikilink_pipes(has_wikilinks)` to immediately after config load
  (line ~312), before data/collection loading and all rendering. Reused the same
  `extension_registry` for the later runtime (removed the duplicate build).

**Fix 4: end-to-end generator integration tests (src/generator.rs)**
- Wrote `test_explicit_label_wikilink_resolves_end_to_end` FIRST and, to prove it
  catches the bug, temporarily forced protection OFF: FAILS — output was
  `<table>...<td>See [[event-tracking</td><td>Event Tracking Guide]] here.</td>...`
  (the exact reported mangling; wikilink never resolved). Reverted the temporary
  toggle; with protection ON the test PASSES.
- Fixture page (`.md`) exercises all scenarios: explicit-label wikilink in prose,
  bare `[[event-tracking]]`, a genuine `| --- |` pipe-table, and `[[a|b]]` in a
  fenced code block. Asserts: inline `<a ...>Event Tracking Guide</a>` (no table,
  explicit label used), bare form resolves, `<table>` still produced for the real
  table, code block stays literal `[[event-tracking|inside code]]`, and NO
  `SENTINEL` char anywhere.
- `test_wikilink_pipe_protect_off_is_noop` proves the hooks are inert when
  disabled. Both flag-touching tests share a `WIKILINK_PROTECT_LOCK` mutex and a
  panic-safe `ProtectGuard` (RAII) so they never overlap and always reset the
  global flag. Ran the full lib suite 3x — stable, no flakiness.

**End-to-end verification (release binary on a fixture site):**

Fixture `_config.yml` had `extensions: [wikilinks: {scope: [wiki]}]`, `baseurl: /podwiki`.
`_wiki/guide.md` output (extension ON):
```
<!DOCTYPE html><html><body><p>See <a href="/podwiki/wiki/event-tracking/">Event Tracking Guide</a> here.</p>
<p>Bare <a href="/podwiki/wiki/event-tracking/">event tracking</a> link.</p>
<table><thead><tr><th>Col A</th><th>Col B</th></tr></thead><tbody>
<tr><td>one</td><td>two</td></tr>
</tbody></table>
<div class="highlighter-rouge"><div class="highlight"><pre class="highlight"><code>[[event-tracking|inside code]]
</code></pre></div></div>
</body></html>
```
Same fixture with the `extensions:` block removed (extension OFF) — byte-identical
to the pre-change build: the `[[a|b]]` line is still a `<table>` (documented
out-of-scope pre-existing behavior), wikilink syntax stays literal, and a
recursive grep confirmed NO sentinel (`U+E000`) anywhere in the output tree.

**Summary:**
- Files modified: `src/extensions/wikilinks.rs` (SENTINEL + protect/restore +
  9 unit tests), `src/frontmatter.rs` (global flag + wrapper/impl split),
  `src/main.rs` (build registry + set flag before collection loading, dedup
  registry build), `src/generator.rs` (2 end-to-end integration tests + helpers).
  `src/template/layout.rs` unchanged (first approach reverted).
- Tests added: 9 unit + 2 integration = 11. Full suite: 4122 passed, 0 failed,
  2 ignored (pre-existing, not mine).
- Build/lint: `cargo-safe test` green, `cargo-safe clippy -- -D warnings` clean
  (only unrelated `liquid-lib` dep warnings), `cargo fmt --check` clean.
- DTC DOM: source (`websites/DataTalksClub/datatalksclub.github.io`) is ABSENT in
  this checkout, so the live 788/790 recount is DEFERRED. Relying on the opt-in
  no-op guarantee: DTC has no `extensions:` block => `has_wikilinks` is false =>
  `PROTECT_WIKILINK_PIPES` stays false => `markdown_to_html_with_options` calls
  the impl directly (byte-identical). Verified via the extension-off fixture that
  output is unchanged and no sentinel appears. QA must run
  `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
  once the DTC source is checked out and confirm it stays exactly 788/790.
- DTC build perf check also deferred (DTC source absent); the fixture build was
  sub-millisecond.
- Deviation from the groomed "put helpers in wikilinks.rs and wire into the
  markdown render path": helpers ARE in wikilinks.rs, but the wiring is at the
  single shared conversion choke point (`frontmatter::markdown_to_html_with_options`)
  rather than per-render-function in the layout engine. This was necessary because
  the collection-item path bypasses the layout engine, and the choke point covers
  every content path with one gated hook. Restore happens inside that same call
  (before content capture / layout wrap / transform), which is at-or-before the
  spec's "immediately before apply_extensions" point and guarantees no leak.

### [QA] 2026-07-02

Independent verification (did NOT trust SWE-reported numbers).

**Build/lint/format:**
- `cargo fmt --check`: clean (exit 0).
- `./scripts/cargo-safe clippy -- -D warnings`: clean (exit 0). Only two
  informational notices about outdated lint names in clippy config
  (`clippy::string_to_string` removed, `clippy::empty_enum` renamed) — pre-existing,
  not code warnings, exit 0.
- `./scripts/cargo-safe test`: 4121 lib passed / 0 fail (full aggregate ~4347 across
  all binaries). NOTE: one flaky failure observed intermittently in
  `template::translate_tag::tests::test_resolve_dynamic_args_with_variable` (~1 of 5
  runs). Root-caused as a PRE-EXISTING global-state race: `TRANSLATIONS`
  (`RwLock<Option<TranslationStore>>`, translate_tag.rs:31) is clobbered by parallel
  tests in translate_tag.rs / wallet_generator.rs / template_generators.rs that call
  `set_translations`/`clear_translations`. NONE of that code is touched by issue 601.
  Not a blocker for this issue, but flagged for a separate follow-up.

**Global-flag soundness (main risk — scrutinized):**
- (a) Type/thread-safety: `PROTECT_WIKILINK_PIPES` is an `AtomicBool` (not `static mut`),
  set once on the main thread in main.rs (~line 312, right after config load, before
  collection loading / parallel rendering) and only read during rendering. Relaxed
  ordering is sufficient because the rayon thread spawn establishes happens-before.
  Mirrors the existing `MARKDOWNIFY_*` flag pattern. SOUND — no data race.
- (b) Cross-site/test bleed: `set_protect_wikilink_pipes(has_wikilinks)` runs at the
  start of every `build_site`, so it is re-set each build — no leak across sites within
  a process. In tests only two tests toggle it, both serialized via a `WIKILINK_PROTECT_LOCK`
  mutex + panic-safe RAII `ProtectGuard` that resets to false on drop. Blast radius is
  limited to markdown containing `[[...|...]]` spans (protect/restore are no-ops
  otherwise); ran the 34 wikilink tests 5x — stable, no flakiness. SOUND.
- (c) OFF => false => byte-identical: when the flag is false the wrapper calls
  `markdown_to_html_with_options_impl` directly (the unchanged old body). Confirmed by
  code AND independently by diff (below). SOUND.

**End-to-end verification (release binary on a fixture podwiki site with baseurl /podwiki):**
- AC1 explicit-label: `See [[event-tracking|event tracking]] here.` =>
  `<p>See <a href="/podwiki/wiki/event-tracking">event tracking</a> here.</p>` — inline
  anchor, NO `<table>`/`<td>`/`<tr>` for that line. PASS.
- AC2 bare form: `[[event-tracking]]` => resolved `<a ...>` anchor. PASS.
- AC3 genuine table: header + `| --- |` separator => `<table>` with `<th>`/`<td>`. PASS.
- AC4 code block: fenced `[[event-tracking|inside code]]` stays literal inside
  `<pre><code>`, no anchor. Recursive grep for U+E000 across the whole ON output tree:
  NO sentinel. PASS.
- AC5 extension OFF byte-identical: built the fixture (no `extensions:` block) with the
  UNCOMMITTED binary and with the COMMITTED (HEAD 529da65) binary via a git worktree,
  then diffed. `wiki/guide.html` and `wiki/event-tracking.html` are BYTE-IDENTICAL
  (only diffs anywhere were feed.xml build timestamp and manifest HashMap key ordering —
  neither is page content). The OFF `guide.html` still shows the documented pre-existing
  mangling (`<td>See [[event-tracking</td><td>event tracking]] here.</td>`) unchanged,
  and the lone-`|` line is still a table (out-of-scope, as spec'd). PASS.
- AC6 wikilinks unit tests: 34 wikilink lib tests green incl. `test_explicit_label_preserved`,
  `test_broken_explicit_label_preserved`, code/pre skip tests. PASS.
- AC7 DTC DOM 788/790: DTC source absent in checkout — recount DEFERRED (as noted by PM/SWE).
  Relying on the opt-in inertness guarantee, independently confirmed by the AC5 byte-identical
  HTML diff and the no-sentinel greps. Not failing solely for absent source. NOTE.
- AC8 test/clippy/fmt green: PASS (modulo the unrelated pre-existing translate flaky test).
- AC9 TDD: log shows genuine RED->GREEN. Fix 4 logged the exact failing output
  (`<table>...<td>See [[event-tracking</td><td>Event Tracking Guide]] here.</td>...`)
  with protection forced off, which I independently reproduced verbatim in the OFF build.
  Helper unit tests written first (compile-error RED). PASS.

- VERDICT: PASS

Notes for orchestrator/PM:
- Pre-existing flaky `test_resolve_dynamic_args_with_variable` (global `TRANSLATIONS`
  RwLock race across parallel translate/wallet tests) is NOT caused by this change but
  should get a separate follow-up issue.
- DTC 788/790 recount must still be run once the DTC source is checked out before final
  acceptance, per the issue's baseline note.

### [PM] 2026-07-02 — Acceptance Review

Reviewed diff (10 files changed; core: `src/extensions/wikilinks.rs`,
`src/frontmatter.rs`, `src/main.rs`, `src/generator.rs`). Independently
re-verified fmt/clippy/tests rather than trusting reported numbers.

**Independent verification performed:**
- `cargo fmt --check`: clean (FMT_OK).
- `./scripts/cargo-safe clippy --release -- -D warnings`: exit 0. Only the two
  pre-existing informational lint-config notices (`clippy::string_to_string`
  removed, `clippy::empty_enum` renamed) — not code warnings.
- `./scripts/cargo-safe test --release --lib wikilink`: **34 passed / 0 failed**,
  including the 9 new protect/restore unit tests, `test_explicit_label_preserved`,
  `test_broken_explicit_label_preserved`, `test_default_label_humanized`, the
  code/pre skip tests, and the 2 new end-to-end integration tests
  (`test_explicit_label_wikilink_resolves_end_to_end`,
  `test_wikilink_pipe_protect_off_is_noop`).
- Read the integration tests: they are meaningful, not smoke tests — they assert
  the resolved inline anchor + explicit label, absence of `<table>`/`<td>` on the
  wikilink line, bare-form resolution, a genuine `| --- |` table still rendering,
  code-block literal preservation, and NO `SENTINEL` (U+E000) leak.
- Confirmed the global flag is sound: `PROTECT_WIKILINK_PIPES` is an `AtomicBool`
  set in `build_site` (main.rs ~L312, before collection loading pre-computes
  `html_content` and before rendering), re-set each build (no cross-site bleed),
  and test-serialized via `WIKILINK_PROTECT_LOCK` + panic-safe `ProtectGuard`.
  Wrapper/impl split in `frontmatter.rs` calls the unchanged impl directly when
  the flag is off => byte-identical.

**Per-criterion verdicts:**
- AC1 explicit-label resolves inline, no table fragment: MET (integration test +
  QA release-binary fixture).
- AC2 bare `[[event-tracking]]` still resolves: MET.
- AC3 genuine pipe-table still renders: MET.
- AC4 `[[a|b]]` in code/pre stays literal, no sentinel leak: MET (asserted +
  recursive grep by QA).
- AC5 extension-off byte-identical: MET — QA diffed the uncommitted binary vs the
  committed HEAD binary via a worktree; `wiki/guide.html` / `event-tracking.html`
  byte-identical (only feed.xml timestamp + manifest key ordering differ).
- AC6 wikilinks unit tests unchanged and passing: MET.
- AC7 DTC DOM 788/790: NOT RE-RUNNABLE — DTC source
  (`websites/DataTalksClub/datatalksclub.github.io`) is absent in this checkout
  (confirmed). Accepted on the opt-in no-op guarantee (DTC has no `extensions:`
  block => `has_wikilinks` false => flag stays false => impl called directly =>
  byte-identical), which is exactly the basis on which #600 was accepted and is
  independently corroborated by the AC5 byte-identical diff + no-sentinel greps.
  **USER ACTION:** re-run
  `bash scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io`
  once the DTC source is present and confirm it stays exactly 788/790.
- AC8 test/clippy/fmt green: MET (see independent runs above).
- AC9 TDD RED->GREEN logged: MET — SWE log shows compile-error RED for the
  helpers and the exact `<td>See [[event-tracking</td>...` mangling with
  protection forced off, which QA independently reproduced.

**Descoping / follow-ups:** none descoped from #601. The pre-existing flaky
`template::translate_tag::tests::test_resolve_dynamic_args_with_variable`
(global `TRANSLATIONS` RwLock race across parallel translate/wallet tests) is
NOT part of #601's scope and did not block it; tracked as new follow-up
**#604** (`docs/tracker/604-flaky-translate-tag-global-translations-race.todo.md`)
with root cause + recommended serialization fix.

- Acceptance criteria: all MET, except AC7 which is non-re-runnable in this
  checkout (DTC source absent) and accepted on the verified byte-identical
  opt-off guarantee, carrying the USER ACTION recount note (same disposition as
  #600).
- VERDICT: ACCEPT
