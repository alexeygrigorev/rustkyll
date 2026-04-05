# Issue #591: Python syntax highlighting class refinements for snippets site

## Problem

The snippets site (alexeygrigorev/snippets) has 17 out of 25 pages with DOM
differences, totaling 4530 diffs. Nearly all diffs are syntax highlighting
CSS class mismatches between syntect (rustkyll) and Rouge (Jekyll) for Python code.

The most impactful class mismatches are:

### 1. `n` vs `nn` for module names in import statements
Rouge classifies module/package names after `import` or `from` as `nn` (Name.Namespace).
Syntect classifies them as generic name `n`.

```python
from pydantic import BaseModel
#     ^^^^^^^^ Rouge: nn, Syntect: n
```

**Affected pages:** All 10 AI snippets, 3 Python snippets (13 pages, ~200+ diffs)

### 2. `sh`/`s` triple-quoted string tokenization differences
Rouge keeps `"""docstring"""` as a single `s` token. Syntect splits triple-quoted
strings into opening delimiter `sh`, content `s`, and closing delimiter `sh`.
This causes the DOM comparator to see misaligned spans.

```python
"""Handles streaming display of agent execution"""
# Rouge: single <span class="s">"""Handles streaming..."""</span>
# Syntect: <span class="sh">"""</span><span class="s">Handles streaming...</span><span class="sh">"""</span>
```

**Affected pages:** Most Python pages with docstrings (~500+ diffs)

### 3. `nc` vs `n` for class names
Rouge classifies class names (after `class`) as `nc` (Name.Class).
Syntect maps them to generic `n` (Name).

```python
class StreamingHandler:
#     ^^^^^^^^^^^^^^^^ Rouge: nc, Syntect: n
```

**Affected pages:** Pages with class definitions (~50+ diffs)

### 4. `nf` vs `n` for function names
Rouge classifies function definitions (after `def`) as `nf` (Name.Function).
Syntect maps them to generic `n` (Name).

```python
def __init__(self):
#   ^^^^^^^^ Rouge: nf, Syntect: n
```

**Affected pages:** Pages with function definitions (~100+ diffs)

### 5. YAML/Makefile/Dockerfile minor class differences
- YAML: `s1` vs `s2` for quoted strings (5 diffs on devops pages)
- Makefile: `nv` vs `n` for variable names (90 diffs on python-project-makefile)
- Dockerfile: token boundary alignment (17 diffs on dockerfile-uv-python)

## Affected Sites

- **snippets** (8/25 matched): 17 pages, 4530 total diffs -- virtually ALL from syntax classes
- **mlbookcamp** (11/15 matched): 4 pages, 809 diffs -- same Python class issues
- **minima** (6/9 matched): codeblocks-ahoy page has Ruby multi-line comment diffs
- **muan-blog** (2214/2218): border-box + first-pull-request pages (~48 diffs)

## Scope

Focus on the Python scope mappings in `src/syntax.rs` which will fix the majority
of diffs. The 4 highest-impact mappings to add/fix:

1. `source.python meta.function-call support.type` -> `n` (already partially mapped)
2. `source.python entity.name.type.class` -> `nc` 
3. `source.python entity.name.function` -> `nf`
4. Python triple-quoted string merging into single `s` token

Items 1-3 are scope mapping additions. Item 4 is a token merging change in the
highlighting output stage.

## Acceptance Criteria

- [ ] Python `import` module names get class `nn` (not `n`)
- [ ] Python class names after `class` get class `nc` (not `n`)
- [ ] Python function names after `def` get class `nf` (not `n`)
- [ ] Python triple-quoted strings render as a single `s` span (matching Rouge)
- [ ] No regressions on existing Python highlighting (DTC, other sites)
- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes
- [ ] DTC DOM match count must not drop below 788/790
- [ ] Snippets site DOM match count improves significantly (target: 15+/25 from 8/25)

## Test Scenarios

### Unit: Python module name class
- `from pydantic import BaseModel` -> module name `pydantic` gets class `nn`
- `import asyncio` -> module name `asyncio` gets class `nn`
- `from collections.abc import Callable` -> `collections` and `abc` get class `nn`

### Unit: Python class name class
- `class MyClass:` -> `MyClass` gets class `nc`
- `class StreamingHandler(BaseHandler):` -> `StreamingHandler` gets class `nc`

### Unit: Python function name class
- `def my_function():` -> `my_function` gets class `nf`
- `def __init__(self):` -> `__init__` gets class `nf`

