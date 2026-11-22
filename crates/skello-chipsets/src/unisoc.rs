//! Unisoc BSL/FDL protocol: HDLC-style framing with 0x7e delimiters,
//! the 0x7d transcode escaping, CRC16 and sum checksums, FDL1/FDL2
//! download and partition access. Framing, commands and the boot flow
//! follow spreadtrum_flash (public domain), NOTICE has the attribution.

use std::time::Duration;

use crate::transport::Link;

pub const HDLC_HEADER: u8 = 0x7E;
pub const HDLC_ESCAPE: u8 = 0x7D;

pub const BSL_CMD_CONNECT: u16 = 0x00;
pub const BSL_CMD_START_DATA: u16 = 0x01;
pub const BSL_CMD_MIDST_DATA: u16 = 0x02;
pub const BSL_CMD_END_DATA: u16 = 0x03;
pub const BSL_CMD_EXEC_DATA: u16 = 0x04;
pub const BSL_CMD_NORMAL_RESET: u16 = 0x05;
pub const BSL_CMD_READ_FLASH: u16 = 0x06;
pub const BSL_CMD_ERASE_FLASH: u16 = 0x0A;
pub const BSL_CMD_READ_START: u16 = 0x10;
pub const BSL_CMD_READ_MIDST: u16 = 0x11;
pub const BSL_CMD_READ_END: u16 = 0x12;
pub const BSL_CMD_KEEP_CHARGE: u16 = 0x13;
pub const BSL_CMD_DISABLE_TRANSCODE: u16 = 0x21;
pub const BSL_CMD_READ_PARTITION: u16 = 0x2D;
pub const BSL_CMD_CHECK_BAUD: u16 = 0x7E;
pub const BSL_CMD_POWER_OFF: u16 = 0x17;

pub const BSL_REP_ACK: u16 = 0x80;
pub const BSL_REP_VER: u16 = 0x81;
pub const BSL_REP_INVALID_CMD: u16 = 0x82;
pub const BSL_REP_UNKNOW_CMD: u16 = 0x83;
pub const BSL_REP_OPERATION_FAILED: u16 = 0x84;
pub const BSL_REP_READ_FLASH: u16 = 0x93;
/// Reply kind to READ_PARTITION: the partition table dump.
pub const BSL_REP_READ_PARTITION: u16 = 0xBA;

/// One entry of the FDL2 partition table: the name is UTF-16 and the
/// size is a raw sector count (the tools differ on the sector unit,
/// so Skello reports the raw value without guessing).
#[derive(Debug, Clone)]
pub struct UniPartition {
    pub name: String,
    pub size_sectors: u64,
}

/// Parses the READ_PARTITION reply (spd_dump format): a run of 0x4C
/// byte entries, the name UTF-16LE at 0x00 (up to 36 units) and the
/// sector count u32LE at 0x48.
pub fn parse_partition_list(data: &[u8]) -> Result<Vec<UniPartition>> {
    if !data.len().is_multiple_of(0x4C) {
        return Err(BslError::Unexpected("partition list is not a 0x4C run"));
    }
    let mut out = Vec::new();
    for entry in data.chunks(0x4C) {
        let mut name = String::new();
        for unit in entry[..0x48].chunks(2) {
            let v = u16::from_le_bytes([unit[0], unit[1]]);
            if v == 0 {
                break;
            }
            name.push(char::from_u32(v as u32).unwrap_or('\u{FFFD}'));
        }
        let size_sectors = u32::from_le_bytes(entry[0x48..0x4C].try_into().unwrap()) as u64;
        if name.is_empty() && size_sectors == 0 {
            continue;
        }
        out.push(UniPartition { name, size_sectors });
    }
    Ok(out)
}

/// VID of the Unisoc BROM/FDL USB port.
pub const VID_UNISOC: u16 = 0x1782;
pub const PID_BROM: u16 = 0x4D00;

