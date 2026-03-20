# Issue 278: Reimplement kramdown parser in Rust

## Problem

We're currently making pulldown-cmark (CommonMark parser) produce kramdown output through increasingly complex postprocessing in `src/kramdown.rs`. Every fix introduces edge cases because the two parsers have fundamentally different behavior:

- Pipe table detection rules differ
- Smart quote direction algorithms differ
- Emphasis delimiter handling differs (mixed `_`/`*`)
- Math handling differs
- HTML comment wrapping differs
- Typographic symbol conversion differs

DTC is stuck at 543/787 (69%) DOM match despite 15+ postprocessing fixes. The remaining 244 pages fail because of fundamental parser differences that can't be patched.

## Solution

Reimplement kramdown's parser and HTML converter natively in Rust. kramdown is MIT licensed (Thomas Leitner, 2009-2013), so this is legally straightforward.

## Scope

From kramdown 2.5.2 source at `/home/alexey/.rvm/gems/ruby-3.3.7/gems/kramdown-2.5.2`:

### Must implement (~4,000 lines of Ruby)

- `parser/kramdown.rb` (377 lines) + 24 submodules (1,932 lines) = 2,309 lines
- `converter/html.rb` (545 lines) — primary output target
- `element.rb` (551 lines) — AST node types
- `options.rb` (645 lines, partial) — option handling
- `utils/entities.rb` (1,000 lines) — HTML entity table (can use Rust crate)

### Can skip

- LaTeX, man, kramdown-round-trip converters
- Parser for HTML input (`parser/html.rb`)
- MathJax/minted integrations
- Unidecoder

### Test cases (199 file pairs)

kramdown has 216 `.text` input files and 199 `.html` expected output files in `test/testcases/`. These can be copied directly (MIT license) and used as conformance tests.

## Architecture

- New module: `src/kramdown_parser/` (separate folder)
- Keep existing `src/kramdown.rs` postprocessor for backward compatibility during migration
- `KramdownParser` struct with `parse(&str) -> Document` method
- `HtmlConverter` struct with `convert(&Document) -> String` method
- Feature flag or config to select parser (pulldown-cmark vs kramdown-native)

## Attribution

kramdown is copyright 2009-2013 Thomas Leitner, MIT License.
Test cases also include MDTest cases by Michel Fortin (MIT License).
Both attributions must be included in the source and LICENSE file.

## Acceptance Criteria

- [ ] `src/kramdown_parser/` module created with proper structure
- [ ] kramdown test cases copied with MIT attribution
- [ ] Parser handles all 16 block element types
- [ ] Parser handles all 10 span element types
- [ ] HTML converter produces output matching kramdown 2.5.2
- [ ] DTC DOM match rate significantly improves (target: >95%)
- [ ] Existing sites don't regress (pulldown-cmark path preserved)
