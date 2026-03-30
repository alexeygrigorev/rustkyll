# DOM Comparison Results

Generated: 2026-03-30 17:25 UTC

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

| Site | DOM Match | File Match | Liquid Leaks |
|------|-----------|------------|-------------|
| academicpages | 10/45 (22%) | 45/45 | 0 |
| alexeygrigorev/aihero | 2/2 (100%) | 2/2 | 0 |
| alexeygrigorev/alexeygrigorev.github.io | 8/8 (100%) | 8/8 | 0 |
| alexeygrigorev/data-science-interviews | 0/0 (N/A%) | 0/6 | 0 |
| alexeygrigorev/kids-horror-stories-ru | 1344/1344 (100%) | 1344/1345 | 0 |
| alexeygrigorev/little-book-of-metals-ru | 48/48 (100%) | 48/48 | 0 |
| alexeygrigorev/mlbookcamp-page | 4/4 (100%) | 15/15 | 0 |
| alexeygrigorev/mlwiki.org | 535/644 (83%) | 645/644 | 5 |
| alexeygrigorev/snippets | 8/25 (32%) | 25/25 | 0 |
| al-folio | 2/60 (3%) | 60/108 | 58 |
| architect-theme | 0/2 (0%) | 2/2 | 0 |
| basically-basic | 0/7 (0%) | 13/38 | 0 |
| beautiful-jekyll | 4/5 (80%) | 6/6 | 0 |
| bitcoin-org | 1/127 (1%) | 142/3562 | 3 |
| cayman-theme | 0/2 (0%) | 2/2 | 0 |
| chirpy | 12/17 (71%) | 17/17 | 1 |
| choosealicense.com | 72/72 (100%) | 72/72 | 0 |
| DataTalksClub/courses | 5/5 (100%) | 5/5 | 0 |
| DataTalksClub/datatalksclub.github.io | 790/790 (100%) | 790/790 | 1 |
| DataTalksClub/docs | 38/57 (67%) | 57/57 | 0 |
| dinky-theme | 0/2 (0%) | 2/2 | 0 |
| documentation-theme-jekyll | 3/100 (3%) | 100/100 | 32 |
| edition-template | 0/11 (0%) | 13/13 | 0 |
| government-github | 8/11 (73%) | 21/21 | 0 |
| hacker-theme | 0/2 (0%) | 2/2 | 0 |
| homebrew-site | 82/134 (61%) | 134/134 | 44 |
| hyde | 6/6 (100%) | 6/6 | 0 |
| hydeout | 20/30 (67%) | 34/34 | 1 |
| jasper2 | 0/10 (0%) | 18/29 | 2 |
| jekyll-docs/docs | 53/125 (42%) | 131/228 | 40 |
| jekyll-docs/lib/blank_template | 1/1 (100%) | 1/1 | 0 |
| jekyll-docs/lib/site_template | JEKYLL_FAIL | 1 pages (rustkyll-only) | 0 |
| jekyll-theme-chirpy | 1/17 (6%) | 17/17 | 2 |
| jekyll-vitepress-theme | 0/17 (0%) | 17/17 | 2 |
| just-the-docs | 16/47 (34%) | 47/47 | 4 |
| lanyon | 6/6 (100%) | 6/6 | 0 |
| large-blog-3000 | 3001/3001 (100%) | 3001/3001 | 0 |
| large-docs-site | 801/801 (100%) | 801/801 | 0 |
| leap-day-theme | 0/2 (0%) | 2/2 | 0 |
| made-mistakes-jekyll | 0/14 (0%) | 16/1303 | 7 |
| mediumish | 0/23 (0%) | 24/23 | 0 |
| merlot-theme | 0/2 (0%) | 2/2 | 0 |
| midnight-theme | 0/2 (0%) | 2/2 | 0 |
| minima | 0/9 (0%) | 9/9 | 1 |
| minimal-mistakes | 0/1 (0%) | 32/1 | 0 |
| mojombo-blog | 16/17 (94%) | 17/17 | 0 |
| muan-blog | 36/39 (92%) | 2219/2218 | 0 |
| opensource-guide | 23/388 (6%) | 390/388 | 0 |
| primer-theme | 0/2 (0%) | 2/2 | 0 |
| programming-historian | 164/529 (31%) | 653/697 | 14 |
| slate-theme | 0/2 (0%) | 2/2 | 0 |
| so-simple-theme | 0/11 (0%) | 11/66 | 1 |
| text-theme | 6/6 (100%) | 11/6 | 0 |
| time-machine-theme | 0/2 (0%) | 2/2 | 0 |
| type-theme | 8/8 (100%) | 8/8 | 1 |
| uswds-site | 135/404 (33%) | 764/404 | 133 |
| wtf-html-css | 0/1 (0%) | 1/1 | 0 |
| yat | 0/20 (0%) | 20/20 | 0 |

## Summary

- Sites compared: 57
- Total DOM matches: 7269 / 9067

## Diff Categories by Site

### slate-theme

```
      8 missing_element
      4 tag_name_differs
```

### programming-historian

```
   1726 attribute_differs
    481 missing_element
    451 text_differs
    124 tag_name_differs
    119 extra_element
     73 missing_attribute
     68 missing_text
     68 extra_attribute
     25 expected_element_got_text
     13 expected_text_got_element
      9 extra_text
```

### hacker-theme

```
      4 tag_name_differs
```

### primer-theme

```
     10 attribute_differs
      6 tag_name_differs
      2 missing_attribute
      2 extra_attribute
```

### midnight-theme

```
     10 attribute_differs
      6 tag_name_differs
      2 missing_attribute
      2 extra_attribute
```

### cayman-theme

```
     10 attribute_differs
      6 tag_name_differs
      2 missing_attribute
      2 extra_attribute
```

