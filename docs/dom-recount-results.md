# DOM Comparison Results

Generated: 2026-03-20 22:45 UTC

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
| academicpages | 1/17 (6%) | 17/45 | 0 |
| alexeygrigorev/aihero | 0/2 (0%) | 2/2 | 0 |
| alexeygrigorev/alexeygrigorev.github.io | 8/8 (100%) | 8/8 | 0 |
| alexeygrigorev/data-science-interviews | 0/0 (N/A%) | 0/6 | 0 |
| alexeygrigorev/kids-horror-stories-ru | 1344/1344 (100%) | 1344/1345 | 0 |
| alexeygrigorev/little-book-of-metals-ru | 38/43 (88%) | 43/48 | 0 |
| alexeygrigorev/mlbookcamp-page | 6/15 (40%) | 15/15 | 0 |
| alexeygrigorev/mlwiki.org | 236/639 (37%) | 640/639 | 0 |
| alexeygrigorev/snippets | 8/25 (32%) | 25/25 | 0 |
| architect-theme | 1/2 (50%) | 2/2 | 0 |
| beautiful-jekyll | 0/5 (0%) | 6/6 | 3 |
| bitcoin-org | BOTH_FAIL | - | - |
| cayman-theme | 1/2 (50%) | 2/2 | 0 |
| chirpy | 0/13 (0%) | 17/17 | 1 |
| choosealicense.com | 17/72 (24%) | 72/72 | 0 |
| DataTalksClub/courses | 5/5 (100%) | 5/5 | 0 |
| DataTalksClub/datatalksclub.github.io | 543/787 (69%) | 787/787 | 1 |
| DataTalksClub/docs | 56/57 (98%) | 57/57 | 0 |
| dinky-theme | 0/2 (0%) | 2/2 | 0 |
| documentation-theme-jekyll | 1/100 (1%) | 100/100 | 25 |
| edition-template | JEKYLL_FAIL | 13 pages (rustkyll-only) | 0 |
| government-github | 1/21 (5%) | 21/21 | 0 |
| hacker-theme | 0/2 (0%) | 2/2 | 0 |
| homebrew-site | JEKYLL_FAIL | 134 pages (rustkyll-only) | 47 |
| hyde | JEKYLL_FAIL | 6 pages (rustkyll-only) | 1 |
| jekyll-docs/docs | 14/125 (11%) | 131/228 | 43 |
| jekyll-docs/lib/blank_template | 1/1 (100%) | 1/1 | 0 |
| jekyll-docs/lib/site_template | JEKYLL_FAIL | 1 pages (rustkyll-only) | 0 |
| jekyll-theme-chirpy | 0/13 (0%) | 17/17 | 3 |
| jekyll-vitepress-theme | 0/17 (0%) | 17/17 | 2 |
| just-the-docs | 0/47 (0%) | 47/47 | 4 |
| lanyon | 4/6 (67%) | 6/6 | 1 |
| large-blog-3000 | 3001/3001 (100%) | 3001/3001 | 0 |
| large-docs-site | 801/801 (100%) | 801/801 | 0 |
| leap-day-theme | 0/2 (0%) | 2/2 | 0 |
| made-mistakes-jekyll | JEKYLL_FAIL | 2 pages (rustkyll-only) | 0 |
| merlot-theme | 0/2 (0%) | 2/2 | 0 |
| midnight-theme | 0/2 (0%) | 2/2 | 0 |
| minima | JEKYLL_FAIL | 9 pages (rustkyll-only) | 1 |
| minimal-mistakes | 0/0 (N/A%) | 1/1 | 0 |
| mojombo-blog | 14/17 (82%) | 17/17 | 0 |
| muan-blog | 1786/2218 (81%) | 2219/2218 | 0 |
| opensource-guide | 23/388 (6%) | 390/388 | 0 |
| primer-theme | 0/2 (0%) | 2/2 | 0 |
| programming-historian | JEKYLL_FAIL | 653 pages (rustkyll-only) | 218 |
| slate-theme | 1/2 (50%) | 2/2 | 0 |
| so-simple-theme | 0/11 (0%) | 11/66 | 1 |
| time-machine-theme | 0/2 (0%) | 2/2 | 0 |
| uswds-site | JEKYLL_FAIL | 764 pages (rustkyll-only) | 228 |
| wtf-html-css | JEKYLL_FAIL | 1 pages (rustkyll-only) | 0 |

## Summary

- Sites compared: 40
- Total DOM matches: 7911 / 9818

## Diff Categories by Site

### slate-theme

```
      6 attribute_differs
      2 text_differs
      2 expected_element_got_text
```

### hacker-theme

```
      7 attribute_differs
      2 text_differs
      2 expected_element_got_text
```

### primer-theme

```
      6 attribute_differs
      3 text_differs
      2 expected_element_got_text
      1 missing_text
      1 missing_element
```

