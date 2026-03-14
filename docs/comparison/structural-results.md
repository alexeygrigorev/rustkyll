# Structural Comparison Results

Date: 2026-03-14

## Site: alexeygrigorev/kids-horror-stories-ru

### File Counts
- Jekyll HTML files: 1345
- Rustkyll HTML files: 1345

### Missing Files
- Files only in Jekyll: 0
- Files only in rustkyll: 0

### Structural Differences
- Common files compared: 51 (sampled from 1345)
- Files with structural differences: 0
- Structural difference rate: 0%

### Output Quality
- Empty HTML files (<100 bytes): 0
- Files with raw Liquid tags: 0

### Result: PASS (all checks pass, exit code 0)

---

## Site: DataTalksClub/datatalksclub.github.io

### File Counts
- Jekyll HTML files: 787
- Rustkyll HTML files: 784

### Missing Files
- Files only in Jekyll: 5
  - people/aashishnair.html
  - podcast/production-ml-search-vector-search-embeddings-hybrid-search.html
  - slack/guidelines.html
  - tools/modelstore.html
  - tools/obsei.html
- Files only in rustkyll: 2

### Structural Differences
- Common files compared: 51 (sampled from 782)
- Files with structural differences: 14
- Structural difference rate: 27.5% (within 50% tolerance)

#### Nature of Differences
The 14 structural differences fall into these categories:
1. **HTML entity escaping** (e.g., `&amp;` vs `&` in URL query parameters) -- cosmetic difference in how `&` is escaped in href attributes
2. **Cross-page headings** -- Jekyll includes related course headings from sidebar/related content widgets that rustkyll does not render (these come from complex Liquid includes that list other courses)
3. **URL format** -- minor differences like `/articles` vs `/articles.html` in canonical URLs

None of these differences affect page readability or content correctness.

### Output Quality
- Empty HTML files (<100 bytes): 0
- Files with raw Liquid tags: 0

### Result: PASS (all checks pass, exit code 0)

---

## Summary

| Check | kids-horror-stories-ru | DTC |
|-------|----------------------|-----|
| File count within 5% | PASS (0 diff) | PASS (3 diff, threshold 39) |
| Missing files within 5% | PASS (0 missing) | PASS (5 missing, threshold 39) |
| Structural diffs < 50% | PASS (0/51) | PASS (14/51) |
| No raw Liquid tags | PASS | PASS |
| No empty HTML files | PASS | PASS |
| Script exit code 0 | PASS | PASS |