### DataTalksClub/docs

```
     26 tag_name_differs
```

### mediumish

```
    184 missing_element
     23 tag_name_differs
     10 attribute_differs
      8 text_differs
      2 missing_text
```

### made-mistakes-jekyll

```
     54 extra_element
     22 tag_name_differs
      6 missing_element
```

### just-the-docs

```
     59 attribute_differs
     45 missing_element
     42 text_differs
     33 expected_element_got_text
     24 tag_name_differs
     19 missing_attribute
     17 extra_element
     11 extra_text
      3 expected_text_got_element
```

### wtf-html-css

```
      6 attribute_differs
      2 extra_element
      1 text_differs
      1 extra_text
```

### chirpy

```
     14 attribute_differs
      5 missing_attribute
      5 extra_attribute
      4 text_differs
      3 tag_name_differs
      2 extra_element
      1 missing_text
      1 extra_text
      1 expected_text_got_element
      1 expected_element_got_text
```

### hydeout

```
     32 missing_element
     18 attribute_differs
     14 text_differs
      9 tag_name_differs
      6 expected_element_got_text
      3 extra_text
      2 missing_text
      2 extra_element
```

### alexeygrigorev/mlwiki.org

```
    246 tag_name_differs
     98 text_differs
     87 attribute_differs
     51 missing_element
     32 expected_element_got_text
     18 missing_text
     17 extra_element
     10 expected_text_got_element
      7 extra_text
```

### leap-day-theme

```
     10 attribute_differs
      6 tag_name_differs
      2 missing_attribute
      2 extra_attribute
```

### bitcoin-org

```
   1126 attribute_differs
    116 missing_text
      6 tag_name_differs
      2 text_differs
      2 missing_attribute
      2 extra_element
      1 expected_element_got_text
```

### merlot-theme

```
      8 missing_element
      4 tag_name_differs
```

### homebrew-site

```
    168 missing_element
     86 text_differs
     84 missing_text
     61 attribute_differs
     44 expected_element_got_text
     28 tag_name_differs
      6 extra_attribute
      5 missing_attribute
      5 expected_text_got_element
      4 extra_element
      2 extra_text
      1 jsonld_value_differs
```

### so-simple-theme

```
     76 attribute_differs
     12 missing_attribute
     11 tag_name_differs
     11 extra_attribute
```

### dinky-theme

```
     10 attribute_differs
      6 tag_name_differs
      2 missing_attribute
      2 extra_attribute
```

### minimal-mistakes

```
      6 tag_name_differs
      3 attribute_differs
      1 missing_attribute
```

### jekyll-vitepress-theme

```
     55 missing_element
     38 tag_name_differs
     37 text_differs
      6 attribute_differs
      3 expected_element_got_text
      1 missing_text
      1 extra_text
      1 expected_text_got_element
```

### edition-template

```
     29 extra_element
     18 tag_name_differs
      2 missing_text
```

### muan-blog

```
     16 attribute_differs
      7 text_differs
      3 missing_text
      3 missing_element
      1 tag_name_differs
```

### uswds-site

```
   1452 attribute_differs
    247 tag_name_differs
    180 missing_attribute
    180 extra_attribute
     89 missing_element
     32 extra_element
      6 expected_element_got_text
      3 missing_text
      2 text_differs
```

### beautiful-jekyll

```
      3 tag_name_differs
      1 text_differs
      1 missing_element
      1 expected_element_got_text
```

### jekyll-theme-chirpy

```
     24 missing_element
     24 extra_element
      8 tag_name_differs
      1 expected_element_got_text
```

### documentation-theme-jekyll

```
    105 text_differs
     88 missing_element
     53 attribute_differs
     47 tag_name_differs
     19 expected_element_got_text
     17 expected_text_got_element
     13 missing_text
     12 extra_element
      2 missing_attribute
      2 extra_text
```

### jasper2

```
     67 attribute_differs
     22 jsonld_value_differs
```

### architect-theme

```
      8 missing_element
      4 tag_name_differs
```

### opensource-guide

```
   3254 missing_element
    365 tag_name_differs
     24 attribute_differs
      6 text_differs
```

### alexeygrigorev/snippets

```
    106 attribute_differs
     43 text_differs
      2 expected_text_got_element
      1 tag_name_differs
```

### minima

```
     33 attribute_differs
     24 tag_name_differs
      7 missing_attribute
      7 extra_attribute
      3 extra_element
```

### basically-basic

```
     28 attribute_differs
     14 missing_attribute
     14 extra_attribute
      7 text_differs
      7 tag_name_differs
```

### government-github

```
     12 missing_element
     10 attribute_differs
      4 tag_name_differs
```

### academicpages

```
     42 text_differs
     23 missing_element
     21 attribute_differs
     11 tag_name_differs
      9 extra_element
      5 missing_text
      2 missing_attribute
      2 extra_text
      2 expected_text_got_element
      1 expected_element_got_text
```

### al-folio

```
    233 extra_attribute
    230 missing_attribute
     63 attribute_differs
     54 tag_name_differs
```

### mojombo-blog

```
      3 text_differs
      3 missing_text
      3 missing_element
      1 tag_name_differs
```

### time-machine-theme

```
      8 missing_element
      4 tag_name_differs
```

### jekyll-docs/docs

```
    213 missing_element
    171 attribute_differs
    149 text_differs
     44 expected_element_got_text
     30 missing_text
     16 tag_name_differs
     12 extra_element
     10 missing_attribute
      6 extra_text
      5 expected_text_got_element
      1 jsonld_value_differs
```

### yat

```
     80 missing_attribute
     80 extra_attribute
     34 attribute_differs
```


Per-site full diff output is in `docs/comparison/dom-details/`.
