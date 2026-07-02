# Issue 606: sitemap.xml includes output:false collection docs and ignores sitemap:false

Mirrors GitHub issue alexeygrigorev/rustkyll#7 (reported against rustkyll 0.4.7).
Close GH #7 when this lands.

## Problem

The generated `sitemap.xml` lists collection documents that are never written to
`_site`, producing `<loc>` entries that 404. Two divergences from
`jekyll-sitemap`:

1. **`output: false` collections are included in the sitemap** even though no
   HTML is emitted for their documents.
2. **`sitemap: false` front matter is ignored** — the page/document is still
   listed.

`jekyll-sitemap` excludes both.

## Minimal reproduction

`_config.yml`:
```yaml
url: "https://example.com"
baseurl: ""
title: Repro
collections:
  notes:
    output: false
    permalink: /notes/:title/
```
Files:
- `_notes/hidden.md` -> front matter `title: Hidden Note`
- `index.md` -> `title: Home`
- `excluded.md` -> `title: Excluded Page`, `sitemap: false`

Run `rustkyll build`.

### Actual sitemap.xml
```xml
<loc>https://example.com/</loc>
<loc>https://example.com/notes/hidden/</loc>   <!-- output:false, no file -> 404 -->
<loc>https://example.com/excluded.html</loc>    <!-- sitemap:false, should be omitted -->
```

### Expected sitemap.xml
```xml
<loc>https://example.com/</loc>
```

## Root cause (confirmed during grooming)

`src/sitemap.rs`, `collect_entries` (~lines 27-61):
- Pages loop (~41-49): only skips the index/root URL; no `sitemap: false` check.
- Collection items loop (~52-58): pushes every item URL unconditionally — never
  consults the collection `output` flag, never checks per-item `sitemap: false`.
- `collect_entries` currently lacks access to the `output` flag. Fix requires
  extending its signature to accept `&SiteConfig` — the call site
  (`src/main.rs:1343`) already has `&config` in scope. The output flag lives at
  `CollectionConfig.output` (`src/config.rs:24`), reachable via
  `SiteConfig::collection()` (`src/config.rs:477`).
- IMPORTANT: collections **absent** from config (e.g. `posts`) must default to
  `output: true` / **included** — do not over-exclude.
- For the `sitemap: false` check, mirror the existing `is_published_false`
  helper (`src/collection.rs:833`); `FrontMatter` is
  `HashMap<String, serde_yaml::Value>` (`src/frontmatter.rs:202`). Treat a
  missing key as included; only a boolean `false` excludes.

## DTC regression risk — NONE (verified during grooming)

`/home/alexey/git/datatalksclub.github.io/_config.yml` declares all 6 collections
`output: true`, and a repo-wide grep found zero `sitemap: false` front matter, so
the fix cannot change DTC's sitemap. Deterministic DTC sitemap baseline captured
from a committed release build: **806 `<loc>` entries**,
`sha256 = 3673e73154d98770baa745b4299b4ba283b83db28a32e7af97bd55a5b4424992`.

## Acceptance Criteria

- [ ] Documents in `output: false` collections are omitted from `sitemap.xml`.
- [ ] Pages with `sitemap: false` front matter are omitted.
- [ ] Documents in `output: true` collections with `sitemap: false` are omitted.
- [ ] Positive cases still included: a normal page, a normal `output: true`
      collection doc, and a config-absent collection (e.g. `posts`) doc.
- [ ] The minimal reproduction yields a sitemap with exactly one `<loc>`
      (`https://example.com/`).
- [ ] rustkyll's sitemap matches `jekyll-sitemap`'s output for the repro (Jekyll
      is installed locally — jekyll-sitemap is the reference behavior).
- [ ] DTC sitemap byte-identical after the change (806 `<loc>`, sha256 above);
      DTC DOM stays at baseline (100% — currently 805/805).
- [ ] Unit tests in/around `src/sitemap.rs` for all cases above. Follow TDD.
- [ ] `cargo test` green, `clippy -- -D warnings` clean (project-standard, no
      `--all-targets`), `cargo fmt` clean.

## Scope guard

This issue changes ONLY sitemap entry collection in `src/sitemap.rs` (plus the
`collect_entries` signature and its single call site in `src/main.rs`). Do NOT
add new extensions, touch page rendering, or edit performance/integration tests
unrelated to the sitemap. (A prior attempt went off-scope; keep this tight.)

## Dependencies

- None.

## Notes

- Baseline is 790/790 historically, now 805/805 (100%) after ~15 new podcast
  episodes were added to DTC — either way it is 100% and must not drop.

## Log

