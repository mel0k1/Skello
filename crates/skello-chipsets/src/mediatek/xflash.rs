//! MediaTek DA V5 (XFlash) protocol: packet framing, the post-jump
//! initialization and the read/write flash commands. Framing and
//! command flow follow mtkclient xflash_lib.py (Apache-2.0), NOTICE
//! has the attribution. The stock flow boots the DA from loader
//! region [1]; region [2] belongs to the legacy/exploit paths and is
//! not used here.

use std::time::Duration;

use crate::mediatek::brom::{read_exact_t, Brom, BromError};
use crate::mediatek::daldr::{pick_loader, stage_image};
use crate::mediatek::gpt::{find_partition, parse_gpt};
use crate::transport::Link;

pub const XFLASH_MAGIC: u32 = 0xFEEEEEEF;
pub const SYNC_SIGNAL: u32 = 0x434E5953;

pub const DT_PROTOCOL_FLOW: u32 = 1;

pub const CMD_SETUP_ENVIRONMENT: u32 = 0x010100;
pub const CMD_SETUP_HW_INIT_PARAMS: u32 = 0x010101;
pub const CMD_WRITE_DATA: u32 = 0x010004;
pub const CMD_READ_DATA: u32 = 0x010005;
pub const CMD_SHUTDOWN: u32 = 0x010007;
pub const CMD_DEVICE_CTRL: u32 = 0x010009;
pub const CMD_GET_PACKET_LENGTH: u32 = 0x040007;

/// Storage types (DaStorage): eMMC is 1 for the common devices.
pub const STORAGE_EMMC: u32 = 1;
/// eMMC user area partition (EmmcPartitionType).
pub const PART_USER: u32 = 0;

const IO_TIMEOUT: Duration = Duration::from_secs(10);
const BOOT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub enum XFlashError {
    Brom(BromError),
    Io(std::io::Error),
    WrongMagic,
    Status(u32),
    /// The DA answered with a non-zero status; the code is included.
    CommandFailed(u32, &'static str),
    PartitionNotFound(String),
    NoStorage,
}

impl std::fmt::Display for XFlashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XFlashError::Brom(e) => write!(f, "{e}"),
            XFlashError::Io(e) => write!(f, "xflash io: {e}"),
            XFlashError::WrongMagic => write!(f, "xflash wrong packet magic"),
            XFlashError::Status(s) => write!(f, "xflash status {s:#010x}"),
            XFlashError::CommandFailed(s, what) => {
                write!(f, "xflash {what} failed, status {s:#010x}")
            }
            XFlashError::PartitionNotFound(name) => write!(f, "partition {name} not found"),
            XFlashError::NoStorage => write!(f, "DA loader has no stage 1 region"),
        }
    }
}

impl std::error::Error for XFlashError {}

impl From<BromError> for XFlashError {
    fn from(e: BromError) -> Self {
        XFlashError::Brom(e)
    }
}

impl From<std::io::Error> for XFlashError {
    fn from(e: std::io::Error) -> Self {
        XFlashError::Io(e)
    }
}

impl From<crate::transport::LinkError> for XFlashError {
    fn from(e: crate::transport::LinkError) -> Self {
        XFlashError::Io(std::io::Error::other(e.to_string()))
    }
}

type Result<T> = std::result::Result<T, XFlashError>;

/// One DA V5 session over an open byte link.
pub struct XFlash {
    link: Box<dyn Link>,
    pub write_packet_length: usize,
    pub read_packet_length: usize,
}

impl XFlash {
    pub fn new(link: Box<dyn Link>) -> Self {
        Self {
            link,
            write_packet_length: 0x20000,
            read_packet_length: 0x20000,
        }
    }

    fn read_exact(&mut self, buf: &mut [u8], timeout: Duration) -> Result<()> {
        read_exact_t(&mut *self.link, buf, timeout)?;
        Ok(())
    }

