# Issue 341: DTC mastering-spacy code block parsed as fenced block in list (22 diffs)

## Problem

On `books/20211213-mastering-spacy.html`, backtick-delimited code sequences inside YAML comment text (processed via `newline_to_br | markdownify` pipeline) are incorrectly parsed as fenced code block boundaries.

The source YAML contains text like:
```
Here's an example:
```>>> import spacy
>>> nlp = spacy.load("en_core_web_md")
...
# Then do your stuff with the pos tags```
```

Jekyll keeps this as inline text within the `<li>`, rendering the backticks literally and the `#` line as plain text. Rustkyll parses the triple backticks as fenced code block delimiters, producing a `<pre><code>` block with mangled `class="language->>>"`  attribute, and the `# Then do your stuff` line becomes an `<h1>` heading.

This accounts for 22 of the 24 DOM diffs on this page (the other 2 are br-sublist nesting diffs covered by issue 336).

## Root cause

The `markdown_to_html_for_filter` function (or pulldown-cmark) treats triple backticks inside list item text as fenced code block boundaries. In the `newline_to_br | markdownify` pipeline, each `\n` has been converted to `<br />\n`, so the backtick sequences appear at line boundaries and are parsed as code fences.

## Descoped from

Issue 337 sub-issue E (originally described as "2 diffs" but actually 24 diffs and not a quick win).

## Dependencies

- Issue 336 covers the br-sublist nesting portion of this page (2 of 24 diffs)
- This issue covers the remaining 22 diffs
