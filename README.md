# flac-decoder

**A FLAC metadata parser** that decodes the binary structure of Free Lossless Audio Codec files — STREAMINFO blocks, block type identification, sample rate / bit depth / channel extraction, and duration computation. Pure Rust with zero dependencies.

## Why It Matters

FLAC is the dominant lossless audio format: it reduces file size by 50–70% compared to raw PCM while preserving every sample exactly. Understanding FLAC's binary structure teaches:

1. **Bitstream parsing**: FLAC packs fields at sub-byte granularity (e.g., sample rate is 20 bits split across byte boundaries). This is a masterclass in reading binary formats with bit manipulation.
2. **Extensible metadata**: FLAC uses a typed block system (STREAMINFO, PADDING, APPLICATION, SEEKTABLE, VORBIS_COMMENT, CUESHEET, PICTURE) where each block has a type byte, a "last block" flag, and a length. This is the same pattern used by PNG, RIFF/WAV, and many binary formats.
3. **Streaming-friendly design**: The "is_last" flag allows parsers to know when metadata ends and audio frames begin, without scanning the entire file.

This crate focuses on metadata parsing (not audio decompression). Audio frame decoding requires LPC (Linear Predictive Coding) and Rice coding — significantly more complex, and well-served by existing libraries like `claxon`.

## How It Works

### FLAC File Structure

```
┌──────────────────────────────┐
│  "fLaC" marker (4 bytes)     │  Stream identifier
├──────────────────────────────┤
│  Metadata Block 1            │
│  ├─ is_last (1 bit)          │
│  ├─ block_type (7 bits)      │
│  ├─ length (24 bits)         │
│  └─ data (length bytes)      │
├──────────────────────────────┤
│  Metadata Block 2            │
│  ...                         │
├──────────────────────────────┤
│  Audio Frames                │  (not parsed by this crate)
│  ...                         │
└──────────────────────────────┘
```

### STREAMINFO Block (34 bytes)

The most important metadata block contains:

| Field | Bits | Range |
|-------|------|-------|
| min_block_size | 16 | 16–65535 samples |
| max_block_size | 16 | 16–65535 samples |
| min_frame_size | 24 | 0–16777215 bytes |
| max_frame_size | 24 | 0–16777215 bytes |
| sample_rate | 20 | 1–655350 Hz |
| channels − 1 | 3 | 0–7 (means 1–8 channels) |
| bps − 1 | 5 | 0–31 (means 1–32 bits/sample) |
| total_samples | 36 | 0–~68 billion |
| MD5 signature | 128 | Fingerprint of unencoded audio |

### Bit Manipulation

FLAC's sub-byte packing requires careful bit manipulation. For example, the sample_rate is split across bytes 10–12:

```
Byte 10: [SSSSSSSS]          ← upper 8 bits
Byte 11: [SSSSSSSS]          ← middle 8 bits
Byte 12: [SSSS....]          ← lower 4 bits (upper nibble)
         [....CCC?]          ← C = channels, ? = top bit of bps
```

The extraction:
```rust
let sample_rate = ((data[10] as u32) << 12)
                | ((data[11] as u32) << 4)
                | ((data[12] >> 4) as u32);
let channels = (data[12] >> 1) & 0x07;
let bps = ((data[12] & 0x01) << 4) | (data[13] >> 4);
```

This pattern — shifting, masking, and ORing bytes — is universal in binary format parsing.

### Duration Computation

$$\text{duration} = \frac{\text{total\_samples}}{\text{sample\_rate}}$$

For a 3-minute song at 44.1 kHz: total_samples ≈ 7,938,000, duration ≈ 180.05 seconds.

### Complexity Analysis

| Operation | Time | Space |
|-----------|------|-------|
| `parse_metadata` | O(n) | O(n) for block data |
| `parse_stream_info` | O(1) | O(1) |
| Duration computation | O(1) | O(1) |

Where n = total metadata size.

### Error Handling

The parser returns `Result<_, String>` with descriptive errors:
- "Missing fLaC marker" — not a FLAC file
- "Truncated metadata block header" — file ends mid-header
- "Truncated metadata block data" — declared length exceeds available data
- "STREAMINFO too short" — STREAMINFO block < 34 bytes

## Quick Start

```rust
use flac_decoder::{parse_metadata, parse_stream_info};

// Parse FLAC metadata from a byte buffer
let flac_data = std::fs::read("song.flac").unwrap();
let (blocks, audio_offset) = parse_metadata(&flac_data).unwrap();

// Extract STREAMINFO
for block in &blocks {
    if let BlockType::StreamInfo = block.block_type {
        let info = parse_stream_info(&block.data).unwrap();
        println!("Sample rate: {} Hz", info.sample_rate);
        println!("Channels: {}", info.num_channels());
        println!("Bits per sample: {}", info.bps());
        println!("Duration: {:.1}s", info.duration_secs());
    }
}
```

## API

### Metadata Parsing
- `parse_metadata(data: &[u8]) -> Result<(Vec<MetadataBlock>, usize), String>` — Parse all metadata blocks, return blocks and offset to audio data
- `parse_stream_info(data: &[u8]) -> Result<StreamInfo, String>` — Parse a 34-byte STREAMINFO block

### Types
- `BlockType` — Enum: `StreamInfo`, `Padding`, `Application`, `SeekTable`, `VorbisComment`, `Cuesheet`, `Picture`, `Unknown(u8)`
- `StreamInfo` — Sample rate, channels, bits per sample, total samples, MD5, block/frame sizes
- `MetadataBlock` — block_type, is_last flag, raw data

### `StreamInfo` Methods
- `duration_secs(&self) -> f64` — total_samples / sample_rate
- `num_channels(&self) -> u8` — channels + 1 (FLAC stores 0-based)
- `bps(&self) -> u8` — bits_per_sample + 1 (FLAC stores 0-based)

## Architecture Notes

The FLAC metadata parser connects to the γ + η = C conservation framework:

- **γ** (gamma) = the parsed metadata (structured information: sample rate, channels, duration)
- **η** (eta) = the unparsed audio frames (raw payload)
- **C** (constant) = the complete FLAC file

The parser extracts γ and marks where η begins (the returned `audio_offset`). This clean separation between metadata and payload is the same pattern used in container formats (MP4, WebM) and network protocols (HTTP headers vs. body).

See the full architecture: [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md)

## References

1. Coalson, J. (2008). "FLAC Format Specification." [xiph.org/flac/format.html](https://xiph.org/flac/format.html) — The authoritative specification.
2. Xiph.Org Foundation. "Ogg FLAC mapping." [xiph.org/flac/ogg_mapping.html](https://xiph.org/flac/ogg_mapping.html)
3. Salomon, D. (2007). *Data Compression: The Complete Reference,* 4th ed. Springer. Chapter 7 (audio compression).
4. Robinson, T. (1994). "SHORTEN: Simple Lossless and Near-Lossless Audio Compression." *Cambridge Conf.* — Precursor to FLAC's predictive coding.
5. claxon — [github.com/ruuda/claxon](https://github.com/ruuda/claxon) — Production FLAC decoder in Rust.

## License

MIT
