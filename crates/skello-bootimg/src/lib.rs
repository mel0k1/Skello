use std::fmt;
use std::io::{Read, Seek, SeekFrom};

pub const BOOT_MAGIC: &[u8; 8] = b"ANDROID!";
pub const AVB_FOOTER_MAGIC: &[u8; 4] = b"AVBf";

const V02_HEADER_LEN: usize = 1656;
const V3_HEADER_LEN: usize = 158;
const FOOTER_LEN: u64 = 64;
const V34_FIXED_PAGE_SIZE: u32 = 4096;
const HEAD_BUFFER: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootImageInfo {
    pub header_version: u32,
    pub kernel_size: u32,
    pub ramdisk_size: u32,
    pub page_size: Option<u32>,
    pub os_version: Option<String>,
    pub os_patch_level: Option<String>,
    pub file_size: u64,
    pub has_avb_footer: bool,
}

#[derive(Debug)]
pub enum BootImgError {
    Io(std::io::Error),
    TooSmall,
    BadMagic,
    UnsupportedHeaderVersion(u32),
}

impl fmt::Display for BootImgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootImgError::Io(e) => write!(f, "io error: {e}"),
            BootImgError::TooSmall => write!(f, "file is too small for a boot image"),
            BootImgError::BadMagic => write!(f, "boot image magic ANDROID! not found"),
            BootImgError::UnsupportedHeaderVersion(v) => {
                write!(f, "unsupported boot image header version {v}")
            }
        }
    }
}

impl std::error::Error for BootImgError {}

impl From<std::io::Error> for BootImgError {
    fn from(e: std::io::Error) -> Self {
        BootImgError::Io(e)
    }
}

pub fn read(path: &str) -> Result<BootImageInfo, BootImgError> {
    let mut file = std::fs::File::open(path)?;
    parse(&mut file)
}

pub fn parse<R: Read + Seek>(reader: &mut R) -> Result<BootImageInfo, BootImgError> {
    let file_size = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(0))?;

    let mut head = [0u8; HEAD_BUFFER];
    let head_len = read_up_to(reader, &mut head)?;
    if head_len < V3_HEADER_LEN {
        return Err(BootImgError::TooSmall);
    }

    let mut footer = [0u8; FOOTER_LEN as usize];
    let has_avb_footer = if file_size >= FOOTER_LEN {
        reader.seek(SeekFrom::End(-(FOOTER_LEN as i64)))?;
        reader.read_exact(&mut footer)?;
        &footer[0..4] == AVB_FOOTER_MAGIC
    } else {
        false
    };

    if head_len < 48 || &head[0..8] != BOOT_MAGIC {
        return Err(BootImgError::BadMagic);
    }
    let header_version = le_u32(&head, 40);
    match header_version {
        0..=2 => {
            if head_len < V02_HEADER_LEN {
                return Err(BootImgError::TooSmall);
            }
            let (os_version, os_patch_level) = decode_os_version(le_u32(&head, 44));
            Ok(BootImageInfo {
                header_version,
                kernel_size: le_u32(&head, 8),
                ramdisk_size: le_u32(&head, 16),
                page_size: Some(le_u32(&head, 36)),
                os_version,
                os_patch_level,
                file_size,
                has_avb_footer,
            })
        }
        3..=4 => {
            let (os_version, os_patch_level) = decode_os_version(le_u32(&head, 16));
            Ok(BootImageInfo {
                header_version,
                kernel_size: le_u32(&head, 8),
                ramdisk_size: le_u32(&head, 12),
                page_size: Some(V34_FIXED_PAGE_SIZE),
                os_version,
                os_patch_level,
                file_size,
                has_avb_footer,
            })
        }
        v => Err(BootImgError::UnsupportedHeaderVersion(v)),
    }
}

fn read_up_to(reader: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    Ok(filled)
}

fn le_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        buf[offset..offset + 4]
            .try_into()
            .expect("offset within head"),
    )
}

