# Issue 286: Kramdown emphasis algorithm rewrite and span edge cases

## Problem

4 emphasis conformance tests fail because the current emphasis parser does not fully match kramdown's stack-based emphasis algorithm. Kramdown has specific rules for handling empty emphasis markers, error recovery for unclosed markers, and nested emphasis combinations (triple `***` and `___`) that differ from CommonMark. Additionally, 2 code span tests fail because they require Rouge syntax highlighting for inline code with language annotations.

## Scope

Fix the emphasis parser in `span_parser.rs` to match kramdown's emphasis algorithm, and implement inline code syntax highlighting with Rouge.

### Emphasis (4 tests)

These tests exercise the kramdown emphasis algorithm defined in `emphasis.rb`:

- **`normal`** (`span/02_emphasis/normal`): Basic `*em*`, `**strong**`, `_em_`, `__strong__`, emphasis at start of line, emphasis across words, emphasis with punctuation boundaries
- **`empty`** (`span/02_emphasis/empty`): `__is **empty` (unclosed double), `****is empty` (empty strong markers), edge cases with no content between markers
- **`errors`** (`span/02_emphasis/errors`): `*star` (unclosed), `**star` (unclosed double), `**a *star*` (mismatched nesting), error recovery producing literal `*` characters
- **`nesting`** (`span/02_emphasis/nesting`): `***test test***`, `*test **test***`, `**test *test***`, `***test* test**`, `_test __test___` -- triple markers split into em+strong correctly, list items with emphasis

### Code spans with syntax highlighting (2 tests)

- **`highlighting`** (`span/03_codespan/highlighting`): `` `x = Class.new`{:.language-ruby} `` renders with Rouge-highlighted `<span>` elements inside the `<code>` tag (e.g., `<span class="n">x</span> <span class="o">=</span>`)
- **`rouge_simple`** (`span/03_codespan/rouge/simple`): Code spans with `syntax_highlighter: rouge` option enabled, producing highlighted output

## Test files

| Test name | Testcase path | Options |
|-----------|---------------|---------|
| `kramdown_span_02_emphasis_normal` | `span/02_emphasis/normal` | none |
| `kramdown_span_02_emphasis_empty` | `span/02_emphasis/empty` | none |
| `kramdown_span_02_emphasis_errors` | `span/02_emphasis/errors` | none |
| `kramdown_span_02_emphasis_nesting` | `span/02_emphasis/nesting` | none |
| `kramdown_span_03_codespan_highlighting` | `span/03_codespan/highlighting` | none (uses `{:.language-ruby}` IAL) |
| `kramdown_span_03_codespan_rouge_simple` | `span/03_codespan/rouge/simple` | `syntax_highlighter: rouge` |

## Approach

1. Study the kramdown Ruby reference at `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/emphasis.rb`
2. Run each failing emphasis test, diff actual vs expected output
3. Rewrite or fix the emphasis parsing in `span_parser.rs` to match kramdown's algorithm:
   - Delimiter stack tracking (open/close markers with type and count)
   - Triple `***` handling: split into `*` + `**` (em + strong)
   - Empty marker handling: `****` becomes literal text
   - Error recovery: unclosed markers become literal `*` or `_`
   - Word boundary rules for `_` emphasis (kramdown uses different rules than CommonMark)
4. For code span highlighting, extend the code span renderer to detect `{:.language-XXX}` IAL and invoke Rouge tokenization on the code content, wrapping tokens in `<span class="...">` tags

## Dependencies

- Issue 282 (Phase 3 spans) should be done or at least stable -- this issue fixes remaining emphasis bugs in the same span parser
- No dependency on block-level issues

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes all existing tests (no regressions)
- [ ] `kramdown_span_02_emphasis_normal` passes
- [ ] `kramdown_span_02_emphasis_empty` passes
- [ ] `kramdown_span_02_emphasis_errors` passes
- [ ] `kramdown_span_02_emphasis_nesting` passes
- [ ] `kramdown_span_03_codespan_highlighting` passes -- code span with `{:.language-ruby}` IAL produces Rouge-highlighted `<span>` elements
- [ ] `kramdown_span_03_codespan_rouge_simple` passes -- code spans with `syntax_highlighter: rouge` produce highlighted output
- [ ] All 6 tests above pass: `./scripts/cargo-safe test --lib kramdown_span_02_emphasis kramdown_span_03_codespan_highlighting kramdown_span_03_codespan_rouge_simple`
- [ ] No regressions in other span tests (the 25+ currently-passing span tests still pass)
- [ ] No regressions in block tests
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` is clean

## Test Scenarios

### Unit: Emphasis normal
- `*em*` renders as `<em>em</em>`
- `**strong**` renders as `<strong>strong</strong>`
- `_em_` renders as `<em>em</em>`
- `__strong__` renders as `<strong>strong</strong>`
- `*At* start` renders emphasis at start of paragraph

### Unit: Emphasis empty/error
- `__is **empty` -- unclosed markers become literal text
- `****is empty` -- quadruple markers produce literal `****`
- `*star` without closing -- literal `*star`
- `**a *star*` -- inner star closes but outer doesn't

### Unit: Emphasis nesting
- `***test test***` renders as `<em><strong>test test</strong></em>`
- `*test **test***` renders as `<em>test <strong>test</strong></em>`
- `**test *test***` renders as `<strong>test <em>test</em></strong>`
- `***test* test**` renders as `<strong><em>test</em> test</strong>`

### Unit: Code span highlighting
- `` `x = Class.new`{:.language-ruby} `` produces `<code class="language-ruby highlighter-rouge"><span class="n">x</span>...</code>`
- Code span without language annotation remains unhighlighted

### Integration
- Run all 6 conformance tests, compare actual vs expected `.html` output
- Run `./scripts/cargo-safe test --lib kramdown_parser::tests::kramdown_span` and verify emphasis/codespan tests are in the pass list

## Ruby reference files

- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/emphasis.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/parser/kramdown/codespan.rb`
- `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2/lib/kramdown/converter/html.rb` (code span rendering with Rouge)