### Unit: Python triple-quoted string merging
- `"""docstring"""` -> renders as single `<span class="s">"""docstring"""</span>`
- Multi-line triple-quoted string -> single span with class `s`

### Integration: Snippets site
- Build snippets site
- Verify AI/Python snippet pages have improved DOM match counts
- Total diff count decreases substantially

## Dependencies

None.

## DOM Baseline

- DTC: 788/790 matched
- Snippets: 8/25 matched, 4530 total diffs

## Log

### [SWE] 2026-04-02

**Analysis: Rouge 3.x vs Rouge 4.x Conflict**

Thorough investigation revealed a fundamental incompatibility between the proposed
changes and the zero-DTC-regression constraint:

- DTC uses Rouge 3.30.0 (via github-pages gem)
- Snippets uses Rouge 4.7.0 (via Jekyll 4.3)
- These versions produce OPPOSITE token classifications for Python code:

| Token Type | Rouge 3.30 (DTC) | Rouge 4.7 (Snippets) | Current Rustkyll |
|---|---|---|---|
| Import module names | `nn` | `n` | `nn` (matches DTC) |
| Constructor calls (Agent()) | `n` | `nc` | `n` (matches DTC) |
| Method calls (runner.run()) | `n` | `nf` | `n` (matches DTC) |
| String delimiters | single `s` span | `sh`+`s`+`sh` split | single `s` (matches DTC) |
| Single-quoted strings | `s` | `sh`+`s`+`sh` | `s` (matches DTC) |

The existing code already has a comment at line 448-450 documenting this:
> "Python string delimiter split (sh+s+sh) and method call reclassification
> (n->nf) are disabled because the DTC site's Rouge version keeps strings
> as single 's' spans and methods as 'n'."

**Failed Hypothesis: YAML single-quote delimiter fix (s2 -> s1)**
- Wrote test: test_issue591_yaml_single_quote_begin_is_s1
- Ran test: FAILS (opening `'` mapped to s2, expected s1)
- Implemented fix: added scope rule for `source.yaml string.quoted.single punctuation.definition.string.begin` -> `s1`
- Ran test: PASSES (delimiter merges with content into single s1 span)
- DTC check: 788/790 with 8 diffs (no regression, acceptable diffs reduced 867->852)
- Snippets check: 8/25 with 4761 diffs (REGRESSION: was 4530, increased by 231)
- Root cause: merged s1 span shifts all subsequent DOM elements, causing cascading diffs
- REVERTED the fix

**Documented behavior tests (TDD for regression prevention)**
- Wrote 8 tests documenting current correct behavior for Rouge 3.x compatibility:
  1. test_issue591_yaml_single_quote_begin_is_s2 - YAML single-quote delimiter stays s2
  2. test_issue591_yaml_double_quote_begin_stays_s2 - YAML double-quote delimiter stays s2
  3. test_issue591_python_import_module_stays_nn_for_rouge3 - import names stay nn
  4. test_issue591_python_constructor_call_is_n_for_rouge3 - constructor calls stay n
  5. test_issue591_python_method_call_is_n_for_rouge3 - method calls stay n
  6. test_issue591_python_string_stays_single_s_for_rouge3 - strings stay single s
  7. test_issue591_python_triple_quote_stays_single_s_for_rouge3 - triple-quoted strings stay single s
  8. test_issue591_python_unicode_import_stays_nn - unicode import module names work
- All 8 tests PASS

**DTC exposure analysis:**
- 7 DTC pages have Python code blocks
- 28 constructor calls across 3 pages
- 74 function calls across 6 pages
- 30 import module names across 3 pages
- 224 string spans across 6 pages
- ANY of the proposed changes would break these matching pages

**Conclusion:**
The acceptance criteria "DTC DOM not below 788/790" and "Snippets improves to 15+/25"
are mutually exclusive. Every proposed Python token change improves snippets at the
cost of regressing DTC. The issue needs to be either:
1. Descoped to Rouge 3.x-only behavior (already implemented), or
2. Redesigned to support per-site Rouge version detection (major architecture change)

**Summary:**
- Files modified: src/syntax.rs (8 regression-prevention tests added)
- Tests added: 8 (documenting Rouge 3.x compatibility constraints)
- Build results: 4050 tests pass, 0 fail, clippy clean, fmt clean
- DTC DOM: 788/790 matched, 8 total diffs (unchanged from baseline)
- DTC build time: 0.91s (under 1.0s threshold)
- Snippets DOM: 8/25 matched, 4530 total diffs (unchanged from baseline)
- Known limitation: Cannot improve snippets/mlbookcamp without regressing DTC due to Rouge version conflict