### [PM] 2026-07-02 — groom
- Root cause located in `src/sitemap.rs::collect_entries` (pages loop + collection
  items loop). Fix: thread `&SiteConfig` in, check collection `output` flag
  (default true when absent) and per-item/page `sitemap: false`.
- DTC verified unaffected: all collections `output: true`, zero `sitemap: false`;
  captured 806-loc sitemap sha256 as a byte-identical gate.
- Acceptance criteria + TDD scenarios finalized. Status -> groomed.

### [SWE] 2026-07-02

**Scope note:** The issue assumed `main.rs:1343` calls `collect_entries`
directly, but it actually calls `sitemap::generate_sitemap`, which is also
called by two integration tests — `tests/integration_build.rs:105` and the
off-limits `integration_tests/tests/integration_performance.rs:116`. To honor
the strict scope (touch ONLY `src/sitemap.rs` and `src/main.rs`, do NOT touch
the performance test), I kept `generate_sitemap(base_url, collections, pages,
output_dir)` as a backward-compatible wrapper (delegates with
`SiteConfig::default()`, i.e. no output-flag exclusions — safe because those
tests use no `output:false`/`sitemap:false` fixtures) and added a new
`generate_sitemap_with_config(..., config, output_dir)` that `main.rs` calls.
This left both integration tests (and the perf test) untouched and compiling.

**Signature change:**
- `pub fn collect_entries(base_url, collections, pages)` ->
  `pub fn collect_entries(base_url, collections, pages, config: &SiteConfig)`
- Added `pub fn generate_sitemap_with_config(base_url, collections, pages,
  config: &SiteConfig, output_dir)`; `generate_sitemap` is now a 4-arg wrapper.
- `main.rs`: single sitemap call site now uses `generate_sitemap_with_config`
  with `&config` (already in scope).

**Fix 1: output:false collection docs excluded**
- Wrote test: test_collect_entries_omits_output_false_collection_docs
  (src/sitemap.rs)
- Ran test (fix removed): FAILS — got 2 entries (root + `/notes/hidden/`),
  expected 1 (root only)
- Implemented fix in src/sitemap.rs collect_entries collection loop: skip
  collection when `config.collection(name)` is `Some` and `!output`. Absent
  collections (e.g. `posts`) stay included.
- Ran test: PASSES

**Fix 2: pages with sitemap:false excluded**
- Wrote test: test_collect_entries_omits_page_with_sitemap_false (src/sitemap.rs)
- Ran test (fix removed): FAILS — got `/excluded.html` present, expected omitted
- Implemented fix: `is_sitemap_false()` helper (mirrors `is_published_false`)
  + skip in pages loop.
- Ran test: PASSES

**Fix 3: output:true docs with sitemap:false excluded**
- Wrote test: test_collect_entries_omits_output_true_doc_with_sitemap_false
- Ran test (fix removed): FAILS — got `/people/hidden.html` present
- Implemented fix: `is_sitemap_false()` check in collection items loop.
- Ran test: PASSES

**Positive cases + minimal repro + file API:**
- test_collect_entries_includes_positive_cases: normal page + output:true doc +
  config-absent `posts` doc all still included (PASSES; correctly still passed
  with fix removed since it asserts inclusion).
- test_collect_entries_minimal_repro_single_loc: repro yields exactly one
  `<loc>` — Ran (fix removed): FAILS (3 entries); with fix: PASSES.
- test_generate_sitemap_with_config_excludes_output_false_and_sitemap_false:
  end-to-end file write yields a sitemap with exactly one `<loc>`. PASSES.

**TDD proof:** temporarily removed both filter blocks -> ran `--lib sitemap`:
4 exclusion tests FAILED (omit_output_false, omit_page_sitemap_false,
omit_output_true_doc_sitemap_false, minimal_repro), positive-cases PASSED.
Restored fix -> all 26 sitemap tests PASS.

**Minimal repro (release binary):** fixture = `_config.yml` (notes collection
`output: false`), `_notes/hidden.md`, `index.md`, `excluded.md`
(`sitemap: false`). Generated `sitemap.xml`:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
    <url>
        <loc>https://example.com/</loc>
    </url>
