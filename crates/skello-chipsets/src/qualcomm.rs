//! Qualcomm EDL: Sahara (programmer download over USB bulk) and
//! Firehose (XML command protocol on top of the running programmer).
//! Packet layouts and the XML command set follow qdlrs (BSD-3-Clause),
//! NOTICE has the attribution.

use std::time::Duration;

use crate::transport::Link;

/// EDL USB ids.
pub const VID_QUALCOMM: u16 = 0x05C6;
pub const PID_EDL: u16 = 0x9008;

pub const SAHARA_HELLO: u32 = 0x1;
pub const SAHARA_HELLO_RESP: u32 = 0x2;
pub const SAHARA_READ_DATA: u32 = 0x3;
pub const SAHARA_END_OF_IMAGE: u32 = 0x4;
pub const SAHARA_DONE: u32 = 0x5;
pub const SAHARA_DONE_RESP: u32 = 0x6;
pub const SAHARA_RESET: u32 = 0x7;
pub const SAHARA_COMMAND_READY: u32 = 0xB;
pub const SAHARA_SWITCH_MODE: u32 = 0xC;
pub const SAHARA_EXEC: u32 = 0xD;
pub const SAHARA_EXEC_RESP: u32 = 0xE;

pub const SAHARA_MODE_IMAGE_TX_PENDING: u32 = 0x0;
pub const SAHARA_MODE_COMMAND: u32 = 0x3;

const SAHARA_VERSION: u32 = 2;
const IO_TIMEOUT: Duration = Duration::from_secs(10);
const BOOT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub enum EdlError {
    Io(std::io::Error),
    BadPacket,
    Status(u32, &'static str),
    Nack(String),
    Timeout(&'static str),
}

impl std::fmt::Display for EdlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdlError::Io(e) => write!(f, "edl io: {e}"),
            EdlError::BadPacket => write!(f, "edl bad packet"),
            EdlError::Status(s, what) => write!(f, "edl status {s:#010x} in {what}"),
            EdlError::Nack(what) => write!(f, "edl NAK in {what}"),
            EdlError::Timeout(what) => write!(f, "edl timeout in {what}"),
        }
    }
}

impl std::error::Error for EdlError {}

impl From<std::io::Error> for EdlError {
    fn from(e: std::io::Error) -> Self {
        EdlError::Io(e)
    }
}

impl From<crate::transport::LinkError> for EdlError {
    fn from(e: crate::transport::LinkError) -> Self {
        EdlError::Io(std::io::Error::other(e.to_string()))
    }
}

type Result<T> = std::result::Result<T, EdlError>;

fn read_exact(link: &mut dyn Link, buf: &mut [u8], timeout: Duration) -> Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    let mut done = 0;
    while done < buf.len() {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        if left.is_zero() {
            return Err(EdlError::Timeout("read"));
        }
        let n = link.read(&mut buf[done..], left)?;
        if n == 0 {
            return Err(EdlError::Timeout("link closed"));
        }
        done += n;
    }
    Ok(())
}

/// One Sahara packet: u32 command, u32 length, body.
#[derive(Debug, Clone)]
pub struct SaharaPacket {
    pub cmd: u32,
    pub body: Vec<u8>,
}

impl SaharaPacket {
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.body.len());
        out.extend_from_slice(&self.cmd.to_le_bytes());
        out.extend_from_slice(&(self.body.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.body);
        out
    }
}

/// A Sahara session: programmer download, then command-mode switch.
pub struct Sahara {
    link: Box<dyn Link>,
}

impl Sahara {
    pub fn new(link: Box<dyn Link>) -> Self {
        Self { link }
    }

    fn write(&mut self, pkt: &SaharaPacket) -> Result<()> {
        self.link.write_all(&pkt.encode())?;
        Ok(())
    }

    fn read(&mut self, timeout: Duration) -> Result<SaharaPacket> {
        let mut head = [0u8; 8];
        read_exact(&mut *self.link, &mut head, timeout)?;
        let cmd = u32::from_le_bytes(head[0..4].try_into().unwrap());
        let len = u32::from_le_bytes(head[4..8].try_into().unwrap()) as usize;
        let mut body = vec![0u8; len];
        read_exact(&mut *self.link, &mut body, timeout)?;
        Ok(SaharaPacket { cmd, body })
    }

