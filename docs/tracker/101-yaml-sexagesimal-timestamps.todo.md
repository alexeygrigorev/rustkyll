# Issue 101: Fix YAML sexagesimal podcast timestamp formatting (D13)

Descoped from issue #90. YAML 1.1 interprets `0:30` as sexagesimal (30 seconds) while YAML 1.2 treats it as string. Affects 2 podcast timestamps per episode. High risk to change YAML parser globally.

## Acceptance criteria
- Podcast timestamps `0:30` render as `0:30` not `30` or `0.5`
- No regressions on other YAML parsing
