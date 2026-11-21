//! MediaTek DA loader file parser: a stock DA binary carries several
//! region tables, one per hardware code. Format after mtkclient
//! daconfig.py (Apache-2.0).

use std::fs;

#[derive(Debug, Clone)]
pub struct EntryRegion {
    pub buf: u32,
    pub len: u32,
    pub start_addr: u32,
    pub start_offset: u32,
    pub sig_len: u32,
}

#[derive(Debug, Clone)]
pub struct DaLoader {
    pub hw_code: u16,
    pub hw_sub_code: u16,
    pub hw_version: u16,
    pub sw_version: u16,
    pub v6: bool,
    pub regions: Vec<EntryRegion>,
}

impl DaLoader {
    /// Regions [1] and [2] are stage 1 and stage 2 of the DA.
    pub fn stage1(&self) -> Option<&EntryRegion> {
        self.regions.get(1)
    }

    pub fn stage2(&self) -> Option<&EntryRegion> {
        self.regions.get(2)
    }
}

#[derive(Debug)]
pub enum DaLoaderError {
    Io(std::io::Error),
    TooShort,
    NoEntry(u16),
}

impl std::fmt::Display for DaLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaLoaderError::Io(e) => write!(f, "da loader read: {e}"),
            DaLoaderError::TooShort => write!(f, "da loader file truncated"),
            DaLoaderError::NoEntry(code) => {
                write!(f, "da loader has no entry for hardware code {code:#06x}")
            }
        }
    }
}

impl std::error::Error for DaLoaderError {}

fn read16(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}

fn read32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

/// Parses a DA loader file and picks the entry matching `hw_code`.
pub fn pick_loader(path: &str, hw_code: u16) -> Result<(DaLoader, Vec<u8>), DaLoaderError> {
    let data = fs::read(path).map_err(DaLoaderError::Io)?;
    if data.len() < 0x6C {
        return Err(DaLoaderError::TooShort);
    }
    let v6 = data.starts_with(b"MTK_DA_v6") || data.windows(9).any(|w| w == b"MTK_DA_v6");
    let count_da = read32(&data, 0x68) as usize;
    // Old loaders use a shorter DA struct (verified by the DADA marker).
    let old_stride =
        data.len() >= 0x6C + 0xD8 + 2 && data[0x6C + 0xD8..0x6C + 0xD8 + 2] == [0xDA, 0xDA];
    let stride = if old_stride { 0xD8 } else { 0xDC };
    for i in 0..count_da {
        let base = 0x6C + i * stride;
        if base + stride > data.len() {
            return Err(DaLoaderError::TooShort);
        }
        let magic = read16(&data, base);
        let code = read16(&data, base + 2);
        if magic != 0x0169 || code != hw_code {
            continue;
        }
        let hw_sub_code = read16(&data, base + 4);
        let hw_version = read16(&data, base + 6);
        let sw_version = read16(&data, base + 8);
        let entry_count = read16(&data, base + 0x12) as usize;
        let mut regions = Vec::new();
        let region_base = base + 0x14;
        if region_base + entry_count * 20 > base + stride {
            return Err(DaLoaderError::TooShort);
        }
        for r in 0..entry_count {
            let ro = region_base + r * 20;
            regions.push(EntryRegion {
                buf: read32(&data, ro),
                len: read32(&data, ro + 4),
                start_addr: read32(&data, ro + 8),
                start_offset: read32(&data, ro + 12),
                sig_len: read32(&data, ro + 16),
            });
        }
        return Ok((
            DaLoader {
                hw_code,
                hw_sub_code,
                hw_version,
                sw_version,
                v6,
                regions,
            },
            data,
        ));
    }
    Err(DaLoaderError::NoEntry(hw_code))
}

/// Extracts the stage image bytes from the loader file for one region.
pub fn stage_image(data: &[u8], region: &EntryRegion) -> Result<Vec<u8>, DaLoaderError> {
    let start = region.buf as usize;
    let end = start + region.len as usize;
    if end > data.len() {
        return Err(DaLoaderError::TooShort);
    }
    Ok(data[start..end].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal loader file: one DA entry with three regions
    /// (boot, stage1, stage2) in the new 0xDC stride layout.
    fn build_loader(hw_code: u16) -> Vec<u8> {
        let mut data = vec![0u8; 0xE00];
        data[..12].copy_from_slice(b"MTK_AllInOne");
        data[0x68..0x6C].copy_from_slice(&1u32.to_le_bytes());
        data[0x6C] = 0x69;
        data[0x6D] = 0x01;
        data[0x6E..0x70].copy_from_slice(&hw_code.to_le_bytes());
        data[0x6C + 0x12..0x6C + 0x14].copy_from_slice(&3u16.to_le_bytes());
        let region_base = 0x6C + 0x14;
        for (i, (buf, len, addr)) in [
            (0x200u32, 0x100u32, 0x100u32),
            (0x300, 0x400, 0x200000),
            (0x700, 0x800, 0x2C00000),
        ]
        .iter()
        .enumerate()
        {
            let ro = region_base + i * 20;
            data[ro..ro + 4].copy_from_slice(&buf.to_le_bytes());
            data[ro + 4..ro + 8].copy_from_slice(&len.to_le_bytes());
            data[ro + 8..ro + 12].copy_from_slice(&addr.to_le_bytes());
        }
        // stage payloads live after the header at their buf offsets
        for i in 0x148..0xE00u32 {
            let off = i as usize;
            data[off] = (i * 3) as u8;
        }
        data
    }

    #[test]
    fn picks_entry_and_stages() {
        let data = build_loader(0x6572);
        let path = std::env::temp_dir().join("skello_test_loader.bin");
        std::fs::write(&path, &data).unwrap();
        let (loader, bytes) = pick_loader(path.to_str().unwrap(), 0x6572).unwrap();
        assert_eq!(loader.hw_code, 0x6572);
        assert_eq!(loader.regions.len(), 3);
        let s1 = loader.stage1().unwrap();
        assert_eq!(s1.start_addr, 0x200000);
        let img = stage_image(&bytes, s1).unwrap();
        assert_eq!(img.len(), 0x400);
        assert_eq!(img[0], 0);
        assert_eq!(img[1], 3);
        assert!(matches!(
            pick_loader(path.to_str().unwrap(), 0x1234),
            Err(DaLoaderError::NoEntry(0x1234))
        ));
        let _ = std::fs::remove_file(&path);
    }
}
