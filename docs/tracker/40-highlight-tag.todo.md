# Issue 40: Support `{% highlight %}` Tag for Syntax Highlighting

## Problem

Complex site testing (Issue 35) revealed that many Jekyll sites use the `{% highlight lang %}...{% endhighlight %}` tag for syntax-highlighted code blocks. This tag is not recognized by rustkyll's Liquid engine, causing a parse error.

## Affected Sites

- Hyde (poole/hyde) -- `{% highlight js %}`
- So Simple Theme -- `{% highlight scss %}`

## Requirements

- Implement the `{% highlight lang %}...{% endhighlight %}` block tag
- At minimum, wrap the content in `<pre><code class="language-{lang}">...</code></pre>` (matching Jekyll's output without a highlighter)
- Optionally support the `linenos` parameter

## Dependencies

None.
