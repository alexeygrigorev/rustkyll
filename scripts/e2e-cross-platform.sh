#!/usr/bin/env bash
# e2e-cross-platform.sh -- Cross-platform end-to-end testing via Docker (Windows + macOS)
#
# Runs rustkyll inside real Windows and macOS VMs (via dockur Docker containers)
# and compares the output against a Linux baseline.
#
# Prerequisites:
#   - Docker installed and running
#   - KVM available (/dev/kvm must exist) -- not needed for --dry-run
#   - dockurr/windows:latest image pulled (docker pull dockurr/windows)
#   - dockurr/macos:latest image pulled for macOS tests (docker pull dockurr/macos)
#   - Sufficient RAM: ~4-8 GB per Windows container, ~8+ GB for macOS
#   - Sufficient disk: ~20 GB free for VM images
#
# Expected runtime:
#   - Windows boot: 5-15 minutes
#   - macOS boot: 10-20 minutes
#   - Total for both platforms: 30-60 minutes
#
# Usage:
#   # Using pre-built binaries (recommended):
#   ./scripts/e2e-cross-platform.sh --binary-dir ./release-binaries
#
#   # Test only Windows:
#   ./scripts/e2e-cross-platform.sh --platform windows --binary-dir ./release-binaries
#
#   # Test only macOS:
#   ./scripts/e2e-cross-platform.sh --platform macos --binary-dir ./release-binaries
#
#   # Download binaries from a GitHub release:
#   ./scripts/e2e-cross-platform.sh --release v0.1.0
#
#   # Dry run (validate prerequisites, print plan, do not start containers):
#   ./scripts/e2e-cross-platform.sh --dry-run
#
# Troubleshooting:
#   - "KVM not available": Ensure /dev/kvm exists. On bare metal, check BIOS virtualization.
#     In a VM, enable nested virtualization.
#   - "Container failed to boot": Check RAM. Windows needs 4+ GB free, macOS 8+ GB.
#     Try increasing RAM_SIZE env var in the container.
#   - "Binary not found": Use --binary-dir pointing to a directory with
#     rustkyll-windows-amd64.exe and/or rustkyll-darwin-amd64
#   - "Image not pulled": Run docker pull dockurr/windows:latest (or dockurr/macos:latest)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# --- Defaults ---
PLATFORM="all"
BINARY_DIR=""
RELEASE_TAG=""
DRY_RUN=0
SITE_DIR="$PROJECT_DIR/datatalksclub.github.io"
WORK_DIR="/tmp/rustkyll-cross-platform-e2e"
BOOT_TIMEOUT=900  # 15 minutes
COMMAND_TIMEOUT=600  # 10 minutes for rustkyll build

# Container names (used for cleanup)
WIN_CONTAINER="rustkyll-e2e-windows"
MAC_CONTAINER="rustkyll-e2e-macos"
CONTAINERS_TO_CLEANUP=()

# --- Argument parsing ---
while [[ $# -gt 0 ]]; do
    case "$1" in
        --platform)
            PLATFORM="$2"
            if [[ "$PLATFORM" != "windows" && "$PLATFORM" != "macos" && "$PLATFORM" != "all" ]]; then
                echo "ERROR: --platform must be one of: windows, macos, all"
                exit 1
            fi
            shift 2
            ;;
        --binary-dir)
            BINARY_DIR="$2"
            shift 2
            ;;
        --release)
            RELEASE_TAG="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        --site-dir)
            SITE_DIR="$2"
            shift 2
            ;;
        --boot-timeout)
            BOOT_TIMEOUT="$2"
            shift 2
            ;;
        --help|-h)
            head -45 "$0" | tail -40
            exit 0
            ;;
        *)
            echo "ERROR: Unknown option: $1"
            echo "Run with --help to see usage."
            exit 1
            ;;
    esac
done

# --- Color output helpers ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
fail()  { echo -e "${RED}[FAIL]${NC}  $*"; }
phase() { echo -e "\n${BLUE}=== $* ===${NC}"; }

