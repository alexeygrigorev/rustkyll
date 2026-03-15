# Issue 113: Improve syntect-to-Rouge token mapping

Affects blog/practical-guide-better-code.html (0.08% pixel diff). Syntax highlighting spans differ between syntect and Rouge — different token boundaries and CSS class names for comments, docstrings, YAML keys.

## Acceptance criteria
- Code blocks produce same span structure as Rouge
- Blog post achieves 0% pixel diff