/// FDL1 payload chunks go to SRAM; the address comes from the call
/// site (per-model XML or the known defaults).
pub const FDL_CHUNK: usize = 528;

const BOOT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub enum BslError {
    Io(std::io::Error),
    /// Delimiter or checksum mismatch on a received packet.
    BadFrame,
    /// The stop button was pressed between chunks.
    Cancelled,
    Unexpected(&'static str),
    Reply(u16, &'static str),
}

impl std::fmt::Display for BslError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BslError::Io(e) => write!(f, "bsl io: {e}"),
            BslError::BadFrame => write!(f, "bsl bad frame"),
            BslError::Cancelled => write!(f, "operation cancelled"),
            BslError::Unexpected(s) => write!(f, "bsl unexpected response: {s}"),
            BslError::Reply(t, s) => write!(f, "bsl reply {t:#04x}: {s}"),
        }
    }
}

impl std::error::Error for BslError {}

impl From<std::io::Error> for BslError {
    fn from(e: std::io::Error) -> Self {
        BslError::Io(e)
    }
}

type Result<T> = std::result::Result<T, BslError>;

/// CRC16 with the polynomial 0x11021 (CCITT/XModem), MSB first, init 0.
pub fn crc16(crc: u16, data: &[u8]) -> u16 {
    let mut crc = crc;
    for b in data {
        crc ^= (*b as u16) << 8;
        for _ in 0..8 {
            let carry = crc & 0x8000;
            crc <<= 1;
            if carry != 0 {
                crc ^= 0x1021;
            }
        }
    }
    crc
}

/// The sum checksum FDL1 and FDL2 use: LE u16 words added together
/// with the odd trailing byte doubled into the high position.
pub fn sum16(data: &[u8]) -> u16 {
    let mut crc: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        crc += data[i] as u32 | ((data[i + 1] as u32) << 8);
        crc &= 0xFFFF;
        i += 2;
    }
    if data.len() & 1 != 0 {
        crc += data[data.len() - 1] as u32;
    }
    crc as u16
}

/// Applies the transcode: 0x7e and 0x7d become 0x7d xor 0x20 pairs.
pub fn transcode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for a in data {
        if *a == HDLC_HEADER || *a == HDLC_ESCAPE {
            out.push(HDLC_ESCAPE);
            out.push(a ^ 0x20);
        } else {
            out.push(*a);
        }
    }
    out
}

/// Reverses the transcode on a full packet body (without delimiters).
pub fn untranscode(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len());
    let mut esc = false;
    for a in data {
        if esc {
            if *a != (HDLC_HEADER ^ 0x20) && *a != (HDLC_ESCAPE ^ 0x20) {
                return Err(BslError::BadFrame);
            }
            out.push(a ^ 0x20);
            esc = false;
        } else if *a == HDLC_ESCAPE {
            esc = true;
        } else {
            out.push(*a);
        }
    }
    if esc {
        return Err(BslError::BadFrame);
    }
    Ok(out)
}

#[derive(Debug)]
pub struct BslPacket {
    pub kind: u16,
    pub data: Vec<u8>,
}

/// A BSL session over an open byte link; transcode is on until the
/// device accepts DISABLE_TRANSCODE.
pub struct Bsl {
    link: Box<dyn Link>,
    pub transcode: bool,
    pub crc16: bool,
}

impl Bsl {
    pub fn new(link: Box<dyn Link>, crc16: bool) -> Self {
        Self {
            link,
            transcode: true,
            crc16,
        }
    }

    pub fn encode(&self, kind: u16, data: &[u8]) -> Vec<u8> {
        let mut raw = Vec::with_capacity(4 + data.len() + 2);
        raw.extend_from_slice(&kind.to_be_bytes());
        raw.extend_from_slice(&(data.len() as u16).to_be_bytes());
        raw.extend_from_slice(data);
        let chk = if self.crc16 {
            crc16(0, &raw)
        } else {
            sum16(&raw)
        };
        raw.extend_from_slice(&chk.to_be_bytes());
        let mut out = vec![HDLC_HEADER];
        if self.transcode {
            out.extend_from_slice(&transcode(&raw));
        } else {
            out.extend_from_slice(&raw);
        }
        out.push(HDLC_HEADER);
        out
    }

