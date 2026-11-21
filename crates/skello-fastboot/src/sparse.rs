use std::fmt;

pub const SPARSE_MAGIC: u32 = 0xED26FF3A;
pub const SPARSE_MAJOR: u16 = 1;
pub const CHUNK_RAW: u16 = 0xCAC1;
pub const CHUNK_FILL: u16 = 0xCAC2;
pub const CHUNK_DONT_CARE: u16 = 0xCAC3;
pub const CHUNK_CRC32: u16 = 0xCAC4;

const FILE_HEADER_SZ: u32 = 28;
const CHUNK_HEADER_SZ: u32 = 12;

#[derive(Debug)]
pub enum SparseError {
    NotSparse,
    Truncated(&'static str),
    BadHeader(&'static str),
    ChunkTooBig(usize),
}

impl fmt::Display for SparseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SparseError::NotSparse => write!(f, "image is not an android sparse image"),
            SparseError::Truncated(s) => write!(f, "sparse image truncated at {s}"),
            SparseError::BadHeader(s) => write!(f, "sparse image bad header: {s}"),
            SparseError::ChunkTooBig(n) => {
                write!(
                    f,
                    "sparse chunk of {n} bytes does not fit the download limit"
                )
            }
        }
    }
}

impl std::error::Error for SparseError {}

/// True when the image starts with the android sparse magic.
pub fn is_sparse(image: &[u8]) -> bool {
    image.len() >= 4 && u32::from_le_bytes(image[0..4].try_into().unwrap()) == SPARSE_MAGIC
}

struct SparseHeader {
    major: u16,
    file_hdr_sz: u16,
    chunk_hdr_sz: u16,
    block_size: u32,
    total_blocks: u32,
    total_chunks: u32,
}

fn parse_header(image: &[u8]) -> Result<SparseHeader, SparseError> {
    if image.len() < FILE_HEADER_SZ as usize {
        return Err(SparseError::Truncated("header"));
    }
    let read16 = |off: usize| u16::from_le_bytes(image[off..off + 2].try_into().unwrap());
    let read32 = |off: usize| u32::from_le_bytes(image[off..off + 4].try_into().unwrap());
    let header = SparseHeader {
        major: read16(4),
        file_hdr_sz: read16(8),
        chunk_hdr_sz: read16(10),
        block_size: read32(12),
        total_blocks: read32(16),
        total_chunks: read32(20),
    };
    if header.file_hdr_sz != FILE_HEADER_SZ as u16 || header.chunk_hdr_sz != CHUNK_HEADER_SZ as u16
    {
        return Err(SparseError::BadHeader("unexpected header sizes"));
    }
    if header.major != SPARSE_MAJOR {
        return Err(SparseError::BadHeader("unsupported major version"));
    }
    if header.block_size == 0 {
        return Err(SparseError::BadHeader("zero block size"));
    }
    Ok(header)
}

/// A chunk as stored in the file: 12-byte header plus its payload.
#[derive(Clone)]
struct Chunk {
    kind: u16,
    chunk_sz: u32,
    total_sz: u32,
    payload: Vec<u8>,
}

impl Chunk {
    fn blocks(&self) -> u32 {
        match self.kind {
            CHUNK_RAW | CHUNK_FILL | CHUNK_DONT_CARE => self.chunk_sz,
            _ => 0,
        }
    }

    fn len(&self) -> usize {
        CHUNK_HEADER_SZ as usize + self.payload.len()
    }

    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.kind.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&self.chunk_sz.to_le_bytes());
        out.extend_from_slice(&self.total_sz.to_le_bytes());
        out.extend_from_slice(&self.payload);
    }
}

fn parse_chunks(image: &[u8], header: &SparseHeader) -> Result<Vec<Chunk>, SparseError> {
    let mut chunks = Vec::new();
    let mut pos = header.file_hdr_sz as usize;
    for _ in 0..header.total_chunks {
        if pos + CHUNK_HEADER_SZ as usize > image.len() {
            return Err(SparseError::Truncated("chunk header"));
        }
        let read16 = |off: usize| u16::from_le_bytes(image[off..off + 2].try_into().unwrap());
        let read32 = |off: usize| u32::from_le_bytes(image[off..off + 4].try_into().unwrap());
        let kind = read16(pos);
        let chunk_sz = read32(pos + 4);
        let total_sz = read32(pos + 8);
        if total_sz < CHUNK_HEADER_SZ {
            return Err(SparseError::BadHeader("chunk smaller than its header"));
        }
        let payload_len = total_sz as usize - CHUNK_HEADER_SZ as usize;
        let end = pos + total_sz as usize;
        if end > image.len() {
            return Err(SparseError::Truncated("chunk payload"));
        }
        match kind {
            CHUNK_RAW => {
                let expected = chunk_sz as usize * header.block_size as usize;
                if payload_len != expected {
                    return Err(SparseError::BadHeader("raw chunk size mismatch"));
                }
            }
            CHUNK_FILL | CHUNK_CRC32 => {
                if payload_len != 4 {
                    return Err(SparseError::BadHeader("fill/crc chunk needs 4 bytes"));
                }
            }
            CHUNK_DONT_CARE => {
                if payload_len != 0 {
                    return Err(SparseError::BadHeader("dont-care chunk has no payload"));
                }
            }
            _ => return Err(SparseError::BadHeader("unknown chunk type")),
        }
        chunks.push(Chunk {
            kind,
            chunk_sz,
            total_sz,
            payload: image[pos + CHUNK_HEADER_SZ as usize..end].to_vec(),
        });
        pos = end;
    }
    if pos != image.len() {
        return Err(SparseError::BadHeader("trailing bytes after chunks"));
    }
    Ok(chunks)
}

