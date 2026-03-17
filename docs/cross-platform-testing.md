# Cross-Platform End-to-End Testing

## Overview

Test rustkyll on real Windows and macOS by building from source inside Docker VMs.
Uses [dockur/windows](https://github.com/dockur/windows) and [dockur/macos](https://github.com/dockur/macos) which run actual VMs inside Docker using KVM passthrough.

The approach: mount the rustkyll source code into the VM, build it natively on that platform, then test it builds the DTC site correctly.

This is a local, on-demand tool run by QA before every release.

## Prerequisites

- Docker installed and running
- KVM available (`/dev/kvm` must exist)
- Docker images pulled:
  - `docker pull dockurr/windows:latest`
  - `docker pull dockurr/macos:latest`
- ~20 GB free disk space for VM images
- 4 GB RAM for Windows, 8 GB for macOS

## How it works

1. Start a Windows/macOS Docker container with KVM
2. Mount the rustkyll source code into the VM
3. Install Rust toolchain inside the VM
4. Run `cargo build --release` inside the VM (builds native binary)
5. Copy the DTC site into the VM
6. Run `rustkyll build` on the DTC site
7. Compare output against Linux baseline (file tree + file contents)
8. Report pass/fail

## Usage

```bash
# Test on Windows
./scripts/e2e-cross-platform.sh --platform windows

# Test on macOS
./scripts/e2e-cross-platform.sh --platform macos

# Test on both
./scripts/e2e-cross-platform.sh --platform all

# Dry run (check prerequisites only)
./scripts/e2e-cross-platform.sh --dry-run
```

## What gets tested

- Binary compiles on the target platform
- DTC site builds without panics (the Unicode panic #78 was caught this way)
- Output page count matches Linux
- Output file tree matches Linux
- No platform-specific rendering differences

## When to run

- Before every release (QA checklist item)
- After fixing platform-specific bugs
- After changes to frontmatter parsing (CRLF handling)
- After adding new dependencies