    /// Reads one packet: delimiters, transcode unfolding, checksum
    /// validation. Extra leading 0x7e bytes (the CHECK_BAUD wake
    /// pattern) are skipped.
    pub fn recv(&mut self) -> Result<BslPacket> {
        let mut raw = Vec::new();
        loop {
            let mut b = [0u8; 1];
            read_byte(&mut *self.link, &mut b, BOOT_TIMEOUT)?;
            if b[0] == HDLC_HEADER {
                if raw.is_empty() {
                    continue;
                }
                break;
            }
            raw.push(b[0]);
        }
        let body = if self.transcode {
            untranscode(&raw)?
        } else {
            raw
        };
        if body.len() < 6 {
            return Err(BslError::BadFrame);
        }
        let (payload, tail) = body.split_at(body.len() - 2);
        let chk = u16::from_be_bytes([tail[0], tail[1]]);
        let expect = if self.crc16 {
            crc16(0, payload)
        } else {
            sum16(payload)
        };
        if chk != expect {
            return Err(BslError::BadFrame);
        }
        let kind = u16::from_be_bytes([payload[0], payload[1]]);
        let len = u16::from_be_bytes([payload[2], payload[3]]) as usize;
        if payload.len() != len + 4 {
            return Err(BslError::BadFrame);
        }
        Ok(BslPacket {
            kind,
            data: payload[4..].to_vec(),
        })
    }

    fn send(&mut self, kind: u16, data: &[u8]) -> Result<()> {
        let pkt = self.encode(kind, data);
        self.link.write_all(&pkt)?;
        Ok(())
    }

    fn send_and_check(&mut self, kind: u16, data: &[u8]) -> Result<BslPacket> {
        self.send(kind, data)?;
        let pkt = self.recv()?;
        if pkt.kind == BSL_REP_ACK {
            return Ok(pkt);
        }
        Err(BslError::Reply(pkt.kind, "command rejected"))
    }

    /// Wakes the BROM: CHECK_BAUD is a run of raw 0x7e bytes; the
    /// device answers BSL_REP_VER with a version string.
    pub fn wake(&mut self, header_bytes: usize) -> Result<String> {
        let wake = vec![HDLC_HEADER; header_bytes];
        self.link.write_all(&wake)?;
        let pkt = self.recv()?;
        if pkt.kind != BSL_REP_VER {
            return Err(BslError::Unexpected("ver expected after wake"));
        }
        Ok(String::from_utf8_lossy(&pkt.data).to_string())
    }

    pub fn connect(&mut self) -> Result<()> {
        self.send_and_check(BSL_CMD_CONNECT, &[]).map(|_| ())
    }

    /// Downloads one FDL image: START_DATA(addr, size), MIDST chunks,
    /// END_DATA, EXEC_DATA.
    pub fn send_fdl(&mut self, addr: u32, image: &[u8]) -> Result<()> {
        // FDL load takes addr and size big-endian (spd_dump send_buf);
        // the partition commands below stay little-endian.
        let mut head = Vec::new();
        head.extend_from_slice(&addr.to_be_bytes());
        head.extend_from_slice(&(image.len() as u32).to_be_bytes());
        self.send_and_check(BSL_CMD_START_DATA, &head)?;
        for chunk in image.chunks(FDL_CHUNK) {
            self.send_and_check(BSL_CMD_MIDST_DATA, chunk)?;
        }
        self.send_and_check(BSL_CMD_END_DATA, &[])?;
        self.send_and_check(BSL_CMD_EXEC_DATA, &[])?;
        Ok(())
    }