    /// Answers the HELLO and reports the mode to work in.
    fn hello(&mut self, mode: u32) -> Result<()> {
        let pkt = self.read(BOOT_TIMEOUT)?;
        if pkt.cmd != SAHARA_HELLO || pkt.body.len() < 12 {
            return Err(EdlError::BadPacket);
        }
        let ver = u32::from_le_bytes(pkt.body[0..4].try_into().unwrap());
        let compatible = u32::from_le_bytes(pkt.body[4..8].try_into().unwrap());
        let device_mode = u32::from_le_bytes(pkt.body[8..12].try_into().unwrap());
        let _ = device_mode;
        let _ = SAHARA_VERSION;
        let mut resp = Vec::with_capacity(48);
        resp.extend_from_slice(&ver.to_le_bytes());
        resp.extend_from_slice(&compatible.to_le_bytes());
        resp.extend_from_slice(&0u32.to_le_bytes()); // status: success
        resp.extend_from_slice(&mode.to_le_bytes());
        resp.extend_from_slice(&[0u8; 24]); // six reserved words
        self.write(&SaharaPacket {
            cmd: SAHARA_HELLO_RESP,
            body: resp,
        })
    }

    /// Serves READ_DATA requests out of the programmer image(s).
    pub fn send_image(&mut self, mode: u32, images: &[Vec<u8>]) -> Result<()> {
        self.hello(mode)?;
        loop {
            let pkt = self.read(BOOT_TIMEOUT)?;
            match pkt.cmd {
                SAHARA_READ_DATA | 0x12 => {
                    let is64 = pkt.cmd == 0x12;
                    let (image, offset, length) = if is64 && pkt.body.len() >= 24 {
                        (
                            u64::from_le_bytes(pkt.body[0..8].try_into().unwrap()) as usize,
                            u64::from_le_bytes(pkt.body[8..16].try_into().unwrap()) as usize,
                            u64::from_le_bytes(pkt.body[16..24].try_into().unwrap()) as usize,
                        )
                    } else if pkt.body.len() >= 12 {
                        (
                            u32::from_le_bytes(pkt.body[0..4].try_into().unwrap()) as usize,
                            u32::from_le_bytes(pkt.body[4..8].try_into().unwrap()) as usize,
                            u32::from_le_bytes(pkt.body[8..12].try_into().unwrap()) as usize,
                        )
                    } else {
                        return Err(EdlError::BadPacket);
                    };
                    let Some(img) = images.get(image) else {
                        return Err(EdlError::Status(image as u32, "image id"));
                    };
                    let end = (offset + length).min(img.len());
                    if offset > end {
                        return Err(EdlError::BadPacket);
                    }
                    self.link.write_all(&img[offset..end])?;
                }
                SAHARA_END_OF_IMAGE => {
                    // body: image id + status; a zero status continues
                    let status = if pkt.body.len() >= 8 {
                        u32::from_le_bytes(pkt.body[4..8].try_into().unwrap())
                    } else {
                        0
                    };
                    if status != 0 {
                        return Err(EdlError::Status(status, "end of image"));
                    }
                }
                SAHARA_DONE => {
                    self.write(&SaharaPacket {
                        cmd: SAHARA_DONE_RESP,
                        body: 0u32.to_le_bytes().to_vec(),
                    })?;
                    return Ok(());
                }
                SAHARA_COMMAND_READY => return Ok(()),
                SAHARA_RESET => return Ok(()),
                _ => return Err(EdlError::BadPacket),
            }
        }
    }

    /// Switches the running programmer into Firehose command mode.
    pub fn switch_to_command_mode(&mut self) -> Result<()> {
        self.hello(SAHARA_MODE_IMAGE_TX_PENDING)?;
        let mut body = Vec::new();
        body.extend_from_slice(&SAHARA_MODE_COMMAND.to_le_bytes());
        self.write(&SaharaPacket {
            cmd: SAHARA_SWITCH_MODE,
            body,
        })?;
        let pkt = self.read(BOOT_TIMEOUT)?;
        if pkt.cmd != SAHARA_COMMAND_READY {
            return Err(EdlError::Status(pkt.cmd, "switch mode"));
        }
        Ok(())
    }

