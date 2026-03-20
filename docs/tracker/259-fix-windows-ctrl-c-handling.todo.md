# Issue 259: Handle Ctrl+C gracefully on Windows Python wrapper

## Problem

When users press Ctrl+C to stop rustkyll on Windows, the Python wrapper shows an ugly `KeyboardInterrupt` stacktrace instead of exiting quietly.

```
  File "...\rustkyll\_main.py", line 73, in main
    result = subprocess.run(args)
  ...
KeyboardInterrupt
```

## Goal

Handle `KeyboardInterrupt` in the Python wrapper so Ctrl+C exits cleanly without a stacktrace.

## Root Cause

In `python/rustkyll/_main.py` line 73, `subprocess.run(args)` on Windows blocks in `WaitForSingleObject`. When Ctrl+C is pressed, Python raises `KeyboardInterrupt` which is unhandled, producing the full traceback.

## Proposed Fix

Wrap the `subprocess.run` call in a try/except:

```python
try:
    result = subprocess.run(args)
    sys.exit(result.returncode)
except KeyboardInterrupt:
    sys.exit(130)  # Standard exit code for Ctrl+C
```

Exit code 130 is the Unix convention for SIGINT (128 + 2). This matches what most CLI tools do.

## Acceptance Criteria

- [ ] Ctrl+C on Windows exits with code 130, no stacktrace
- [ ] Ctrl+C on Unix still works (os.execvp replaces the process, so Python never sees it)
- [ ] Normal exit codes are preserved
- [ ] Tests pass

## Dependencies

- None