    /// Full boot: wake BROM, connect, FDL1 to SRAM, then re-wake FDL1
    /// (sum checksum from there on), connect, FDL2.
    pub fn boot(
        &mut self,
        fdl1: &[u8],
        fdl1_addr: u32,
        fdl2: &[u8],
        fdl2_addr: u32,
    ) -> Result<String> {
        let ver = self.wake(1)?;
        self.connect()?;
        self.send_fdl(fdl1_addr, fdl1)?;
        // FDL1 answers with the sum checksum from here on.
        self.crc16 = false;
        let ver2 = self.wake(4)?;
        self.connect()?;
        self.send_fdl(fdl2_addr, fdl2)?;
        Ok(format!("{ver} | {ver2}"))
    }

    /// Flashes a whole .pac firmware: boots the FDL pair from the
    /// package itself, writes every file entry marked for flashing
    /// under its partition name, then resets the device. The CRC
    /// checks run before anything touches the flash.
    pub fn flash_pac(&mut self, data: &[u8]) -> Result<String> {
        use crate::unisoc_pac::{extract_entry, parse_pac};
        let fw = parse_pac(data).map_err(|e| BslError::Io(std::io::Error::other(e.to_string())))?;
        if !fw.head_crc_ok {
            return Err(BslError::Unexpected("package header crc mismatch"));
        }
        if fw.data_crc_ok == Some(false) {
            return Err(BslError::Unexpected("package data crc mismatch"));
        }
        let (fdl1, fdl2) = fw
            .fdl_pair()
            .ok_or(BslError::Unexpected("no fdl pair in package"))?;
        let fdl1_data = extract_entry(data, fdl1)
            .map_err(|e| BslError::Io(std::io::Error::other(e.to_string())))?;
        let fdl2_data = extract_entry(data, fdl2)
            .map_err(|e| BslError::Io(std::io::Error::other(e.to_string())))?;
        let ver = self.boot(fdl1_data, fdl1.addr[0], fdl2_data, fdl2.addr[0])?;
        let mut written = Vec::new();
        for entry in fw.flash_files() {
            let blob = extract_entry(data, entry)
                .map_err(|e| BslError::Io(std::io::Error::other(e.to_string())))?;
            self.write_partition(&entry.name, blob)?;
            written.push(entry.name.clone());
        }
        let _ = self.reset();
        Ok(format!(
            "{ver}; flashed {}: {}",
            written.len(),
            written.join(", ")
        ))
    }

    /// Writes `name` = UTF-16, size, then the data blocks.
    pub fn write_partition(&mut self, name: &str, data: &[u8]) -> Result<()> {
        if skello_cancel::is_requested() {
            return Err(BslError::Cancelled);
        }
        let pkt = partition_request(name, data.len() as u64, data.len() > u32::MAX as usize);
        self.send_and_check(BSL_CMD_START_DATA, &pkt)?;
        for chunk in data.chunks(0x800) {
            if skello_cancel::is_requested() {
                return Err(BslError::Cancelled);
            }
            self.send_and_check(BSL_CMD_MIDST_DATA, chunk)?;
        }
        self.send_and_check(BSL_CMD_END_DATA, &[])?;
        Ok(())
    }

