# Issue 290: Rouge-compatible syntax highlighting token mapping

## Problem

Syntax highlighting diffs are the largest remaining DOM difference category across multiple sites. Rustkyll uses syntect (Sublime Text grammars) while Jekyll uses Rouge (Pygments-based). They produce different CSS class names for the same code tokens.

Examples:
- `class='n'` vs `class='nb'` (Name vs Name.Builtin)
- `class='o'` vs `class='k'` (Operator vs Keyword)
- `class='nf'` vs `class='p'` (Name.Function vs Punctuation)
- Token boundary differences (where one token ends and next begins)

Affects: architect-theme, cayman-theme, mlbookcamp-page, mlwiki.org, mojombo-blog, lanyon, DTC, and others.

## Approach

Keep syntect for parsing (it handles 200+ languages well). Replace the syntect-scope → CSS-class mapping with a comprehensive Rouge-compatible mapping table.

### Steps

1. **Map Rouge token hierarchy to syntect scopes**: Rouge's token.rb defines tokens like `Keyword.Declaration` → `kd`, `Name.Builtin` → `nb`. Syntect uses Sublime Text scopes like `keyword.declaration`, `support.function.builtin`. Build a mapping table from syntect scopes to Rouge CSS shortcodes.

2. **Test per-language**: For the languages that appear in benchmark sites (Ruby, Python, JavaScript, YAML, Bash, JSON, XML, SQL, Go, Java), run both Rouge and syntect on the same code blocks and compare output token-by-token.

3. **Handle token boundary differences**: Where syntect splits tokens differently from Rouge, add language-specific post-processing (we already do this for Ruby and JSON).

4. **Validate against benchmark sites**: Run DOM comparison on affected sites and verify syntax highlighting diffs are eliminated.

## License

Rouge is MIT licensed (Copyright (c) 2012 Jeanine Adkisson). Some lexers are BSD 2-Clause (from Pygments). Both licenses permit porting token mappings. Must include copyright notices.

Rouge token definitions: `/home/alexey/.rvm/gems/ruby-3.3.7/gems/rouge-4.7.0/lib/rouge/token.rb`

## Key Files

- `src/syntax.rs` — current syntect-to-Rouge mapping (partial)
- Rouge token.rb — definitive token hierarchy and shortcodes
- Rouge lexers in `lib/rouge/lexers/` — per-language token rules

## Acceptance Criteria

- [ ] Comprehensive syntect-scope → Rouge-class mapping table
- [ ] Per-language test coverage for top 10 languages in benchmark sites
- [ ] Zero syntax highlighting diffs on mojombo-blog (3 pages)
- [ ] Significant reduction in syntax highlighting diffs across all sites
- [ ] `cargo test` passes, `cargo clippy` clean
- [ ] Rouge MIT + Pygments BSD copyright notices included
