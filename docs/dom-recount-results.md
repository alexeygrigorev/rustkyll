# DOM Comparison Results

Generated: 2026-03-18 12:59 UTC

rustkyll version: rustkyll 0.2.2

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
| academicpages | 1/17 (6%) | 17/45 | 0 |
| alexeygrigorev/aihero | 0/2 (0%) | 2/2 | 0 |
| alexeygrigorev/alexeygrigorev.github.io | 7/8 (88%) | 8/8 | 0 |
| alexeygrigorev/data-science-interviews | 0/0 (N/A%) | 0/6 | 0 |
| alexeygrigorev/kids-horror-stories-ru | 1342/1344 (100%) | 1344/1345 | 0 |
| alexeygrigorev/little-book-of-metals-ru | 37/43 (86%) | 43/48 | 0 |
| alexeygrigorev/mlbookcamp-page | 7/15 (47%) | 15/15 | 0 |
| alexeygrigorev/mlwiki.org | 214/639 (33%) | 640/639 | 5 |
| alexeygrigorev/snippets | 8/25 (32%) | 25/25 | 0 |
| architect-theme | 1/2 (50%) | 2/2 | 0 |
| beautiful-jekyll | 0/5 (0%) | 6/6 | 3 |
| bitcoin-org | JEKYLL_FAIL | - | - |
| cayman-theme | 1/2 (50%) | 2/2 | 0 |
| choosealicense.com | 15/72 (21%) | 72/72 | 0 |
| DataTalksClub/courses | 5/5 (100%) | 5/5 | 0 |
| DataTalksClub/datatalksclub.github.io | 520/787 (66%) | 787/787 | 1 |
| DataTalksClub/docs | 0/57 (0%) | 57/57 | 33 |
| dinky-theme | 0/2 (0%) | 2/2 | 0 |
| documentation-theme-jekyll | 1/100 (1%) | 100/100 | 39 |
| edition-template | JEKYLL_FAIL | - | - |
| government-github | 1/21 (5%) | 21/21 | 4 |
| hacker-theme | 0/2 (0%) | 2/2 | 0 |
| homebrew-site | JEKYLL_FAIL | - | - |
| hyde | JEKYLL_FAIL | - | - |
| jekyll-docs/docs | 14/125 (11%) | 131/228 | 44 |
| jekyll-docs/lib/blank_template | 1/1 (100%) | 1/1 | 0 |
| jekyll-docs/lib/site_template | JEKYLL_FAIL | - | - |
| just-the-docs | 0/47 (0%) | 47/47 | 18 |
| large-blog-3000 | 3000/3001 (100%) | 3001/3001 | 0 |
| large-docs-site | 801/801 (100%) | 801/801 | 0 |
| leap-day-theme | 0/2 (0%) | 2/2 | 0 |
| made-mistakes-jekyll | JEKYLL_FAIL | - | - |
| merlot-theme | 0/2 (0%) | 2/2 | 0 |
| midnight-theme | 0/2 (0%) | 2/2 | 0 |
| minima | JEKYLL_FAIL | - | - |
| minimal-mistakes | 0/0 (N/A%) | 1/1 | 0 |
| mojombo-blog | 14/17 (82%) | 17/17 | 0 |
| muan-blog | 29/2218 (1%) | 2218/2218 | 6 |
| opensource-guide | 23/388 (6%) | 390/388 | 0 |
| primer-theme | 0/2 (0%) | 2/2 | 0 |
| programming-historian | JEKYLL_FAIL | - | - |
| slate-theme | 1/2 (50%) | 2/2 | 0 |
| so-simple-theme | 0/11 (0%) | 11/66 | 1 |
| time-machine-theme | 0/2 (0%) | 2/2 | 0 |
| uswds-site | JEKYLL_FAIL | - | - |
| wtf-html-css | JEKYLL_FAIL | - | - |

## Summary

- Sites compared: 36
- Total DOM matches: 6043 / 9769

## Diff Categories by Site

### slate-theme

```
      6 attribute_differs
      4 text_differs
```

### hacker-theme

```
      8 attribute_differs
      3 text_differs
```

### primer-theme

```
      8 attribute_differs
      3 text_differs
      1 missing_element
```

### alexeygrigorev/little-book-of-metals-ru

```
     10 attribute_differs
      5 missing_element
```

### midnight-theme

```
     18 extra_element
      2 missing_element
```

### cayman-theme

```
      6 attribute_differs
      4 text_differs
```

### DataTalksClub/datatalksclub.github.io

```
   1736 missing_element
    105 jsonld_value_differs
     59 text_differs
     59 tag_name_differs
     39 extra_element
     35 attribute_differs
     20 missing_text
     15 extra_text
     15 expected_element_got_text
     13 expected_text_got_element
      8 missing_attribute
```

### DataTalksClub/docs

```
    404 extra_element
     57 tag_name_differs
```

### large-blog-3000

```
      7 text_differs
      3 attribute_differs
```

### just-the-docs

```
    272 extra_element
     47 tag_name_differs
```

### alexeygrigorev/kids-horror-stories-ru

```
      2 text_differs
```

### alexeygrigorev/mlwiki.org

```
   1058 text_differs
    991 tag_name_differs
    401 missing_element
    230 missing_text
    183 expected_element_got_text
    121 attribute_differs
    100 extra_element
     74 extra_text
     68 expected_text_got_element
```

### leap-day-theme

```
      8 attribute_differs
      3 text_differs
```

### merlot-theme

```
      8 attribute_differs
      3 text_differs
```

### alexeygrigorev/mlbookcamp-page

```
     37 attribute_differs
      9 text_differs
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
      8 attribute_differs
      3 text_differs
```

### alexeygrigorev/alexeygrigorev.github.io

```
      8 text_differs
      2 attribute_differs
```

### muan-blog

```
  11315 attribute_differs
   1339 text_differs
    152 missing_element
     98 missing_text
     43 extra_attribute
     36 extra_element
     27 tag_name_differs
      9 expected_element_got_text
      8 expected_text_got_element
      7 missing_attribute
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

### documentation-theme-jekyll

```
    393 tag_name_differs
    135 text_differs
    133 extra_element
     91 attribute_differs
     76 missing_attribute
     19 missing_element
     18 extra_attribute
      7 extra_text
      4 missing_text
      3 expected_text_got_element
```

### architect-theme

```
      6 attribute_differs
      4 text_differs
```

### opensource-guide

```
   2589 extra_element
    689 tag_name_differs
     98 missing_element
     87 attribute_differs
      1 text_differs
```

### alexeygrigorev/snippets

```
     61 extra_element
     34 tag_name_differs
```

### government-github

```
     30 attribute_differs
     23 tag_name_differs
     21 missing_element
     11 extra_element
     10 text_differs
```

### academicpages

```
     89 attribute_differs
     23 tag_name_differs
     12 text_differs
      8 extra_element
      2 missing_attribute
      1 missing_text
      1 missing_element
```

### choosealicense.com

```
    211 jsonld_value_differs
    179 attribute_differs
     44 text_differs
     44 jsonld_missing_field
     24 missing_element
      8 tag_name_differs
      2 missing_attribute
      2 extra_element
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
     18 extra_element
      2 missing_element
```

### jekyll-docs/docs

```
    742 attribute_differs
    212 extra_attribute
    106 missing_attribute
     20 missing_element
      7 extra_element
      2 tag_name_differs
      2 expected_element_got_text
```


Per-site full diff output is in `docs/comparison/dom-details/`.