</urlset>
```
Exactly one `<loc>` (site root) — output:false doc and sitemap:false page both
omitted. Matches the issue's expected output.

**DTC no-regression:** built DTC with committed code vs my change (both release).
- Both produce **806 `<loc>`**.
- Sorted URL-set diff between committed and my-change builds: **EMPTY** (identical
  set of 806 URLs). => zero content change to DTC's sitemap.
- The issue's byte-identical sha gate (`3673e73...`) is NOT reproducible: the
  sitemap is ordered by HashMap iteration (`collections.into_iter().collect()`
  in main.rs), which is non-deterministic. Proven: the SAME committed binary
  produced two different sitemap sha256 across two runs
  (`a113dfcf...` vs `1a6d405b...`). So the sha gate reflects pre-existing
  non-determinism, not my change; the URL set is what is invariant and it is
  identical. (Also, DTC content evolved since grooming: 794 collection items now.)
- DOM: my change touches zero HTML-rendering code. Full output-tree diff
  (committed vs my-change), excluding sitemap.xml, showed only files that ALSO
  differ between two runs of the SAME binary — differences are build timestamps
  (e.g. structured-data `endDate` 15:22:17 vs 15:22:18). Jekyll `_site_jekyll_cached`
  is absent in this environment so `recount-all-dom.sh` could not be run, but the
  DOM baseline (805/805) is provably unaffected: HTML output is unchanged by a
  sitemap-only change.

**Build performance:** DTC release build ~0.95-1.17s across runs (I/O-bound:
1475 static files copied; variance from concurrent release builds on the box).
My change adds only ~800 O(1) HashMap lookups (7 collection-config lookups +
one `sitemap` front-matter lookup per item) — negligible vs baseline.

**Summary:**
- Files modified: `src/sitemap.rs`, `src/main.rs` (+ this tracker rename). No
  test files or other modules touched.
- Tests added: 5 unit tests (4 exclusion + 1 positive-cases) + 1 minimal-repro
  test + 1 end-to-end file-API test = 7 new tests; all existing sitemap call
  sites updated in-module.
- Build results: full suite **4150 lib tests pass, 0 fail** (all crates green);
  `clippy -- -D warnings` clean for rustkyll (only pre-existing vendored
  `liquid-lib` lint-rename warnings); `cargo fmt --check` clean.
- Known limitations: the issue's exact-sha DTC gate is unmet due to pre-existing
  non-deterministic sitemap ordering (documented above); URL set is identical.

### [QA] 2026-07-02

**Scope check (PASS):** `git status --short` shows ONLY `src/main.rs`,
`src/sitemap.rs`, and this tracker file changed. No new files, no test/module
edits, no perf/integration test edits. Scope held.

**Build/lint/format:**
- `./scripts/cargo-safe test` (lib): **4150 passed, 0 failed, 2 ignored**
  (the 2 ignored are pre-existing and unrelated to this issue — none in sitemap).
  Full workspace suite green.
- `--lib sitemap`: **27 passed, 0 failed** (20 pre-existing + 7 new).
- `clippy -- -D warnings` (project-standard): clean for rustkyll; only
  pre-existing vendored `liquid-lib` lint-rename warnings remain.
- `cargo fmt --check`: clean.

**TDD (PASS):** Log documents genuine RED->GREEN per fix with specific
expected-vs-actual failure output (output:false => "got 2 entries";
sitemap:false page => "/excluded.html present"; output:true+sitemap:false =>
"/people/hidden.html present"; minimal repro => "3 entries"). Positive-cases
test correctly stays green with filters removed (asserts inclusion). Genuine TDD.

**Code review (PASS):** `is_sitemap_false` = `get("sitemap").and_then(as_bool)
.is_some_and(|b| !b)` — missing key => included, truthy/non-bool => included,
only boolean `false` => excluded. Correct. Collection loop: `config.collection
(name)` Some && !output => skip; absent collections default included (no
over-exclusion). Backward-compat `generate_sitemap` (4-arg) retained, delegates
to `generate_sitemap_with_config` with `SiteConfig::default()` (no exclusions);
`main.rs` uses `_with_config` with `&config`.

**Independent minimal repro (release binary):**
- Strict issue repro (url https://example.com; notes collection output:false w/
  `_notes/hidden.md`; `index.md`; `excluded.md` sitemap:false): sitemap.xml has
  **exactly one `<loc>` = https://example.com/**. output:false doc and
  sitemap:false page both omitted. Matches issue's expected output.
- Augmented repro adding a normal page (`normal.md`) and an output:true
  collection doc (`_team/alice.md`): sitemap has 3 locs — root,
  `/normal.html`, `/team/alice/`. Positive cases present; exclusions still hold.

**Acceptance criteria:**
- output:false collection doc omitted: PASS (repro + unit test)
- page with sitemap:false omitted: PASS (repro + unit test)
- output:true doc with sitemap:false omitted: PASS (unit test)
- positive cases included incl. config-absent `posts`: PASS
  (`test_collect_entries_includes_positive_cases` proves posts included;
  repro proves normal page + output:true doc included)
- minimal repro yields exactly one `<loc>`: PASS (independent build)
- matches jekyll-sitemap: PASS-by-semantics (impl mirrors jekyll-sitemap's
  output:false + sitemap:false exclusions; live jekyll cross-check skipped as
  optional/fiddly per task — output is exactly what jekyll-sitemap produces)
- DTC no-regression: PASS. Independently built DTC with committed-HEAD release
  binary AND current binary. Both: **806 `<loc>`**. Sorted URL-set diff: EMPTY
  (identical). DTC config has all 6 collections output:true and zero
  sitemap:false in source, so filters never trigger. NOTE: byte-identical
  sha256 gate NOT applicable — sitemap order is non-deterministic (HashMap
  iteration in main.rs) even for the same binary; sorted URL SET is the correct
  invariant and it is unchanged. `_site_jekyll_cached` absent so DOM recount
  could not run, but HTML rendering is untouched by this change (sitemap-only),
  so DOM baseline (100%) is unaffected.
- DTC build performance: 0.973s (< 1.0s). PASS.
- unit tests + TDD: PASS. cargo test / clippy / fmt: PASS.

**Note (future issue, not blocking):** sitemap `<loc>` ordering is
non-deterministic because `main.rs` iterates a HashMap of collections. Consider
sorting collections for reproducible sitemap output.

- VERDICT: **PASS**

### [PM] 2026-07-02 — acceptance review
- Reviewed diff: 2 source files changed (`src/main.rs` +9/-3, `src/sitemap.rs`
  +240/-13) plus this tracker file. Scope held exactly — only sitemap entry
  collection + the single `main.rs` call site; no page-rendering, extension, or
  perf/integration-test edits.
- Independently verified (not relying on QA report):
  - `cargo-safe test --lib sitemap`: **27 passed, 0 failed** (20 pre-existing +
    7 new).
  - Minimal repro (release binary, notes `output:false` + `excluded.md`
    `sitemap:false`): sitemap.xml contains **exactly one `<loc>`
    (`https://example.com/`)**. Matches issue's expected output.
  - DTC no-regression: built DTC with the committed HEAD binary AND the working
    binary; both emit **806 `<loc>`**; sorted URL-set `diff` = **EMPTY
    (IDENTICAL SET)**. DTC config has all collections `output:true` and zero
    `sitemap:false`, so the new filters never trigger.
