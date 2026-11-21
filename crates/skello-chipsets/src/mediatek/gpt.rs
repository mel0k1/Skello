//! Minimal GPT reader: enough to map a partition name to its byte
//! offset on the eMMC user area, for the DA read/write commands.

#[derive(Debug, Clone)]
pub struct GptPartition {
    pub name: String,
    pub first_lba: u64,
    pub last_lba: u64,
}

#[derive(Debug)]
pub enum GptError {
    NotGpt,
    TooShort,
}

impl std::fmt::Display for GptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GptError::NotGpt => write!(f, "no EFI PART signature"),
            GptError::TooShort => write!(f, "gpt truncated"),
        }
    }
}

impl std::error::Error for GptError {}

fn u16le(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}

fn u32le(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

fn u64le(data: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(data[off..off + 8].try_into().unwrap())
}

/// Parses the GPT from a raw disk image: `disk` starts at LBA 0 with
/// the protective MBR, `lba_size` is usually 512.
pub fn parse_gpt(disk: &[u8], lba_size: usize) -> Result<Vec<GptPartition>, GptError> {
    let header = lba_size;
    if disk.len() < header + 92 {
        return Err(GptError::TooShort);
    }
    if &disk[header..header + 8] != b"EFI PART" {
        return Err(GptError::NotGpt);
    }
    let first_entry = u64le(disk, header + 72) as usize;
    let entry_count = u32le(disk, header + 80) as usize;
    let entry_size = u32le(disk, header + 84) as usize;
    let mut out = Vec::new();
    for i in 0..entry_count {
        let base = first_entry * lba_size + i * entry_size;
        if base + entry_size > disk.len() {
            return Err(GptError::TooShort);
        }
        // type GUID all-zero marks an unused entry
        if disk[base..base + 16].iter().all(|b| *b == 0) {
            continue;
        }
        let first_lba = u64le(disk, base + 32);
        let last_lba = u64le(disk, base + 40);
        let name_raw = &disk[base + 56..base + 128];
        let mut name = String::new();
        for c in name_raw.chunks(2) {
            let unit = u16le(c, 0);
            if unit == 0 {
                break;
            }
            name.push(char::from_u32(unit as u32).unwrap_or('\u{FFFD}'));
        }
        out.push(GptPartition {
            name,
            first_lba,
            last_lba,
        });
    }
    Ok(out)
}

/// Looks up a partition by name and returns its byte range.
pub fn find_partition(parts: &[GptPartition], name: &str, lba_size: usize) -> Option<(u64, u64)> {
    let lba = lba_size as u64;
    parts
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .map(|p| (p.first_lba * lba, (p.last_lba - p.first_lba + 1) * lba))
}

/// Builds a GPT disk image for tests: protective MBR, header at LBA 1
/// and entries laid out at LBA 2, all sectors 512 bytes.
#[cfg(test)]
pub(crate) fn build_gpt_image(entries: &[(&str, u64, u64)]) -> Vec<u8> {
    let lba = 512usize;
    let mut disk = vec![0u8; lba * 34];
    disk[lba..lba + 8].copy_from_slice(b"EFI PART");
    // entry array at LBA 2, count and size
    disk[lba + 72..lba + 80].copy_from_slice(&2u64.to_le_bytes());
    disk[lba + 80..lba + 84].copy_from_slice(&(entries.len() as u32).to_le_bytes());
    disk[lba + 84..lba + 88].copy_from_slice(&128u32.to_le_bytes());
    for (i, (name, first, last)) in entries.iter().enumerate() {
        let base = 2 * lba + i * 128;
        disk[base..base + 16].fill(0x11); // non-zero type GUID
        disk[base + 32..base + 40].copy_from_slice(&first.to_le_bytes());
        disk[base + 40..base + 48].copy_from_slice(&last.to_le_bytes());
        for (j, c) in name.encode_utf16().enumerate() {
            disk[base + 56 + j * 2..base + 58 + j * 2].copy_from_slice(&c.to_le_bytes());
        }
    }
    disk
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_gpt(entries: &[(&str, u64, u64)]) -> Vec<u8> {
        let lba = 512usize;
        let entry_array_lba = 2u64;
        let mut disk = vec![0u8; lba * (3 + entries.len())];
        disk[lba..lba + 8].copy_from_slice(b"EFI PART");
        disk[lba + 72..lba + 80].copy_from_slice(&entry_array_lba.to_le_bytes());
        disk[lba + 80..lba + 84].copy_from_slice(&(entries.len() as u32).to_le_bytes());
        disk[lba + 84..lba + 88].copy_from_slice(&128u32.to_le_bytes());
        for (i, (name, first, last)) in entries.iter().enumerate() {
            let base = lba * 2 + i * 128;
            disk[base] = 1;
            disk[base + 32..base + 40].copy_from_slice(&first.to_le_bytes());
            disk[base + 40..base + 48].copy_from_slice(&last.to_le_bytes());
            for (c, unit) in name.chars().enumerate() {
                disk[base + 56 + c * 2..base + 56 + c * 2 + 2]
                    .copy_from_slice(&(unit as u16).to_le_bytes());
            }
        }
        disk
    }

    #[test]
    fn parses_and_finds_partitions() {
        let disk = build_gpt(&[("seccfg", 1024, 1055), ("boot", 2048, 4095)]);
        let parts = parse_gpt(&disk, 512).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].name, "seccfg");
        let (off, size) = find_partition(&parts, "SECCFG", 512).unwrap();
        assert_eq!(off, 1024 * 512);
        assert_eq!(size, 32 * 512);
        assert!(find_partition(&parts, "missing", 512).is_none());
    }

    #[test]
    fn rejects_non_gpt() {
        let mut junk = vec![0u8; 604];
        junk[..4].copy_from_slice(b"JUNK");
        assert!(matches!(parse_gpt(&junk, 512), Err(GptError::NotGpt)));
    }
}
