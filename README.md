# FLAC Metadata Decoder

**A zero-dependency Rust library for parsing FLAC stream metadata**, including the STREAMINFO block, Vorbis comments, seek tables, and picture blocks — all without decoding a single audio sample.

## Why It Matters

FLAC (Free Lossless Audio Codec) is the most widely used lossless audio format in open-source music players, archival systems, and streaming platforms. Before any audio can be decoded, applications must parse the metadata blocks to learn the sample rate, channel count, bit depth, and total duration. This crate provides that parsing layer with zero external dependencies, making it ideal for embedded systems, WebAssembly targets, and lightweight audio indexers that need to scan thousands of files quickly without pulling in a full codec.

## How It Works

A FLAC file begins with the 4-byte magic marker `fLaC`, followed by a chain of metadata blocks. Each block starts with a 1-byte header containing a "last block" flag (bit 7) and a 7-bit block type, followed by a 3-byte big-endian length. The parser walks this chain using `parse_metadata`, returning a vector of `MetadataBlock` structs and the byte offset where audio frames begin.

The `STREAMINFO` block (type 0) is the most information-dense: 34 bytes packing minimum/maximum block and frame sizes, a 20-bit sample rate, 3-bit channel count (stored as `channels - 1`), 5-bit bit depth (stored as `bps - 1`), a 36-bit total sample count, and a 128-bit MD5 checksum of the raw audio. The parser extracts these via careful bit manipulation — for example, the sample rate spans bytes 10–12 using 20 bits split across byte boundaries: bits 19–8 from bytes 10–11 and bits 7–4 from the upper nibble of byte 12.

The library supports all standard block types: `STREAMINFO`, `PADDING`, `APPLICATION`, `SEEKTABLE`, `VORBIS_COMMENT` (the de facto tag format), `CUESHEET`, and `PICTURE`. Unknown block types are preserved as `Unknown(u8)` so forward-compatible parsing is guaranteed.

## Quick Start

```rust
use flac_decoder::{parse_metadata, parse_stream_info};

fn main() {
    let flac_bytes = std::fs::read("song.flac").unwrap();
    let (blocks, audio_offset) = parse_metadata(&flac_bytes).unwrap();

    for block in &blocks {
        println!("{} ({} bytes, last={})",
            block.block_type, block.data.len(), block.is_last);
    }

    // Parse STREAMINFO from the first block
    if let Some(info_block) = blocks.iter().find(|b| b.block_type == flac_decoder::BlockType::StreamInfo) {
        let info = parse_stream_info(&info_block.data).unwrap();
        println!("{} Hz, {} channels, {}-bit, {:.1} seconds",
            info.sample_rate,
            info.num_channels(),
            info.bps(),
            info.duration_secs());
    }

    println!("Audio data starts at byte {}", audio_offset);
}
```

## API

| Type / Function | Description |
|---|---|
| `parse_metadata(data)` | Parse all metadata blocks; returns `(Vec<MetadataBlock>, audio_offset)` |
| `parse_stream_info(data)` | Decode a 34-byte STREAMINFO block into a `StreamInfo` struct |
| `BlockType` | Enum of FLAC block types with `Display` formatting |
| `StreamInfo` | Parsed stream info with `duration_secs()`, `num_channels()`, `bps()` helpers |
| `MetadataBlock` | A single metadata block with type, is_last flag, and raw data |

## Architecture Notes

Part of the SuperInstance media processing toolkit. This metadata parser feeds into the broader audio pipeline, providing the stream parameters needed by downstream decoders. See the [Architecture Guide](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md).

## License

MIT