    /// Reads a partition by name via READ_START/READ_MIDST/READ_END.
    pub fn read_partition(&mut self, name: &str, size: u64) -> Result<Vec<u8>> {
        if skello_cancel::is_requested() {
            return Err(BslError::Cancelled);
        }
        let pkt = partition_request(name, size, size > u32::MAX as u64);
        self.send_and_check(BSL_CMD_READ_START, &pkt)?;
        let mut out = Vec::new();
        let mut iters = 0u32;
        loop {
            if skello_cancel::is_requested() {
                return Err(BslError::Cancelled);
            }
            // Each MIDST asks for the next chunk explicitly; an
            // all-zero request makes some FDL2 answer empty frames
            // forever.
            let want = size.saturating_sub(out.len() as u64).min(0x800) as u32;
            let mut req = Vec::with_capacity(8);
            req.extend_from_slice(&want.to_le_bytes());
            req.extend_from_slice(&(out.len() as u32).to_le_bytes());
            self.send(BSL_CMD_READ_MIDST, &req)?;
            let pkt = self.recv()?;
            if pkt.kind == BSL_REP_READ_FLASH {
                if pkt.data.is_empty() {
                    iters += 1;
                    if iters > 65536 {
                        return Err(BslError::Reply(pkt.kind, "read made no progress"));
                    }
                } else {
                    iters = 0;
                }
                out.extend_from_slice(&pkt.data);
                if out.len() as u64 >= size {
                    break;
                }
            } else if pkt.kind == BSL_REP_ACK {
                break;
            } else {
                return Err(BslError::Reply(pkt.kind, "read midst"));
            }
        }
        self.send_and_check(BSL_CMD_READ_END, &[])?;
        Ok(out)
    }

    /// Asks for the partition table (READ_PARTITION reply, 0xBA).
    pub fn read_partition_list(&mut self) -> Result<Vec<u8>> {
        self.send(BSL_CMD_READ_PARTITION, &[])?;
        let pkt = self.recv()?;
        if pkt.kind != BSL_REP_READ_PARTITION {
            return Err(BslError::Reply(pkt.kind, "read partition"));
        }
        Ok(pkt.data)
    }

    /// Boot + partition table in one call: the FDL session comes up,
    /// the transcode goes off and the table is read back.
    pub fn partition_table(
        &mut self,
        fdl1: &[u8],
        fdl1_addr: u32,
        fdl2: &[u8],
        fdl2_addr: u32,
    ) -> Result<Vec<UniPartition>> {
        self.boot(fdl1, fdl1_addr, fdl2, fdl2_addr)?;
        self.disable_transcode()?;
        let raw = self.read_partition_list()?;
        parse_partition_list(&raw)
    }

    pub fn erase_partition(&mut self, name: &str, size: u64) -> Result<()> {
        let pkt = partition_request(name, size, size > u32::MAX as u64);
        self.send_and_check(BSL_CMD_ERASE_FLASH, &pkt)?;
        Ok(())
    }

    /// Disables the transcode; the link stops escaping after this.
    pub fn disable_transcode(&mut self) -> Result<()> {
        self.send_and_check(BSL_CMD_DISABLE_TRANSCODE, &[])?;
        self.transcode = false;
        Ok(())
    }

    pub fn keep_charge(&mut self, enable: u32) -> Result<()> {
        self.send_and_check(BSL_CMD_KEEP_CHARGE, &enable.to_le_bytes())
            .map(|_| ())
    }

    pub fn power_off(&mut self) -> Result<()> {
        self.send_and_check(BSL_CMD_POWER_OFF, &[]).map(|_| ())
    }

    /// Reboots the device into the normal mode.
    pub fn reset(&mut self) -> Result<()> {
        self.send_and_check(BSL_CMD_NORMAL_RESET, &[]).map(|_| ())
    }
}

fn read_byte(link: &mut dyn Link, buf: &mut [u8; 1], timeout: Duration) -> Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        if left.is_zero() {
            return Err(BslError::Io(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "bsl read timeout",
            )));
        }
        let n = link.read(buf, left)?;
        if n == 1 {
            return Ok(());
        }
    }
}

