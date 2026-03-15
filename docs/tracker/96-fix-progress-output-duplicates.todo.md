# Issue 96: Fix duplicate progress output lines

## Problem

Build output shows duplicate lines for each phase — first the phase name, then the same name with counts:

```
Loading data files...
Loading data files... 6 files
Loading collections...
Loading collections... 7 collections, 777 items
Copying static files...
Copying static files... 1453 files
```

Should be a single line per phase, updated in place:

```
Loading data files... 6 files
Loading collections... 7 collections, 777 items
Copying static files... 1453 files
```

## Goal

Each phase shows ONE line with the final count. No duplicate "starting" line. On TTY, update the line in place. On non-TTY (piped), print once after the phase completes.

## Acceptance criteria

- Each phase appears as one line in output (not two)
- Line shows phase name + count (e.g. "Loading collections... 7 collections, 777 items")
- On TTY: line updates in place (carriage return)
- On non-TTY: single line printed after phase completes
- Rendering phase still shows progress bar
- --quiet suppresses everything
- No performance regression
