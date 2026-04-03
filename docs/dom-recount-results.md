# DOM Comparison Results

Generated: 2026-04-03 22:47 UTC

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
| academicpages | 10/45 (22%) | 45 / 45 | 0 | 0 | 0 |
| alexeygrigorev/aihero | 2/2 (100%) | 2 / 2 | 0 | 0 | 0 |
| alexeygrigorev/alexeygrigorev.github.io | 8/8 (100%) | 8 / 8 | 0 | 0 | 0 |
| alexeygrigorev/data-science-interviews | 0/6 (0%) | 0 / 6 | 6 | 0 | 0 |
| alexeygrigorev/kids-horror-stories-ru | 1344/1345 (100%) | 1344 / 1345 | 1 | 0 | 0 |
| alexeygrigorev/little-book-of-metals-ru | 48/48 (100%) | 48 / 48 | 0 | 0 | 0 |
| alexeygrigorev/mlbookcamp-page | 4/26 (15%) | 4 / 26 | 11 | 11 | 0 |
| alexeygrigorev/mlwiki.org | 534/645 (83%) | 644 / 645 | 0 | 1 | 5 |
| alexeygrigorev/snippets | 8/25 (32%) | 25 / 25 | 0 | 0 | 0 |
| al-folio | 2/123 (2%) | 87 / 123 | 21 | 15 | 64 |
| architect-theme | 0/2 (0%) | 2 / 2 | 0 | 0 | 0 |
| basically-basic | 0/44 (0%) | 7 / 44 | 31 | 6 | 0 |
| beautiful-jekyll | 5/7 (71%) | 5 / 7 | 1 | 1 | 0 |
| bitcoin-org | 1/3577 (0%) | 960 / 3577 | 2602 | 15 | 3 |
| cayman-theme | 0/2 (0%) | 2 / 2 | 0 | 0 | 0 |
| chirpy | 0/17 (0%) | 17 / 17 | 0 | 0 | 0 |
| choosealicense.com | 25/72 (35%) | 72 / 72 | 0 | 0 | 0 |
| DataTalksClub/courses | 5/5 (100%) | 5 / 5 | 0 | 0 | 0 |
| DataTalksClub/datatalksclub.github.io | 596/790 (75%) | 790 / 790 | 0 | 0 | 1 |
| DataTalksClub/docs | 38/57 (67%) | 57 / 57 | 0 | 0 | 0 |
| dinky-theme | 0/2 (0%) | 2 / 2 | 0 | 0 | 0 |
| documentation-theme-jekyll | 3/98 (3%) | 100 / 98 | 0 | 0 | 31 |
| edition-template | 0/15 (0%) | 11 / 15 | 2 | 2 | 0 |
| government-github | 8/31 (26%) | 11 / 31 | 10 | 10 | 0 |
| hacker-theme | 0/2 (0%) | 2 / 2 | 0 | 0 | 0 |
| homebrew-site | 85/134 (63%) | 134 / 134 | 0 | 0 | 44 |
| hyde | 4/6 (67%) | 6 / 6 | 0 | 0 | 0 |
| hydeout | 19/38 (50%) | 30 / 38 | 4 | 4 | 1 |
| jasper2 | 0/37 (0%) | 21 / 37 | 8 | 8 | 2 |
| jekyll-docs/docs | 22/234 (9%) | 125 / 234 | 103 | 6 | 40 |
| jekyll-docs/lib/blank_template | 1/1 (100%) | 1 / 1 | 0 | 0 | 0 |
| jekyll-docs/lib/site_template | JEKYLL_FAIL | 1 (rustkyll-only) | - | - | 0 |
| jekyll-theme-chirpy | 0/17 (0%) | 17 / 17 | 0 | 0 | 1 |
| jekyll-vitepress-theme | 0/17 (0%) | 17 / 17 | 0 | 0 | 2 |
| just-the-docs | 16/47 (34%) | 47 / 47 | 0 | 0 | 4 |
| lanyon | 6/6 (100%) | 6 / 6 | 0 | 0 | 0 |
| large-blog-3000 | 3001/3001 (100%) | 3001 / 3001 | 0 | 0 | 0 |
| large-docs-site | 801/801 (100%) | 801 / 801 | 0 | 0 | 0 |
| leap-day-theme | 0/2 (0%) | 2 / 2 | 0 | 0 | 0 |
| made-mistakes-jekyll | 1/1303 (0%) | 1039 / 1303 | 264 | 0 | 13 |
| mediumish | 0/24 (0%) | 23 / 24 | 0 | 1 | 0 |
| merlot-theme | 0/2 (0%) | 2 / 2 | 0 | 0 | 0 |
| midnight-theme | 0/2 (0%) | 2 / 2 | 0 | 0 | 0 |
| minima | 0/9 (0%) | 9 / 9 | 0 | 0 | 1 |
| minimal-mistakes | 0/32 (0%) | 1 / 32 | 0 | 31 | 0 |
| mojombo-blog | 14/17 (82%) | 17 / 17 | 0 | 0 | 0 |
| muan-blog | 2178/2254 (97%) | 2183 / 2254 | 35 | 36 | 0 |
| opensource-guide | 23/390 (6%) | 388 / 390 | 0 | 2 | 0 |
| primer-theme | 0/2 (0%) | 2 / 2 | 0 | 0 | 0 |
| programming-historian | 164/821 (20%) | 529 / 821 | 168 | 124 | 14 |
| slate-theme | 0/2 (0%) | 2 / 2 | 0 | 0 | 0 |
| so-simple-theme | 0/66 (0%) | 11 / 66 | 55 | 0 | 1 |
| text-theme | 5/11 (45%) | 6 / 11 | 0 | 5 | 0 |
| time-machine-theme | 0/2 (0%) | 2 / 2 | 0 | 0 | 0 |
| type-theme | 7/8 (88%) | 8 / 8 | 0 | 0 | 0 |
| uswds-site | 135/764 (18%) | 404 / 764 | 0 | 360 | 44 |
| wtf-html-css | 0/1 (0%) | 1 / 1 | 0 | 0 | 0 |
| yat | 0/20 (0%) | 20 / 20 | 0 | 0 | 0 |

