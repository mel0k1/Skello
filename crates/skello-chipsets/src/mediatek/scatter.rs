//! The MediaTek scatter file: a text layout description that pairs
//! partition offsets with image files. The format is a plain
//! key/value list; parsing follows the documented structure, the
//! flash flow composes over the DA writeflash.

use std::io;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScatterEntry {
    pub index: String,
    pub name: String,
    pub file_name: Option<String>,
    pub is_download: bool,
    pub linear_start_addr: u64,
    pub physical_start_addr: u64,
    pub partition_size: u64,
    /// EMMC_BOOT1_BOOT2 entries land in the hardware boot area.
    pub boot_area: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Scatter {
    pub platform: String,
    pub block_size: u64,
    pub entries: Vec<ScatterEntry>,
}

#[derive(Debug)]
pub enum ScatterError {
    /// (line, problem)
    BadLine(usize, String),
    Io(io::Error),
}

impl std::fmt::Display for ScatterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScatterError::BadLine(line, what) => write!(f, "scatter line {line}: {what}"),
            ScatterError::Io(e) => write!(f, "scatter io: {e}"),
        }
    }
}

impl std::error::Error for ScatterError {}

impl From<io::Error> for ScatterError {
    fn from(e: io::Error) -> Self {
        ScatterError::Io(e)
    }
}

fn parse_value(line_no: usize, key: &str, value: &str) -> Result<String, ScatterError> {
    let v = value.trim();
    if v.is_empty() {
        return Err(ScatterError::BadLine(line_no, format!("empty {key}")));
    }
    Ok(v.to_string())
}

fn parse_num(line_no: usize, value: &str) -> Result<u64, ScatterError> {
    let v = value.trim();
    let v = v
        .strip_prefix("0x")
        .or_else(|| v.strip_prefix("0X"))
        .unwrap_or(v);
    u64::from_str_radix(v, 16)
        .map_err(|_| ScatterError::BadLine(line_no, format!("not a hex number: {v}")))
}

/// Parses scatter text. Entries start with `- partition_index:`
/// and run until the next `-` line or the end.
pub fn parse_scatter(text: &str) -> Result<Scatter, ScatterError> {
    let mut out = Scatter::default();
    let mut current: Option<ScatterEntry> = None;
    let mut general = false;
    let mut line_no = 0usize;
    for raw in text.lines() {
        line_no += 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("- ") {
            let Some(head) = line.strip_prefix("- ") else {
                continue;
            };
            let head = head.trim();
            if let Some(entry) = current.take() {
                out.entries.push(entry);
            }
            if head.starts_with("partition_index:") {
                general = false;
                current = Some(ScatterEntry {
                    index: head
                        .strip_prefix("partition_index:")
                        .unwrap_or(head)
                        .trim()
                        .to_owned(),
                    ..Default::default()
                });
                continue;
            }
            if head == "general: MTK_PLATFORM_CFG"
                || head.starts_with("info")
                || head.starts_with("config_version")
            {
                // the general section continues (nested items)
                general = true;
                continue;
            }
            general = false;
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(ScatterError::BadLine(
                line_no,
                format!("not key: value ({line})"),
            ));
        };
        let key = key.trim();
        let value = value.trim();
        match (general, key) {
            (true, "platform") => out.platform = value.to_owned(),
            (true, "block_size") => out.block_size = parse_num(line_no, value)?,
            (false, "partition_name") => {
                if let Some(e) = current.as_mut() {
                    e.name = parse_value(line_no, key, value)?;
                }
            }
            (false, "file_name") => {
                if let Some(e) = current.as_mut() {
                    let name = value.to_owned();
                    e.file_name = if name.eq_ignore_ascii_case("NONE") {
                        None
                    } else {
                        Some(name)
                    };
                }
            }
            (false, "is_download") => {
                if let Some(e) = current.as_mut() {
                    e.is_download = value.eq_ignore_ascii_case("true");
                }
            }
            (false, "linear_start_addr") => {
                if let Some(e) = current.as_mut() {
                    e.linear_start_addr = parse_num(line_no, value)?;
                }
            }
            (false, "physical_start_addr") => {
                if let Some(e) = current.as_mut() {
                    e.physical_start_addr = parse_num(line_no, value)?;
                }
            }
            (false, "partition_size") => {
                if let Some(e) = current.as_mut() {
                    e.partition_size = parse_num(line_no, value)?;
                }
            }
            (false, "region") => {
                if let Some(e) = current.as_mut() {
                    e.boot_area = value.contains("BOOT1");
                }
            }
            _ => {}
        }
    }
    if let Some(entry) = current.take() {
        out.entries.push(entry);
    }
    if out.entries.is_empty() {
        return Err(ScatterError::BadLine(0, "no partitions found".into()));
    }
    Ok(out)
}

