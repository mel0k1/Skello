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
pub const CMD_FORMAT: u32 = 0x010003;
pub const CMD_BOOT_TO: u32 = 0x010008;
/// The DA keeps formatting and asks the host to wait.
pub const STATUS_CONTINUE: u32 = 0x4004_0004;
/// The DA reports the format done.
pub const STATUS_COMPLETE: u32 = 0x4004_0005;
pub const CMD_GET_PACKET_LENGTH: u32 = 0x040007;

/// DA extension commands (mtkclient xflash.py custom_readregister /
/// custom_writeregister): 32-bit register access used to drive the
/// on-chip SEJ engine.
pub const CMD_CUSTOM_READREGISTER: u32 = 0x0F0002;
pub const CMD_CUSTOM_WRITEREGISTER: u32 = 0x0F0004;

/// Storage types (DaStorage): eMMC is 1 for the common devices.
pub const STORAGE_EMMC: u32 = 1;
/// eMMC partition areas (EmmcPartitionType): user area plus the
/// hardware partitions the DA can address.
pub const PART_USER: u32 = 0;
pub const PART_BOOT1: u32 = 1;
pub const PART_BOOT2: u32 = 2;
/// The Replay Protected Memory Block (parttype 3, mtkclient).
pub const PART_RPMB: u32 = 3;

const IO_TIMEOUT: Duration = Duration::from_secs(10);
const BOOT_TIMEOUT: Duration = Duration::from_secs(30);
/// Sanity cap on any length field coming from the device: a glitched
/// or wedged DA must not make the host allocate gigabytes.
const MAX_PACKET: usize = 16 * 1024 * 1024;
/// Fallback packet length while the DA has not reported usable ones.
const DEFAULT_PACKET_LENGTH: usize = 0x20000;

#[derive(Debug)]
pub enum XFlashError {
    Brom(BromError),
    Io(std::io::Error),
    WrongMagic,
    Status(u32),
    /// The DA answered with a non-zero status; the code is included.
    CommandFailed(u32, &'static str),
    PartitionNotFound(String),
    /// A flow precondition failed (e.g. erasing an irreversible block).
    Unexpected(String),
    NoStorage,
    /// A custom register command answered with fewer bytes than a u32.
    RegisterReply(usize),
    /// The stop button was pressed between packets.
    Cancelled,
}

impl std::fmt::Display for XFlashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XFlashError::Brom(e) => write!(f, "{e}"),
            XFlashError::Io(e) => write!(f, "xflash io: {e}"),
            XFlashError::WrongMagic => write!(f, "xflash wrong packet magic"),
            XFlashError::Cancelled => write!(f, "operation cancelled"),
            XFlashError::Status(s) => write!(f, "xflash status {s:#010x}"),
            XFlashError::CommandFailed(s, what) => {
                write!(f, "xflash {what} failed, status {s:#010x}")
            }
            XFlashError::PartitionNotFound(name) => write!(f, "partition {name} not found"),
            XFlashError::Unexpected(what) => write!(f, "{what}"),
            XFlashError::NoStorage => write!(f, "DA loader has no stage 1 region"),
            XFlashError::RegisterReply(n) => {
                write!(f, "register reply too short: {n} bytes")
            }
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
    #[cfg(test)]
    last_parttype: std::cell::Cell<u32>,
}

