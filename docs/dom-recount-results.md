# DOM Comparison Results

Generated: 2026-03-20 17:55 UTC

rustkyll version: rustkyll 0.2.3

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

| Site | DOM Match | File Match | Liquid Leaks |
|------|-----------|------------|-------------|
| DataTalksClub/datatalksclub.github.io | 541/787 (69%) | 787/787 | 1 |

## Summary

- Sites compared: 1
- Total DOM matches: 541 / 787

## Diff Categories by Site

### DataTalksClub/datatalksclub.github.io

```
    604 jsonld_value_differs
    322 missing_element
    194 expected_element_got_text
     78 text_differs
     45 attribute_differs
     40 tag_name_differs
     37 extra_element
     23 missing_text
     13 extra_text
      8 missing_attribute
      5 expected_text_got_element
```


Per-site full diff output is in `docs/comparison/dom-details/`.