### alexeygrigorev/little-book-of-metals-ru

```
      5 missing_element
```

### midnight-theme

```
      7 attribute_differs
      2 text_differs
      2 expected_element_got_text
```

### cayman-theme

```
      6 attribute_differs
      2 text_differs
      2 expected_element_got_text
```

### DataTalksClub/datatalksclub.github.io

```
    604 jsonld_value_differs
    328 missing_element
    192 expected_element_got_text
     67 text_differs
     43 tag_name_differs
     43 attribute_differs
     39 extra_element
     25 missing_text
     15 extra_text
      9 expected_text_got_element
      8 missing_attribute
```

### DataTalksClub/docs

```
      1 text_differs
      1 tag_name_differs
      1 missing_text
      1 missing_element
```

### just-the-docs

```
    132 extra_attribute
     97 attribute_differs
     94 tag_name_differs
     88 missing_element
     34 text_differs
      3 missing_text
      3 extra_element
```

### chirpy

```
     75 attribute_differs
     18 missing_text
     14 tag_name_differs
      9 missing_element
      8 extra_element
      3 missing_attribute
      3 extra_attribute
```

### alexeygrigorev/mlwiki.org

```
   1052 text_differs
   1017 tag_name_differs
    509 missing_element
    231 missing_text
    160 attribute_differs
    107 expected_element_got_text
     66 extra_element
     48 extra_text
     31 expected_text_got_element
```

### leap-day-theme

```
      7 attribute_differs
      2 text_differs
      2 expected_element_got_text
```

### merlot-theme

```
      7 attribute_differs
      2 text_differs
      2 expected_element_got_text
```

### alexeygrigorev/mlbookcamp-page

```
     44 attribute_differs
      7 text_differs
      7 tag_name_differs
      3 missing_element
      2 extra_element
      2 expected_text_got_element
      2 expected_element_got_text
      1 missing_text
```

### so-simple-theme

```
     44 attribute_differs
     33 missing_attribute
     33 extra_attribute
```

### dinky-theme

```
      7 attribute_differs
      2 text_differs
      2 expected_element_got_text
```

### jekyll-vitepress-theme

```
     77 tag_name_differs
     52 missing_element
     37 text_differs
      1 extra_element
      1 expected_element_got_text
      1 attribute_differs
```

### muan-blog

```
    837 attribute_differs
    352 extra_attribute
     41 text_differs
     26 missing_element
     19 tag_name_differs
     15 extra_element
     14 missing_text
     10 expected_text_got_element
     10 expected_element_got_text
      4 extra_text
```

### lanyon

```
      5 tag_name_differs
      5 attribute_differs
      4 text_differs
```

### beautiful-jekyll

```
     13 extra_element
      7 tag_name_differs
      3 missing_element
```

### alexeygrigorev/aihero

```
     16 attribute_differs
      4 tag_name_differs
```

### jekyll-theme-chirpy

```
     32 extra_element
     17 missing_element
      9 tag_name_differs
      1 expected_element_got_text
```

### documentation-theme-jekyll

```
    381 tag_name_differs
    156 text_differs
    107 attribute_differs
     90 missing_attribute
     63 extra_element
     19 missing_element
     18 extra_attribute
      7 extra_text
      4 missing_text
      3 expected_text_got_element
```

### architect-theme

```
      6 attribute_differs
      2 text_differs
      2 expected_element_got_text
```

### opensource-guide

```
   1277 missing_element
   1117 attribute_differs
    646 tag_name_differs
    298 extra_text
     80 text_differs
     72 extra_attribute
     40 expected_element_got_text
     18 missing_text
      1 extra_element
```

### alexeygrigorev/snippets

```
     61 extra_element
     34 tag_name_differs
```

### government-github

```
     41 missing_element
     40 attribute_differs
     10 text_differs
      4 tag_name_differs
      3 missing_attribute
      1 jsonld_value_differs
      1 extra_attribute
```

### academicpages

```
     89 attribute_differs
     22 tag_name_differs
     12 text_differs
      6 extra_element
      2 missing_element
      2 missing_attribute
      1 missing_text
```

### choosealicense.com

```
    207 jsonld_value_differs
    118 attribute_differs
     55 missing_element
     46 text_differs
      5 missing_attribute
      3 extra_attribute
```

### mojombo-blog

```
      4 text_differs
      3 missing_element
      3 attribute_differs
      2 expected_element_got_text
      1 tag_name_differs
      1 missing_text
```

### time-machine-theme

```
      8 attribute_differs
      2 text_differs
      2 expected_element_got_text
```

### jekyll-docs/docs

```
    756 attribute_differs
    216 missing_attribute
    108 extra_attribute
     16 missing_element
      7 extra_element
      2 tag_name_differs
```


Per-site full diff output is in `docs/comparison/dom-details/`.