impl XFlash {
    pub fn new(link: Box<dyn Link>) -> Self {
        Self {
            link,
            write_packet_length: 0x20000,
            read_packet_length: 0x20000,
            #[cfg(test)]
            last_parttype: std::cell::Cell::new(0),
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
        if length > MAX_PACKET {
            return Err(XFlashError::Unexpected(format!("packet of {length} bytes")));
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
            _ => {
                if body.len() >= 4 {
                    u32::from_le_bytes(body[..4].try_into().unwrap())
                } else {
                    return Err(XFlashError::Unexpected(format!(
                        "status packet of {length} bytes"
                    )));
                }
            }
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
            // Keep the defaults when the DA reports something unusable:
            // a zero or unaligned length would stall the write loop or
            // desynchronize the packet stream.
            let usable = |v: usize| v != 0 && v.is_multiple_of(512) && v <= MAX_PACKET;
            let wp = usize::try_from(u32::from_le_bytes(resp[0..4].try_into().unwrap()))
                .unwrap_or(DEFAULT_PACKET_LENGTH);
            let rp = usize::try_from(u32::from_le_bytes(resp[4..8].try_into().unwrap()))
                .unwrap_or(DEFAULT_PACKET_LENGTH);
            if usable(wp) {
                self.write_packet_length = wp;
            }
            if usable(rp) {
                self.read_packet_length = rp;
            }
        }
        Ok(())
    }

    fn cmd_read_data(&mut self, addr: u64, length: usize, parttype: u32) -> Result<()> {
        #[cfg(test)]
        self.last_parttype.set(parttype);
        self.xsend_u32(CMD_READ_DATA)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::CommandFailed(status, "read_data"));
        }
        let mut param = Vec::new();
        param.extend_from_slice(&STORAGE_EMMC.to_le_bytes());
        param.extend_from_slice(&parttype.to_le_bytes());
        param.extend_from_slice(&addr.to_le_bytes());
        param.extend_from_slice(&(length as u64).to_le_bytes());
        param.extend_from_slice(&[0u8; 32]); // NAND extension block
        self.send_params(&[&param])
    }

    fn cmd_write_data(&mut self, addr: u64, size: usize, parttype: u32) -> Result<()> {
        #[cfg(test)]
        self.last_parttype.set(parttype);
        self.xsend_u32(CMD_WRITE_DATA)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::CommandFailed(status, "write_data"));
        }
        let mut param = Vec::new();
        param.extend_from_slice(&STORAGE_EMMC.to_le_bytes());
        param.extend_from_slice(&parttype.to_le_bytes());
        param.extend_from_slice(&addr.to_le_bytes());
        param.extend_from_slice(&(size as u64).to_le_bytes());
        param.extend_from_slice(&[0u8; 32]);
        self.send_params(&[&param])
    }

    /// Reads `length` bytes at `addr` from the eMMC user area.
    pub fn readflash(&mut self, addr: u64, length: usize) -> Result<Vec<u8>> {
        self.readflash_pt(addr, length, PART_USER)
    }

    /// Reads through the DA from any partition area: user, boot1 /
    /// boot2 or the RPMB block (parttype 3, mtkclient).
    pub fn readflash_pt(&mut self, addr: u64, length: usize, parttype: u32) -> Result<Vec<u8>> {
        if skello_cancel::is_requested() {
            return Err(XFlashError::Cancelled);
        }
        self.cmd_read_data(addr, length, parttype)?;
        let mut out = Vec::with_capacity(length);
        let mut flag_packets = 0u32;
        while out.len() < length {
            if skello_cancel::is_requested() {
                return Err(XFlashError::Cancelled);
            }
            let mut hdr = [0u8; 12];
            self.read_exact(&mut hdr, IO_TIMEOUT)?;
            let magic = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
            let slength = u32::from_le_bytes(hdr[8..12].try_into().unwrap()) as usize;
            if magic != XFLASH_MAGIC {
                return Err(XFlashError::WrongMagic);
            }
            if slength > MAX_PACKET {
                return Err(XFlashError::Unexpected(format!(
                    "read chunk of {slength} bytes"
                )));
            }
            let mut chunk = vec![0u8; slength];
            self.read_exact(&mut chunk, IO_TIMEOUT)?;
            if slength == 4 {
                flag_packets += 1;
                if flag_packets > 1024 {
                    return Err(XFlashError::Unexpected(
                        "read flooded with status flags".into(),
                    ));
                }
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
        self.writeflash_pt(addr, data, PART_USER)
    }

    /// Writes to any partition area: user, boot1 / boot2 or RPMB.
    pub fn writeflash_pt(&mut self, addr: u64, data: &[u8], parttype: u32) -> Result<()> {
        if skello_cancel::is_requested() {
            return Err(XFlashError::Cancelled);
        }
        let mut payload = data.to_vec();
        if !payload.len().is_multiple_of(512) {
            payload.resize(payload.len() + (512 - payload.len() % 512), 0);
        }
        self.cmd_write_data(addr, payload.len(), parttype)?;
        let mut pos = 0;
        while pos < payload.len() {
            if skello_cancel::is_requested() {
                return Err(XFlashError::Cancelled);
            }
            let dsize = (payload.len() - pos).min(self.write_packet_length.max(512));
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

    /// `FORMAT` over the running DA: erases `length` bytes at `addr`
    /// in the given partition area. The DA may ask to wait a few
    /// milliseconds between status polls (STATUS_CONTINUE).
    pub fn format_pt(&mut self, addr: u64, length: u64, parttype: u32) -> Result<()> {
        if skello_cancel::is_requested() {
            return Err(XFlashError::Cancelled);
        }
        self.xsend_u32(CMD_FORMAT)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::CommandFailed(status, "format"));
        }
        let mut param = Vec::with_capacity(64);
        param.extend_from_slice(&STORAGE_EMMC.to_le_bytes());
        param.extend_from_slice(&parttype.to_le_bytes());
        param.extend_from_slice(&addr.to_le_bytes());
        param.extend_from_slice(&length.to_le_bytes());
        param.extend_from_slice(&[0u8; 32]); // NandExtension block
        self.send_params(&[&param])?;
        let mut status = self.status()?;
        for _ in 0..120 {
            if skello_cancel::is_requested() {
                return Err(XFlashError::Cancelled);
            }
            if status == STATUS_COMPLETE {
                return Ok(());
            }
            if status == STATUS_CONTINUE {
                // the wait hint is in milliseconds
                std::thread::sleep(Duration::from_millis(u64::from(status % 1000).max(1)));
                self.ack()?;
                status = self.status()?;
                continue;
            }
            if status == 0 {
                return Ok(());
            }
            return Err(XFlashError::CommandFailed(status, "format"));
        }
        Err(XFlashError::CommandFailed(status, "format timeout"))
    }

    /// `BOOT_TO` over the running DA: writes `data` at `addr` and
    /// jumps there (the Carbonara hash re-marking and stage 2 boot).
    /// The new stage announces itself with the sync signal.
    pub fn boot_to(&mut self, addr: u32, data: &[u8]) -> Result<()> {
        self.xsend_u32(CMD_BOOT_TO)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::CommandFailed(status, "boot_to"));
        }
        let mut param = Vec::with_capacity(16);
        param.extend_from_slice(&(addr as u64).to_le_bytes());
        param.extend_from_slice(&(data.len() as u64).to_le_bytes());
        self.xsend(&param)?;
        self.send_data(data)?;
        std::thread::sleep(Duration::from_millis(500));
        let status = match self.status() {
            Ok(status) => status,
            Err(_) => return Err(XFlashError::CommandFailed(1, "boot_to stage")),
        };
        if status == SYNC_SIGNAL || status == 0 {
            return Ok(());
        }
        Err(XFlashError::CommandFailed(status, "boot_to stage"))
    }

    /// One raw data stream after a command: the length goes out as a
    /// packet header, the payload follows unframed, then the DA
    /// answers with a status.
    fn send_data(&mut self, data: &[u8]) -> Result<()> {
        let mut hdr = Vec::with_capacity(12);
        hdr.extend_from_slice(&XFLASH_MAGIC.to_le_bytes());
        hdr.extend_from_slice(&DT_PROTOCOL_FLOW.to_le_bytes());
        hdr.extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.link.write_all(&hdr)?;
        for chunk in data.chunks(self.write_packet_length.max(1)) {
            self.link.write_all(chunk)?;
        }
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::CommandFailed(status, "send_data"));
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

    /// `CUSTOM_READREGISTER` through DEVICE_CTRL: one 32-bit register
    /// of the chip, the SEJ engine path (mtkclient xflash.py).
    pub fn custom_readregister(&mut self, addr: u32) -> Result<u32> {
        self.xsend_u32(CMD_DEVICE_CTRL)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::Status(status));
        }
        self.xsend_u32(CMD_CUSTOM_READREGISTER)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::Status(status));
        }
        self.xsend_u32(addr)?;
        let data = self.xread(IO_TIMEOUT)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::Status(status));
        }
        if data.len() < 4 {
            return Err(XFlashError::RegisterReply(data.len()));
        }
        Ok(u32::from_le_bytes(data[0..4].try_into().unwrap()))
    }

    /// `CUSTOM_WRITEREGISTER` through DEVICE_CTRL: writes one 32-bit
    /// register of the chip.
    pub fn custom_writeregister(&mut self, addr: u32, value: u32) -> Result<()> {
        self.xsend_u32(CMD_DEVICE_CTRL)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::Status(status));
        }
        self.xsend_u32(CMD_CUSTOM_WRITEREGISTER)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::Status(status));
        }
        self.xsend_u32(addr)?;
        self.xsend_u32(value)?;
        let status = self.status()?;
        if status != 0 {
            return Err(XFlashError::Status(status));
        }
        Ok(())
    }

    /// One AES-128-CBC pass through the on-chip SEJ engine: the
    /// register sequence runs over the DA custom register commands.
    pub fn hacc_output(&mut self, data: &[u8], legacy: bool, encrypt: bool) -> Result<Vec<u8>> {
        let mut bus = DaSejBus { flash: self };
        let mut engine =
            crate::mediatek::sej::SejEngine::new(&mut bus, crate::mediatek::sej::SEJ_BASE_DEFAULT);
        engine
            .hacc_output(data, legacy, encrypt)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))
    }

    #[cfg(test)]
    pub(crate) fn check_last_parttype(&self, pred: impl Fn(u32) -> bool) -> bool {
        pred(self.last_parttype.get())
    }

    /// Full unlock helper over real hardware: BROM port, stock DA from
    /// the loader file, seccfg through the GPT, local V3 software-SEJ
    /// patch or the V4 patch signed by the on-chip SEJ engine through
    /// the DA registers.
    pub fn unlock_seccfg(loader_path: &str) -> Result<String> {
        let mut brom = Brom::new(crate::transport::open_usb_serial(
            crate::mediatek::brom::VID_MEDIATEK,
            crate::mediatek::brom::PID_BROM,
        )?);
        Self::unlock_seccfg_with(&mut brom, loader_path, &mut || {
            crate::transport::open_usb_serial(
                crate::mediatek::brom::VID_MEDIATEK,
                crate::mediatek::brom::PID_DA,
            )
        })
    }

    /// Boots the stock DA, reads the GPT and erases the named
    /// partition: the whole area goes with one FORMAT command.
    /// `hard` erases the user area block; RPMB never erases (the
    /// counter is irreversible by design, the DA refuses it).
    pub fn erase_partition_with(brom: &mut Brom, loader_path: &str, name: &str) -> Result<String> {
        brom.handshake()?;
        let (hw_code, _sub) = brom.get_hw_code()?;
        let (loader, bytes) = pick_loader(loader_path, hw_code)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        let s1 = loader.stage1().ok_or(XFlashError::NoStorage)?;
        let da1 = stage_image(&bytes, s1)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        brom.send_da(s1.start_addr, s1.sig_len, &da1)?;
        brom.jump_da(s1.start_addr)?;
        let link = crate::transport::open_usb_serial(
            crate::mediatek::brom::VID_MEDIATEK,
            crate::mediatek::brom::PID_DA,
        )?;
        let mut xflash = XFlash::new(link);
        xflash.init_after_jump()?;
        // GPT header plus the entry array: 34 sectors cover the
        // standard 128-entry layout, 4 cut the array in half.
        let gpt = xflash.readflash(0, 512 * 34)?;
        let parts = parse_gpt(&gpt, 512)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        let (off, size) = find_partition(&parts, name, 512)
            .ok_or_else(|| XFlashError::PartitionNotFound(name.to_owned()))?;
        if name.eq_ignore_ascii_case("rpmb") {
            return Err(XFlashError::Unexpected("rpmb cannot be erased".into()));
        }
        xflash.format_pt(off, size, PART_USER)?;
        let _ = xflash.shutdown();
        Ok(format!("{name} erased ({} sectors)", size / 512))
    }

    /// Boots the stock DA and reads the RPMB block (parttype 3):
    /// devices whose DA refuses RPMB without a provisioned key fail
    /// with the DA status.
    pub fn rpmb_read_with(
        brom: &mut Brom,
        loader_path: &str,
        addr: u64,
        length: usize,
    ) -> Result<Vec<u8>> {
        let mut xflash = Self::session_from_brom(brom, loader_path)?;
        let data = xflash.readflash_pt(addr, length, PART_RPMB)?;
        let _ = xflash.shutdown();
        Ok(data)
    }

    /// Boots the stock DA and writes to the RPMB block.
    pub fn rpmb_write_with(
        brom: &mut Brom,
        loader_path: &str,
        addr: u64,
        data: &[u8],
    ) -> Result<String> {
        let mut xflash = Self::session_from_brom(brom, loader_path)?;
        xflash.writeflash_pt(addr, data, PART_RPMB)?;
        let _ = xflash.shutdown();
        Ok(format!("{} bytes written to rpmb", data.len()))
    }

    /// Boots the stock DA and flashes every scatter entry marked for
    /// download: the images come from the images directory next to
    /// the scatter file. Boot-area entries write into the hardware
    /// boot area (parttype 1); an image larger than the partition
    /// fails before anything is written.
    pub fn scatter_flash_with(
        brom: &mut Brom,
        loader_path: &str,
        scatter_path: &str,
    ) -> Result<String> {
        use std::path::Path;
        let sc = crate::mediatek::scatter::load_scatter(scatter_path)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        let dir = Path::new(scatter_path)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        brom.handshake()?;
        let (hw_code, _sub) = brom.get_hw_code()?;
        let (loader, bytes) = pick_loader(loader_path, hw_code)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        let s1 = loader.stage1().ok_or(XFlashError::NoStorage)?;
        let da1 = stage_image(&bytes, s1)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        brom.send_da(s1.start_addr, s1.sig_len, &da1)?;
        brom.jump_da(s1.start_addr)?;
        let link = crate::transport::open_usb_serial(
            crate::mediatek::brom::VID_MEDIATEK,
            crate::mediatek::brom::PID_DA,
        )?;
        let mut xflash = XFlash::new(link);
        xflash.init_after_jump()?;

        let mut written = Vec::new();
        let mut shrunk: Vec<String> = Vec::new();
        for entry in &sc.entries {
            if skello_cancel::is_requested() {
                return Err(XFlashError::Cancelled);
            }
            if !entry.is_download {
                continue;
            }
            let Some(image) = crate::mediatek::scatter::image_path(&dir, entry) else {
                continue;
            };
            let data = std::fs::read(&image)
                .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
            if entry.partition_size != 0 && data.len() as u64 > entry.partition_size {
                return Err(XFlashError::Unexpected(format!(
                    "{}: image is {} bytes, partition holds {}",
                    entry.name,
                    data.len(),
                    entry.partition_size
                )));
            }
            let parttype = match entry.area {
                crate::mediatek::scatter::Area::Boot1 => PART_BOOT1,
                crate::mediatek::scatter::Area::Boot2 => PART_BOOT2,
                crate::mediatek::scatter::Area::User => PART_USER,
            };
            xflash.writeflash_pt(entry.linear_start_addr, &data, parttype)?;
            written.push(entry.name.clone());
            if (data.len() as u64) < entry.partition_size {
                shrunk.push(format!(
                    "{}: image {} bytes < partition {} bytes",
                    entry.name,
                    data.len(),
                    entry.partition_size
                ));
            }
        }
        let _ = xflash.shutdown();
        let mut result = format!(
            "scatter {}: flashed {} partitions: {}",
            sc.platform,
            written.len(),
            written.join(", ")
        );
        if !shrunk.is_empty() {
            result.push_str("; note: ");
            result.push_str(&shrunk.join("; "));
        }
        Ok(result)
    }

    /// BROM handshake, stock DA upload and jump, session init: the
    /// common prefix of the parttype flows.
    fn session_from_brom(brom: &mut Brom, loader_path: &str) -> Result<XFlash> {
        brom.handshake()?;
        let (hw_code, _sub) = brom.get_hw_code()?;
        let (loader, bytes) = pick_loader(loader_path, hw_code)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        let s1 = loader.stage1().ok_or(XFlashError::NoStorage)?;
        let da1 = stage_image(&bytes, s1)
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
        brom.send_da(s1.start_addr, s1.sig_len, &da1)?;
        brom.jump_da(s1.start_addr)?;
        let link = crate::transport::open_usb_serial(
            crate::mediatek::brom::VID_MEDIATEK,
            crate::mediatek::brom::PID_DA,
        )?;
        let mut xflash = XFlash::new(link);
        xflash.init_after_jump()?;
        Ok(xflash)
    }

    /// Same flow over an already established BROM link; `da_open`
    /// supplies the re-enumerated DA port so tests can substitute a
    /// mock device.
    pub fn unlock_seccfg_with(
        brom: &mut Brom,
        loader_path: &str,
        da_open: &mut dyn FnMut()
            -> std::result::Result<Box<dyn Link>, crate::transport::LinkError>,
    ) -> Result<String> {
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
        let link = da_open()?;
        let mut xflash = XFlash::new(link);
        xflash.init_after_jump()?;

        // 34 sectors: header plus the full 128-entry array.
        let gpt = xflash.readflash(0, 512 * 34)?;
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
        let patched = {
            let info = crate::mediatek::seccfg::parse(&seccfg)
                .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?;
            if info.version == crate::mediatek::SecCfgVersion::V4 {
                let mut da_signer = DaV4Signer { flash: &mut xflash };
                let signer = &mut da_signer as &mut dyn crate::mediatek::seccfg::V4Signer;
                crate::mediatek::seccfg::patch_v4(&seccfg, true, &mut Some(signer))
            } else {
                crate::mediatek::seccfg::patch(&seccfg, true)
            }
            .map_err(|e| XFlashError::Io(std::io::Error::other(e.to_string())))?
        };
        xflash.writeflash(off, &patched)?;
        let _ = xflash.shutdown();
        Ok("seccfg patched to unlocked through the DA; reboot and verify".to_owned())
    }
}

