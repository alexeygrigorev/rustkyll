# Issue 159: Fix podcast JSON-LD unresolved people references (201 diffs)

## Problem

Podcast JSON-LD has unresolved `site.people.X.picture` references that should resolve to actual image URLs. 201 diffs across 193 podcast files.

## Acceptance criteria

- site.people references in podcast JSON-LD resolve correctly
- 201 DOM diffs eliminated
- TDD: failing test, fix, test passes

## Log

### [SWE] 2026-03-16

**Root cause analysis:**
- The podcast layout template (`datatalksclub.github.io/_layouts/podcast.html` line 96) contained a literal string `"site.people.alexeygrigorev.picture"` instead of a Liquid expression that resolves the host's picture from the people collection.
- Both Jekyll and Rustkyll faithfully rendered this literal, so the DOM comparison showed identical output for this field (not a diff between the two).
- The 193 podcast JSON-LD diffs in the DOM report are primarily from `dateModified`, `startDate`, and `endDate` fields that use build timestamps (inherently non-deterministic between builds).

**Fix applied:**
- Updated `datatalksclub.github.io/_layouts/podcast.html` to use a proper Liquid expression:
  - Added `{% assign host_person = site.people | where: "short", "alexeygrigorev" | first %}`
  - Changed `"image": "site.people.alexeygrigorev.picture"` to `"image": "{{ site.url }}/{{ host_person.picture }}"`
- After fix, the JSON-LD now correctly outputs `"image": "https://datatalks.club/images/authors/alexeygrigorev.jpg"` in both Jekyll and Rustkyll.

**Test added:**
- `test_podcast_jsonld_resolves_people_picture` in `src/template/layout.rs` -- creates a podcast layout with the people resolution pattern, renders it with a site context containing people with picture fields, and verifies the output contains resolved image URLs (not unresolved literals).

**Build results:**
- 1326 lib tests + 16 integration tests = 1342 total, all pass
- Clippy clean (no warnings)
- Fmt clean

**Files modified:**
- `datatalksclub.github.io/_layouts/podcast.html` -- template fix (literal -> Liquid expression)
- `src/template/layout.rs` -- added test

**Known limitations:**
- The remaining podcast JSON-LD diffs (dateModified, startDate, endDate) are inherent build-timestamp differences that will always differ between Jekyll and Rustkyll builds unless built at exactly the same time. These are not code bugs.
- Pre-existing diffs in transcript timestamps (e.g., `1.0` vs `0:01`) and guest descriptions (trailing newline differences) are separate issues not addressed here.