    /// Sends one XFlash packet: magic + data type + length + payload.
    pub fn xsend(&mut self, data: &[u8]) -> Result<()> {
        let mut hdr = Vec::with_capacity(12 + data.len());
        hdr.extend_from_slice(&XFLASH_MAGIC.to_le_bytes());
        hdr.extend_from_slice(&DT_PROTOCOL_FLOW.to_le_bytes());
        hdr.extend_from_slice(&(data.len() as u32).to_le_bytes());
        hdr.extend_from_slice(data);
        self.link.write_all(&hdr)?;
        Ok(())
    }

    pub fn xsend_u32(&mut self, value: u32) -> Result<()> {
        self.xsend(&value.to_le_bytes())
    }

    /// Reads one packet body after validating the magic header.
    pub fn xread(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        let mut hdr = [0u8; 12];
        self.read_exact(&mut hdr, timeout)?;
        let magic = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
        let length = u32::from_le_bytes(hdr[8..12].try_into().unwrap()) as usize;
        if magic != XFLASH_MAGIC {
            return Err(XFlashError::WrongMagic);
        }
        let mut body = vec![0u8; length];
        self.read_exact(&mut body, timeout)?;
        Ok(body)
    }

    /// Reads a status word packet: u16 0 or u32 0 are success.
    pub fn status(&mut self) -> Result<u32> {
        let mut hdr = [0u8; 12];
        self.read_exact(&mut hdr, BOOT_TIMEOUT)?;
        let magic = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
        let length = u32::from_le_bytes(hdr[8..12].try_into().unwrap()) as usize;
        if magic != XFLASH_MAGIC {
            return Err(XFlashError::WrongMagic);
        }
        let mut body = vec![0u8; length];
        self.read_exact(&mut body, BOOT_TIMEOUT)?;
        Ok(match length {
            2 => u16::from_le_bytes(body.try_into().unwrap()) as u32,
            4 => u32::from_le_bytes(body.try_into().unwrap()),
            _ => u32::from_le_bytes(body[..4.min(body.len())].try_into().unwrap_or([0, 0, 0, 0])),
        })
    }

