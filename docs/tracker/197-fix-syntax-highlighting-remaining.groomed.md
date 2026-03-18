# Issue 197: Fix remaining syntax highlighting differences (574 pages)

## Checklist Category

**Syntax highlighting differences** -- 574 pages

## Problem

574 pages have syntax highlighting token class differences between rustkyll's highlighter and Jekyll's Rouge.

Breakdown by site:
- large-docs-site (500): JSON string token merging -- being investigated in issue 193
- alexeygrigorev-mlwiki.org (47): Mostly XML/HTML code blocks with different token boundaries and classes
- DTC (11): Various languages (SQL, Bash, Dockerfile, Python) with keyword class mismatches (`class='k'` vs `class='n'`)
- alexeygrigorev-mlbookcamp-page (6): Bash/YAML/Python code blocks
- 10 theme sites (1 each): Usually JavaScript/Ruby code blocks

## Goal

Match Rouge token classes exactly for all languages used across benchmark sites.

## Dependencies

- Issue 193 (large-docs-site JSON tokens) -- in-progress. Covers 500 of the 574 pages.
- Issue 177 (syntax highlighting XML tokens) -- done.
- Issue 151 (fix syntax highlighting remaining) -- done. Previous round of fixes.
- Issue 180 (fix JSON string token splitting) -- in-progress.

## Sub-tasks

### Sub-task 1: Investigation (do this FIRST)

1. Read `docs/comparison/dom-details/DataTalksClub-datatalksclub.github.io.txt` and extract all syntax highlighting diffs. Categorize by language:
   - SQL: `class='k'` vs `class='n'` pattern (keywords not recognized)
   - Bash: token boundary differences
   - Dockerfile: token classification differences
   - Other languages

2. Read `docs/comparison/dom-details/alexeygrigorev-mlwiki.org.txt` and extract syntax highlighting diffs. Note which are XML/HTML vs other languages.

3. Read a theme site dom-details file (e.g., `architect-theme.txt`) to see the specific code block diff.

4. For each language, identify the specific token(s) that differ and what Rouge classifies them as.

### Sub-task 2: Fix SQL keyword recognition

DTC diffs show SQL keywords like `SELECT`, `FROM`, `WHERE` getting `class='n'` (name) instead of `class='k'` (keyword). This is a scope mapping issue in `src/syntax.rs`.

### Sub-task 3: Fix XML/HTML token boundaries in mlwiki.org

The XML diffs show token text splitting differently: e.g., `</action>` as one token vs split across multiple spans.

### Sub-task 4: Fix remaining language-specific token diffs

Address Bash, Dockerfile, Python, JavaScript token classification issues.

## TDD Test Scenarios

### Test 1: SQL keywords classified as 'k' (write FIRST, verify it fails)

```rust
#[test]
fn test_sql_keywords_classified_correctly() {
    // Setup: Highlight this SQL code:
    //   SELECT name FROM users WHERE id = 1;
    //
    // Assert: SELECT, FROM, WHERE tokens have class='k' (keyword),
    //   not class='n' (name).
    //
    // Verify it FAILS before fixing scope mapping.
}
```

### Test 2: XML closing tags as single token

```rust
#[test]
fn test_xml_closing_tag_token_boundary() {
    // Setup: Highlight this XML:
    //   <action>do_thing</action>
    //   <plugin>my-plugin</plugin>
    //
    // Assert: </action> appears as a single span with class='nt' (name.tag),
    //   not split across multiple spans.
    //
    // Verify it FAILS before implementing.
}
```

### Test 3: Bash token classification

```rust
#[test]
fn test_bash_flag_token_classification() {
    // Setup: Highlight Bash:
    //   docker run --rm --name postgresql
    //
    // Assert: --rm and --name have correct Rouge-compatible classes.
    //
    // Verify it FAILS before implementing.
}
```

### Test 4 (integration, #[ignore]): Build DTC and verify syntax highlighting

```rust
#[test]
#[ignore]
fn test_dtc_syntax_highlighting_matches() {
    // Build DTC site
    // Parse important-sql-fact-that-everyone-should-know.html
    // Check SQL keyword spans have class='k'
}
```

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes with language-specific syntax highlighting tests
- [ ] Investigation documents exact token classification diffs per language
- [ ] SQL keywords (SELECT, FROM, WHERE, etc.) classified as `class='k'`
- [ ] XML/HTML closing tags have correct token boundaries matching Rouge
- [ ] DTC syntax highlighting diffs reduced to zero (11 pages fixed)
- [ ] Theme site code block diffs fixed (10 pages)
- [ ] mlwiki.org syntax diffs: fix what can be fixed with scope mapping changes; document any that require deeper changes
- [ ] large-docs-site (500 pages) tracked separately in issue 193
