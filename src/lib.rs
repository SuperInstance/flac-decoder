//! FLAC metadata decoder (no audio decode)

use std::fmt;

/// FLAC metadata block type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockType {
    StreamInfo,
    Padding,
    Application,
    SeekTable,
    VorbisComment,
    Cuesheet,
    Picture,
    Unknown(u8),
}

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockType::StreamInfo => write!(f, "STREAMINFO"),
            BlockType::Padding => write!(f, "PADDING"),
            BlockType::Application => write!(f, "APPLICATION"),
            BlockType::SeekTable => write!(f, "SEEKTABLE"),
            BlockType::VorbisComment => write!(f, "VORBIS_COMMENT"),
            BlockType::Cuesheet => write!(f, "CUESHEET"),
            BlockType::Picture => write!(f, "PICTURE"),
            BlockType::Unknown(b) => write!(f, "UNKNOWN({})", b),
        }
    }
}

/// FLAC STREAMINFO block
#[derive(Debug)]
pub struct StreamInfo {
    pub min_block_size: u16,
    pub max_block_size: u16,
    pub min_frame_size: u32,
    pub max_frame_size: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub bits_per_sample: u8,
    pub total_samples: u64,
    pub md5: [u8; 16],
}

impl StreamInfo {
    pub fn duration_secs(&self) -> f64 {
        if self.sample_rate == 0 { return 0.0; }
        self.total_samples as f64 / self.sample_rate as f64
    }

    pub fn num_channels(&self) -> u8 {
        self.channels + 1
    }

    pub fn bps(&self) -> u8 {
        self.bits_per_sample + 1
    }
}

/// A FLAC metadata block
#[derive(Debug)]
pub struct MetadataBlock {
    pub block_type: BlockType,
    pub is_last: bool,
    pub data: Vec<u8>,
}

fn read_u16_be(data: &[u8]) -> u16 {
    u16::from_be_bytes([data[0], data[1]])
}

fn read_u32_be(data: &[u8]) -> u32 {
    u32::from_be_bytes([data[0], data[1], data[2], data[3]])
}

fn parse_block_type(raw: u8) -> BlockType {
    match raw {
        0 => BlockType::StreamInfo,
        1 => BlockType::Padding,
        2 => BlockType::Application,
        3 => BlockType::SeekTable,
        4 => BlockType::VorbisComment,
        5 => BlockType::Cuesheet,
        6 => BlockType::Picture,
        b => BlockType::Unknown(b),
    }
}

/// Parse STREAMINFO from its block data (34 bytes)
pub fn parse_stream_info(data: &[u8]) -> Result<StreamInfo, String> {
    if data.len() < 34 {
        return Err("STREAMINFO too short".into());
    }
    let min_block_size = read_u16_be(&data[0..2]);
    let max_block_size = read_u16_be(&data[2..4]);
    let min_frame_size = (data[4] as u32) << 16 | (data[5] as u32) << 8 | data[6] as u32;
    let max_frame_size = (data[7] as u32) << 16 | (data[8] as u32) << 8 | data[9] as u32;
    let sample_rate = ((data[10] as u32) << 12) | ((data[11] as u32) << 4) | ((data[12] >> 4) as u32);
    let channels = (data[12] >> 1) & 0x07;
    let bits_per_sample = ((data[12] & 0x01) << 4) | ((data[13] >> 4) as u8);
    let total_samples = ((data[13] as u64 & 0x0F) << 32)
        | (data[14] as u64) << 24 | (data[15] as u64) << 16
        | (data[16] as u64) << 8 | data[17] as u64;
    let mut md5 = [0u8; 16];
    md5.copy_from_slice(&data[18..34]);

    Ok(StreamInfo { min_block_size, max_block_size, min_frame_size, max_frame_size, sample_rate, channels, bits_per_sample, total_samples, md5 })
}

/// Parse all FLAC metadata blocks, returning them in order
pub fn parse_metadata(data: &[u8]) -> Result<(Vec<MetadataBlock>, usize), String> {
    if data.len() < 4 || &data[0..4] != b"fLaC" {
        return Err("Missing fLaC marker".into());
    }
    let mut blocks = Vec::new();
    let mut offset = 4;
    loop {
        if offset + 4 > data.len() {
            return Err("Truncated metadata block header".into());
        }
        let is_last = (data[offset] & 0x80) != 0;
        let block_type = parse_block_type(data[offset] & 0x7F);
        let block_len = read_u32_be(&data[offset+1..offset+5]) as usize;
        offset += 4;
        if offset + block_len > data.len() {
            return Err("Truncated metadata block data".into());
        }
        let block_data = data[offset..offset+block_len].to_vec();
        blocks.push(MetadataBlock { block_type, is_last, data: block_data });
        offset += block_len;
        if is_last { break; }
    }
    Ok((blocks, offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_data() {
        assert!(parse_metadata(&[]).is_err());
    }
}

/// FNV-1a 64 — the digest every substrate in the SuperInstance fleet agrees on.
pub const FNV_OFFSET: u64 = 0xcbf29ce484222325;
pub const FNV_PRIME: u64 = 0x100000001b3;

#[inline]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in bytes {
        h = (h ^ b as u64).wrapping_mul(FNV_PRIME);
    }
    h
}

/// True if this crate's FNV-1a still agrees with the rest of the fleet.
pub fn canary_holds() -> bool {
    fnv1a64("café Δ 日本語".as_bytes()) == 0x024a555471370b18d
}