    /// Sends the parameters of one command: every parameter travels as
    /// its own packet, then the device answers a status.
    fn send_params(&mut self, params: &[&[u8]]) -> Result<()> {
        for param in params {
            let mut hdr = Vec::with_capacity(12);
            hdr.extend_from_slice(&XFLASH_MAGIC.to_le_bytes());
            hdr.extend_from_slice(&DT_PROTOCOL_FLOW.to_le_bytes());
            hdr.extend_from_slice(&(param.len() as u32).to_le_bytes());
            self.link.write_all(&hdr)?;
            for chunk in param.chunks(0x200) {
                self.link.write_all(chunk)?;
            }
        }
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::Status(status));
        }
        Ok(())
    }

    fn ack(&mut self) -> Result<()> {
        self.xsend_u32(0)
    }

    /// Post-jump initialization: the DA announces itself with one
    /// 0xC0 byte, then host sync, environment, hardware init and the
    /// DA-side SYNC_SIGNAL conclude the handshake.
    pub fn init_after_jump(&mut self) -> Result<()> {
        let mut sync = [0u8; 1];
        self.read_exact(&mut sync, BOOT_TIMEOUT)?;
        if sync[0] != 0xC0 {
            return Err(XFlashError::WrongMagic);
        }
        self.xsend_u32(SYNC_SIGNAL)?;
        self.setup_env()?;
        self.setup_hw_init()?;
        let expected = SYNC_SIGNAL.to_le_bytes();
        let res = self.xread(BOOT_TIMEOUT)?;
        if res != expected {
            return Err(XFlashError::CommandFailed(0, "da sync after setup"));
        }
        self.query_packet_length()?;
        Ok(())
    }

    fn setup_env(&mut self) -> Result<()> {
        self.xsend_u32(CMD_SETUP_ENVIRONMENT)?;
        self.send_params(&[
            &0x01u32.to_le_bytes(), // DA log level
            &0x02u32.to_le_bytes(), // log channel: USB
            &0x01u32.to_le_bytes(), // host OS: linux
            &0u32.to_le_bytes(),    // UFS provision
            &0u32.to_le_bytes(),
        ])
    }

    fn setup_hw_init(&mut self) -> Result<()> {
        self.xsend_u32(CMD_SETUP_HW_INIT_PARAMS)?;
        self.send_params(&[&0u32.to_le_bytes()])
    }

    fn query_packet_length(&mut self) -> Result<()> {
        self.xsend_u32(CMD_DEVICE_CTRL)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::Status(status));
        }
        self.xsend_u32(CMD_GET_PACKET_LENGTH)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::Status(status));
        }
        let resp = self.xread(BOOT_TIMEOUT)?;
        if resp.len() >= 8 {
            self.write_packet_length =
                usize::try_from(u32::from_le_bytes(resp[0..4].try_into().unwrap()))
                    .unwrap_or(self.write_packet_length);
            self.read_packet_length =
                usize::try_from(u32::from_le_bytes(resp[4..8].try_into().unwrap()))
                    .unwrap_or(self.read_packet_length);
        }
        Ok(())
    }

    fn cmd_read_data(&mut self, addr: u64, length: usize) -> Result<()> {
        self.xsend_u32(CMD_READ_DATA)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::CommandFailed(status, "read_data"));
        }
        let mut param = Vec::new();
        param.extend_from_slice(&STORAGE_EMMC.to_le_bytes());
        param.extend_from_slice(&PART_USER.to_le_bytes());
        param.extend_from_slice(&addr.to_le_bytes());
        param.extend_from_slice(&(length as u64).to_le_bytes());
        param.extend_from_slice(&[0u8; 32]); // NAND extension block
        self.send_params(&[&param])
    }

    fn cmd_write_data(&mut self, addr: u64, size: usize) -> Result<()> {
        self.xsend_u32(CMD_WRITE_DATA)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::CommandFailed(status, "write_data"));
        }
        let mut param = Vec::new();
        param.extend_from_slice(&STORAGE_EMMC.to_le_bytes());
        param.extend_from_slice(&PART_USER.to_le_bytes());
        param.extend_from_slice(&addr.to_le_bytes());
        param.extend_from_slice(&(size as u64).to_le_bytes());
        param.extend_from_slice(&[0u8; 32]);
        self.send_params(&[&param])
    }

    /// Reads `length` bytes at `addr` from the eMMC user area.
    pub fn readflash(&mut self, addr: u64, length: usize) -> Result<Vec<u8>> {
        self.cmd_read_data(addr, length)?;
        let mut out = Vec::with_capacity(length);
        while out.len() < length {
            let mut hdr = [0u8; 12];
            self.read_exact(&mut hdr, IO_TIMEOUT)?;
            let magic = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
            let slength = u32::from_le_bytes(hdr[8..12].try_into().unwrap()) as usize;
            if magic != XFLASH_MAGIC {
                return Err(XFlashError::WrongMagic);
            }
            let mut chunk = vec![0u8; slength];
            self.read_exact(&mut chunk, IO_TIMEOUT)?;
            if slength == 4 {
                let flag = u32::from_le_bytes(chunk.try_into().unwrap());
                if flag != 0 {
                    return Err(XFlashError::CommandFailed(flag, "read"));
                }
                continue;
            }
            self.ack()?;
            out.extend_from_slice(&chunk);
        }
        out.truncate(length);
        let final_status = self.status()?;
        if final_status != 0 {
            return Err(XFlashError::CommandFailed(final_status, "read end"));
        }
        Ok(out)
    }

    /// Writes data to `addr` in the eMMC user area, padded to 512.
    pub fn writeflash(&mut self, addr: u64, data: &[u8]) -> Result<()> {
        let mut payload = data.to_vec();
        if !payload.len().is_multiple_of(512) {
            payload.resize(payload.len() + (512 - payload.len() % 512), 0);
        }
        self.cmd_write_data(addr, payload.len())?;
        let mut pos = 0;
        while pos < payload.len() {
            let dsize = (payload.len() - pos).min(self.write_packet_length);
            let mut chunk = payload[pos..pos + dsize].to_vec();
            if !chunk.len().is_multiple_of(512) {
                chunk.resize(chunk.len() + (512 - chunk.len() % 512), 0);
            }
            let checksum = simple_checksum(&chunk);
            self.send_params(&[&0u32.to_le_bytes(), &checksum.to_le_bytes(), &chunk])?;
            pos += dsize;
        }
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::CommandFailed(status, "write end"));
        }
        Ok(())
    }

    /// Shuts the DA down: the device reboots.
    pub fn shutdown(&mut self) -> Result<()> {
        self.xsend_u32(CMD_SHUTDOWN)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::CommandFailed(status, "shutdown"));
        }
        Ok(())
    }

    /// Full unlock helper over real hardware: BROM port, stock DA from
    /// the loader file, seccfg through the GPT, local V3 software-SEJ
    /// patch, write back. V4 and hardware-keyed partitions are refused
    /// by the seccfg layer until the SEJ stage exists.
    pub fn unlock_seccfg(loader_path: &str) -> Result<String> {
        let mut brom = Brom::new(crate::transport::open_usb_serial(
            crate::mediatek::brom::VID_MEDIATEK,
            crate::mediatek::brom::PID_BROM,
        )?);
        Self::unlock_seccfg_from_brom(&mut brom, loader_path)
    }

    /// Same flow over an already established BROM link (mockable).
    pub fn unlock_seccfg_from_brom(brom: &mut Brom, loader_path: &str) -> Result<String> {
        brom.handshake()?;
        let (hw_code, _sub) = brom.get_hw_code()?;
        let (loader, bytes) = pick_loader(loader_path, hw_code)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        let s1 = loader.stage1().ok_or(XFlashError::NoStorage)?;
        let da1 = stage_image(&bytes, s1)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        brom.send_da(s1.start_addr, s1.sig_len, &da1)?;
        brom.jump_da(s1.start_addr)?;
        // After the jump the device re-enumerates as the DA port.
        let link = crate::transport::open_usb_serial(
            crate::mediatek::brom::VID_MEDIATEK,
            crate::mediatek::brom::PID_DA,
        )?;
        let mut xflash = XFlash::new(link);
        xflash.init_after_jump()?;

        let gpt = xflash.readflash(0, 512 * 4)?;
        let parts = parse_gpt(&gpt, 512)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        let (off, _size) = find_partition(&parts, "seccfg", 512)
            .ok_or_else(|| XFlashError::PartitionNotFound("seccfg".to_owned()))?;
        let seccfg = xflash.readflash(off, 0x2000)?;
        match crate::mediatek::seccfg::parse(&seccfg) {
            Ok(info) if info.state == crate::mediatek::LockState::Unlocked => {
                return Ok("device already unlocked".to_owned());
            }
            Ok(_) => {}
            Err(e) => {
                return Err(XFlashError::Io(std::io::Error::other(e.to_string())));
            }
        }
        let patched = crate::mediatek::seccfg::patch(&seccfg, true)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        xflash.writeflash(off, &patched)?;
        let _ = xflash.shutdown();
        Ok("seccfg patched to unlocked through the DA; reboot and verify".to_owned())
    }
}

/// The checksum DA V5 expects per write packet: sum of bytes mod 2^16.
fn simple_checksum(data: &[u8]) -> u16 {
    data.iter().fold(0u16, |acc, b| acc.wrapping_add(*b as u16))
}