/// Partition request packet: name in UTF-16 (36 units), u32 size,
/// u32 size_hi, u64 dummy.
fn partition_request(name: &str, size: u64, mode64: bool) -> Vec<u8> {
    let mut pkt = vec![0u8; 36 * 2 + 4 + 4 + 8];
    for (i, unit) in name.chars().take(35).enumerate() {
        pkt[i * 2..i * 2 + 2].copy_from_slice(&(unit as u16).to_le_bytes());
    }
    pkt[72..76].copy_from_slice(&(size as u32).to_le_bytes());
    if mode64 {
        pkt[76..80].copy_from_slice(&((size >> 32) as u32).to_le_bytes());
    }
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockLink;
    use std::cell::RefCell;

    /// Builds one 0x4C partition-table entry: UTF-16 name, size at 0x48.
    fn uni_entry(name: &str, size_sectors: u32) -> Vec<u8> {
        let mut e = vec![0u8; 0x4C];
        for (i, c) in name.encode_utf16().enumerate() {
            e[i * 2..i * 2 + 2].copy_from_slice(&c.to_le_bytes());
        }
        e[0x48..0x4C].copy_from_slice(&size_sectors.to_le_bytes());
        e
    }

    #[test]
    fn parses_partition_list() {
        let mut raw = uni_entry("prod", 4_194_304);
        raw.extend(uni_entry("misc", 2_048));
        let parts = parse_partition_list(&raw).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].name, "prod");
        assert_eq!(parts[0].size_sectors, 4_194_304);
        assert_eq!(parts[1].name, "misc");
        // trailing zero entries are dropped
        raw.extend(vec![0u8; 0x4C]);
        assert_eq!(parse_partition_list(&raw).unwrap().len(), 2);
        assert!(parse_partition_list(&[0u8; 10]).is_err());
    }

    #[test]
    fn partition_table_over_fdl() {
        let mut bsl = Bsl::new(Box::new(mock_bsl()), true);
        let table = bsl
            .partition_table(&[0u8; 16], 0x40000, &[0u8; 16], 0x50000000)
            .unwrap();
        assert_eq!(table.len(), 2);
        assert_eq!(table[0].name, "prod");
        // the transcode is off by the time the table is read
        assert!(!bsl.transcode);
    }

    #[test]
    fn transcode_roundtrip() {
        let data = vec![0x7e, 0x11, 0x7d, 0x22, 0x7e];
        let enc = transcode(&data);
        assert_eq!(enc, vec![0x7d, 0x5e, 0x11, 0x7d, 0x5d, 0x22, 0x7d, 0x5e]);
        let dec = untranscode(&enc).unwrap();
        assert_eq!(dec, data);
        assert!(untranscode(&[0x7d, 0x41]).is_err());
    }

    #[test]
    fn crc16_reference() {
        // spreadtrum_flash spd_crc16(0, "123456789", 9) == 0x31c3
        assert_eq!(crc16(0, b"123456789"), 0x31C3);
    }

    #[test]
    fn sum16_reference() {
        // two LE words 0x0201 + 0x0403
        assert_eq!(sum16(&[1, 2, 3, 4]), 0x0604);
        assert_eq!(sum16(&[1, 2, 3, 4, 5]), 0x0604 + 5);
    }

    /// Device side: parses one host packet and queues an ACK/VER.
    /// Starts in CRC16 mode (BROM), switches to the sum checksum
    /// after EXEC_DATA launches FDL1, tracks the transcode mode.
    fn mock_bsl() -> MockLink {
        let mut link = MockLink::new();
        let state = RefCell::new((true, true, Vec::<u8>::new()));
        link.device = Some(Box::new(move |out, input| {
            let mut st = state.borrow_mut();
            let (crc16_mode, transcode_mode, acc) = &mut *st;
            acc.extend_from_slice(input);
            loop {
                // a short run of raw delimiters is the CHECK_BAUD wake
                if !acc.is_empty() && acc.iter().all(|b| *b == HDLC_HEADER) && acc.len() < 8 {
                    out.extend(encode_device_packet(
                        BSL_REP_VER,
                        b"FDL1 SP ResetReason=0 CHIP ID = 0x98180000",
                        *crc16_mode,
                        *transcode_mode,
                    ));
                    acc.clear();
                    break;
                }
                if acc.is_empty() || acc[0] != HDLC_HEADER {
                    break;
                }
                let Some(end) = find_frame_end(acc) else {
                    break;
                };
                let frame: Vec<u8> = acc.drain(..end).collect();
                let raw = &frame[1..frame.len() - 1];
                let body: Vec<u8> = if *transcode_mode {
                    untranscode(raw).unwrap_or_default()
                } else {
                    raw.to_vec()
                };
                if body.len() < 6 {
                    continue;
                }
                let kind = u16::from_be_bytes([body[0], body[1]]);
                let expect = if *crc16_mode {
                    crc16(0, &body[..body.len() - 2])
                } else {
                    sum16(&body[..body.len() - 2])
                };
                let chk = u16::from_be_bytes([body[body.len() - 2], body[body.len() - 1]]);
                if chk != expect {
                    continue;
                }
                if kind == BSL_CMD_DISABLE_TRANSCODE {
                    *transcode_mode = false;
                }
                let reply_kind = match kind {
                    BSL_CMD_READ_PARTITION => BSL_REP_READ_PARTITION,
                    BSL_CMD_READ_MIDST => BSL_REP_READ_FLASH,
                    _ => BSL_REP_ACK,
                };
                let data: Vec<u8> = if kind == BSL_CMD_READ_PARTITION {
                    uni_entry("prod", 4_194_304)
                        .into_iter()
                        .chain(uni_entry("misc", 2_048))
                        .collect()
                } else if kind == BSL_CMD_READ_MIDST {
                    vec![0xAB; 16]
                } else {
                    Vec::new()
                };
                let reply = encode_device_packet(reply_kind, &data, *crc16_mode, *transcode_mode);
                out.extend(reply);
                // after EXEC_DATA launches FDL1 the checksum mode changes
                if kind == BSL_CMD_EXEC_DATA {
                    *crc16_mode = false;
                }
            }
        }));
        link
    }

    fn find_frame_end(acc: &[u8]) -> Option<usize> {
        // look for the closing delimiter after at least 2 bytes
        (2..acc.len())
            .find(|i| acc[*i] == HDLC_HEADER)
            .map(|i| i + 1)
    }

    fn encode_device_packet(
        kind: u16,
        data: &[u8],
        crc16_mode: bool,
        transcode_mode: bool,
    ) -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(&kind.to_be_bytes());
        raw.extend_from_slice(&(data.len() as u16).to_be_bytes());
        raw.extend_from_slice(data);
        let chk = if crc16_mode {
            crc16(0, &raw)
        } else {
            sum16(&raw)
        };
        raw.extend_from_slice(&chk.to_be_bytes());
        let mut out = vec![HDLC_HEADER];
        if transcode_mode {
            out.extend(transcode(&raw));
        } else {
            out.extend(&raw);
        }
        out.push(HDLC_HEADER);
        out
    }

    #[test]
    fn wake_connect_and_fdl1_roundtrip() {
        let mock = mock_bsl();
        let mut bsl = Bsl::new(Box::new(mock), true);
        let ver = bsl.wake(1).unwrap();
        assert!(ver.contains("FDL1"));
        bsl.connect().unwrap();
        let image: Vec<u8> = (0..600).map(|i| (i % 251) as u8).collect();
        bsl.send_fdl(0x00090000, &image).unwrap();
    }

    #[test]
    fn full_boot_and_partition_ops() {
        let mock = mock_bsl();
        let mut bsl = Bsl::new(Box::new(mock), true);
        let fdl1: Vec<u8> = vec![0x11; 400];
        let fdl2: Vec<u8> = vec![0x22; 1200];
        let ver = bsl.boot(&fdl1, 0x00090000, &fdl2, 0x50000000).unwrap();
        assert!(ver.contains("FDL1"));
        bsl.disable_transcode().unwrap();
        bsl.write_partition("seccfg", &vec![0x5A; 1024]).unwrap();
        let data = bsl.read_partition("seccfg", 16).unwrap();
        assert_eq!(data, vec![0xAB; 16]);
        bsl.erase_partition("seccfg", 1024).unwrap();
        bsl.keep_charge(0).unwrap();
        bsl.reset().unwrap();
    }
}

