//! The Unisoc firmware package (.pac): header, the file directory
//! and CRC checks. The format follows the Unlicense unpac tool of
//! spreadtrum_flash (ilyakurdyukov); NOTICE has the attribution.
//! FDL binaries ship inside the package for the exact model, so the
//! flasher boots them from the same file it flashes.

use std::io;

pub const PAC_MAGIC: u32 = !0x0005_0005;
/// The directory header: version strings, counters and CRCs.
pub const PAC_HEADER_SIZE: usize = 2124;
/// One directory entry with the id/name strings and the offsets.
pub const PAC_ENTRY_SIZE: usize = 3080;

pub const FILE_TYPE_OPERATION: u32 = 0;
pub const FILE_TYPE_FILE: u32 = 1;
pub const FILE_TYPE_XML: u32 = 2;
pub const FILE_TYPE_FDL: u32 = 0x101;

#[derive(Debug, Clone)]
pub struct PacEntry {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub offset: u64,
    pub kind: u32,
    pub flash_use: u32,
    /// Load addresses (FDL1/FDL2 use the first).
    pub addr: [u32; 5],
}

impl PacEntry {
    /// The byte range in the package, empty entries skipped.
    pub fn data_range(&self) -> Option<std::ops::Range<usize>> {
        if self.offset == 0 || self.size == 0 {
            return None;
        }
        let start = self.offset as usize;
        let end = start.checked_add(self.size as usize)?;
        Some(start..end)
    }
}

#[derive(Debug, Clone)]
pub struct PacFirmware {
    pub fw_name: String,
    pub fw_version: String,
    pub head_crc_ok: bool,
    pub data_crc_ok: Option<bool>,
    pub entries: Vec<PacEntry>,
}

impl PacFirmware {
    /// The FDL pair to boot, in package order.
    pub fn fdl_pair(&self) -> Option<(&PacEntry, &PacEntry)> {
        let fdls: Vec<&PacEntry> = self
            .entries
            .iter()
            .filter(|e| e.kind == FILE_TYPE_FDL && e.addr[0] != 0)
            .collect();
        match fdls.as_slice() {
            [first, second, ..] => Some((first, second)),
            _ => None,
        }
    }

    /// The files flashed onto the device: real files marked for
    /// flashing (the XML scripts and operations excluded).
    pub fn flash_files(&self) -> Vec<&PacEntry> {
        self.entries
            .iter()
            .filter(|e| {
                e.kind == FILE_TYPE_FILE
                    && e.flash_use == 1
                    && !e.name.is_empty()
                    && e.data_range().is_some()
            })
            .collect()
    }
}

fn utf16_at(data: &[u8], off: usize, words: usize) -> String {
    let mut out = String::new();
    for i in 0..words {
        let pos = off + i * 2;
        if pos + 1 >= data.len() {
            break;
        }
        let word = u16::from_le_bytes([data[pos], data[pos + 1]]);
        if word == 0 {
            break;
        }
        // the package strings are ASCII-ish; keep the printable
        // subset, everything else reads as '?'
        let c = match char::from_u32(word as u32) {
            Some(c) if (' '..'~').contains(&c) => c,
            _ => '?',
        };
        out.push(c);
    }
    out
}

fn u32_at(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

fn u16_at(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}

/// The Modbus-style CRC16 (poly 0xA001) the package uses.
pub fn crc16(crc: u16, data: &[u8]) -> u16 {
    data.iter().fold(crc as u32, |mut crc, &b| {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xA001
            } else {
                crc >> 1
            };
        }
        crc & 0xFFFF
    }) as u16
}

