# Issue 104: Fix JSON-LD date metadata (D14, D15)

Descoped from issue #90. JSON-LD structured data uses different date values than Jekyll:
- D14: datePublished format differs
- D15: dateModified format differs

Low priority — no user-visible impact, only affects search engine metadata.

## Acceptance criteria
- JSON-LD datePublished matches Jekyll format
- JSON-LD dateModified matches Jekyll format
- Structured data validates with Google's testing tool