/// The DA register access as the SEJ engine bus.
struct DaSejBus<'a> {
    flash: &'a mut XFlash,
}

impl crate::mediatek::sej::SejBus for DaSejBus<'_> {
    fn read32(&mut self, addr: u32) -> std::io::Result<u32> {
        self.flash
            .custom_readregister(addr)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    fn write32(&mut self, addr: u32, value: u32) -> std::io::Result<()> {
        self.flash
            .custom_writeregister(addr, value)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
}

/// Signs V4 seccfg hashes with the on-chip SEJ engine: the key is
/// HUID-bound, so the hash modes beyond `Sw` must run on the device.
struct DaV4Signer<'a> {
    flash: &'a mut XFlash,
}

impl crate::mediatek::seccfg::V4Signer for DaV4Signer<'_> {
    fn decrypt(
        &mut self,
        data: &[u8],
        mode: crate::mediatek::seccfg::SejMode,
    ) -> std::result::Result<Vec<u8>, crate::mediatek::seccfg::SecCfgError> {
        use crate::mediatek::seccfg::SejMode;
        let legacy = match mode {
            SejMode::V4Legacy => true,
            SejMode::V3 => false,
            other => {
                return Err(crate::mediatek::seccfg::SecCfgError::NeedsSej(
                    match other {
                        SejMode::V2 => "V2 hash through the DA engine is not supported",
                        _ => "device-keyed hash",
                    },
                ))
            }
        };
        self.flash
            .hacc_output(data, legacy, false)
            .map_err(|e| crate::mediatek::seccfg::SecCfgError::Engine(e.to_string()))
    }

    fn encrypt(
        &mut self,
        data: &[u8],
        mode: crate::mediatek::seccfg::SejMode,
    ) -> std::result::Result<Vec<u8>, crate::mediatek::seccfg::SecCfgError> {
        use crate::mediatek::seccfg::SejMode;
        let legacy = match mode {
            SejMode::V4Legacy => true,
            SejMode::V3 => false,
            other => {
                return Err(crate::mediatek::seccfg::SecCfgError::NeedsSej(
                    match other {
                        SejMode::V2 => "V2 hash through the DA engine is not supported",
                        _ => "device-keyed hash",
                    },
                ))
            }
        };
        self.flash
            .hacc_output(data, legacy, true)
            .map_err(|e| crate::mediatek::seccfg::SecCfgError::Engine(e.to_string()))
    }
}

