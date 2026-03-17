# DOM Comparison Results

Generated: 2026-03-17 17:33 UTC

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

JSON-LD `<script>` tags are compared field-by-field (not as raw text). The `dateModified` field is excluded since it changes every build.

| Site | DOM Match | File Match | Liquid Leaks |
|------|-----------|------------|--------------|
| academicpages | 1/17 (6%) | 17/45 | 0 |
| alexeygrigorev/aihero | 0/2 (0%) | 2/2 | 0 |
| alexeygrigorev/alexeygrigorev.github.io | 7/8 (88%) | 8/8 | 0 |
| alexeygrigorev/data-science-interviews | N/A | 0/6 | 0 |
| alexeygrigorev/kids-horror-stories-ru | 1342/1344 (100%) | 1344/1345 | 0 |
| alexeygrigorev/little-book-of-metals-ru | 1/43 (2%) | 43/48 | 0 |
| alexeygrigorev/mlbookcamp-page | 6/15 (40%) | 15/15 | 0 |
| alexeygrigorev/mlwiki.org | 211/639 (33%) | 640/639 | 5 |
| alexeygrigorev/snippets | 8/25 (32%) | 25/25 | 1 |
| architect-theme | 0/2 (0%) | 2/2 | 0 |
| beautiful-jekyll | 0/5 (0%) | 6/6 | 3 |
| bitcoin-org | JEKYLL_FAIL | - | - |
| cayman-theme | 0/2 (0%) | 2/2 | 0 |
| choosealicense.com | 1/72 (1%) | 72/72 | 3 |
| DataTalksClub/courses | 5/5 (100%) | 5/5 | 0 |
| DataTalksClub/datatalksclub.github.io | 500/787 (64%) | 787/787 | 1 |
| DataTalksClub/docs | 0/57 (0%) | 57/57 | 33 |
| dinky-theme | 0/2 (0%) | 2/2 | 0 |
| documentation-theme-jekyll | 1/100 (1%) | 100/100 | 90 |
| edition-template | JEKYLL_FAIL | - | - |
| government-github | 0/21 (0%) | 21/21 | 4 |
| hacker-theme | 0/2 (0%) | 2/2 | 0 |
| homebrew-site | JEKYLL_FAIL | - | - |
| hyde | JEKYLL_FAIL | - | - |
| jekyll-docs/docs | 0/125 (0%) | 131/228 | 71 |
| jekyll-docs/lib/blank_template | 1/1 (100%) | 1/1 | 0 |
| jekyll-docs/lib/site_template | JEKYLL_FAIL | - | - |
| just-the-docs | 0/47 (0%) | 47/47 | 18 |
| large-blog-3000 | 3000/3001 (100%) | 3001/3001 | 0 |
| large-docs-site | 301/801 (38%) | 801/801 | 0 |
| leap-day-theme | 0/2 (0%) | 2/2 | 0 |
| made-mistakes-jekyll | JEKYLL_FAIL | - | - |
| merlot-theme | 0/2 (0%) | 2/2 | 0 |
| midnight-theme | 0/2 (0%) | 2/2 | 0 |
| minima | JEKYLL_FAIL | - | - |
| minimal-mistakes | N/A | 1/1 | 0 |
| mojombo-blog | 11/17 (65%) | 17/17 | 0 |
| muan-blog | 29/2218 (1%) | 2218/2218 | 7 |
| opensource-guide | 23/388 (6%) | 390/388 | 0 |
| primer-theme | RUSTKYLL_FAIL | - | - |
| programming-historian | JEKYLL_FAIL | - | - |
| slate-theme | 0/2 (0%) | 2/2 | 0 |
| so-simple-theme | 0/11 (0%) | 11/66 | 1 |
| time-machine-theme | 0/2 (0%) | 2/2 | 0 |
| uswds-site | JEKYLL_FAIL | - | - |
| wtf-html-css | JEKYLL_FAIL | - | - |

Column definitions:
- DOM Match: files with zero DOM differences / common HTML files compared
- File Match: rustkyll HTML files / Jekyll HTML files
- Liquid Leaks: rustkyll HTML files containing raw `{%` or `{{` tags

## Summary

- Sites compared: 35
- Total DOM matches: 5448 / 9767 (56%)

## Diff Categories by Site

### slate-theme

```
      6 attribute_differs
      2 text_differs
      2 jsonld_missing_field
      1 jsonld_value_differs
      1 jsonld_extra_field
      1 expected_element_got_text
```

### hacker-theme