/// The community-published bootloader signing key for T618-class
/// devices (unisoc-unlock, see NOTICE): data, not code.
pub const UNISOC_T618_KEY: &str = include_str!("unisoc_t618_key.pem");

#[derive(Debug)]
pub enum TokenError {
    /// The identifier token is not a hex string.
    BadToken,
    /// The key material could not be loaded.
    Key,
    #[allow(dead_code)]
    Io(std::io::Error),
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::BadToken => write!(f, "identifier token is not a hex string"),
            TokenError::Key => write!(f, "signing key could not be loaded"),
            TokenError::Io(e) => write!(f, "token io: {e}"),
        }
    }
}

impl std::error::Error for TokenError {}

/// Signs the Unisoc bootloader identifier token: the hex token is
/// zero-padded to 64 bytes and signed with RSA-4096 PKCS#1 v1.5 over
/// SHA-256 (the unisoc-unlock procedure). Returns the signature the
/// host sends through `fastboot download` before
/// `flashing unlock_bootloader`.
pub fn sign_identifier_token(
    token_hex: &str,
    key_pem: &str,
) -> std::result::Result<Vec<u8>, TokenError> {
    let token = token_hex.trim();
    if token.is_empty() || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(TokenError::BadToken);
    }
    let mut padded = token.to_ascii_lowercase();
    while padded.len() < 128 {
        padded.push('0');
    }
    padded.truncate(128);
    let raw = hex::decode(&padded).map_err(|_| TokenError::BadToken)?;
    sign_token_bytes(&raw, key_pem)
}

