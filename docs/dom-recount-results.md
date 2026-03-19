# DOM Comparison Results

Generated: 2026-03-19 10:37 UTC

rustkyll version: rustkyll 0.2.3

## How to run

```bash
# Recount all sites
./scripts/recount-all-dom.sh

# Recount a single site
./scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io
```

Prerequisites: Jekyll (via Ruby/Bundler), rustkyll (built via `cargo build --release`), and `uv` (for running dom_compare.py with its beautifulsoup4 dependency).

The script builds both Jekyll and rustkyll for each site, runs DOM comparison via `scripts/dom_compare.py`, and writes results here. Per-site diff details are saved in `docs/comparison/dom-details/`.

## All Sites

| Site | DOM Match | File Match | Liquid Leaks |
|------|-----------|------------|-------------|
| alexeygrigorev/little-book-of-metals-ru | 38/43 (88%) | 43/48 | 0 |

## Summary

- Sites compared: 1
- Total DOM matches: 38 / 43

## Diff Categories by Site

### alexeygrigorev/little-book-of-metals-ru

```
      5 missing_element
```


Per-site full diff output is in `docs/comparison/dom-details/`.
