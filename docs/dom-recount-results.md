# DOM Comparison Results

Generated: 2026-04-03 23:12 UTC

rustkyll version: rustkyll 0.3.0

## How to run

```bash
# Recount all sites
./scripts/recount-all-dom.sh

# Recount a single site
./scripts/recount-all-dom.sh --site DataTalksClub/datatalksclub.github.io

# Force Jekyll rebuild (clear cache)
./scripts/recount-all-dom.sh --no-cache
```

Prerequisites: Jekyll (via Ruby/Bundler), rustkyll (built via `cargo build --release`), and `uv` (for running dom_compare.py with its beautifulsoup4 dependency).

The script builds both Jekyll and rustkyll for each site, runs DOM comparison via `scripts/dom_compare.py`, and writes results here. Per-site diff details are saved in `docs/comparison/dom-details/`.

Jekyll output is deterministic and cached in `_site_jekyll_cached/` per site directory. Only rustkyll output is rebuilt each time. Use `--no-cache` to force a Jekyll rebuild.

## All Sites

| Site | DOM Match | Common / Total | Only-Jekyll | Only-Rustkyll | Liquid Leaks |
|------|-----------|----------------|-------------|---------------|-------------|
| DataTalksClub/datatalksclub.github.io | 596/790 (75%) | 790 / 790 | 0 | 0 | 1 |

## Summary

- Sites compared: 1
- Total DOM matches: 596 / 790

## Diff Categories by Site

### DataTalksClub/datatalksclub.github.io

```
    133 jsonld_value_differs
    122 jsonld_missing_field
```


Per-site full diff output is in `docs/comparison/dom-details/`.
