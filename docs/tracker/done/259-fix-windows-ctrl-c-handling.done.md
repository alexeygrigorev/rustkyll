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

Handle `KeyboardInterrupt` in the Python wrapper so Ctrl+C exits cleanly with exit code 130 and no stacktrace.

## Root Cause

In `python/rustkyll/_main.py` line 73, `subprocess.run(args)` on Windows blocks in `WaitForSingleObject`. When Ctrl+C is pressed, Python raises `KeyboardInterrupt` which is unhandled, producing the full traceback.

## Proposed Fix

Wrap the `subprocess.run` call in a try/except in the Windows branch of `main()`:

```python
try:
    result = subprocess.run(args)
    sys.exit(result.returncode)
except KeyboardInterrupt:
    sys.exit(130)  # Standard exit code for Ctrl+C
```

Exit code 130 is the Unix convention for SIGINT (128 + 2). This matches what most CLI tools do.

No changes needed for the Unix path -- `os.execvp` replaces the Python process entirely, so Python never sees the signal.

## Scope

- File to modify: `python/rustkyll/_main.py` (lines 71-74, the Windows branch)
- File to modify: `python/tests/test_main.py` (add new test class)
- Nothing else changes. No Rust code involved.

## Acceptance Criteria

- [ ] The Windows branch in `main()` wraps `subprocess.run(args)` in `try/except KeyboardInterrupt`
- [ ] When `KeyboardInterrupt` is raised during `subprocess.run`, `sys.exit(130)` is called
- [ ] Normal exit codes from the subprocess are still forwarded unchanged (existing test `test_windows_exit_code_forwarded` still passes)
- [ ] The Unix branch (`os.execvp`) is not modified
- [ ] All existing tests in `python/tests/test_main.py` still pass
- [ ] At least one new test verifies the KeyboardInterrupt handling

## Test Scenarios

### Unit: KeyboardInterrupt handling on Windows path

- Mock `subprocess.run` to raise `KeyboardInterrupt`. Patch `sys.platform` to `"win32"`. Call `main()`. Assert `SystemExit` is raised with code 130.
- Mock `subprocess.run` to return `returncode=0`. Patch `sys.platform` to `"win32"`. Call `main()`. Assert `SystemExit` with code 0 (regression check -- already covered by existing test but confirm it still passes).
- Mock `subprocess.run` to return `returncode=42`. Patch `sys.platform` to `"win32"`. Call `main()`. Assert `SystemExit` with code 42 (regression check -- already covered by existing test).

### Regression: existing tests

- All existing tests in `TestArgumentForwarding`, `TestPlatformDetection`, `TestUnsupportedPlatformError`, etc. must continue to pass unchanged.

## Dependencies

- None

## Log

### [SWE] 2026-03-20
- Wrote test `TestWindowsCtrlCHandling::test_keyboard_interrupt_exits_with_130` in `python/tests/test_main.py`
- Ran test: FAILS as expected -- KeyboardInterrupt propagates unhandled, pytest itself gets interrupted
- Implemented fix: wrapped `subprocess.run(args)` + `sys.exit(result.returncode)` in `try/except KeyboardInterrupt` with `sys.exit(130)` in `python/rustkyll/_main.py`
- Ran all tests: PASSES -- 18 passed, 0 failed
- Unix branch (`os.execvp`) left unchanged
- All existing tests pass (regression check confirmed)
- Files modified: `python/rustkyll/_main.py`, `python/tests/test_main.py`
