# Issue 168: Fix remaining non-syntax-highlighting DOM diffs

## Categories to fix

1. **People JSON-LD description truncation** (9 diffs) — `$500,000` space stripped, description truncation point differs
2. **Podcast listing URL space** (1 diff) — href has space instead of hyphen
3. **Books.html inline code class** (1 diff) — missing class on `<code>`
4. **Slack JSON-LD null vs empty** (1 diff) — `null` vs `""` for dates
5. **Heading IDs leading numbers** (~9 diffs) — `1-datatalksclub` → `datatalksclub` regression
6. **Book Q&A markdown rendering** (238 diffs, 31 files) — nested lists, `<br>` tags, kramdown attributes

## Affected pages

### People (9 files, 1 diff each — JSON-LD description truncation)
- people/danbecker.html
- people/dannyma.html
- people/demetriosbrinkmann.html
- people/elenasamuylova.html
- people/grainnemcknight.html
- people/luisoliveira.html
- people/neallathia.html
- people/patriciocerdamardini.html
- people/paulorland.html

### Podcast (193+ files — JSON-LD text)
- podcast.html (URL space in href)
- podcast/*.html (dateModified/description text)

### Books (31 files — markdown Q&A rendering)
- books/20210531-advanced-algorithms-and-data-structures.html (32 diffs)
- books/20230807-driving-data-quality-with-data-contracts.html (31 diffs)
- books/20211213-mastering-spacy.html (30 diffs)
- books/20210405-the-practitioners-guide-to-graph-data.html (28 diffs)
- books/20210426-tiny-python-projects.html (23 diffs)
- And 26 more with 12-21 diffs each

### Blog heading IDs (leading numbers stripped)
- blog/8-newsletters-for-data-science-ai-and-ml-enthusiasts.html (9 diffs)
- blog/ai-dev-tools-zoomcamp-2025-free-course.html (6 diffs)
- blog/ai-tools-for-personal-productivity.html (3 diffs)

### Single-diff pages
- books.html (inline code class missing)
- people.html (1 diff)
- slack/guidelines.html (JSON-LD null vs empty)

## Acceptance criteria

- Each category investigated and fixed
- DOM diff count reduced
- TDD: failing test per fix
