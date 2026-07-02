# Issue 607: Make sitemap.xml output deterministic (sort entries)

Follow-up from #606 (sitemap output:false / sitemap:false exclusions).

## Problem

The generated `sitemap.xml` `<loc>` ordering is non-deterministic: `src/main.rs`
builds `collections_vec` by iterating a `HashMap` (`collections.into_iter()
.collect()`), so collection order — and therefore `<loc>` order in the sitemap —
varies between builds of the *same* binary. During #606 acceptance, two runs of
the identical committed binary produced two different sitemap `sha256` values
while the sorted URL *set* was identical.

Consequences:
- Byte-diff / sha256 gates on `sitemap.xml` (as attempted in #606) are
  meaningless because output is not reproducible.
- Reproducible builds and diff-based regression checks cannot rely on
  `sitemap.xml`.

## Scope

Make sitemap entry ordering deterministic so identical inputs produce a
byte-identical `sitemap.xml`. Likely approaches (SWE to choose during grooming):
- Sort collections by name before iterating in `src/main.rs`, and/or
- Sort the final `SitemapEntry` list (e.g. by `loc`) in
  `src/sitemap.rs::collect_entries`.

Keep the change minimal and confined to sitemap ordering. Do NOT change which
URLs are included (the #606 exclusion semantics must be preserved).

## Acceptance Criteria

- [ ] Two consecutive builds of the same source produce a byte-identical
      `sitemap.xml` (stable sha256).
- [ ] The set of `<loc>` entries is unchanged from #606 behavior (DTC still
      806 URLs; the #606 exclusions still apply).
- [ ] Ordering is well-defined and documented (e.g. sorted by `loc`).
- [ ] DTC DOM baseline unaffected (sitemap-only; HTML rendering untouched).
- [ ] Unit test asserting deterministic/sorted ordering in `src/sitemap.rs`.
- [ ] `cargo test` green, `clippy -- -D warnings` clean, `cargo fmt` clean.

## Dependencies

- #606 (should land first; this builds on the same code path).

## Notes

- Once deterministic, a byte-identical DTC sitemap sha256 gate becomes a valid
  regression check and can be re-introduced.