```
      7 attribute_differs
      2 text_differs
      2 jsonld_missing_field
      1 jsonld_value_differs
      1 jsonld_extra_field
      1 expected_element_got_text
```

### alexeygrigorev/little-book-of-metals-ru

```
    328 attribute_differs
     72 extra_element
     18 tag_name_differs
```

### midnight-theme

```
      7 attribute_differs
      2 text_differs
      2 jsonld_missing_field
      1 jsonld_value_differs
      1 jsonld_extra_field
      1 expected_element_got_text
```

### cayman-theme

```
      6 attribute_differs
      2 jsonld_value_differs
      2 jsonld_missing_field
      1 text_differs
      1 jsonld_extra_field
      1 expected_element_got_text
```

### DataTalksClub/datatalksclub.github.io

```
    601 jsonld_value_differs
     71 missing_element
     58 text_differs
     47 attribute_differs
     40 extra_element
     34 tag_name_differs
     24 missing_text
     16 expected_text_got_element
     14 extra_text
     10 expected_element_got_text
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
    271 extra_element
     47 tag_name_differs
```

### alexeygrigorev/kids-horror-stories-ru

```
      2 text_differs
```

### alexeygrigorev/mlwiki.org

```
   1099 text_differs
    614 tag_name_differs
    412 missing_element
    269 expected_element_got_text
    247 attribute_differs
    125 extra_element
    104 missing_text
     96 expected_text_got_element
     89 extra_text
      1 extra_attribute
```

### leap-day-theme

```
      7 attribute_differs
      2 text_differs
      2 jsonld_missing_field
      1 jsonld_value_differs
      1 jsonld_extra_field
      1 expected_element_got_text
```

### merlot-theme

```
      7 attribute_differs
      2 text_differs
      2 jsonld_missing_field
      1 jsonld_value_differs
      1 jsonld_extra_field
      1 expected_element_got_text
```

### alexeygrigorev/mlbookcamp-page

```
     38 attribute_differs
     13 text_differs
      7 tag_name_differs
      3 missing_element
      2 extra_element
      2 expected_text_got_element
      2 expected_element_got_text
      1 missing_text
```

### large-docs-site

```
    500 text_differs
    500 missing_element
```

### so-simple-theme

```
     17 missing_element
      8 extra_element
      5 tag_name_differs
```

### dinky-theme

```
      7 attribute_differs
      2 text_differs
      2 jsonld_missing_field
      1 jsonld_value_differs
      1 jsonld_extra_field
      1 expected_element_got_text
```

### alexeygrigorev/alexeygrigorev.github.io

```
      8 text_differs
      2 attribute_differs
```

### muan-blog

```
  10009 attribute_differs
   4178 missing_attribute
   2797 extra_attribute
    673 text_differs
    123 extra_element
    119 tag_name_differs
      3 extra_text
      2 missing_element
      1 expected_element_got_text
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
    422 extra_element
    267 tag_name_differs
     24 missing_element
      2 expected_element_got_text
      1 extra_text
```

### architect-theme

```
      6 attribute_differs
      2 text_differs
      2 jsonld_missing_field
      1 jsonld_value_differs
      1 jsonld_extra_field
      1 expected_element_got_text
```

### opensource-guide

```
   2589 extra_element
    662 tag_name_differs
    162 attribute_differs
     52 missing_attribute
     52 extra_attribute
     19 missing_element
      1 text_differs
```

### alexeygrigorev/snippets

```
     61 extra_element
     34 tag_name_differs
```

### government-github

```
     33 attribute_differs
     23 tag_name_differs
     21 missing_element
     11 text_differs
     11 extra_element
```

### academicpages

```
     23 tag_name_differs
     12 extra_element
      9 missing_element
      1 expected_element_got_text
```

### choosealicense.com

```
     80 missing_element
     51 expected_element_got_text
     48 attribute_differs
     47 extra_element
     30 tag_name_differs
     16 text_differs
      2 extra_text
```

### mojombo-blog

```
     16 text_differs
      9 attribute_differs
      3 missing_element
      2 expected_element_got_text
      1 tag_name_differs
      1 missing_text
```

### time-machine-theme

```
      8 attribute_differs
      2 text_differs
      2 jsonld_missing_field
      1 jsonld_value_differs
      1 jsonld_extra_field
      1 expected_element_got_text
```

### jekyll-docs/docs

```
    708 extra_element
    212 tag_name_differs
     42 attribute_differs
     22 missing_element
     14 text_differs
      2 expected_element_got_text
```


Per-site full diff output is in `docs/comparison/dom-details/`.