fn build_part(chunks: &[Chunk], block_size: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let total_blocks: u32 = chunks.iter().map(|c| c.blocks()).sum();
    let mut raw_bytes = Vec::new();
    for chunk in chunks {
        if chunk.kind == CHUNK_RAW {
            raw_bytes.extend_from_slice(&chunk.payload);
        }
    }
    out.extend_from_slice(&SPARSE_MAGIC.to_le_bytes());
    out.extend_from_slice(&SPARSE_MAJOR.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(FILE_HEADER_SZ as u16).to_le_bytes());
    out.extend_from_slice(&(CHUNK_HEADER_SZ as u16).to_le_bytes());
    out.extend_from_slice(&block_size.to_le_bytes());
    out.extend_from_slice(&total_blocks.to_le_bytes());
    out.extend_from_slice(&(chunks.len() as u32).to_le_bytes());
    out.extend_from_slice(&crc32fast::hash(&raw_bytes).to_le_bytes());
    for chunk in chunks {
        chunk.encode(&mut out);
    }
    out
}

/// Splits a sparse image into parts each fitting `max_download` bytes:
/// the flash protocol takes one download per part, and the sparse chunk
/// offsets keep every write at its place on the partition.
pub fn sparse_plan(image: &[u8], max_download: usize) -> Result<Vec<Vec<u8>>, SparseError> {
    let min_part = (FILE_HEADER_SZ + CHUNK_HEADER_SZ) as usize;
    if max_download < min_part {
        return Err(SparseError::ChunkTooBig(max_download));
    }
    let header = parse_header(image)?;
    let chunks = parse_chunks(image, &header)?;

    // Phase 1: raw chunks larger than one download get split into several
    // raw chunks of whole blocks, so every chunk fits one part.
    let per_part_payload = max_download - min_part;
    let blocks_per_part = per_part_payload / header.block_size as usize;
    if blocks_per_part == 0 {
        return Err(SparseError::ChunkTooBig(max_download));
    }
    let block = header.block_size as usize;
    let mut fit: Vec<Chunk> = Vec::new();
    for chunk in chunks {
        if chunk.kind == CHUNK_RAW && chunk.len() > per_part_payload {
            for slice in chunk.payload.chunks(blocks_per_part * block) {
                fit.push(Chunk {
                    kind: CHUNK_RAW,
                    chunk_sz: (slice.len() / block) as u32,
                    total_sz: CHUNK_HEADER_SZ + slice.len() as u32,
                    payload: slice.to_vec(),
                });
            }
        } else {
            fit.push(chunk);
        }
    }

    // Phase 2: greedy packing of chunks into parts.
    let mut parts = Vec::new();
    let mut current: Vec<Chunk> = Vec::new();
    let mut current_len = FILE_HEADER_SZ as usize;
    for chunk in &fit {
        if current_len + chunk.len() > max_download && !current.is_empty() {
            parts.push(build_part(&current, header.block_size));
            current.clear();
            current_len = FILE_HEADER_SZ as usize;
        }
        current.push(chunk.clone());
        current_len += chunk.len();
    }
    if !current.is_empty() {
        parts.push(build_part(&current, header.block_size));
    }
    if parts.is_empty() {
        return Err(SparseError::NotSparse);
    }
    let packed_blocks: u32 = parts
        .iter()
        .filter_map(|p| parse_header(p).ok())
        .map(|h| h.total_blocks)
        .sum();
    if packed_blocks != header.total_blocks {
        return Err(SparseError::BadHeader("block total mismatch"));
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_chunk(data: &[u8]) -> Chunk {
        raw_chunk_b(data, 4096)
    }

    fn raw_chunk_b(data: &[u8], block: u32) -> Chunk {
        Chunk {
            kind: CHUNK_RAW,
            chunk_sz: (data.len() / block as usize) as u32,
            total_sz: CHUNK_HEADER_SZ + data.len() as u32,
            payload: data.to_vec(),
        }
    }

    fn fill_chunk(blocks: u32, value: u32) -> Chunk {
        Chunk {
            kind: CHUNK_FILL,
            chunk_sz: blocks,
            total_sz: CHUNK_HEADER_SZ + 4,
            payload: value.to_le_bytes().to_vec(),
        }
    }

    fn build_image(block_size: u32, chunks: &[Chunk]) -> Vec<u8> {
        build_part(chunks, block_size)
    }

    fn part_raw_data(part: &[u8]) -> Vec<u8> {
        let header = parse_header(part).unwrap();
        let chunks = parse_chunks(part, &header).unwrap();
        let mut out = Vec::new();
        for chunk in chunks {
            if chunk.kind == CHUNK_RAW {
                out.extend_from_slice(&chunk.payload);
            }
        }
        out
    }

    #[test]
    fn parse_rejects_non_sparse() {
        assert!(!is_sparse(b"BOOT!magic"));
        assert!(matches!(
            parse_header(b"AB"),
            Err(SparseError::Truncated("header"))
        ));
    }

    #[test]
    fn roundtrip_single_part() {
        let image = build_image(
            4096,
            &[raw_chunk(&vec![7u8; 4096 * 2]), fill_chunk(3, 0xAB)],
        );
        let parts = sparse_plan(&image, 1 << 20).unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], image);
    }

    #[test]
    fn splits_by_chunks_respecting_limit() {
        let image = build_image(
            4096,
            &[
                raw_chunk(&vec![1u8; 4096]),
                raw_chunk(&vec![2u8; 4096]),
                raw_chunk(&vec![3u8; 4096]),
            ],
        );
        let max = 28 + 12 + 4096 + 12 + 4096;
        let parts = sparse_plan(&image, max).unwrap();
        assert_eq!(parts.len(), 2);
        let first = parse_header(&parts[0]).unwrap();
        assert_eq!(first.total_blocks, 2);
        assert_eq!(first.total_chunks, 2);
        for part in &parts {
            assert!(part.len() <= max);
            assert_eq!(
                u32::from_le_bytes(part[24..28].try_into().unwrap()),
                crc32fast::hash(&part_raw_data(part))
            );
        }
        let total_blocks: u32 = parts
            .iter()
            .map(|p| parse_header(p).unwrap().total_blocks)
            .sum();
        assert_eq!(total_blocks, 3);
    }

    #[test]
    fn splits_oversized_raw_chunk() {
        let image = build_image(4096, &[raw_chunk(&vec![9u8; 4096 * 5])]);
        let max = 28 + 12 + 4096 * 2;
        let parts = sparse_plan(&image, max).unwrap();
        assert_eq!(parts.len(), 3);
        let total: u32 = parts
            .iter()
            .map(|p| parse_header(p).unwrap().total_blocks)
            .sum();
        assert_eq!(total, 5);
        for part in &parts {
            assert!(part.len() <= max);
        }
        let blocks: Vec<u32> = parts
            .iter()
            .map(|p| parse_header(p).unwrap().total_blocks)
            .collect();
        assert_eq!(blocks, vec![2, 2, 1]);
    }

    #[test]
    fn fill_and_dont_care_survive_splitting() {
        let image = build_image(
            4096,
            &[
                fill_chunk(10, 0x1234),
                raw_chunk(&vec![5u8; 4096]),
                Chunk {
                    kind: CHUNK_DONT_CARE,
                    chunk_sz: 2,
                    total_sz: CHUNK_HEADER_SZ,
                    payload: Vec::new(),
                },
            ],
        );
        let parts = sparse_plan(&image, 28 + 12 + 4096).unwrap();
        assert_eq!(parts.len(), 3);
        let kinds: Vec<u16> = parts
            .iter()
            .map(|p| u16::from_le_bytes(p[28..30].try_into().unwrap()))
            .collect();
        assert_eq!(kinds, vec![CHUNK_FILL, CHUNK_RAW, CHUNK_DONT_CARE]);
    }

    #[test]
    fn rejects_truncated_and_bad_types() {
        let image = build_image(4, &[raw_chunk_b(&[1u8; 16], 4)]);
        assert!(matches!(
            parse_chunks(&image[..20], &parse_header(&image).unwrap()),
            Err(SparseError::Truncated("chunk header"))
        ));
        let mut broken = image.clone();
        broken[28] = 0xFF;
        broken[29] = 0xFF;
        assert!(matches!(
            parse_chunks(&broken, &parse_header(&image).unwrap()),
            Err(SparseError::BadHeader("unknown chunk type"))
        ));
    }
}