# --- Cleanup trap ---
cleanup() {
    local exit_code=$?
    if [[ ${#CONTAINERS_TO_CLEANUP[@]} -gt 0 ]]; then
        phase "Cleaning up containers"
        for container in "${CONTAINERS_TO_CLEANUP[@]}"; do
            if docker ps -a --format '{{.Names}}' | grep -q "^${container}$" 2>/dev/null; then
                info "Stopping and removing container: $container"
                docker stop "$container" --time 10 2>/dev/null || true
                docker rm -f "$container" 2>/dev/null || true
            fi
        done
    fi
    exit "$exit_code"
}
trap cleanup EXIT INT TERM

# --- Prerequisite checks ---
check_prerequisites() {
    phase "Checking prerequisites"
    local failed=0

    # Docker
    if command -v docker &>/dev/null; then
        ok "Docker is installed"
    else
        fail "Docker is not installed"
        failed=1
    fi

    if docker info &>/dev/null; then
        ok "Docker daemon is running"
    else
        fail "Docker daemon is not running (try: sudo systemctl start docker)"
        failed=1
    fi

    # KVM (not required for --dry-run)
    if [[ "$DRY_RUN" -eq 0 ]]; then
        if [[ -e /dev/kvm ]]; then
            ok "KVM is available (/dev/kvm exists)"
        else
            fail "KVM is not available (/dev/kvm does not exist)"
            echo "    On bare metal: enable VT-x/AMD-V in BIOS"
            echo "    In a VM: enable nested virtualization"
            failed=1
        fi
    else
        if [[ -e /dev/kvm ]]; then
            ok "KVM is available (/dev/kvm exists)"
        else
            warn "KVM is not available (not required for --dry-run)"
        fi
    fi

    # Docker images
    if [[ "$PLATFORM" == "windows" || "$PLATFORM" == "all" ]]; then
        if docker image inspect dockurr/windows:latest &>/dev/null; then
            ok "dockurr/windows:latest image is available"
        else
            fail "dockurr/windows:latest image not found (run: docker pull dockurr/windows)"
            failed=1
        fi
    fi

    if [[ "$PLATFORM" == "macos" || "$PLATFORM" == "all" ]]; then
        if docker image inspect dockurr/macos:latest &>/dev/null; then
            ok "dockurr/macos:latest image is available"
        else
            fail "dockurr/macos:latest image not found (run: docker pull dockurr/macos)"
            failed=1
        fi
    fi

    # Site directory
    if [[ -d "$SITE_DIR" ]]; then
        ok "Site directory exists: $SITE_DIR"
    else
        fail "Site directory not found: $SITE_DIR"
        failed=1
    fi

    # Linux binary (for baseline)
    if [[ -x "$PROJECT_DIR/target/release/rustkyll" ]]; then
        ok "Linux binary exists: $PROJECT_DIR/target/release/rustkyll"
    else
        warn "Linux binary not found. Will attempt to build."
    fi

    return $failed
}

# --- Binary acquisition ---
acquire_binaries() {
    phase "Acquiring platform binaries"

    local need_windows=0
    local need_macos=0
    [[ "$PLATFORM" == "windows" || "$PLATFORM" == "all" ]] && need_windows=1
    [[ "$PLATFORM" == "macos" || "$PLATFORM" == "all" ]] && need_macos=1

    mkdir -p "$WORK_DIR/binaries"

    if [[ -n "$BINARY_DIR" ]]; then
        # Mode 1: Use pre-built binaries from directory
        info "Using pre-built binaries from: $BINARY_DIR"

        if [[ ! -d "$BINARY_DIR" ]]; then
            fail "Binary directory does not exist: $BINARY_DIR"
            return 1
        fi

        local missing=0
        if [[ "$need_windows" -eq 1 ]]; then
            if [[ -f "$BINARY_DIR/rustkyll-windows-amd64.exe" ]]; then
                cp "$BINARY_DIR/rustkyll-windows-amd64.exe" "$WORK_DIR/binaries/rustkyll.exe"
                ok "Windows binary: rustkyll-windows-amd64.exe"
            else
                fail "Missing: $BINARY_DIR/rustkyll-windows-amd64.exe"
                missing=1
            fi
        fi

        if [[ "$need_macos" -eq 1 ]]; then
            if [[ -f "$BINARY_DIR/rustkyll-darwin-amd64" ]]; then
                cp "$BINARY_DIR/rustkyll-darwin-amd64" "$WORK_DIR/binaries/rustkyll-macos"
                chmod +x "$WORK_DIR/binaries/rustkyll-macos"
                ok "macOS binary: rustkyll-darwin-amd64"
            else
                fail "Missing: $BINARY_DIR/rustkyll-darwin-amd64"
                missing=1
            fi
        fi

        if [[ "$missing" -eq 1 ]]; then
            return 1
        fi

    elif [[ -n "$RELEASE_TAG" ]]; then
        # Mode 2: Download from GitHub release
        info "Downloading binaries from GitHub release: $RELEASE_TAG"

        local repo
        repo=$(gh repo view --json nameWithOwner -q '.nameWithOwner' 2>/dev/null || echo "")
        if [[ -z "$repo" ]]; then
            fail "Could not determine GitHub repository. Ensure 'gh' is authenticated."
            return 1
        fi

        if [[ "$need_windows" -eq 1 ]]; then
            info "Downloading rustkyll-windows-amd64.exe..."
            if gh release download "$RELEASE_TAG" \
                --repo "$repo" \
                --pattern "rustkyll-windows-amd64.exe" \
                --dir "$WORK_DIR/binaries" 2>/dev/null; then
                mv "$WORK_DIR/binaries/rustkyll-windows-amd64.exe" "$WORK_DIR/binaries/rustkyll.exe"
                ok "Downloaded Windows binary"
            else
                fail "Failed to download rustkyll-windows-amd64.exe from release $RELEASE_TAG"
                return 1
            fi
        fi

        if [[ "$need_macos" -eq 1 ]]; then
            info "Downloading rustkyll-darwin-amd64..."
            if gh release download "$RELEASE_TAG" \
                --repo "$repo" \
                --pattern "rustkyll-darwin-amd64" \
                --dir "$WORK_DIR/binaries" 2>/dev/null; then
                mv "$WORK_DIR/binaries/rustkyll-darwin-amd64" "$WORK_DIR/binaries/rustkyll-macos"
                chmod +x "$WORK_DIR/binaries/rustkyll-macos"
                ok "Downloaded macOS binary"
            else
                fail "Failed to download rustkyll-darwin-amd64 from release $RELEASE_TAG"
                return 1
            fi
        fi

    else
        # Mode 3: Cross-compile using cross
        info "Attempting cross-compilation using 'cross'"

        if ! command -v cross &>/dev/null; then
            fail "'cross' is not installed. Install with: cargo install cross"
            echo "    Alternatively, use --binary-dir or --release to provide pre-built binaries."
            return 1
        fi

        if [[ "$need_windows" -eq 1 ]]; then
            info "Cross-compiling for x86_64-pc-windows-msvc..."
            if (cd "$PROJECT_DIR" && cross build --release --target x86_64-pc-windows-msvc); then
                cp "$PROJECT_DIR/target/x86_64-pc-windows-msvc/release/rustkyll.exe" \
                   "$WORK_DIR/binaries/rustkyll.exe"
                ok "Windows binary cross-compiled"
            else
                fail "Cross-compilation for Windows failed"
                return 1
            fi
        fi

        if [[ "$need_macos" -eq 1 ]]; then
            info "Cross-compiling for x86_64-apple-darwin..."
            if (cd "$PROJECT_DIR" && cross build --release --target x86_64-apple-darwin); then
                cp "$PROJECT_DIR/target/x86_64-apple-darwin/release/rustkyll" \
                   "$WORK_DIR/binaries/rustkyll-macos"
                chmod +x "$WORK_DIR/binaries/rustkyll-macos"
                ok "macOS binary cross-compiled"
            else
                fail "Cross-compilation for macOS failed"
                return 1
            fi
        fi
    fi

    ok "All required binaries acquired"
    return 0
}

# --- Linux baseline generation ---
generate_linux_baseline() {
    phase "Generating Linux baseline"

    local linux_binary="$PROJECT_DIR/target/release/rustkyll"
    local linux_output="$WORK_DIR/output-linux"

    if [[ ! -x "$linux_binary" ]]; then
        info "Building Linux binary..."
        (cd "$PROJECT_DIR" && ./scripts/cargo-safe build --release)
    fi

    rm -rf "$linux_output"
    info "Running rustkyll build on Linux..."
    "$linux_binary" build --source "$SITE_DIR" --destination "$linux_output" 2>&1 | tail -5

    local file_count
    file_count=$(find "$linux_output" -type f | wc -l)
    ok "Linux baseline generated: $file_count files in $linux_output"
}

# --- Wait for dockur container to boot ---
# Dockur containers log specific messages when the OS is ready.
# For Windows: "Windows is ready" or BCD boot message
# For macOS: "macOS is ready" or similar
wait_for_boot() {
    local container="$1"
    local platform="$2"
    local timeout="$3"

    info "Waiting for $platform to boot (timeout: ${timeout}s)..."
    local start_time
    start_time=$(date +%s)

    while true; do
        local elapsed
        elapsed=$(( $(date +%s) - start_time ))

        if [[ "$elapsed" -ge "$timeout" ]]; then
            fail "$platform failed to boot within ${timeout}s"
            return 1
        fi

        # Check container is still running
        if ! docker ps --format '{{.Names}}' | grep -q "^${container}$" 2>/dev/null; then
            fail "$platform container stopped unexpectedly"
            docker logs "$container" 2>&1 | tail -20
            return 1
        fi

        # Check logs for boot completion
        local logs
        logs=$(docker logs "$container" 2>&1 || true)

        if [[ "$platform" == "windows" ]]; then
            # dockur/windows logs "Ready" or provides RDP when Windows is booted
            if echo "$logs" | grep -qi "ready\|started successfully\|BCD\|rdp"; then
                ok "$platform booted in ${elapsed}s"
                return 0
            fi
        elif [[ "$platform" == "macOS" ]]; then
            if echo "$logs" | grep -qi "ready\|started successfully\|vnc"; then
                ok "$platform booted in ${elapsed}s"
                return 0
            fi
        fi

        # Print progress every 30 seconds
        if (( elapsed % 30 == 0 )) && (( elapsed > 0 )); then
            info "Still waiting for $platform boot... (${elapsed}s elapsed)"
        fi

        sleep 5
    done
}

# --- Run test on Windows ---
test_windows() {
    phase "Testing on Windows"

    local shared_dir="$WORK_DIR/win-shared"
    local output_dir="$WORK_DIR/output-windows"

    rm -rf "$shared_dir" "$output_dir"
    mkdir -p "$shared_dir/site" "$shared_dir/output"

    # Copy binary and site into shared folder
    info "Preparing shared folder for Windows container..."
    cp "$WORK_DIR/binaries/rustkyll.exe" "$shared_dir/"

    # Copy site files (exclude .git and large unnecessary directories)
    rsync -a --exclude='.git' --exclude='node_modules' --exclude='vendor' \
        "$SITE_DIR/" "$shared_dir/site/"

    # Create a batch script that Windows will run
    # The shared folder appears as \\host.lan\Data inside Windows (dockur convention)
    cat > "$shared_dir/run-rustkyll.bat" << 'BATCH_EOF'
@echo off
echo === Starting rustkyll build on Windows ===
cd /d \\host.lan\Data
echo Current directory: %CD%
dir rustkyll.exe
echo.
echo Running rustkyll build...
rustkyll.exe build --source site --destination output
if %ERRORLEVEL% NEQ 0 (
    echo RUSTKYLL_STATUS=FAILED
    exit /b 1
)
echo RUSTKYLL_STATUS=SUCCESS
echo === Build complete ===
dir output /s /b | find /c /v ""
BATCH_EOF

    # Start Windows container
    info "Starting Windows container..."
    docker run -d \
        --name "$WIN_CONTAINER" \
        --device /dev/kvm \
        --cap-add NET_ADMIN \
        -v "$shared_dir:/data" \
        -e RAM_SIZE=4G \
        -e CPU_CORES=2 \
        -p 8006:8006 \
        -p 3389:3389 \
        dockurr/windows:latest

    CONTAINERS_TO_CLEANUP+=("$WIN_CONTAINER")

    # Wait for Windows to boot
    if ! wait_for_boot "$WIN_CONTAINER" "windows" "$BOOT_TIMEOUT"; then
        fail "Windows boot timed out"
        return 1
    fi

    # Give Windows a bit more time after initial boot detection to stabilize
    info "Waiting 30s for Windows to stabilize after boot..."
    sleep 30

    # The shared folder at /data is mounted as a Samba share accessible via \\host.lan\Data
    # We need to execute the batch script. With dockur, we can use docker exec to run
    # commands in the QEMU management context, but to run inside Windows we need
    # to use the RDP/SSH or wait for Windows to auto-run a script.
    #
    # dockur supports automatic script execution: if a .bat file is placed at
    # /data/oem/runonce.bat, it will be executed on first login.
    # However, for more control, we use the built-in Samba share approach:
    # 1. The /data directory is shared as \\host.lan\Data inside Windows
    # 2. We can use smbclient from the host to interact, or
    # 3. We rely on the run-rustkyll.bat being invoked via auto-login

    # Alternative approach: use docker exec to run commands in the container's
    # helper environment, which can trigger execution inside the VM
    info "Attempting to run rustkyll inside Windows VM..."
    info "Executing via shared folder mechanism..."

    # Wait for the output to appear (the batch script runs inside Windows)
    # We poll the shared output directory for results
    local cmd_start
    cmd_start=$(date +%s)

    # Try to trigger execution via the container's built-in mechanisms
    # dockur containers allow executing commands via the /run/exec endpoint
    # or through docker exec which accesses the host-side helper
    docker exec "$WIN_CONTAINER" bash -c '
        # Wait for Windows to be accessible via Samba
        for i in $(seq 1 60); do
            if smbclient //localhost/Data -N -c "ls" 2>/dev/null | grep -q "rustkyll"; then
                echo "Samba share accessible"
                break
            fi
            sleep 5
        done
        # Execute the batch file inside Windows via smbclient/winexe or similar
        # The dockur container has tools to interact with the guest VM
    ' 2>/dev/null || true

    # Poll for output files (the batch script writes to shared_dir/output)
    info "Polling for build output (timeout: ${COMMAND_TIMEOUT}s)..."
    while true; do
        local elapsed
        elapsed=$(( $(date +%s) - cmd_start ))

        if [[ "$elapsed" -ge "$COMMAND_TIMEOUT" ]]; then
            fail "Windows build timed out after ${COMMAND_TIMEOUT}s"
            warn "This may mean the batch script did not auto-execute."
            warn "Check the Windows VM via VNC at http://localhost:8006"
            warn "You may need to manually run: \\\\host.lan\\Data\\run-rustkyll.bat"
            return 1
        fi

        # Check if output directory has files
        local output_count
        output_count=$(find "$shared_dir/output" -type f 2>/dev/null | wc -l)
        if [[ "$output_count" -gt 0 ]]; then
            ok "Windows build produced $output_count files in ${elapsed}s"
            break
        fi

        if (( elapsed % 30 == 0 )) && (( elapsed > 0 )); then
            info "Still waiting for Windows build output... (${elapsed}s elapsed)"
        fi

        sleep 5
    done

    # Copy output
    cp -r "$shared_dir/output" "$output_dir"
    ok "Windows output extracted to $output_dir"
    return 0
}

# --- Run test on macOS ---
test_macos() {
    phase "Testing on macOS"

    local shared_dir="$WORK_DIR/mac-shared"
    local output_dir="$WORK_DIR/output-macos"

    rm -rf "$shared_dir" "$output_dir"
    mkdir -p "$shared_dir/site" "$shared_dir/output"

    # Copy binary and site into shared folder
    info "Preparing shared folder for macOS container..."
    cp "$WORK_DIR/binaries/rustkyll-macos" "$shared_dir/rustkyll"
    chmod +x "$shared_dir/rustkyll"

    rsync -a --exclude='.git' --exclude='node_modules' --exclude='vendor' \
        "$SITE_DIR/" "$shared_dir/site/"

    # Create a shell script for macOS to run
    cat > "$shared_dir/run-rustkyll.sh" << 'SH_EOF'
#!/bin/bash
echo "=== Starting rustkyll build on macOS ==="
cd /Volumes/Data 2>/dev/null || cd /mnt/data 2>/dev/null || {
    echo "Could not find shared data volume"
    exit 1
}
chmod +x ./rustkyll
./rustkyll build --source ./site --destination ./output
echo "RUSTKYLL_STATUS=$?"
echo "=== Build complete ==="
SH_EOF

    # Start macOS container
    info "Starting macOS container..."
    docker run -d \
        --name "$MAC_CONTAINER" \
        --device /dev/kvm \
        --cap-add NET_ADMIN \
        -v "$shared_dir:/data" \
        -e RAM_SIZE=8G \
        -e CPU_CORES=2 \
        -p 8007:8006 \
        -p 5901:5900 \
        dockurr/macos:latest

    CONTAINERS_TO_CLEANUP+=("$MAC_CONTAINER")

    # Wait for macOS to boot
    if ! wait_for_boot "$MAC_CONTAINER" "macOS" "$BOOT_TIMEOUT"; then
        fail "macOS boot timed out"
        return 1
    fi

    # Give macOS time to stabilize
    info "Waiting 30s for macOS to stabilize after boot..."
    sleep 30

    info "Attempting to run rustkyll inside macOS VM..."

    # Poll for output files
    local cmd_start
    cmd_start=$(date +%s)
    info "Polling for build output (timeout: ${COMMAND_TIMEOUT}s)..."

    while true; do
        local elapsed
        elapsed=$(( $(date +%s) - cmd_start ))

        if [[ "$elapsed" -ge "$COMMAND_TIMEOUT" ]]; then
            fail "macOS build timed out after ${COMMAND_TIMEOUT}s"
            warn "Check the macOS VM via VNC at http://localhost:8007"
            warn "You may need to manually run the script from the shared volume."
            return 1
        fi

        local output_count
        output_count=$(find "$shared_dir/output" -type f 2>/dev/null | wc -l)
        if [[ "$output_count" -gt 0 ]]; then
            ok "macOS build produced $output_count files in ${elapsed}s"
            break
        fi

        if (( elapsed % 30 == 0 )) && (( elapsed > 0 )); then
            info "Still waiting for macOS build output... (${elapsed}s elapsed)"
        fi

        sleep 5
    done

    # Copy output
    cp -r "$shared_dir/output" "$output_dir"
    ok "macOS output extracted to $output_dir"
    return 0
}

# --- Compare platform output against Linux baseline ---
compare_output() {
    local platform="$1"
    local platform_dir="$WORK_DIR/output-$platform"
    local linux_dir="$WORK_DIR/output-linux"

    phase "Comparing $platform output against Linux baseline"

    if [[ ! -d "$platform_dir" ]]; then
        fail "No output found for $platform at $platform_dir"
        return 1
    fi

    # Normalize CRLF to LF in platform output (Windows produces CRLF)
    if [[ "$platform" == "windows" ]]; then
        info "Normalizing Windows line endings (CRLF -> LF)..."
        find "$platform_dir" -type f \( -name "*.html" -o -name "*.xml" -o -name "*.css" -o -name "*.js" -o -name "*.json" -o -name "*.txt" \) \
            -exec sed -i 's/\r$//' {} +
    fi

    # Compare file trees
    local linux_files platform_files
    linux_files=$(cd "$linux_dir" && find . -type f | sort)
    platform_files=$(cd "$platform_dir" && find . -type f | sort)

    local linux_count platform_count
    linux_count=$(echo "$linux_files" | wc -l)
    platform_count=$(echo "$platform_files" | wc -l)

    info "Linux file count:    $linux_count"
    info "$platform file count: $platform_count"

    # Files only in Linux
    local only_linux
    only_linux=$(comm -23 <(echo "$linux_files") <(echo "$platform_files"))
    local only_linux_count
    only_linux_count=$(echo "$only_linux" | grep -c . || true)

    # Files only in platform
    local only_platform
    only_platform=$(comm -13 <(echo "$linux_files") <(echo "$platform_files"))
    local only_platform_count
    only_platform_count=$(echo "$only_platform" | grep -c . || true)

    # Common files
    local common_files
    common_files=$(comm -12 <(echo "$linux_files") <(echo "$platform_files"))
    local common_count
    common_count=$(echo "$common_files" | grep -c . || true)

    info "Files in common:     $common_count"
    info "Only in Linux:       $only_linux_count"
    info "Only in $platform:   $only_platform_count"

    if [[ "$only_linux_count" -gt 0 ]]; then
        warn "Files missing from $platform output:"
        echo "$only_linux" | head -20 | sed 's/^/    /'
        if [[ "$only_linux_count" -gt 20 ]]; then
            warn "  ... and $((only_linux_count - 20)) more"
        fi
    fi

    if [[ "$only_platform_count" -gt 0 ]]; then
        warn "Extra files in $platform output:"
        echo "$only_platform" | head -20 | sed 's/^/    /'
    fi

    # Compare content of common files
    local diff_count=0
    local checked_count=0

    while IFS= read -r file; do
        [[ -z "$file" ]] && continue
        checked_count=$((checked_count + 1))

        if ! diff -q "$linux_dir/$file" "$platform_dir/$file" &>/dev/null; then
            diff_count=$((diff_count + 1))
            if [[ "$diff_count" -le 5 ]]; then
                warn "Content differs: $file"
                diff --unified=3 "$linux_dir/$file" "$platform_dir/$file" 2>/dev/null | head -20
                echo "    ..."
            fi
        fi
    done <<< "$common_files"

    info "Files checked:  $checked_count"
    info "Files differing: $diff_count"

    # Verdict
    local failed=0
    if [[ "$only_linux_count" -gt 0 || "$only_platform_count" -gt 0 ]]; then
        fail "$platform: file tree mismatch ($only_linux_count missing, $only_platform_count extra)"
        failed=1
    else
        ok "$platform: file tree matches Linux baseline"
    fi

    if [[ "$diff_count" -gt 0 ]]; then
        fail "$platform: $diff_count files have content differences"
        failed=1
    else
        ok "$platform: all file contents match Linux baseline"
    fi

    return $failed
}

# --- Dry run ---
dry_run() {
    phase "Dry run mode"
    info "Validating prerequisites and printing execution plan."
    echo ""

    if ! check_prerequisites; then
        fail "Prerequisites check failed (some may be optional for dry-run)"
    fi

    echo ""
    phase "Execution plan"

    info "Work directory: $WORK_DIR"
    info "Site directory: $SITE_DIR"
    info "Platform(s):    $PLATFORM"

    if [[ -n "$BINARY_DIR" ]]; then
        info "Binary source:  pre-built from $BINARY_DIR"
    elif [[ -n "$RELEASE_TAG" ]]; then
        info "Binary source:  GitHub release $RELEASE_TAG"
    else
        info "Binary source:  cross-compile with 'cross'"
    fi

    echo ""
    info "Steps that would be executed:"
    echo "  1. Acquire platform binaries"
    echo "  2. Generate Linux baseline (run native rustkyll build)"

    if [[ "$PLATFORM" == "windows" || "$PLATFORM" == "all" ]]; then
        echo "  3. Start dockurr/windows container ($WIN_CONTAINER)"
        echo "     - Mount shared folder with binary + site at /data"
        echo "     - Wait for Windows boot (timeout: ${BOOT_TIMEOUT}s)"
        echo "     - Run rustkyll.exe build inside Windows VM"
        echo "     - Extract _site/ output"
        echo "     - Compare with Linux baseline (normalize CRLF)"
        echo "     - Stop and remove container"
    fi

    if [[ "$PLATFORM" == "macos" || "$PLATFORM" == "all" ]]; then
        echo "  4. Start dockurr/macos container ($MAC_CONTAINER)"
        echo "     - Mount shared folder with binary + site at /data"
        echo "     - Wait for macOS boot (timeout: ${BOOT_TIMEOUT}s)"
        echo "     - Run rustkyll build inside macOS VM"
        echo "     - Extract _site/ output"
        echo "     - Compare with Linux baseline"
        echo "     - Stop and remove container"
    fi

    echo "  5. Print summary report (pass/fail per platform)"
    echo "  6. Clean up all containers"

    echo ""
    ok "Dry run complete. Use without --dry-run to execute."
    return 0
}

# --- Main ---
main() {
    phase "Cross-platform end-to-end test"
    info "Platform: $PLATFORM"
    info "Work directory: $WORK_DIR"
    info "Site: $SITE_DIR"
    echo ""

    # Dry run mode
    if [[ "$DRY_RUN" -eq 1 ]]; then
        dry_run
        return $?
    fi

    # Check prerequisites
    if ! check_prerequisites; then
        fail "Prerequisites check failed. Fix the issues above and retry."
        return 1
    fi

    # Create work directory
    mkdir -p "$WORK_DIR"

    # Acquire binaries
    if ! acquire_binaries; then
        fail "Failed to acquire platform binaries"
        return 1
    fi

    # Generate Linux baseline
    generate_linux_baseline

    # Track results
    local win_result="SKIP"
    local mac_result="SKIP"

    # Test Windows
    if [[ "$PLATFORM" == "windows" || "$PLATFORM" == "all" ]]; then
        if test_windows; then
            if compare_output "windows"; then
                win_result="PASS"
            else
                win_result="FAIL (output differs)"
            fi
        else
            win_result="FAIL (build failed)"
        fi
    fi

    # Test macOS
    if [[ "$PLATFORM" == "macos" || "$PLATFORM" == "all" ]]; then
        if test_macos; then
            if compare_output "macos"; then
                mac_result="PASS"
            else
                mac_result="FAIL (output differs)"
            fi
        else
            mac_result="FAIL (build failed)"
        fi
    fi

    # Summary
    phase "Summary"
    echo ""
    echo "  Platform    Result"
    echo "  ----------  ------"
    printf "  %-12s" "Linux"
    echo "PASS (baseline)"
    printf "  %-12s" "Windows"
    if [[ "$win_result" == "PASS" ]]; then
        echo -e "${GREEN}$win_result${NC}"
    elif [[ "$win_result" == "SKIP" ]]; then
        echo -e "${YELLOW}$win_result${NC}"
    else
        echo -e "${RED}$win_result${NC}"
    fi
    printf "  %-12s" "macOS"
    if [[ "$mac_result" == "PASS" ]]; then
        echo -e "${GREEN}$mac_result${NC}"
    elif [[ "$mac_result" == "SKIP" ]]; then
        echo -e "${YELLOW}$mac_result${NC}"
    else
        echo -e "${RED}$mac_result${NC}"
    fi
    echo ""

    # Exit code
    local failed=0
    if [[ "$win_result" == FAIL* ]]; then failed=1; fi
    if [[ "$mac_result" == FAIL* ]]; then failed=1; fi

    if [[ "$failed" -eq 0 ]]; then
        ok "All tested platforms passed!"
    else
        fail "Some platforms failed. See details above."
    fi

    return $failed
}

main