/// Loads a scatter file from disk.
pub fn load_scatter(path: &str) -> Result<Scatter, ScatterError> {
    let text = std::fs::read_to_string(path)?;
    parse_scatter(&text)
}

/// Resolves an entry's image path inside the images directory.
/// Boot-area entries keep the scatter's file name.
pub fn image_path(scatter_dir: &Path, entry: &ScatterEntry) -> Option<std::path::PathBuf> {
    let name = entry.file_name.as_deref()?;
    if name.is_empty() {
        return None;
    }
    let path = scatter_dir.join(name);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
############################################################################################################
#
#  General Setting
#
############################################################################################################
- general: MTK_PLATFORM_CFG
  info:
  - config_version: V1.1.2
    platform: MT6768
    project: sample
    storage: EMMC
    boot_channel: MSDC_0
    block_size: 0x20000

############################################################################################################
#
#  Layout Setting
#
############################################################################################################
- partition_index: SYS0
  partition_name: preloader
  file_name: preloader.bin
  is_download: true
  type: TMP_MBR_ROM1
  linear_start_addr: 0x0
  physical_start_addr: 0x0
  partition_size: 0x40000
  region: EMMC_BOOT1_BOOT2
  storage: HW_STORAGE_EMMC
  boundary_check: true
  is_reserved: false
  operation_type: INVISIBLE
  dynamic_partition: false
  reserved: 0x0

- partition_index: SYS2
  partition_name: boot
  file_name: boot.img
  is_download: true
  type: EXT4IMG
  linear_start_addr: 0x1D40000
  physical_start_addr: 0x1D40000
  partition_size: 0x1000000
  region: EMMC_BOOT_1
  storage: HW_STORAGE_EMMC
  boundary_check: true
  is_reserved: false
  operation_type: UPDATE
  dynamic_partition: false
  reserved: 0x0

- partition_index: SYS3
  partition_name: para
  file_name: NONE
  is_download: false
  type: NONE
  linear_start_addr: 0x1C000
  physical_start_addr: 0x1C000
  partition_size: 0x80000
  region: EMMC_BOOT_1
  storage: HW_STORAGE_EMMC
  boundary_check: true
  is_reserved: false
  operation_type: INVISIBLE
  dynamic_partition: false
  reserved: 0x0
"#;

    #[test]
    fn parses_general_and_entries() {
        let sc = parse_scatter(SAMPLE).unwrap();
        assert_eq!(sc.platform, "MT6768");
        assert_eq!(sc.block_size, 0x20000);
        assert_eq!(sc.entries.len(), 3);
        let preloader = &sc.entries[0];
        assert_eq!(preloader.name, "preloader");
        assert_eq!(preloader.file_name.as_deref(), Some("preloader.bin"));
        assert!(preloader.is_download);
        assert_eq!(preloader.linear_start_addr, 0x0);
        assert_eq!(preloader.partition_size, 0x40000);
        assert!(preloader.boot_area);
        let boot = &sc.entries[1];
        assert_eq!(boot.linear_start_addr, 0x1D40000);
        assert!(!boot.boot_area);
        let para = &sc.entries[2];
        assert!(para.file_name.is_none());
        assert!(!para.is_download);
    }

    #[test]
    fn rejects_broken_lines() {
        assert!(parse_scatter("hello world").is_err());
        assert!(parse_scatter("- partition_index: SYS0\n  linear_start_addr: zz\n").is_err());
        assert!(parse_scatter("").is_err());
    }

    #[test]
    fn image_resolution_uses_the_directory() {
        let dir = std::env::temp_dir().join("skello_scatter_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("boot.img"), b"IMAGE").unwrap();
        let sc = parse_scatter(SAMPLE).unwrap();
        let boot = image_path(&dir, &sc.entries[1]).unwrap();
        assert_eq!(std::fs::read(boot).unwrap(), b"IMAGE");
        // preloader.bin does not exist in the temp dir
        assert!(image_path(&dir, &sc.entries[0]).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
