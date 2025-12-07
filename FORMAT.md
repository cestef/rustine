# Rustine Patch Format v2

## Overview
Extensible binary format for storing binary patches with optional checksums, reverse patches, and metadata.

## Binary Layout

```
┌─────────────────────────────────────────────┐
│ Header (13 bytes)                           │
├─────────────────────────────────────────────┤
│ Magic:   "RUSTINE2" (8 bytes)              │
│ Version: u8         (1 byte)  = 2          │
│ Flags:   u32 LE     (4 bytes)              │
└─────────────────────────────────────────────┘
┌─────────────────────────────────────────────┐
│ Optional Fields (based on flags)            │
├─────────────────────────────────────────────┤
│ [if FLAG_BASE_CHECKSUM]                    │
│   base_checksum: [u8; 32]                  │
│                                             │
│ [if FLAG_OUTPUT_CHECKSUM]                  │
│   output_checksum: [u8; 32]                │
│                                             │
│ [if FLAG_METADATA]                         │
│   metadata_len: u32 LE                     │
│   metadata: [u8; metadata_len] (JSON)      │
└─────────────────────────────────────────────┘
┌─────────────────────────────────────────────┐
│ Forward Patch (required)                    │
├─────────────────────────────────────────────┤
│ forward_len: u64 LE (8 bytes)              │
│ forward_data: [u8; forward_len]            │
└─────────────────────────────────────────────┘
┌─────────────────────────────────────────────┐
│ Reverse Patch (optional)                    │
├─────────────────────────────────────────────┤
│ [if FLAG_REVERSE_PATCH]                    │
│   reverse_len: u64 LE                      │
│   reverse_data: [u8; reverse_len]          │
└─────────────────────────────────────────────┘
```

## Flags

```rust
const FLAG_BASE_CHECKSUM:   u32 = 1 << 0;  // 0x00000001
const FLAG_OUTPUT_CHECKSUM: u32 = 1 << 1;  // 0x00000002
const FLAG_REVERSE_PATCH:   u32 = 1 << 2;  // 0x00000004
const FLAG_METADATA:        u32 = 1 << 3;  // 0x00000008
// bits 4-31 reserved for future use
```

## Features

- **Checksums**: Optional SHA256 hashes for base and output files
- **Reverse patches**: Embedded reverse patch for `--reverse` flag
- **Metadata**: Optional JSON metadata (timestamps, filenames, etc.)
- **Backward compat**: Old format (RUSTINE1) still readable
- **Extensible**: Flag-based feature detection

## Example Usage

### Minimal patch (no extras)
```
Magic:       RUSTINE2
Version:     2
Flags:       0x00000000
Forward len: 1234
Forward:     [bsdiff data...]
```

### Full-featured patch
```
Magic:           RUSTINE2
Version:         2
Flags:           0x00000007  (checksums + reverse)
Base checksum:   [32 bytes SHA256]
Output checksum: [32 bytes SHA256]
Forward len:     1234
Forward:         [bsdiff data...]
Reverse len:     1234
Reverse:         [bsdiff data...]
```