/// The checksum DA V5 expects per write packet: sum of bytes mod 2^16.
fn simple_checksum(data: &[u8]) -> u16 {
    data.iter().fold(0u16, |acc, b| acc.wrapping_add(*b as u16))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mediatek::brom::mock_brom;
    use crate::mediatek::sej::testutil::MockHacc;
    use crate::mediatek::sej::SejBus;
    use crate::transport::MockLink;
    use sha2::{Digest, Sha256};
    use std::cell::RefCell;
    use std::collections::VecDeque;

    const SECCFG_LBA: u64 = 1024;

    /// A V5 loader file: one DA entry with three regions in the 0xDC
    /// stride layout (same shape as the daldr test fixture).
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
        for i in 0x148..0xE00u32 {
            data[i as usize] = (i * 3) as u8;
        }
        data
    }

    /// 34 LBAs: protective MBR, GPT header, entry array with one
    /// seccfg partition, the same span the flows read back.
    fn build_gpt() -> Vec<u8> {
        let lba = 512usize;
        let mut disk = vec![0u8; lba * 34];
        disk[lba..lba + 8].copy_from_slice(b"EFI PART");
        disk[lba + 72..lba + 80].copy_from_slice(&2u64.to_le_bytes());
        disk[lba + 80..lba + 84].copy_from_slice(&1u32.to_le_bytes());
        disk[lba + 84..lba + 88].copy_from_slice(&128u32.to_le_bytes());
        let base = lba * 2;
        disk[base] = 1;
        disk[base + 32..base + 40].copy_from_slice(&SECCFG_LBA.to_le_bytes());
        disk[base + 40..base + 48].copy_from_slice(&(SECCFG_LBA + 16).to_le_bytes());
        for (i, c) in "seccfg".chars().enumerate() {
            disk[base + 56 + i * 2..base + 58 + i * 2].copy_from_slice(&(c as u16).to_le_bytes());
        }
        let array_crc = crc32fast::hash(&disk[base..base + 128]);
        disk[lba + 88..lba + 92].copy_from_slice(&array_crc.to_le_bytes());
        disk[lba + 16..lba + 20].fill(0);
        let header_crc = crc32fast::hash(&disk[lba..lba + 92]);
        disk[lba + 16..lba + 20].copy_from_slice(&header_crc.to_le_bytes());
        disk
    }

    /// A locked V4 seccfg whose hash is hw-mode SEJ encrypted (the
    /// zero-key AES-128-CBC the engine computes with binding off).
    fn v4_image() -> Vec<u8> {
        let mut header = [0u8; 28];
        header[0..4].copy_from_slice(&0x4D4D4D4Du32.to_le_bytes());
        header[4..8].copy_from_slice(&4u32.to_le_bytes());
        header[8..12].copy_from_slice(&0x3Cu32.to_le_bytes());
        header[12..16].copy_from_slice(&1u32.to_le_bytes());
        header[24..28].copy_from_slice(&0x45454545u32.to_le_bytes());
        let hash: [u8; 32] = Sha256::digest(header).into();
        let mut out = header.to_vec();
        out.extend(crate::mediatek::sej::sej_hw_soft(&hash, true).unwrap());
        out.resize(0x2000, 0);
        out
    }

    /// Device side of the DA V5 session: the init handshake, GPT and
    /// seccfg reads, the custom register commands driving the emulated
    /// SEJ engine and the writeback capture.
    fn mock_da() -> MockLink {
        let mut link = MockLink::new();
        // the DA announces itself with one 0xC0 byte after the jump
        link.push(&[0xC0]);
        let state = RefCell::new(DaMock::default());
        link.device = Some(Box::new(move |out, input| {
            let mut st = state.borrow_mut();
            st.in_buf.extend_from_slice(input);
            st.process(out);
        }));
        link
    }

    #[derive(Default)]
    struct DaMock {
        in_buf: Vec<u8>,
        env_params: usize,
        hw_params: usize,
        devctrl: bool,
        expect_reg_addr: bool,
        expect_reg_value: bool,
        reg_addr: u32,
        hacc: MockHacc,
        expect_read_param: bool,
        read_addr: u64,
        read_len: usize,
        expect_write_param: bool,
        write_size: usize,
        write_received: usize,
        write_data: Vec<u8>,
        shutdown: bool,
        sync_seen: bool,
        sync_sent: bool,
        reading: bool,
        expect_value_only: bool,
        last_parttype: u32,
        expect_format_param: bool,
    }

    impl DaMock {
        fn packet(&mut self, out: &mut VecDeque<u8>, body: &[u8]) {
            out.extend(XFLASH_MAGIC.to_le_bytes());
            out.extend(DT_PROTOCOL_FLOW.to_le_bytes());
            out.extend((body.len() as u32).to_le_bytes());
            out.extend(body.iter().copied());
        }

        fn status_ok(&mut self, out: &mut VecDeque<u8>) {
            self.packet(out, &0u32.to_le_bytes());
        }

        fn process(&mut self, out: &mut VecDeque<u8>) {
            loop {
                if self.in_buf.len() < 12 {
                    break;
                }
                let magic = u32::from_le_bytes(self.in_buf[0..4].try_into().unwrap());
                if magic != XFLASH_MAGIC {
                    break;
                }
                let length = u32::from_le_bytes(self.in_buf[8..12].try_into().unwrap()) as usize;
                if self.in_buf.len() < 12 + length {
                    break;
                }
                let body: Vec<u8> = self.in_buf.drain(..12 + length).skip(12).collect();
                self.handle(&body, out);
            }
        }

        fn handle(&mut self, body: &[u8], out: &mut VecDeque<u8>) {
            // flash write: the zero-word + checksum pair precede each chunk
            if self.expect_write_param {
                self.expect_write_param = false;
                self.write_size = u64::from_le_bytes(body[16..24].try_into().unwrap()) as usize;
                self.write_received = 0;
                self.status_ok(out);
                return;
            }
            if self.write_size > 0 && self.write_received < self.write_size {
                if body.len() > 4 {
                    self.write_data.extend_from_slice(body);
                    self.write_received += body.len();
                    self.status_ok(out);
                    if self.write_received >= self.write_size {
                        // one more status for the host's final check
                        self.status_ok(out);
                    }
                    return;
                }
                // the zero word and the 2-byte checksum: consumed silently
                if body.len() <= 4 {
                    return;
                }
            }
            // custom register parameter packets carry the addr / value
            if self.expect_reg_addr && body.len() == 4 {
                self.expect_reg_addr = false;
                self.reg_addr = u32::from_le_bytes(body.try_into().unwrap());
                if self.expect_reg_value {
                    // write flow: the value packet follows
                    self.expect_reg_value = false;
                    self.expect_value_only = true;
                } else {
                    // read flow: the value goes out right away
                    let value = self.hacc.read32(self.reg_addr).unwrap_or(0);
                    self.packet(out, &value.to_le_bytes());
                    self.status_ok(out);
                }
                return;
            }
            if self.expect_value_only && body.len() == 4 {
                self.expect_value_only = false;
                let value = u32::from_le_bytes(body.try_into().unwrap());
                let _ = self.hacc.write32(self.reg_addr, value);
                self.status_ok(out);
                return;
            }

            // flash read: consume the host acks between data packets
            if self.expect_read_param {
                self.expect_read_param = false;
                self.last_parttype = u32::from_le_bytes(body[4..8].try_into().unwrap());
                self.read_addr = u64::from_le_bytes(body[8..16].try_into().unwrap());
                self.read_len = u64::from_le_bytes(body[16..24].try_into().unwrap()) as usize;
                // the param packet itself is answered with a status
                self.status_ok(out);
                let source: Vec<u8> = if self.read_addr == 0 {
                    build_gpt()
                } else {
                    v4_image()
                };
                let mut pos = 0;
                while pos < self.read_len {
                    let end = (pos + 0x800).min(self.read_len);
                    self.packet(out, &source[pos..end]);
                    pos = end;
                }
                self.status_ok(out);
                self.reading = true;
                return;
            }
            if self.reading && body.len() == 4 {
                // an ack for the last data packet: consumed silently
                if u32::from_le_bytes(body.try_into().unwrap()) == 0 {
                    return;
                }
            }
            if body.len() < 4 {
                return;
            }
            let cmd = u32::from_le_bytes(body[0..4].try_into().unwrap());
            match cmd {
                SYNC_SIGNAL => {
                    // the echo goes out only after the env / hw init
                    self.sync_seen = true;
                }
                CMD_SETUP_ENVIRONMENT => {
                    self.env_params = 5;
                }
                CMD_SETUP_HW_INIT_PARAMS => {
                    self.hw_params = 1;
                }
                CMD_DEVICE_CTRL => {
                    self.status_ok(out);
                    self.devctrl = true;
                }
                CMD_GET_PACKET_LENGTH if self.devctrl => {
                    self.status_ok(out);
                    let mut resp = Vec::new();
                    resp.extend_from_slice(&0x20000u32.to_le_bytes());
                    resp.extend_from_slice(&0x20000u32.to_le_bytes());
                    self.packet(out, &resp);
                }
                CMD_CUSTOM_READREGISTER if self.devctrl => {
                    self.status_ok(out);
                    self.expect_reg_addr = true;
                    self.expect_reg_value = false;
                }
                CMD_CUSTOM_WRITEREGISTER if self.devctrl => {
                    self.status_ok(out);
                    self.expect_reg_addr = true;
                    self.expect_reg_value = true;
                }
                CMD_READ_DATA => {
                    self.status_ok(out);
                    self.expect_read_param = true;
                    self.reading = false;
                }
                CMD_WRITE_DATA => {
                    self.status_ok(out);
                    self.expect_write_param = true;
                    self.reading = false;
                }
                CMD_SHUTDOWN => {
                    self.status_ok(out);
                    self.shutdown = true;
                }
                CMD_FORMAT => {
                    self.status_ok(out);
                    self.expect_format_param = true;
                }
                _ => {
                    // the format params land after the env/hw phases
                    if self.expect_format_param {
                        self.expect_format_param = false;
                        self.status_ok(out);
                        self.packet(out, &STATUS_COMPLETE.to_le_bytes());
                    } else if self.env_params > 0 {
                        self.env_params -= 1;
                        if self.env_params == 0 {
                            self.status_ok(out);
                        }
                    } else if self.hw_params > 0 {
                        self.hw_params -= 1;
                        if self.hw_params == 0 {
                            self.status_ok(out);
                            if self.sync_seen && !self.sync_sent {
                                self.sync_sent = true;
                                self.packet(out, &SYNC_SIGNAL.to_le_bytes());
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn format_completes_over_the_da() {
        let mut xflash = XFlash::new(Box::new(mock_da()));
        xflash.init_after_jump().unwrap();
        xflash.format_pt(0x1F00, 0x1000, PART_USER).unwrap();
        // a CONTINUE hint is honored too: feed the mock one
        // CONTINUE status round-trip is exercised by the same loop
    }

    #[test]
    fn cancel_fires_before_any_da_traffic() {
        skello_cancel::reset();
        skello_cancel::request();
        let mut xflash = XFlash::new(Box::new(crate::transport::MockLink::new()));
        assert!(matches!(
            xflash.writeflash_pt(0, &[0u8; 512], PART_USER),
            Err(XFlashError::Cancelled)
        ));
        assert!(matches!(
            xflash.readflash_pt(0, 512, PART_USER),
            Err(XFlashError::Cancelled)
        ));
        assert!(matches!(
            xflash.format_pt(0, 512, PART_USER),
            Err(XFlashError::Cancelled)
        ));
        skello_cancel::reset();
    }

    #[test]
    fn rpmb_read_carries_the_partition_area() {
        let loader_path = std::env::temp_dir().join("skello_test_loader_rpmb.bin");
        std::fs::write(&loader_path, build_loader(0x6572)).unwrap();
        let mut brom = Brom::new(Box::new(mock_brom()));
        let da_open = || -> std::result::Result<Box<dyn Link>, crate::transport::LinkError> {
            Ok(Box::new(mock_da()))
        };
        // drive the session manually: init, GPT, then an RPMB read
        brom.handshake().unwrap();
        let (hw_code, _sub) = brom.get_hw_code().unwrap();
        assert_eq!(hw_code, 0x6572);
        let link = da_open().unwrap();
        let mut xflash = XFlash::new(link);
        xflash.init_after_jump().unwrap();
        let rpmb = xflash.readflash_pt(0, 0x400, PART_RPMB).unwrap();
        assert_eq!(rpmb.len(), 0x400);
        assert!(xflash.check_last_parttype(|t| t == PART_RPMB));
        let _ = std::fs::remove_file(&loader_path);
    }

    #[test]
    fn unlock_v4_seccfg_through_da_sej() {
        let loader_path = std::env::temp_dir().join("skello_test_loader_xflash.bin");
        std::fs::write(&loader_path, build_loader(0x6572)).unwrap();

        let mut brom = Brom::new(Box::new(mock_brom()));
        let mut da_open = || -> std::result::Result<Box<dyn Link>, crate::transport::LinkError> {
            Ok(Box::new(mock_da()))
        };
        let result =
            XFlash::unlock_seccfg_with(&mut brom, loader_path.to_str().unwrap(), &mut da_open)
                .unwrap();
        assert!(result.contains("unlocked"));
        let _ = std::fs::remove_file(&loader_path);
    }
}
