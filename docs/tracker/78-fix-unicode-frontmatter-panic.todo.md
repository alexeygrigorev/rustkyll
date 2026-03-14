# Issue 78: Fix Unicode byte boundary panic in frontmatter parsing

## Priority

CRITICAL — blocks actual usage of rustkyll on the DTC site via `uvx rustkyll build`. Confirmed still present in v0.1.3.

## Problem

On Windows, building the DTC site panics at `src/frontmatter.rs:61:33`:

```
byte index 31342 is not a char boundary; it is inside ''' (bytes 31341..31344)
```

The curly/smart quote `'` (U+2019, 3 bytes in UTF-8) is being sliced at a byte boundary that falls inside the multi-byte character. This is a string slicing bug where code uses byte indices on a UTF-8 string without checking character boundaries.

The affected content is a podcast episode front matter with:
```yaml
title: 'Building a Sustainable Data Freelancing Career: Market Validation, Client
  Acquisition & Strategic Positioning'
```

The closing `'` is a curly quote (U+2019), not a straight ASCII quote.

## Root cause

In `src/frontmatter.rs` around line 61, there's likely a string slice operation like `&content[..idx]` or `&content[idx..]` where `idx` is a byte position that lands inside a multi-byte UTF-8 character. Rust panics on this because string slices must be at char boundaries.

## Goal

Fix the frontmatter parser to handle multi-byte UTF-8 characters correctly. No panics on any valid UTF-8 input.

## Reproduction

Build the DTC site — the panic occurs on a podcast episode with curly quotes in the title.

The DTC site must be at the latest commit to reproduce:
```
commit 8a9789e4dd13ccf666cec18080c5f1705a9fb082 (HEAD -> main, origin/main)
Author: Alexey Grigorev
Date:   Thu Mar 12 21:01:24 2026 +0100
    Add Snowplow sponsor and adjust sponsor logo sizes
```

The local copy used for development may be older and missing the problematic podcast episode.

To reproduce:
```bash
# Update or clone the latest DTC site
git clone --depth 1 https://github.com/DataTalksClub/datatalksclub.github.io.git
cargo run --release -- build --source datatalksclub.github.io/
```

If it doesn't panic on Linux, the issue may be related to line ending differences (CRLF vs LF) affecting byte offsets. Git on Windows may check out files with CRLF, shifting byte positions.

## Approach

1. Find the exact line in frontmatter.rs that does the byte-index slice
2. Replace with char-boundary-safe slicing (e.g., `content.get(..idx)`, or find the nearest char boundary)
3. Add unit tests with multi-byte UTF-8 characters in front matter (curly quotes, emoji, CJK characters)
4. Test on the actual DTC site

## Cross-platform testing

Consider using Docker for Windows testing:
- `docker run -it mcr.microsoft.com/windows/servercore:ltsc2022` (requires Windows host with Hyper-V)
- Or use cross-compilation: `cargo build --target x86_64-pc-windows-gnu` on Linux, then test the binary via Wine
- Or use the CI Windows runner to run the test

## Dependencies

None

## Acceptance criteria

- No panic on DTC site build (the specific podcast episode with curly quotes)
- No panic on any valid UTF-8 front matter input
- Unit tests with multi-byte characters (curly quotes, emoji, CJK) in front matter
- String slicing in frontmatter.rs uses char-boundary-safe operations
- All existing tests still pass
- The fix works on both Linux and Windows
