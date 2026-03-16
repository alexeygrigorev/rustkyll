# Issue 154: Fix remaining extra JSON-LD script tags (100 diffs, 99 files)

## Problem

rustkyll emits JSON-LD on 99 pages where Jekyll doesn't. These are extra <script type="application/ld+json"> blocks.

## Goal

Only emit JSON-LD on pages where Jekyll emits it. Investigate which page types get JSON-LD in Jekyll and match.

## Acceptance criteria

- JSON-LD script count matches Jekyll on every page
- No extra JSON-LD on pages Jekyll doesn't have it
- No missing JSON-LD on pages Jekyll does have it