/// Parses the header and the directory; the image data is not
/// copied. `data_crc_ok` stays None unless the whole image is in
/// memory (the check walks every file).
pub fn parse_pac(data: &[u8]) -> Result<PacFirmware, PacError> {
    if data.len() < PAC_HEADER_SIZE {
        return Err(PacError::Truncated("header", data.len()));
    }
    let magic = u32_at(data, 2116);
    if magic != PAC_MAGIC {
        return Err(PacError::BadMagic);
    }
    let head_crc = u16_at(data, 2120);
    let head_crc_ok = crc16(0, &data[..PAC_HEADER_SIZE - 4]) == head_crc;
    let file_count = u32_at(data, 1076) as usize;
    let dir_offset = u32_at(data, 1080) as usize;
    if file_count > 1024 {
        return Err(PacError::TooManyFiles(file_count));
    }
    if dir_offset != PAC_HEADER_SIZE {
        return Err(PacError::BadDirOffset(dir_offset));
    }

    let mut entries = Vec::with_capacity(file_count.min(1024));
    let mut data_crc = 0u16;
    let mut crc_walked = 0usize;
    for i in 0..file_count {
        let base = dir_offset + i * PAC_ENTRY_SIZE;
        let end = base
            .checked_add(PAC_ENTRY_SIZE)
            .ok_or(PacError::BadDirOffset(base))?;
        if end > data.len() {
            return Err(PacError::Truncated("directory entry", data.len()));
        }
        if u32_at(data, base) as usize != PAC_ENTRY_SIZE {
            return Err(PacError::BadEntrySize(base));
        }
        let size = (u32_at(data, base + 1528) as u64) << 32 | u32_at(data, base + 1536) as u64;
        let offset = (u32_at(data, base + 1532) as u64) << 32 | u32_at(data, base + 1548) as u64;
        let kind = u32_at(data, base + 1540);
        let flash_use = u32_at(data, base + 1544);
        let addr_num = (u32_at(data, base + 1556) as usize).min(5);
        let mut addr = [0u32; 5];
        for (j, a) in addr.iter_mut().take(addr_num).enumerate() {
            *a = u32_at(data, base + 1560 + j * 4);
        }
        let entry = PacEntry {
            id: utf16_at(data, base + 4, 256),
            name: utf16_at(data, base + 516, 256),
            size,
            offset,
            kind,
            flash_use,
            addr,
        };
        if let Some(range) = entry.data_range() {
            if range.end > data.len() {
                return Err(PacError::Truncated("file data", data.len()));
            }
            data_crc = crc16(data_crc, &data[range]);
            crc_walked += 1;
        }
        entries.push(entry);
    }

    let data_crc_ok = if crc_walked > 0 {
        Some(data_crc == u16_at(data, 2122))
    } else {
        None
    };

    Ok(PacFirmware {
        fw_name: utf16_at(data, 52, 256),
        fw_version: utf16_at(data, 564, 256),
        head_crc_ok,
        data_crc_ok,
        entries,
    })
}

/// Copies one entry's bytes out of the package.
pub fn extract_entry<'a>(data: &'a [u8], entry: &PacEntry) -> Result<&'a [u8], PacError> {
    let range = entry
        .data_range()
        .ok_or_else(|| PacError::EmptyEntry(entry.name.clone()))?;
    data.get(range)
        .ok_or(PacError::Truncated("file data", data.len()))
}

#[derive(Debug)]
pub enum PacError {
    BadMagic,
    /// (where, have)
    Truncated(&'static str, usize),
    TooManyFiles(usize),
    BadDirOffset(usize),
    BadEntrySize(usize),
    EmptyEntry(String),
}

impl std::fmt::Display for PacError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacError::BadMagic => write!(f, "not a Unisoc .pac package"),
            PacError::Truncated(what, have) => {
                write!(f, "package truncated in {what} ({have} bytes read)")
            }
            PacError::TooManyFiles(n) => write!(f, "unrealistic file count {n}"),
            PacError::BadDirOffset(o) => write!(f, "unexpected directory offset {o:#x}"),
            PacError::BadEntrySize(o) => write!(f, "bad entry size at {o:#x}"),
            PacError::EmptyEntry(name) => write!(f, "entry {name} has no data"),
        }
    }
}

impl std::error::Error for PacError {}

impl From<PacError> for crate::unisoc::BslError {
    fn from(e: PacError) -> Self {
        crate::unisoc::BslError::Io(io::Error::other(e.to_string()))
    }
}