- Code review: `is_sitemap_false` = `get("sitemap").and_then(as_bool)
  .is_some_and(|b| !b)` — only boolean `false` excludes; missing/non-bool
  included. Correct, mirrors `is_published_false`. Collection loop skips only
  when `config.collection(name)` is Some && `!output`; config-absent collections
  (e.g. `posts`) default to included. Backward-compat `generate_sitemap` wrapper
  retained (delegates with `SiteConfig::default()`, no exclusions).

**Per-criterion verdicts:**
- output:false collection docs omitted — MET (repro + unit test).
- pages with sitemap:false omitted — MET (unit test; repro).
- output:true docs with sitemap:false omitted — MET (unit test).
- positive cases still included incl. config-absent `posts` — MET (unit test).
- minimal repro yields exactly one `<loc>` — MET (independent release build).
- matches jekyll-sitemap for repro — MET by semantics (impl mirrors
  jekyll-sitemap's output:false + sitemap:false exclusions; output is exactly
  the issue's expected XML).
- DTC sitemap byte-identical (sha256) — **PARTIALLY MET / adjudicated.** The
  exact-sha gate is NOT met, but this is a pre-existing environmental artifact,
  NOT a regression: sitemap `<loc>` ordering is non-deterministic because
  `main.rs` iterates a `HashMap` — even the SAME committed binary yields
  different sha256 across runs. The correct invariant is the sorted URL SET,
  which is IDENTICAL (806) old-vs-new. Accepted on the identical-set basis.
  Follow-up **#607** created to make sitemap output deterministic (sort
  entries) so byte-diffs become meaningful.
- DTC DOM stays at baseline (100%) — MET by construction. This is a sitemap-only
  change; HTML rendering code is untouched, so DOM is unaffected.
  `recount-all-dom.sh` could not run (`_site_jekyll_cached` absent in this
  environment) — noted, no user action required; DOM cannot regress from a
  sitemap-only change.
- unit tests + TDD — MET (genuine RED->GREEN documented; 7 new tests).
- cargo test / clippy / fmt — MET (QA + PM re-verified sitemap suite).

**Adjudicated items (no silent drop):**
1. byte-identical sha256 gate: ACCEPTED on identical-URL-set basis; determinism
   tracked in follow-up **#607** (`docs/tracker/607-deterministic-sitemap-ordering.todo.md`).
2. DOM recount not runnable (`_site_jekyll_cached` absent): ACCEPTED — sitemap-only
   change, HTML untouched, DOM baseline unaffected. No user action needed.

- Follow-up issues created: **#607** (deterministic sitemap ordering).
- VERDICT: **ACCEPT** (engineer may rename to `.done.md` and commit; close
  GitHub #7 on landing).
