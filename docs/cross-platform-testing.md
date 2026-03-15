# Cross-Platform End-to-End Testing

## Overview

The `scripts/e2e-cross-platform.sh` script tests rustkyll on real Windows and macOS
environments using Docker containers powered by [dockur](https://github.com/dockur/windows).
These containers run actual VMs inside Docker using KVM passthrough.

This is a **local, on-demand tool** intended to be run by QA before every release.
It is NOT a CI workflow (KVM is not available on standard GitHub Actions runners).

## Prerequisites

- **Docker** installed and running
- **KVM** available (`/dev/kvm` must exist)
  - On bare metal: enable VT-x/AMD-V in BIOS
  - In a VM: enable nested virtualization
- **Docker images** pulled:
  - `docker pull dockurr/windows:latest` (for Windows testing)
  - `docker pull dockurr/macos:latest` (for macOS testing)
- **Disk space**: ~20 GB free for VM images
- **RAM**:
  - Windows container: 4 GB
  - macOS container: 8 GB
  - Plus your normal system usage
- **rustkyll binaries** for each target platform (see Binary Acquisition below)

## Usage

### Basic usage with pre-built binaries

```bash
# Test both Windows and macOS
./scripts/e2e-cross-platform.sh --binary-dir ./release-binaries

# Test only Windows
./scripts/e2e-cross-platform.sh --platform windows --binary-dir ./release-binaries

# Test only macOS
./scripts/e2e-cross-platform.sh --platform macos --binary-dir ./release-binaries
```

### Download binaries from a GitHub release

```bash
./scripts/e2e-cross-platform.sh --release v0.1.0
```

### Cross-compile (requires `cross` tool)

```bash
# Will attempt to cross-compile for each platform
./scripts/e2e-cross-platform.sh
```

### Dry run (validate prerequisites only)

```bash
./scripts/e2e-cross-platform.sh --dry-run
```

## Binary Acquisition

The script supports three modes for obtaining platform-specific binaries:

1. **`--binary-dir <path>`**: Use pre-built binaries from a local directory.
   Expected files:
   - `rustkyll-windows-amd64.exe` (Windows)
   - `rustkyll-darwin-amd64` (macOS)

2. **`--release <tag>`**: Download binaries from a GitHub release using `gh`.

3. **Default (no flag)**: Cross-compile using the `cross` tool.

## Expected Runtime

| Phase | Duration |
|-------|----------|
| Windows boot | 5-15 minutes |
| macOS boot | 10-20 minutes |
| rustkyll build (per platform) | 1-5 minutes |
| Output comparison | < 1 minute |
| **Total (both platforms)** | **30-60 minutes** |

## How It Works

1. **Linux baseline**: Runs the native Linux rustkyll binary to generate reference output
2. **Container startup**: Launches a dockur VM container with KVM passthrough
3. **Shared folder**: Mounts binary + site source as `/data` (appears as `\\host.lan\Data` in Windows, or as a mounted volume in macOS)
4. **Build execution**: Runs rustkyll inside the VM via the shared folder
5. **Output extraction**: Copies the generated `_site/` output back to the host
6. **Comparison**: Compares file tree and contents against the Linux baseline
   - Windows output is normalized (CRLF -> LF) before comparison
7. **Cleanup**: Stops and removes all containers (even on failure/Ctrl+C)

## Troubleshooting

### KVM not available
```
[FAIL] KVM is not available (/dev/kvm does not exist)
```
- On bare metal: check BIOS for VT-x/AMD-V setting
- In a VM: enable nested virtualization on the hypervisor
- Verify: `ls -la /dev/kvm`

### Container failed to boot
- Check available RAM: `free -h`
- Windows needs 4+ GB free, macOS needs 8+ GB
- Try increasing boot timeout: `--boot-timeout 1200`
- Check container logs: `docker logs rustkyll-e2e-windows`

### Binary not found
```
[FAIL] Missing: /path/to/binaries/rustkyll-windows-amd64.exe
```
- Download release binaries: `gh release download v0.1.0`
- Or build them via the release workflow

### Image not pulled
```
[FAIL] dockurr/windows:latest image not found
```
- Run: `docker pull dockurr/windows:latest`
- For macOS: `docker pull dockurr/macos:latest`

### Orphaned containers
If the script was killed without cleanup:
```bash
docker stop rustkyll-e2e-windows rustkyll-e2e-macos 2>/dev/null
docker rm rustkyll-e2e-windows rustkyll-e2e-macos 2>/dev/null
```

## Flags Reference

| Flag | Description | Default |
|------|-------------|---------|
| `--platform <windows\|macos\|all>` | Which platform(s) to test | `all` |
| `--binary-dir <path>` | Directory with pre-built binaries | (none) |
| `--release <tag>` | Download binaries from GitHub release | (none) |
| `--dry-run` | Validate prerequisites, print plan, do not run | off |
| `--site-dir <path>` | Path to the site source | `datatalksclub.github.io/` |
| `--boot-timeout <seconds>` | Max time to wait for VM boot | 900 (15 min) |
| `--help` | Show usage information | |