    /// Full boot: serve the programmer, enter command mode.
    pub fn boot_programmer(images: &[Vec<u8>]) -> Result<()> {
        let mut sahara = Sahara::new(crate::transport::open_usb_serial(VID_QUALCOMM, PID_EDL)?);
        sahara.send_image(SAHARA_MODE_IMAGE_TX_PENDING, images)?;
        sahara.switch_to_command_mode()
    }
}

/// A Firehose session: XML commands over the running programmer.
pub struct Firehose {
    link: Box<dyn Link>,
    pub sector_size: usize,
    pub send_buffer_size: usize,
    /// Bytes read past the end of the last XML response: raw data that
    /// arrived together with the reply and belongs to a read.
    pending: Vec<u8>,
}

impl Firehose {
    pub fn new(link: Box<dyn Link>) -> Self {
        Self {
            link,
            sector_size: 512,
            send_buffer_size: 0x10000,
            pending: Vec::new(),
        }
    }

    /// Drains the pending buffer into `out`, pulling from the link for
    /// the rest, until `want` bytes are collected. A `<data` marker
    /// means the next XML response started: it and everything after
    /// stays pending for `read_response`.
    fn fill(&mut self, out: &mut Vec<u8>, want: usize) -> Result<()> {
        while out.len() < want {
            if !self.pending.is_empty() {
                let take = (want - out.len()).min(self.pending.len());
                out.extend(self.pending.drain(..take));
                continue;
            }
            let mut chunk = [0u8; 4096];
            let n = self.link.read(&mut chunk, IO_TIMEOUT)?;
            if n == 0 {
                return Err(EdlError::Timeout("read data"));
            }
            let xml_start = chunk[..n].windows(5).position(|w| w == b"<data");
            if let Some(pos) = xml_start {
                out.extend_from_slice(&chunk[..pos]);
                self.pending.extend(chunk[pos..n].to_vec());
                continue;
            }
            out.extend_from_slice(&chunk[..n]);
        }
        Ok(())
    }

