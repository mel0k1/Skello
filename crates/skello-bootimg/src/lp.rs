//! Android dynamic-partitions metadata (liblp) reader for super
//! images. Parses the geometry block, then metadata slot 0: groups,
//! partitions and their extents. Layout per
//! fs_mgr/liblp/include/liblp/metadata_format.h: geometry at offset 0
//! and its backup at 4096, one metadata copy per slot starting at
//! 8192, each copy `metadata_max_size` bytes.

/// `LP_METADATA_GEOMETRY_MAGIC`.
const GEOMETRY_MAGIC: u32 = 0x616C_4467;
/// `LP_METADATA_HEADER_MAGIC`.
const HEADER_MAGIC: u32 = 0x414C_5030;
/// `LP_SECTOR_SIZE`: extents count in 512-byte sectors.
pub const SECTOR: u64 = 512;
/// Reserved prefix + primary geometry + backup geometry.
const METADATA_BASE: usize = 4096 + 2 * 4096;

const GEOMETRY_STRUCT_SIZE: u32 = 52;
const PARTITION_ENTRY_SIZE: u32 = 52;
const EXTENT_ENTRY_SIZE: u32 = 24;
const GROUP_ENTRY_SIZE: u32 = 48;

const ATTR_READONLY: u32 = 1 << 0;

/// One partition group with its size cap (0 = unlimited).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LpGroup {
    pub name: String,
    pub maximum_size: u64,
}

/// One logical partition, sized by the sum of its extents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LpPartition {
    pub name: String,
    pub size_bytes: u64,
    pub group: String,
    pub readonly: bool,
    pub extents: usize,
}

/// The parsed metadata slot 0 of a super image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LpMetadata {
    pub metadata_max_size: u32,
    pub slot_count: u32,
    pub logical_block_size: u32,
    pub groups: Vec<LpGroup>,
    pub partitions: Vec<LpPartition>,
}

/// One table descriptor: offset is relative to the end of the header.
struct TableDescriptor {
    offset: usize,
    num_entries: usize,
    entry_size: usize,
}

fn descriptor(buf: &[u8], at: usize) -> Result<TableDescriptor, String> {
    let d = buf
        .get(at..at + 12)
        .ok_or("header truncated (table descriptor)")?;
    Ok(TableDescriptor {
        offset: u32::from_le_bytes(d[0..4].try_into().unwrap()) as usize,
        num_entries: u32::from_le_bytes(d[4..8].try_into().unwrap()) as usize,
        entry_size: u32::from_le_bytes(d[8..12].try_into().unwrap()) as usize,
    })
}

fn le32(buf: &[u8], at: usize) -> Result<u32, String> {
    let b = buf.get(at..at + 4).ok_or("buffer truncated (u32)")?;
    Ok(u32::from_le_bytes(b.try_into().unwrap()))
}

fn le64(buf: &[u8], at: usize) -> Result<u64, String> {
    let b = buf.get(at..at + 8).ok_or("buffer truncated (u64)")?;
    Ok(u64::from_le_bytes(b.try_into().unwrap()))
}

/// C-string from a fixed field, lossy UTF-8.
fn fixed_name(buf: &[u8], at: usize, len: usize) -> String {
    let end = buf[at..at + len]
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(len);
    String::from_utf8_lossy(&buf[at..at + end]).into_owned()
}

