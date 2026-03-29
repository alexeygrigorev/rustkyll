# Issue 534: Syntax highlighting token class mismatches in minima code blocks

## Problem

The `codeblocks-ahoy.html` post in minima contains multiple code blocks in Ruby, Diff,
Sass, YAML, HTML, and Liquid. Rustkyll's syntax highlighter produces different token
classes than Jekyll/Rouge for many tokens, causing ~200 DOM differences on this one page.

This issue is specific to minima's code blocks but shares root causes with issue 471
(general syntax highlighting token mismatches).

### Key differences

**Ruby tokens:**
- `=begin`/`=end` multi-line comments: Jekyll uses `<span class="cm">`, rustkyll uses `<span class="s">`
- Integer literals: Jekyll `mi`, rustkyll `m`
- Float literals: Jekyll `mf`, rustkyll `m`
- Hex literals: Jekyll `mh`, rustkyll `m`
- Octal literals: Jekyll `mo`, rustkyll `m`
- Include keyword: Jekyll `kp`, rustkyll `nf`
- Constants (LIPSUM, Enumerable): Jekyll `no`, rustkyll `n`
- Instance variables (@layout): Jekyll `vi`, rustkyll `n`
- String delimiters: Jekyll wraps full `"string"` in `s2`, rustkyll uses separate `dl`+`s2`+`dl`
- Symbols (:layout): Jekyll `ss`, rustkyll `no`
- Boolean false: Jekyll `kp`, rustkyll `kc`
- Rescue/ArgumentError: Jekyll `no`, rustkyll `n`
- Operator `||=`: Jekyll single `o`, rustkyll split `ow`+`o`

**Diff tokens:**
- Deleted lines: Jekyll `gd`, rustkyll `p`
- Added lines: Jekyll `gi`, rustkyll `p`
- Unchanged lines: Jekyll `p`, rustkyll matches

**Sass tokens:**
- Selector (.card): Jekyll `nc`, rustkyll `na`
- Properties: Jekyll `nl`, rustkyll omitted/different
- Values: Jekyll uses `nb`/`m`/`mh`, rustkyll uses different classes

**Liquid tokens:**
- Jekyll highlights Liquid syntax with proper token classes
- Rustkyll outputs plain text (no highlighting at all)

**HTML in highlight tag:**
- `<span class="hll">` for highlighted lines: Jekyll supports, rustkyll does not
- `<meta charset="utf-8" />`: Jekyll `nt`+`na`+`s`+`nt`, rustkyll `nt`+`na`+`s`+`p`
  (self-closing `/>` is `nt` in Jekyll, `p` in rustkyll)

**YAML tokens:**
- Numeric values: Jekyll `m`, rustkyll `s`

## Dependencies

- Related to issue 471 (syntax-highlighting-token-mismatches) but minima-specific
- Related to issue 443 (jekyll-vitepress-syntax-highlighting)

## Scope

This is a large issue spanning multiple language grammars. Focus on:
1. Ruby token class mappings (biggest impact: ~150 diffs)
2. Diff token class mappings (~10 diffs)
3. Liquid syntax highlighting support (~10 diffs)
4. HTML highlight line support (`hll` class) (~20 diffs)

## Acceptance Criteria

- [ ] `cargo build` compiles without errors
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` produces no changes
- [ ] `cargo test` passes
- [ ] DTC DOM baseline: 790/790 (must not regress)
- [ ] Minima `codeblocks-ahoy.html`: Ruby `=begin`/`=end` uses class `cm` (not `s`)
- [ ] Minima `codeblocks-ahoy.html`: Integer literals use class `mi` (not `m`)
- [ ] Minima `codeblocks-ahoy.html`: Diff deleted lines use class `gd` (not `p`)
- [ ] Minima `codeblocks-ahoy.html`: Diff added lines use class `gi` (not `p`)
- [ ] Minima `codeblocks-ahoy.html`: Liquid code block has syntax highlighting
- [ ] At least 5 new unit tests for the corrected token classes

## Test Scenarios

### Unit: Ruby highlighting
- Ruby `=begin`/`=end` block -> token class `cm`
- Ruby integer `42` -> token class `mi`
- Ruby float `3.14` -> token class `mf`
- Ruby `include` keyword -> token class `kp`
- Ruby instance variable `@foo` -> token class `vi`

### Unit: Diff highlighting
- Diff `- deleted line` -> token class `gd`
- Diff `+ added line` -> token class `gi`

### Unit: Liquid highlighting
- Liquid `{% assign %}` -> proper token classes (not plain text)

### Integration: minima build
- Build minima, count DOM diffs on codeblocks-ahoy.html (target: < 50, down from 225)

## Baselines

- DTC: 790/790
- Minima codeblocks-ahoy.html: 225 diffs (this is the biggest single-page diff)
