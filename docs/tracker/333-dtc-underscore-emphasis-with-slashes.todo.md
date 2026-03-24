# Issue 333: DTC underscore emphasis with slashes (pulldown-cmark context bug)

## Problem

On `books/20210412-ai-and-machine-learning-for-coders.html`, `_CI/CD_` is output literally as `_CI/CD_` instead of `<em>CI/CD</em>`. The `/` character inside underscore emphasis prevents the emphasis from being recognized.

This was originally Problem 3 in issue 275. The bug only reproduces in the context of the full DTC book page -- simplified unit tests pass correctly. This is a context-dependent pulldown-cmark emphasis parsing bug.

## Affected pages

- `books/20210412-ai-and-machine-learning-for-coders.html` -- 16 diffs caused by `_word/word_` not parsed as emphasis

## Root Cause

Kramdown's rule for underscore emphasis requires word boundaries. The `/` character in `_CI/CD_` may prevent the closing `_` from being recognized as a valid emphasis boundary. pulldown-cmark follows CommonMark rules which are stricter than kramdown about intra-word underscore emphasis. This only manifests in certain document contexts.

## Dependencies

- Issue 275 (done) -- Problem 1 fix is already merged

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `./scripts/cargo-safe test` passes
- [ ] `./scripts/cargo-safe clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] Input `_CI/CD_` in the context of the full DTC book page produces `<em>CI/CD</em>`
- [ ] DTC DOM comparison: `books/20210412-ai-and-machine-learning-for-coders.html` emphasis diffs resolved (16 diffs reduced significantly)
- [ ] DTC DOM comparison overall: no regression from current 751/790
- [ ] No regressions on other sites
- [ ] Tests use actual DTC file content to reproduce the context-dependent bug (not just simplified inputs)

## Notes

- The simple unit tests `test_issue275b_underscore_emphasis_with_slash` etc. pass in isolation; the bug requires full document context
- May need a preprocessing step to convert `_word/word_` patterns to `*word/word*` (asterisk emphasis) before passing to pulldown-cmark, since asterisk emphasis is less strict about word boundaries
- Alternatively, may need a postprocessing step to detect literal `_text_` patterns that should have been emphasis