/// Signs the already-decoded token bytes.
pub fn sign_token_bytes(raw: &[u8], key_pem: &str) -> std::result::Result<Vec<u8>, TokenError> {
    use rsa::pkcs1::DecodeRsaPrivateKey;
    use rsa::pkcs1v15::SigningKey;
    use rsa::signature::{SignatureEncoding, Signer};
    let key = rsa::RsaPrivateKey::from_pkcs1_pem(key_pem).map_err(|_| TokenError::Key)?;
    let signing = SigningKey::<sha2::Sha256>::new(key);
    Ok(signing.sign(raw).to_vec())
}

#[cfg(test)]
mod token_tests {
    use super::*;

    #[test]
    fn token_signature_verifies() {
        let token = "0123456789abcdef0123456789abcdef";
        let signature = sign_identifier_token(token, UNISOC_T618_KEY).unwrap();
        // RSA-4096: the signature is 512 bytes
        assert_eq!(signature.len(), 512);

        use rsa::pkcs1::DecodeRsaPrivateKey;
        use rsa::pkcs1v15::{Signature, VerifyingKey};
        use rsa::signature::Verifier;
        let key = rsa::RsaPrivateKey::from_pkcs1_pem(UNISOC_T618_KEY).unwrap();
        let verifying = VerifyingKey::<sha2::Sha256>::new(key.into());
        // the verification input is the padded token
        let mut padded = token.to_ascii_lowercase();
        while padded.len() < 128 {
            padded.push('0');
        }
        let raw = hex::decode(&padded).unwrap();
        verifying
            .verify(&raw, &Signature::try_from(signature.as_slice()).unwrap())
            .unwrap();
    }

    #[test]
    fn rejects_non_hex_tokens() {
        assert!(matches!(
            sign_identifier_token("zzzz", UNISOC_T618_KEY),
            Err(TokenError::BadToken)
        ));
        assert!(matches!(
            sign_identifier_token("", UNISOC_T618_KEY),
            Err(TokenError::BadToken)
        ));
    }
}
