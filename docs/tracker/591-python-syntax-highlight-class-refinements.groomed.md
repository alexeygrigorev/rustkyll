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