    fn xml_encode(op: &str, kvps: &[(&str, &str)]) -> Vec<u8> {
        let mut inner = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" ?><data><{}", op);
        for (k, v) in kvps {
            inner.push_str(&format!(" {k}=\"{v}\""));
        }
        inner.push_str("/></data>");
        let mut out = inner.into_bytes();
        out.push(0);
        out
    }

    /// Writes one XML command and collects the response until the
    /// end-of-data marker, then returns the payload between markers.
    fn xml_command(&mut self, op: &str, kvps: &[(&str, &str)]) -> Result<String> {
        let cmd = Self::xml_encode(op, kvps);
        self.link.write_all(&cmd)?;
        self.read_response(op)
    }

    fn read_response(&mut self, what: &str) -> Result<String> {
        // Firehose replies may be preceded by raw binary data (reads),
        // so the search runs over bytes and only the XML part is
        // converted to text.
        let mut buf = std::mem::take(&mut self.pending);
        let mut chunk = [0u8; 1024];
        let deadline = std::time::Instant::now() + BOOT_TIMEOUT;
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                return Err(EdlError::Timeout("xml response"));
            }
            let n = self.link.read(&mut chunk, left)?;
            buf.extend_from_slice(&chunk[..n]);
            let marker_start = b"<data";
            let marker_end = b"</data>";
            let xml_start = buf
                .windows(marker_start.len())
                .position(|w| w == marker_start);
            if let Some(xml_start) = xml_start {
                let rest = &buf[xml_start..];
                if let Some(end_off) = rest.windows(marker_end.len()).position(|w| w == marker_end)
                {
                    let mut xml_end = xml_start + end_off + marker_end.len();
                    // Firehose answers end with a NUL terminator: it
                    // belongs to the XML, not to the read data.
                    if buf.get(xml_end) == Some(&0) {
                        xml_end += 1;
                    }
                    // bytes after the XML stay pending (raw read data)
                    self.pending.extend(buf[xml_end..].to_vec());
                    let xml = String::from_utf8_lossy(&buf[xml_start..xml_end]).into_owned();
                    if xml.contains("value=\"NAK\"") {
                        return Err(EdlError::Nack(what.to_owned()));
                    }
                    return Ok(xml.to_string());
                }
            }
            if n == 0 {
                return Err(EdlError::Timeout("xml response empty read"));
            }
        }
    }

    /// Sends the configure command; the programmer reports its buffer
    /// sizes, which Skello accepts as given.
    pub fn configure(&mut self, storage: &str) -> Result<()> {
        let resp = self.xml_command(
            "configure",
            &[
                ("MemoryName", storage),
                ("ZLPAwareHost", "1"),
                ("SkipStorageInit", "0"),
            ],
        )?;
        if resp.contains("value=\"ACK\"") {
            Ok(())
        } else {
            Err(EdlError::Nack("configure".to_owned()))
        }
    }

    pub fn nop(&mut self) -> Result<()> {
        let resp = self.xml_command("nop", &[("MaxPacketSize", "4096")])?;
        if resp.contains("value=\"ACK\"") {
            Ok(())
        } else {
            Err(EdlError::Nack("nop".to_owned()))
        }
    }

    /// Programs `sectors` sectors from `data` at `start_sector`.
    pub fn program(&mut self, phys_part: u8, start_sector: &str, data: &[u8]) -> Result<()> {
        let sectors = data.len().div_ceil(self.sector_size);
        let resp = self.xml_command(
            "program",
            &[
                ("SECTOR_SIZE_IN_BYTES", &self.sector_size.to_string()),
                ("num_partition_sectors", &sectors.to_string()),
                ("physical_partition_number", &phys_part.to_string()),
                ("start_sector", start_sector),
            ],
        )?;
        if !resp.contains("value=\"ACK\"") {
            return Err(EdlError::Nack("program".to_owned()));
        }
        for chunk in data.chunks(self.send_buffer_size) {
            self.link.write_all(chunk)?;
        }
        // Zero length packet ends the stream.
        let _ = self.link.write_all(&[]);
        let final_resp = self.read_response("program end")?;
        if !final_resp.contains("value=\"ACK\"") {
            return Err(EdlError::Nack("program end".to_owned()));
        }
        Ok(())
    }

    /// Reads `sectors` sectors at `start_sector` into a Vec: the
    /// programmer answers ACK, then raw sectors, then the final XML.
    pub fn read(&mut self, phys_part: u8, start_sector: &str, sectors: usize) -> Result<Vec<u8>> {
        let resp = self.xml_command(
            "read",
            &[
                ("SECTOR_SIZE_IN_BYTES", &self.sector_size.to_string()),
                ("num_partition_sectors", &sectors.to_string()),
                ("physical_partition_number", &phys_part.to_string()),
                ("start_sector", start_sector),
            ],
        )?;
        if !resp.contains("value=\"ACK\"") {
            return Err(EdlError::Nack("read".to_owned()));
        }
        let want = sectors * self.sector_size;
        let mut out = Vec::with_capacity(want);
        self.fill(&mut out, want)?;
        out.truncate(want);
        let _ = self.read_response("read end")?;
        Ok(out)
    }

    /// Reboots the device.
    pub fn reset(&mut self) -> Result<()> {
        self.xml_command("power", &[("value", "reboot")])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockLink;
    use std::cell::RefCell;

    fn sahara_packet(cmd: u32, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&cmd.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn sahara_roundtrip_with_mock() {
        let mut mock = MockLink::new();
        let programmer: Vec<u8> = (0..5000).map(|i| (i % 253) as u8).collect();
        let prog_len = programmer.len();
        let state = RefCell::new(0u8);
        mock.device = Some(Box::new(move |out, input| {
            let mut st = state.borrow_mut();
            let cmd = if input.len() >= 4 {
                u32::from_le_bytes(input[0..4].try_into().unwrap())
            } else {
                0
            };
            match (*st, cmd, input.len()) {
                (0, SAHARA_HELLO_RESP, 48) => {
                    // request the whole programmer image
                    let mut body = Vec::new();
                    body.extend_from_slice(&0u32.to_le_bytes());
                    body.extend_from_slice(&0u32.to_le_bytes());
                    body.extend_from_slice(&(prog_len as u32).to_le_bytes());
                    out.extend(sahara_packet(SAHARA_READ_DATA, &body));
                    *st = 1;
                }
                (1, _, n) if n > 8 => {
                    // the raw image chunk arrived; end the image
                    let mut eoi = Vec::new();
                    eoi.extend_from_slice(&0u32.to_le_bytes());
                    eoi.extend_from_slice(&0u32.to_le_bytes());
                    out.extend(sahara_packet(SAHARA_END_OF_IMAGE, &eoi));
                    out.extend(sahara_packet(SAHARA_DONE, &[]));
                    *st = 2;
                }
                (2, SAHARA_DONE_RESP, _) => {
                    // a fresh HELLO for the next handshake round
                    let mut hello = Vec::new();
                    hello.extend_from_slice(&2u32.to_le_bytes());
                    hello.extend_from_slice(&2u32.to_le_bytes());
                    hello.extend_from_slice(&SAHARA_MODE_COMMAND.to_le_bytes());
                    hello.extend_from_slice(&[0u8; 24]);
                    out.extend(sahara_packet(SAHARA_HELLO, &hello));
                    *st = 3;
                }
                (3, SAHARA_HELLO_RESP, 48) => {
                    // switch mode request follows the second hello resp
                    *st = 4;
                }
                (4, SAHARA_SWITCH_MODE, _) => {
                    out.extend(sahara_packet(SAHARA_COMMAND_READY, &[]));
                    *st = 5;
                }
                _ => {}
            }
        }));
        // the device announces itself with HELLO
        let mut hello = Vec::new();
        hello.extend_from_slice(&2u32.to_le_bytes());
        hello.extend_from_slice(&2u32.to_le_bytes());
        hello.extend_from_slice(&SAHARA_MODE_IMAGE_TX_PENDING.to_le_bytes());
        hello.extend_from_slice(&[0u8; 24]);
        mock.push(&sahara_packet(SAHARA_HELLO, &hello));
        let mut sahara = Sahara::new(Box::new(mock));
        sahara
            .send_image(SAHARA_MODE_IMAGE_TX_PENDING, &[programmer])
            .unwrap();
        // the mock queues the second HELLO when the DONE_RESP is read:
        // it answers COMMAND_READY directly for the switch packet
        sahara.switch_to_command_mode().unwrap();
    }

    #[test]
    fn firehose_program_and_read_roundtrip() {
        let sector = 512usize;
        let mut mock = MockLink::new();
        let data: Vec<u8> = (0..2048).map(|i| (i % 249) as u8).collect();
        let data_ref = data.clone();
        let data_ref2 = data.clone();
        let state = RefCell::new(0usize);
        mock.device = Some(Box::new(move |out, input| {
            let mut st = state.borrow_mut();
            let text = String::from_utf8_lossy(input);
            if text.contains("<program") {
                out.extend(ack_xml("program"));
                *st = 1;
            } else if *st == 1 {
                // host streams the image; answer the final ACK after it
                if input.is_empty() {
                    out.extend(ack_xml("program end"));
                    *st = 0;
                }
            } else if text.contains("<read") {
                out.extend(ack_xml("read"));
                out.extend(&data_ref2);
                out.extend(ack_xml("read end"));
            } else if text.contains("<configure")
                || text.contains("<nop")
                || text.contains("<power")
            {
                out.extend(ack_xml("cmd"));
            }
        }));
        let mut fh = Firehose::new(Box::new(mock));
        fh.configure("ufs").unwrap();
        fh.program(0, "0", &data).unwrap();
        let got = fh.read(0, "0", 4).unwrap();
        assert_eq!(got, data_ref);
        fh.reset().unwrap();
        let _ = sector;
    }

    fn ack_xml(tag: &'static str) -> Vec<u8> {
        let xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" ?><data><response value=\"ACK\" rawstatus=\"false\" status=\"OK\" tag=\"{tag}\"/></data>"
        );
        let mut out = xml.into_bytes();
        out.push(0);
        out
    }
}