fn decode_os_version(v: u32) -> (Option<String>, Option<String>) {
    if v == 0 {
        return (None, None);
    }
    let version = (v >> 11) & 0x1f_ffff;
    let patch = v & 0x7ff;
    let major = (version >> 14) & 0x7f;
    let minor = (version >> 7) & 0x7f;
    let micro = version & 0x7f;
    let year = ((patch >> 4) & 0x7f) + 2000;
    let month = patch & 0xf;
    (
        Some(format!("{major}.{minor}.{micro}")),
        Some(format!("{year}-{month:02}")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn v0_image() -> Vec<u8> {
        let mut v = vec![0u8; V02_HEADER_LEN];
        v[0..8].copy_from_slice(BOOT_MAGIC);
        v[8..12].copy_from_slice(&16384u32.to_le_bytes());
        v[16..20].copy_from_slice(&524288u32.to_le_bytes());
        v[36..40].copy_from_slice(&4096u32.to_le_bytes());
        v[40..44].copy_from_slice(&0u32.to_le_bytes());
        v.extend(std::iter::repeat_n(0xaau8, 4096));
        v
    }

    fn v3_image() -> Vec<u8> {
        let mut v = vec![0u8; V3_HEADER_LEN];
        v[0..8].copy_from_slice(BOOT_MAGIC);
        v[8..12].copy_from_slice(&33554432u32.to_le_bytes());
        v[12..16].copy_from_slice(&1048576u32.to_le_bytes());
        let os = (12u32 << 25) | 373;
        v[16..20].copy_from_slice(&os.to_le_bytes());
        v[20..24].copy_from_slice(&158u32.to_le_bytes());
        v[40..44].copy_from_slice(&3u32.to_le_bytes());
        v.extend(std::iter::repeat_n(0x55u8, 8192));
        v
    }

    fn with_avb_footer(mut image: Vec<u8>) -> Vec<u8> {
        let mut footer = vec![0u8; 64];
        footer[0..4].copy_from_slice(AVB_FOOTER_MAGIC);
        footer[4..8].copy_from_slice(&1u32.to_le_bytes());
        image.extend(footer);
        image
    }

    fn parse_bytes(bytes: Vec<u8>) -> Result<BootImageInfo, BootImgError> {
        parse(&mut Cursor::new(bytes))
    }

    #[test]
    fn parses_v0_header() {
        let info = parse_bytes(v0_image()).unwrap();
        assert_eq!(info.header_version, 0);
        assert_eq!(info.kernel_size, 16384);
        assert_eq!(info.ramdisk_size, 524288);
        assert_eq!(info.page_size, Some(4096));
        assert_eq!(info.os_version, None);
        assert_eq!(info.os_patch_level, None);
        assert_eq!(info.file_size, V02_HEADER_LEN as u64 + 4096);
        assert!(!info.has_avb_footer);
    }

    #[test]
    fn parses_v3_header_with_os_version() {
        let info = parse_bytes(v3_image()).unwrap();
        assert_eq!(info.header_version, 3);
        assert_eq!(info.kernel_size, 33554432);
        assert_eq!(info.ramdisk_size, 1048576);
        assert_eq!(info.page_size, Some(4096));
        assert_eq!(info.os_version.as_deref(), Some("12.0.0"));
        assert_eq!(info.os_patch_level.as_deref(), Some("2023-05"));
    }

    #[test]
    fn detects_avb_footer() {
        let info = parse_bytes(with_avb_footer(v0_image())).unwrap();
        assert!(info.has_avb_footer);
        assert_eq!(info.file_size, V02_HEADER_LEN as u64 + 4096 + 64);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut image = v0_image();
        image[7] = b'?';
        assert!(matches!(parse_bytes(image), Err(BootImgError::BadMagic)));
    }

    #[test]
    fn rejects_truncated_v0() {
        let image = v0_image()[..1024].to_vec();
        assert!(matches!(parse_bytes(image), Err(BootImgError::TooSmall)));
    }

    #[test]
    fn rejects_unsupported_version() {
        let mut image = v0_image();
        image[40..44].copy_from_slice(&7u32.to_le_bytes());
        assert!(matches!(
            parse_bytes(image),
            Err(BootImgError::UnsupportedHeaderVersion(7))
        ));
    }

    #[test]
    fn decodes_known_os_version_values() {
        let (os, patch) = decode_os_version((12u32 << 25) | 373);
        assert_eq!(os.as_deref(), Some("12.0.0"));
        assert_eq!(patch.as_deref(), Some("2023-05"));
        assert_eq!(decode_os_version(0), (None, None));
    }

    #[test]
    fn reads_real_file() {
        let dir = std::env::temp_dir().join("skello_bootimg_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("boot.img");
        std::fs::write(&path, with_avb_footer(v3_image())).unwrap();
        let info = read(path.to_str().unwrap()).unwrap();
        assert_eq!(info.header_version, 3);
        assert!(info.has_avb_footer);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
