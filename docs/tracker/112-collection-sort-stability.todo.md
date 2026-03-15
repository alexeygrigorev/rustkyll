# Issue 112: Fix collection sort stability for tie-breaking

Affects podcast.html (0.05% pixel diff). Two episodes with identical season=3, episode=4 appear in different order. Need consistent tie-breaking (e.g. by filename) when sort keys are equal.

## Acceptance criteria
- Same-value sort keys produce same order as Jekyll
- podcast.html achieves 0% pixel diff