## Summary

- Sites compared: 57
- Total DOM matches: 9123 / 17065

## Diff Categories by Site

### slate-theme

```
      8 missing_element
      2 tag_name_differs
      1 attribute_differs
```

### programming-historian

```
   1741 attribute_differs
    481 missing_element
    454 text_differs
    122 tag_name_differs
    121 extra_element
     68 missing_text
     66 missing_attribute
     57 extra_attribute
     25 expected_element_got_text
     13 expected_text_got_element
      9 extra_text
```

### hacker-theme

```
      2 tag_name_differs
      1 attribute_differs
```

### type-theme

```
      1 text_differs
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

### DataTalksClub/datatalksclub.github.io

```
    133 jsonld_value_differs
    122 jsonld_missing_field
```

### DataTalksClub/docs

```
     26 tag_name_differs
```

### text-theme

```
      2 attribute_differs
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
   2014 missing_element
    129 extra_element
     60 tag_name_differs
      6 attribute_differs
      2 missing_attribute
      2 extra_attribute
```

### just-the-docs

```
     61 attribute_differs
     45 missing_element
     42 text_differs
     33 expected_element_got_text
     21 tag_name_differs
     19 missing_attribute
     18 extra_element
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
     17 text_differs
     12 attribute_differs
      6 extra_element
      5 tag_name_differs
      4 missing_element
      3 expected_element_got_text
      1 missing_text
      1 missing_attribute
      1 jsonld_value_differs
      1 extra_text
      1 expected_text_got_element
```

### hydeout

```
     32 missing_element
     19 attribute_differs
     16 text_differs
     11 extra_element
      8 tag_name_differs
      6 expected_element_got_text
      3 extra_text
      1 missing_text
```

### alexeygrigorev/mlwiki.org

```
    251 tag_name_differs
    100 text_differs
     81 attribute_differs
     55 missing_element
     34 expected_element_got_text
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
   7948 attribute_differs
    561 missing_element
    125 extra_element
    118 tag_name_differs
     69 expected_element_got_text
     53 extra_text
     45 missing_attribute
     27 text_differs
     13 missing_text
```

### merlot-theme

```
      8 missing_element
      2 tag_name_differs
      1 attribute_differs
```

### homebrew-site

```
    168 missing_element
     89 text_differs
     84 missing_text
     64 attribute_differs
     44 expected_element_got_text
     11 tag_name_differs
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
     34 missing_attribute
     34 extra_attribute
     19 tag_name_differs
     18 missing_element
     17 missing_text
     17 extra_text
     15 attribute_differs
      3 text_differs
      2 expected_element_got_text
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
      7 text_differs
      7 attribute_differs
      3 tag_name_differs
      3 missing_text
      3 missing_element
```

### uswds-site

```
   1452 attribute_differs
    233 tag_name_differs
    180 missing_attribute
    180 extra_attribute
    103 missing_element
     27 extra_element
      6 expected_element_got_text
      3 missing_text
      2 text_differs
```

### jekyll-theme-chirpy

```
     26 missing_element
     24 extra_element
      8 tag_name_differs
      1 expected_element_got_text
```

### documentation-theme-jekyll

```
    101 text_differs
     89 missing_element
     54 tag_name_differs
     39 attribute_differs
     17 expected_text_got_element
     16 expected_element_got_text
     15 missing_text
     14 extra_element
      3 extra_text
      2 missing_attribute
```

### jasper2

```
    140 attribute_differs
     22 jsonld_value_differs
     11 missing_element
      8 text_differs
```

### architect-theme

```
      8 missing_element
      2 tag_name_differs
      1 attribute_differs
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

### hyde

```
     18 extra_element
      2 tag_name_differs
```

### academicpages

```
     44 text_differs
     23 missing_element
     20 attribute_differs
     10 tag_name_differs
      5 extra_element
      2 missing_text
      2 extra_text
      2 expected_text_got_element
      1 expected_element_got_text
```

### al-folio

```
    197 extra_attribute
    195 missing_attribute
    118 tag_name_differs
     52 attribute_differs
```

### choosealicense.com

```
     47 attribute_differs
```

### mojombo-blog

```
     10 attribute_differs
      3 text_differs
      3 missing_text
      3 missing_element
      2 tag_name_differs
```

### time-machine-theme

```
      8 missing_element
      2 tag_name_differs
      1 attribute_differs
```

### jekyll-docs/docs

```
    264 attribute_differs
    175 missing_element
    133 text_differs
     39 expected_element_got_text
     23 missing_text
     12 tag_name_differs
      9 missing_attribute
      8 extra_element
      5 extra_text
      4 expected_text_got_element
      1 jsonld_value_differs
```

### yat

```
     80 missing_attribute
     80 extra_attribute
     36 attribute_differs
```


Per-site full diff output is in `docs/comparison/dom-details/`.