/// Parses metadata slot 0 from a raw super image buffer. Sparse
/// images must be expanded by the caller first.
pub fn parse_metadata(buf: &[u8]) -> Result<LpMetadata, String> {
    // geometry: magic, struct_size, checksum[32], then the fields
    let max_size = le32(buf, 40)?;
    let slot_count = le32(buf, 44)?.max(1);
    let logical_block_size = le32(buf, 48)?;
    if le32(buf, 0)? != GEOMETRY_MAGIC || le32(buf, 4)? != GEOMETRY_STRUCT_SIZE {
        return Err("not a super image (geometry magic)".to_owned());
    }

    let base = METADATA_BASE;
    let header = buf
        .get(base..base + 132)
        .ok_or("image too small for lp metadata")?;
    if le32(header, 0)? != HEADER_MAGIC {
        return Err("lp metadata header magic mismatch".to_owned());
    }
    let major = u16::from_le_bytes(header[4..6].try_into().unwrap());
    if major != 10 {
        return Err(format!("unsupported lp metadata major version {major}"));
    }
    let header_size = le32(header, 8)? as usize;
    let tables_base = base + header_size;

    let partitions_desc = descriptor(header, 80)?;
    let extents_desc = descriptor(header, 92)?;
    let groups_desc = descriptor(header, 104)?;
    if partitions_desc.entry_size != PARTITION_ENTRY_SIZE as usize
        || extents_desc.entry_size != EXTENT_ENTRY_SIZE as usize
        || groups_desc.entry_size != GROUP_ENTRY_SIZE as usize
    {
        return Err("unexpected lp table entry sizes".to_owned());
    }

    let mut groups = Vec::new();
    for i in 0..groups_desc.num_entries {
        let at = tables_base + groups_desc.offset + i * groups_desc.entry_size;
        groups.push(LpGroup {
            name: fixed_name(buf, at, 36),
            maximum_size: le64(buf, at + 40)?,
        });
    }

    // partition entry: name[36], attributes, first_extent_index,
    // num_extents, group_index
    let mut partitions = Vec::new();
    for i in 0..partitions_desc.num_entries {
        let at = tables_base + partitions_desc.offset + i * partitions_desc.entry_size;
        let attrs = le32(buf, at + 36)?;
        let first_extent = le32(buf, at + 40)? as usize;
        let num_extents = le32(buf, at + 44)? as usize;
        let group_index = le32(buf, at + 48)? as usize;
        let mut size_bytes = 0u64;
        for e in 0..num_extents {
            let ext =
                tables_base + extents_desc.offset + (first_extent + e) * extents_desc.entry_size;
            size_bytes = size_bytes.saturating_add(le64(buf, ext)?.saturating_mul(SECTOR));
        }
        partitions.push(LpPartition {
            name: fixed_name(buf, at, 36),
            size_bytes,
            group: groups
                .get(group_index)
                .map(|g| g.name.clone())
                .unwrap_or_default(),
            readonly: attrs & ATTR_READONLY != 0,
            extents: num_extents,
        });
    }

    Ok(LpMetadata {
        metadata_max_size: max_size,
        slot_count,
        logical_block_size,
        groups,
        partitions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal super image buffer: geometry, one group
    /// ("main", 4 GiB cap), one partition ("system", 1 MiB via a
    /// single linear extent).
    fn sample_image() -> Vec<u8> {
        let mut buf = vec![0u8; 64 * 1024];
        // geometry at 0
        buf[0..4].copy_from_slice(&GEOMETRY_MAGIC.to_le_bytes());
        buf[4..8].copy_from_slice(&GEOMETRY_STRUCT_SIZE.to_le_bytes());
        buf[40..44].copy_from_slice(&4096u32.to_le_bytes()); // max_size
        buf[44..48].copy_from_slice(&2u32.to_le_bytes()); // slots
        buf[48..52].copy_from_slice(&4096u32.to_le_bytes()); // block size
                                                             // backup geometry at 4096
        buf[4096..4100].copy_from_slice(&GEOMETRY_MAGIC.to_le_bytes());

        let header_size = 128usize;
        // tables: partitions(52) | extents(24) | groups(48)
        let (p_off, e_off, g_off) = (0usize, 52usize, 76usize);
        let mut t = Vec::new();
        t.extend_from_slice(b"system"); // name[36]
        t.resize(p_off + 52, 0);
        t[36..40].copy_from_slice(&0u32.to_le_bytes()); // attrs
        t[40..44].copy_from_slice(&0u32.to_le_bytes()); // first extent
        t[44..48].copy_from_slice(&1u32.to_le_bytes()); // num extents
        t[48..52].copy_from_slice(&0u32.to_le_bytes()); // group index
        t.resize(e_off + 24, 0);
        t[e_off..e_off + 8].copy_from_slice(&2048u64.to_le_bytes()); // 1 MiB
        t[e_off + 8..e_off + 12].copy_from_slice(&0u32.to_le_bytes()); // linear
        t[e_off + 12..e_off + 20].copy_from_slice(&0u64.to_le_bytes()); // target
        t.resize(g_off + 48, 0);
        t[g_off..g_off + 48].fill(0);
        t[g_off..g_off + 4].copy_from_slice(b"main");
        t[g_off + 40..g_off + 48].copy_from_slice(&4u64.to_le_bytes()); // cap MiB

        let base = METADATA_BASE;
        buf[base..base + 4].copy_from_slice(&HEADER_MAGIC.to_le_bytes());
        buf[base + 4..base + 6].copy_from_slice(&10u16.to_le_bytes());
        buf[base + 6..base + 8].copy_from_slice(&1u16.to_le_bytes());
        buf[base + 8..base + 12].copy_from_slice(&(header_size as u32).to_le_bytes());
        // partitions descriptor
        buf[base + 80..base + 84].copy_from_slice(&(p_off as u32).to_le_bytes());
        buf[base + 84..base + 88].copy_from_slice(&1u32.to_le_bytes());
        buf[base + 88..base + 92].copy_from_slice(&PARTITION_ENTRY_SIZE.to_le_bytes());
        // extents descriptor
        buf[base + 92..base + 96].copy_from_slice(&(e_off as u32).to_le_bytes());
        buf[base + 96..base + 100].copy_from_slice(&1u32.to_le_bytes());
        buf[base + 100..base + 104].copy_from_slice(&EXTENT_ENTRY_SIZE.to_le_bytes());
        // groups descriptor
        buf[base + 104..base + 108].copy_from_slice(&(g_off as u32).to_le_bytes());
        buf[base + 108..base + 112].copy_from_slice(&1u32.to_le_bytes());
        buf[base + 112..base + 116].copy_from_slice(&GROUP_ENTRY_SIZE.to_le_bytes());
        let at = base + header_size;
        buf[at..at + t.len()].copy_from_slice(&t);
        buf
    }

    #[test]
    fn parses_geometry_groups_and_partitions() {
        let meta = parse_metadata(&sample_image()).unwrap();
        assert_eq!(meta.slot_count, 2);
        assert_eq!(meta.metadata_max_size, 4096);
        assert_eq!(meta.logical_block_size, 4096);
        assert_eq!(meta.groups.len(), 1);
        assert_eq!(meta.groups[0].name, "main");
        assert_eq!(meta.groups[0].maximum_size, 4);
        assert_eq!(meta.partitions.len(), 1);
        let p = &meta.partitions[0];
        assert_eq!(p.name, "system");
        assert_eq!(p.size_bytes, 2048 * SECTOR);
        assert_eq!(p.group, "main");
        assert!(!p.readonly);
        assert_eq!(p.extents, 1);
    }

    #[test]
    fn rejects_non_super_buffers() {
        assert!(parse_metadata(&[0u8; 64]).is_err());
        let mut buf = sample_image();
        buf[0] = 0;
        assert!(parse_metadata(&buf).is_err());
        let mut buf = sample_image();
        buf[METADATA_BASE] = 0;
        assert!(parse_metadata(&buf).is_err());
    }
}
