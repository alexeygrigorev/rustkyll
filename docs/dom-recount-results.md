# DOM Comparison Results

Generated: 2026-03-18 16:20 UTC

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
| opensource-guide | 23/388 (6%) | 390/388 | 0 |

## Summary

- Sites compared: 1
- Total DOM matches: 23 / 388

## Diff Categories by Site

### opensource-guide

```
   1121 attribute_differs
   1059 missing_element
    554 tag_name_differs
    307 extra_attribute
    293 extra_text
    163 expected_element_got_text
     53 text_differs
      2 expected_text_got_element
      1 missing_text
```


Per-site full diff output is in `docs/comparison/dom-details/`.