/// Builds a small package for tests: header, two entries and the
/// payloads back to back.
#[cfg(test)]
pub fn build_test_pac(entries: &[(&str, u32, u32, &[u8])]) -> Vec<u8> {
    let mut dir_entries = Vec::new();
    let mut data_area = Vec::new();
    for (id, kind, addr0, blob) in entries {
        let mut e = vec![0u8; PAC_ENTRY_SIZE];
        e[0..4].copy_from_slice(&(PAC_ENTRY_SIZE as u32).to_le_bytes());
        for (i, c) in id.encode_utf16().enumerate() {
            e[4 + i * 2..6 + i * 2].copy_from_slice(&c.to_le_bytes());
        }
        for (i, c) in id.encode_utf16().enumerate() {
            e[516 + i * 2..518 + i * 2].copy_from_slice(&c.to_le_bytes());
        }
        e[1528..1532].copy_from_slice(&0u32.to_le_bytes());
        e[1532..1536].copy_from_slice(&0u32.to_le_bytes());
        e[1536..1540].copy_from_slice(&(blob.len() as u32).to_le_bytes());
        e[1540..1544].copy_from_slice(&kind.to_le_bytes());
        e[1544..1548].copy_from_slice(&1u32.to_le_bytes());
        let off = PAC_HEADER_SIZE + PAC_ENTRY_SIZE * entries.len() + data_area.len();
        e[1548..1552].copy_from_slice(&(off as u32).to_le_bytes());
        e[1556..1560].copy_from_slice(&1u32.to_le_bytes());
        e[1560..1564].copy_from_slice(&addr0.to_le_bytes());
        dir_entries.extend_from_slice(&e);
        data_area.extend_from_slice(blob);
    }
    let mut head = vec![0u8; PAC_HEADER_SIZE];
    let version: Vec<u16> = "PAC_something".encode_utf16().collect();
    for (i, c) in version.iter().enumerate() {
        head[i * 2..2 + i * 2].copy_from_slice(&c.to_le_bytes());
    }
    head[48..52].copy_from_slice(
        &((PAC_HEADER_SIZE + PAC_ENTRY_SIZE * entries.len() + data_area.len()) as u32)
            .to_le_bytes(),
    );
    let fw: Vec<u16> = "TEST_FW".encode_utf16().collect();
    for (i, c) in fw.iter().enumerate() {
        head[52 + i * 2..54 + i * 2].copy_from_slice(&c.to_le_bytes());
    }
    head[1076..1080].copy_from_slice(&(entries.len() as u32).to_le_bytes());
    head[1080..1084].copy_from_slice(&(PAC_HEADER_SIZE as u32).to_le_bytes());
    head[2116..2120].copy_from_slice(&PAC_MAGIC.to_le_bytes());
    let mut out = Vec::new();
    out.extend_from_slice(&head);
    out.extend_from_slice(&dir_entries);
    out.extend_from_slice(&data_area);
    // head crc over everything except the two crc fields
    let head_crc = crc16(0, &out[..PAC_HEADER_SIZE - 4]);
    out[2120..2122].copy_from_slice(&head_crc.to_le_bytes());
    let mut data_crc = 0u16;
    for blob in entries.iter().map(|e| e.3) {
        data_crc = crc16(data_crc, blob);
    }
    out[2122..2124].copy_from_slice(&data_crc.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_entries_roundtrip() {
        let fdl1 = vec![0x11u8; 600];
        let boot = vec![0x22u8; 900];
        let pac = build_test_pac(&[
            ("FDL1", FILE_TYPE_FDL, 0x4000_0000, &fdl1),
            ("FDL2", FILE_TYPE_FDL, 0x9F00_0000, &boot),
            ("l_boot", FILE_TYPE_FILE, 0, &boot),
        ]);
        let fw = parse_pac(&pac).unwrap();
        assert_eq!(fw.fw_name, "TEST_FW");
        assert!(fw.head_crc_ok);
        assert_eq!(fw.data_crc_ok, Some(true));
        assert_eq!(fw.entries.len(), 3);
        let (first, second) = fw.fdl_pair().unwrap();
        assert_eq!(first.name, "FDL1");
        assert_eq!(first.addr[0], 0x4000_0000);
        assert_eq!(second.name, "FDL2");
        let files = fw.flash_files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "l_boot");
        assert_eq!(extract_entry(&pac, files[0]).unwrap(), boot.as_slice());
        assert_eq!(extract_entry(&pac, first).unwrap(), fdl1.as_slice());
    }

    #[test]
    fn rejects_bad_magic_and_crc() {
        let mut pac = build_test_pac(&[("FDL1", FILE_TYPE_FDL, 0x4000_0000, &[0u8; 8])]);
        pac[2116..2120].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        assert!(matches!(parse_pac(&pac), Err(PacError::BadMagic)));
        let mut pac = build_test_pac(&[("FDL1", FILE_TYPE_FDL, 0x4000_0000, &[0u8; 8])]);
        pac[2120] ^= 0xFF;
        let fw = parse_pac(&pac).unwrap();
        assert!(!fw.head_crc_ok);
    }

    #[test]
    fn truncated_input_fails_honestly() {
        let pac = build_test_pac(&[("FDL1", FILE_TYPE_FDL, 0x4000_0000, &[7u8; 64])]);
        assert!(matches!(
            parse_pac(&pac[..100]),
            Err(PacError::Truncated("header", 100))
        ));
        let cut = PAC_HEADER_SIZE + PAC_ENTRY_SIZE - 10;
        assert!(matches!(
            parse_pac(&pac[..cut]),
            Err(PacError::Truncated("directory entry", _))
        ));
    }
}
